pub mod files;
pub mod input;
pub mod transport;
use input::Draft;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    future::Future,
    path::Path,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    task::{Context, Poll, Waker},
};
use tezbar_extension_sdk::{
    BoxFuture, CommandConcurrency, CoreResult, Extension, ExtensionContext,
};
use transport::{Failure, Transport, Upload};
pub const ID: &str = "com.tezbar.native.image-to-ascii";
const COMMAND: &str = "convert-image-to-ascii";
const MAX_SESSIONS: usize = 16;
const UNKNOWN: &str = "Could not confirm upload; outcome unknown. Retry Previous Upload sends the frozen image and options again and may repeat service work. A new submit uploads the current form.";
const COPY_UNKNOWN: &str = "Could not confirm copy. Outcome unknown. Retry Copy Exact Ascii resends the same text without another upload.";
#[derive(Default)]
pub struct Adapter {
    sessions: Mutex<HashMap<String, Session>>,
    sequence: AtomicU64,
    #[cfg(test)]
    transport: Option<Transport>,
}
struct Session {
    opened: ExtensionContext,
    form_token: u64,
    view_token: u64,
    field_revision: u64,
    draft: Draft,
    result: Option<String>,
    frozen: Option<Upload>,
    upload_unknown: bool,
    copy_unknown: bool,
    notice: String,
}
fn cancelled(ctx: &ExtensionContext) -> bool {
    let mut task = Context::from_waker(Waker::noop());
    matches!(
        Box::pin(ctx.cancelled()).as_mut().poll(&mut task),
        Poll::Ready(())
    )
}
fn lock<T>(m: &Mutex<T>) -> Result<std::sync::MutexGuard<'_, T>, String> {
    m.lock().map_err(|_| "Session unavailable".into())
}
fn node(kind: &str, props: Value) -> Value {
    json!({"type":kind,"props":props})
}
fn action(id: String, title: &str, kind: &str) -> Value {
    json!({"id":id,"title":title,"kind":kind})
}
fn render(s: &Session) -> Value {
    let (root, actions) = if let Some(text) = &s.result {
        (
            json!({"type":"Detail","props":{"markdown":input::fence(text),"literalText":text,"backActionId":format!("d:{}:back",s.view_token)},"metadata":{"type":"Detail.Metadata","children":[node("Detail.Metadata.Label",json!({"title":"Status","text":s.notice}))]}}),
            vec![
                action(
                    format!("d:{}:copy", s.view_token),
                    if s.copy_unknown {
                        "Retry Copy Exact Ascii"
                    } else {
                        "Copy Ascii Image"
                    },
                    "action",
                ),
                action(format!("d:{}:back", s.view_token), "Back", "action"),
            ],
        )
    } else {
        let change = format!("f:{}:change", s.form_token);
        let mut fields = vec![
            node(
                "Form.FilePicker",
                json!({"id":"image","title":"Select an Image","value":s.draft.image,"allowMultipleSelection":false,"canChooseDirectories":false,"canChooseFiles":true,"nativeAbsolutePath":true,"actionId":change}),
            ),
            node(
                "Form.TextField",
                json!({"id":"width","title":"Width","value":s.draft.width,"actionId":change}),
            ),
            node(
                "Form.TextField",
                json!({"id":"height","title":"Height","value":s.draft.height,"actionId":change}),
            ),
            node(
                "Form.Description",
                json!({"title":"Note","text":"If only width or height is provided, the other dimension will be calculated to maintain the aspect ratio."}),
            ),
            node(
                "Form.Checkbox",
                json!({"id":"complex","title":"Complex","label":"Convert the image with a wider array of ascii characters.","value":s.draft.complex,"actionId":change}),
            ),
            node(
                "Form.Description",
                json!({"title":"Upload","text":"Submit uploads this image to ascii-image-converter.zeabur.app. Choose an image you want to send to that service."}),
            ),
        ];
        if !s.notice.is_empty() {
            fields.push(node(
                "Form.Description",
                json!({"title":"Status","text":s.notice}),
            ));
        }
        let mut actions = vec![action(
            format!("f:{}:{}:submit", s.form_token, s.field_revision),
            "Convert to Ascii",
            "submit-form",
        )];
        if s.upload_unknown && s.frozen.is_some() {
            actions.push(action(
                format!("f:{}:{}:retry", s.form_token, s.field_revision),
                "Retry Previous Upload",
                "action",
            ));
        }
        (
            json!({"type":"Form","props":{"formToken":s.form_token.to_string(),"formRevision":s.field_revision},"children":fields}),
            actions,
        )
    };
    let mut out = tezbar_extension_sdk::view("Convert Image to Ascii", root, actions);
    out["refreshMs"] = Value::Null;
    if !s.notice.is_empty() {
        out["message"] = json!(if s.result.is_none() {
            format!("Error: {}", s.notice)
        } else {
            s.notice.clone()
        });
    }
    out
}
fn fields(s: &mut Session, payload: &Value, change: bool) -> Result<bool, String> {
    let mut v = payload["formValues"]
        .as_object()
        .ok_or("All current fields are required.")?
        .clone();
    if v.remove("__formToken").as_ref().and_then(Value::as_str) != Some(&s.form_token.to_string()) {
        return Ok(false);
    }
    let rev = v
        .remove("__formRevision")
        .and_then(|v| v.as_u64())
        .filter(|n| *n <= 9_007_199_254_740_991)
        .ok_or("All-field revision required.")?;
    if rev < s.field_revision || (change && rev == s.field_revision) {
        return Ok(false);
    }
    let draft: Draft = serde_json::from_value(Value::Object(v))
        .map_err(|_| "All four typed fields are required.")?;
    if !draft.budget() {
        return Err(
            "Form budget exceeded (one file, path 4096 bytes, dimensions 256 bytes).".into(),
        );
    }
    if rev == s.field_revision && draft != s.draft {
        return Ok(false);
    }
    s.field_revision = rev;
    s.draft = draft;
    Ok(true)
}
pub fn strict_ack(v: &Value) -> bool {
    v.as_object().is_some_and(|o| {
        o.get("ok") == Some(&Value::Bool(true))
            && o.keys().all(|k| k == "ok" || k == "message")
            && o.get("message").is_none_or(Value::is_string)
    })
}
fn prepare(d: &Draft) -> Result<Upload, String> {
    // Match source validation order: empty -> file metadata -> suffix -> width -> height.
    let p = d.image.first().ok_or("Please select an image file.")?;
    let path = Path::new(p);
    files::ordinary_path(path, false).map_err(|_| {
        "Selected file is not valid. Select an ordinary absolute file without symlinks."
    })?;
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Selected filename must be UTF-8.")?;
    if name.len() > 512
        || name
            .chars()
            .any(|c| c.is_control() || c == '"' || c == '\\')
    {
        return Err("Selected filename must be at most 512 bytes without control characters, quotes or backslashes.".into());
    }
    if !input::allowed(name) || name.starts_with('.') && !name[1..].contains('.') {
        return Err(
            "Images must be in one of the following formats: .jpeg, .jpg, .png, .bmp, .webp".into(),
        );
    }
    let width = input::dimension(&d.width, "Width")?;
    let height = input::dimension(&d.height, "Height")?;
    let bytes = files::read_selected(path)?;
    Ok(Upload {
        bytes,
        filename: name.into(),
        complex: d.complex,
        width,
        height,
    })
}
enum Work {
    Upload(Upload, u64),
    Copy(String, u64),
}
impl Adapter {
    fn next(&self) -> u64 {
        self.sequence.fetch_add(1, Ordering::Relaxed) + 1
    }
    fn rotate_form(&self, s: &mut Session) {
        s.form_token = self.next();
        s.view_token = s.form_token;
        s.field_revision = 0;
    }
    fn transport(&self) -> Result<Transport, String> {
        #[cfg(test)]
        if let Some(t) = &self.transport {
            return Ok(t.clone());
        }
        Transport::production()
    }
}
impl Extension for Adapter {
    fn id(&self) -> &'static str {
        ID
    }
    fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }
    fn commands(&self) -> &'static [&'static str] {
        &[COMMAND]
    }
    fn concurrency(&self, _: &str) -> CommandConcurrency {
        CommandConcurrency::RejectWhileBusy
    }
    fn shutdown<'a>(&'a self, _: &'a ExtensionContext) -> BoxFuture<'a, CoreResult> {
        Box::pin(async move {
            lock(&self.sessions)?.clear();
            Ok(json!({}))
        })
    }
    fn handle<'a>(
        &'a self,
        ctx: &'a ExtensionContext,
        command: &'a str,
        operation: &'a str,
        payload: &'a Value,
        _state: &'a mut Value,
    ) -> BoxFuture<'a, CoreResult> {
        Box::pin(async move {
            let sid = payload["sessionId"]
                .as_str()
                .filter(|s| s.len() <= 256)
                .ok_or("Invalid session ID")?
                .to_owned();
            if operation == "extension:dispose-session" {
                lock(&self.sessions)?.remove(&sid);
                return Ok(json!({}));
            }
            if cancelled(ctx) {
                return Err("Operation cancelled".into());
            }
            if command != COMMAND {
                return Err("Unknown command".into());
            }
            if operation == "extension:run-command" {
                let token = self.next();
                {
                    let mut sessions = lock(&self.sessions)?;
                    // An open can be canceled before the SDK stages its session.
                    sessions.retain(|_, s| !cancelled(&s.opened));
                    if sessions.len() >= MAX_SESSIONS {
                        return Err("At most 16 image converter sessions may be open.".into());
                    }
                    sessions.insert(
                        sid.clone(),
                        Session {
                            opened: ctx.clone(),
                            form_token: token,
                            view_token: token,
                            field_revision: 0,
                            draft: Draft::default(),
                            result: None,
                            frozen: None,
                            upload_unknown: false,
                            copy_unknown: false,
                            notice: String::new(),
                        },
                    );
                }
                let selected = ctx
                    .host("host.selection.read", json!({"kind":"files"}))
                    .await;
                if cancelled(ctx) {
                    lock(&self.sessions)?.remove(&sid);
                    return Err("Selection cancelled".into());
                }
                let mut sessions = lock(&self.sessions)?;
                let s = sessions.get_mut(&sid).ok_or("Session disposed")?;
                if s.form_token != token {
                    return Ok(tezbar_extension_sdk::unchanged());
                }
                s.draft.image =
                    serde_json::from_value(input::prefill(&selected.unwrap_or(Value::Null)))
                        .unwrap_or_default();
                return Ok(render(s));
            }
            let work = {
                let mut sessions = lock(&self.sessions)?;
                sessions.retain(|_, s| !cancelled(&s.opened));
                let s = sessions
                    .get_mut(&sid)
                    .ok_or("Session expired; reopen the converter.")?;
                if operation == "extension:refresh-session" {
                    return Ok(render(s));
                }
                if operation != "extension:invoke-action" {
                    return Err("Unsupported operation".into());
                }
                let id = payload["actionId"]
                    .as_str()
                    .filter(|s| s.len() <= 96)
                    .ok_or("Invalid action ID")?;
                if s.result.is_none() {
                    let change = id == format!("f:{}:change", s.form_token);
                    let submit = id == format!("f:{}:{}:submit", s.form_token, s.field_revision);
                    let retry = id == format!("f:{}:{}:retry", s.form_token, s.field_revision)
                        && s.upload_unknown
                        && s.frozen.is_some();
                    if !change && !submit && !retry {
                        return Ok(tezbar_extension_sdk::unchanged());
                    }
                    if retry
                        && payload.get("formValues").is_some_and(|v| {
                            !v.is_null() && v.as_object().is_none_or(|v| !v.is_empty())
                        })
                    {
                        let mut expected = serde_json::to_value(&s.draft).unwrap();
                        expected["__formToken"] = json!(s.form_token.to_string());
                        expected["__formRevision"] = json!(s.field_revision);
                        if payload["formValues"] != expected {
                            return Ok(tezbar_extension_sdk::unchanged());
                        }
                    }
                    if !retry {
                        match fields(s, payload, change) {
                            Ok(true) => {}
                            Ok(false) => return Ok(tezbar_extension_sdk::unchanged()),
                            Err(e) => {
                                s.notice = e;
                                self.rotate_form(s);
                                return Ok(render(s));
                            }
                        }
                    }
                    if change {
                        s.frozen = None;
                        s.upload_unknown = false;
                        s.notice.clear();
                        return Ok(render(s));
                    }
                    let upload = if retry {
                        s.frozen.as_ref().unwrap().clone()
                    } else {
                        match prepare(&s.draft) {
                            Ok(u) => u,
                            Err(e) => {
                                s.notice = e;
                                s.frozen = None;
                                s.upload_unknown = false;
                                self.rotate_form(s);
                                return Ok(render(s));
                            }
                        }
                    };
                    // The checkpoint is authoritative outside SDK staged state, before await.
                    self.rotate_form(s);
                    s.upload_unknown = true;
                    s.notice = UNKNOWN.into();
                    s.frozen = Some(upload.clone());
                    Work::Upload(upload, s.view_token)
                } else {
                    let prefix = format!("d:{}:", s.view_token);
                    let Some(kind) = id.strip_prefix(&prefix) else {
                        return Ok(tezbar_extension_sdk::unchanged());
                    };
                    if payload.get("formValues").is_some_and(|v| {
                        !v.is_null() && v.as_object().is_none_or(|v| !v.is_empty())
                    }) {
                        return Ok(tezbar_extension_sdk::unchanged());
                    }
                    match kind {
                        "back" => {
                            s.result = None;
                            s.copy_unknown = false;
                            s.notice.clear();
                            self.rotate_form(s);
                            return Ok(render(s));
                        }
                        "copy" => {
                            s.view_token = self.next();
                            s.copy_unknown = true;
                            s.notice = COPY_UNKNOWN.into();
                            Work::Copy(s.result.as_ref().unwrap().clone(), s.view_token)
                        }
                        _ => return Ok(tezbar_extension_sdk::unchanged()),
                    }
                }
            };
            let token = match &work {
                Work::Upload(_, t) | Work::Copy(_, t) => *t,
            };
            let outcome = match work {
                Work::Upload(upload, _) => {
                    let transport = self.transport();
                    let result = match transport {
                        Ok(t) => {
                            tokio::select! {biased;_=ctx.cancelled()=>return Err("Upload cancelled; outcome unknown. Refresh for an explicit frozen-byte retry.".into()),r=t.send(&upload)=>r}
                        }
                        Err(_) => Err(Failure::Known("Could not prepare secure upload.")),
                    };
                    match result {
                        Ok(text) => Ok(Some(text)),
                        Err(Failure::Known(m)) => Err(Some(m)),
                        Err(Failure::Unknown) => Err(None),
                    }
                }
                Work::Copy(text, _) => {
                    let reply = ctx.host("host.clipboard.write", json!({"text":text})).await;
                    if reply.as_ref().is_ok_and(strict_ack) {
                        Ok(None)
                    } else {
                        Err(None)
                    }
                }
            };
            if cancelled(ctx) {
                return Err(
                    "Operation cancelled; outcome unknown. Refresh to retry explicitly.".into(),
                );
            }
            let mut sessions = lock(&self.sessions)?;
            let s = sessions.get_mut(&sid).ok_or("Session disposed")?;
            if s.view_token != token {
                return Ok(tezbar_extension_sdk::unchanged());
            }
            if s.result.is_some() {
                if outcome.is_ok() {
                    s.copy_unknown = false;
                    s.notice = "Ascii image copied to clipboard".into();
                }
            } else {
                match outcome {
                    Ok(Some(text)) => {
                        s.result = Some(text);
                        s.frozen = None;
                        s.upload_unknown = false;
                        s.notice = "Image converted successfully".into();
                    }
                    Err(Some(m)) => {
                        s.frozen = None;
                        s.upload_unknown = false;
                        s.notice = m.into();
                    }
                    _ => {}
                }
            }
            Ok(render(s))
        })
    }
}
#[cfg(test)]
mod tests;
