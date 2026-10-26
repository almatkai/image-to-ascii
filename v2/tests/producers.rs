// GENERATED qa/producers.py. Only tauri command attribute removed; actual
// native picker body and clipboard:write producer run against inert OS mocks.
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
mod tauri {
    #[derive(Clone)]
    pub struct AppHandle {
        pub path: Option<std::path::PathBuf>,
        pub calls: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    }
}
mod tauri_plugin_dialog {
    use super::tauri::AppHandle;
    pub trait DialogExt {
        fn dialog(&self) -> AppHandle;
    }
    impl DialogExt for AppHandle {
        fn dialog(&self) -> AppHandle {
            self.clone()
        }
    }
    pub struct FilePath(pub std::path::PathBuf);
    impl FilePath {
        pub fn into_path(self) -> Result<std::path::PathBuf, String> {
            Ok(self.0)
        }
    }
    impl AppHandle {
        pub fn file(self) -> Self {
            self
        }
        pub fn pick_file(self, callback: impl FnOnce(Option<FilePath>)) {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            callback(self.path.map(FilePath))
        }
    }
}
// Generic explicit single-file selection. No extension-specific route or scan.
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;
use tokio::sync::oneshot;

pub async fn pick_extension_file(app: AppHandle) -> Result<Option<String>, String> {
    let (sender, receiver) = oneshot::channel();
    app.dialog().file().pick_file(move |path| {
        let _ = sender.send(path);
    });
    let Some(path) = receiver.await.map_err(|_| "Could not open file dialog")? else {
        return Ok(None);
    };
    let path = path
        .into_path()
        .map_err(|_| "Could not resolve selected file")?;
    if !path.is_absolute()
        || !std::fs::symlink_metadata(&path)
            .map_err(|e| e.to_string())?
            .is_file()
    {
        return Err("Select one ordinary absolute file".into());
    }
    let path = path.to_str().ok_or("Selected path must be UTF-8")?;
    if path.len() > 4096 {
        return Err("Selected path is too long".into());
    }
    Ok(Some(path.into()))
}

pub fn text_arg(args: &[Value], n: usize) -> Result<&str, String> {
    args.get(n)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Argument {} must be a string", n + 1))
}

#[derive(Default)]
struct Clipboard {
    writes: Vec<String>,
    fail: bool,
}
impl Clipboard {
    fn write_text(&mut self, text: String) -> Result<(), String> {
        if self.fail {
            return Err("inert permission error".into());
        }
        self.writes.push(text);
        Ok(())
    }
    #[allow(clippy::needless_return)] // Frozen host branch copied verbatim.
    fn handle(&mut self, c: &str, a: &[Value]) -> Result<Value, String> {
        match c {
            "clipboard:write" => {
                self.write_text(text_arg(a, 0)?.into())?;
                return Ok(json!({"ok":true}));
            }
            _ => panic!("unexpected clipboard producer"),
        }
    }
}
fn owned() -> tempfile::TempDir {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(".qa/producers");
    std::fs::create_dir_all(&p).unwrap();
    tempfile::tempdir_in(p).unwrap()
}
#[tokio::test]
async fn native_picker_explicit_cancel_relative_directory_symlink() {
    let d = owned();
    let p = d.path().join("Selected.JPG");
    std::fs::write(&p, b"inert").unwrap();
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    for path in [None, Some(p.clone())] {
        let got = pick_extension_file(tauri::AppHandle {
            path: path.clone(),
            calls: calls.clone(),
        })
        .await
        .unwrap();
        assert_eq!(got, path.map(|p| p.to_str().unwrap().to_owned()));
    }
    for path in [
        PathBuf::from("relative"),
        d.path().into(),
        d.path().join("missing"),
    ] {
        assert!(pick_extension_file(tauri::AppHandle {
            path: Some(path),
            calls: calls.clone()
        })
        .await
        .is_err());
    }
    #[cfg(unix)]
    {
        let link = d.path().join("link");
        std::os::unix::fs::symlink(&p, &link).unwrap();
        assert!(pick_extension_file(tauri::AppHandle {
            path: Some(link),
            calls: calls.clone()
        })
        .await
        .is_err());
    }
    assert!(calls.load(std::sync::atomic::Ordering::SeqCst) >= 5);
}
#[test]
fn actual_clipboard_producer_closed_ack_and_error() {
    let mut c = Clipboard::default();
    let text = "data:image/JPEG;base64,SGVsbG8=";
    let ack = c.handle("clipboard:write", &[json!(text)]).unwrap();
    assert!(image_to_ascii_native::strict_ack(&ack));
    assert_eq!(c.writes, [text]);
    assert!(c.handle("clipboard:write", &[json!(null)]).is_err());
    c.fail = true;
    assert!(c.handle("clipboard:write", &[json!(text)]).is_err());
    assert_eq!(c.writes.len(), 1);
}

mod selection {
    use super::*;
    use std::time::Duration;
    type CoreResult = Result<Value, String>;
    async fn selected_files() -> CoreResult {
        #[cfg(target_os = "macos")]
        let mut command = {
            let mut cmd = tokio::process::Command::new("/usr/bin/osascript");
            cmd.args(["-l","JavaScript","-e",r#"JSON.stringify(Application('Finder').selection().slice(0,128).map(function(item){return item.url();}))"#]);
            cmd
        };
        #[cfg(target_os = "windows")]
        let mut command = {
            let mut cmd = tokio::process::Command::new("powershell.exe");
            cmd.args(["-NoProfile","-NonInteractive","-Command",r#"$ErrorActionPreference='Stop';Add-Type -TypeDefinition 'using System;using System.Runtime.InteropServices;public static class TezbarSelectionWindow{[DllImport("user32.dll")]public static extern IntPtr GetForegroundWindow();}';$foreground=[TezbarSelectionWindow]::GetForegroundWindow().ToInt64();$paths=@();$shell=New-Object -ComObject Shell.Application;foreach($window in $shell.Windows()){if([long]$window.HWND -eq $foreground){$paths=@($window.Document.SelectedItems() | Select-Object -First 128 | ForEach-Object {$_.Path});break}};ConvertTo-Json -InputObject @($paths) -Compress"#]).creation_flags(0x08000000);
            cmd
        };
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        return Err("File selection is unsupported on this platform".into());
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        {
            command
                .stdin(std::process::Stdio::null())
                .kill_on_drop(true);
            let output = tokio::time::timeout(Duration::from_secs(10), command.output())
                .await
                .map_err(|_| "File selection timed out")?
                .map_err(|e| e.to_string())?;
            if !output.status.success() {
                return Err("Cannot read selected files from Finder/Explorer".into());
            }
            if output.stdout.len() > 1024 * 1024 {
                return Err("File selection response too large".into());
            }
            let values: Vec<String> = serde_json::from_slice(&output.stdout)
                .map_err(|_| "Invalid file selection response")?;
            if values.len() > 128 {
                return Err("Too many selected files".into());
            }
            let mut files = Vec::new();
            for value in values {
                #[cfg(target_os = "macos")]
                let path = reqwest::Url::parse(&value)
                    .map_err(|e| e.to_string())?
                    .to_file_path()
                    .map_err(|_| "Selection URL is not a local file")?;
                #[cfg(target_os = "windows")]
                let path = PathBuf::from(value);
                if path.is_absolute() && path.is_file() {
                    files.push(path);
                }
            }
            Ok(json!(files))
        }
    }
    mod tokio {
        pub use ::tokio::*;
        pub mod process {
            use std::{collections::VecDeque, ffi::OsStr, sync::Mutex};
            pub static OUTPUT: Mutex<VecDeque<(bool, Vec<u8>)>> = Mutex::new(VecDeque::new());
            pub static CALLS: Mutex<Vec<Vec<String>>> = Mutex::new(Vec::new());
            pub struct Command {
                args: Vec<String>,
            }
            pub struct Status(bool);
            impl Status {
                pub fn success(&self) -> bool {
                    self.0
                }
            }
            pub struct Output {
                pub status: Status,
                pub stdout: Vec<u8>,
            }
            impl Command {
                pub fn new(s: impl AsRef<OsStr>) -> Self {
                    Self {
                        args: vec![s.as_ref().to_string_lossy().into()],
                    }
                }
                pub fn args<I, S>(&mut self, a: I) -> &mut Self
                where
                    I: IntoIterator<Item = S>,
                    S: AsRef<OsStr>,
                {
                    self.args.extend(
                        a.into_iter()
                            .map(|s| s.as_ref().to_string_lossy().into_owned()),
                    );
                    self
                }
                pub fn stdin(&mut self, _: std::process::Stdio) -> &mut Self {
                    self
                }
                pub fn kill_on_drop(&mut self, _: bool) -> &mut Self {
                    self
                }
                #[cfg(target_os = "windows")]
                pub fn creation_flags(&mut self, _: u32) -> &mut Self {
                    self
                }
                pub async fn output(&mut self) -> Result<Output, std::io::Error> {
                    CALLS.lock().unwrap().push(self.args.clone());
                    let (ok, stdout) = OUTPUT
                        .lock()
                        .unwrap()
                        .pop_front()
                        .expect("Unmocked selected producer denied");
                    Ok(Output {
                        status: Status(ok),
                        stdout,
                    })
                }
            }
        }
    }
    #[::tokio::test]
    async fn actual_selection_producer_uses_owned_files_only() {
        let d = owned();
        let image = d.path().join("Selected Image.JPG");
        std::fs::write(&image, b"inert").unwrap();
        let value = if cfg!(target_os = "macos") {
            reqwest::Url::from_file_path(&image).unwrap().to_string()
        } else {
            image.to_str().unwrap().into()
        };
        {
            let mut answers = tokio::process::OUTPUT.lock().unwrap();
            answers.push_back((true, serde_json::to_vec(&vec![value]).unwrap()));
            answers.push_back((false, b"private OS error".to_vec()));
            answers.push_back((true, b"bad-json".to_vec()));
            answers.push_back((true, vec![b'a'; 1024 * 1024 + 1]));
            answers.push_back((true, serde_json::to_vec(&vec!["invalid"; 129]).unwrap()));
            drop(answers);
        }
        assert_eq!(selected_files().await.unwrap(), json!([image]));
        for _ in 0..4 {
            assert!(selected_files().await.is_err());
        }
        let calls = tokio::process::CALLS.lock().unwrap();
        assert_eq!(calls.len(), 5);
        #[cfg(target_os = "macos")]
        assert!(calls[0]
            .iter()
            .any(|s| s.contains("Application('Finder').selection()")));
        #[cfg(target_os = "windows")]
        assert!(calls[0].iter().any(|s| s.contains("GetForegroundWindow")));
    }
}
