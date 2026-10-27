use super::*;
use futures_util::stream;
use std::{io::BufReader, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::mpsc,
};

fn owned() -> tempfile::TempDir {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(".qa/unit");
    std::fs::create_dir_all(&p).unwrap();
    tempfile::tempdir_in(p).unwrap()
}
#[test]
fn original_numeric_oracle_and_precise_exclusions() {
    let cases: Value = serde_json::from_str(include_str!("../qa/oracle-cases.json")).unwrap();
    for c in cases["numeric"].as_array().unwrap() {
        let text = c["text"].as_str().unwrap();
        let got = input::dimension(text, "Width");
        match c["nativeExpected"].as_str() {
            Some("resource-rejected") => assert_eq!(
                got.unwrap_err(),
                "Width must be finite and at most 10,000 for this upload.",
                "{text}"
            ),
            Some("invalid") => assert_eq!(
                got.unwrap_err(),
                "Width must be a positive number.",
                "{text}"
            ),
            _ => assert_eq!(
                got.unwrap(),
                c["wire"].as_str().map(str::to_owned),
                "{text}"
            ),
        }
    }
    for bad in ["0xg", "-0x1", "0b2", "1e", "+", "1 2", "\u{0085}2"] {
        assert!(input::number(bad).is_nan());
    }
}
#[test]
fn selection_and_response_oracle() {
    assert_eq!(
        input::prefill(&json!(["/owned/A.JPEG"])),
        json!(["/owned/A.JPEG"])
    );
    for v in [
        Value::Null,
        json!([]),
        json!(["/owned/a.png", "/owned/b.png"]),
        json!(["relative.jpg"]),
        json!(["/owned/a.gif"]),
        json!({"files":["/owned/a.png"]}),
        json!([{"path":"/owned/a.png"}]),
    ] {
        assert_eq!(input::prefill(&v), json!([]));
    }
    let cases: Value = serde_json::from_str(include_str!("../qa/oracle-cases.json")).unwrap();
    for c in cases["responses"].as_array().unwrap() {
        let got = input::decode(c["body"].as_str().unwrap().as_bytes());
        if c["nativeExpected"] == "unicode-rejected" {
            assert_eq!(
                got.unwrap_err(),
                "Service returned text with unsupported unpaired Unicode surrogates."
            );
        } else if c["type"] == "string" {
            assert_eq!(got.unwrap(), c["result"].as_str().unwrap());
        } else {
            assert_eq!(got.unwrap_err(), "Service returned JSON that is not text.");
        }
    }
    assert_eq!(
        input::decode(&[255]).unwrap_err(),
        "Service response is not valid UTF-8 text."
    );
    assert!(input::decode(&vec![b'a'; input::MAX_RESULT + 1]).is_err());
    let text = "```\n~~~\n<img src=x onerror=evil>\n![x](https://blocked.invalid)\n\u{202e}";
    assert_eq!(input::fence(text), format!("````\n{text}\n````"));
}
#[test]
fn explicit_files_and_validation_order() {
    let dir = owned();
    let path = dir.path().join("Photo.JpG");
    std::fs::write(&path, b"fake-image\0").unwrap();
    let mut d = Draft {
        image: vec![path.to_str().unwrap().into()],
        ..Draft::default()
    };
    assert_eq!(prepare(&d).unwrap().bytes, b"fake-image\0");
    d.width = "NaN".into();
    d.image = vec![];
    assert_eq!(prepare(&d).err().unwrap(), "Please select an image file.");
    d.image = vec![dir.path().join("missing.png").to_str().unwrap().into()];
    assert!(prepare(&d)
        .err()
        .unwrap()
        .starts_with("Selected file is not valid"));
    let gif = dir.path().join("a.gif");
    std::fs::write(&gif, b"fake").unwrap();
    d.image = vec![gif.to_str().unwrap().into()];
    assert!(prepare(&d).err().unwrap().starts_with("Images must"));
    d.image = vec![path.to_str().unwrap().into()];
    assert_eq!(
        prepare(&d).err().unwrap(),
        "Width must be a positive number."
    );
    d.width = "80".into();
    d.height = "0".into();
    assert_eq!(
        prepare(&d).err().unwrap(),
        "Height must be a positive number."
    );
    for p in [
        dir.path(),
        Path::new("relative.jpg"),
        Path::new("/dev/null"),
    ] {
        assert!(files::read_selected(p).is_err());
    }
    #[cfg(unix)]
    {
        let link = dir.path().join("link.jpg");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert!(files::read_selected(&link).is_err());
        let link_dir = dir.path().join("linked");
        std::os::unix::fs::symlink(dir.path(), &link_dir).unwrap();
        assert!(files::read_selected(&link_dir.join("Photo.JpG")).is_err());
        use std::os::unix::ffi::OsStrExt;
        let fifo = dir.path().join("pipe.jpg");
        let c = std::ffi::CString::new(fifo.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o600) }, 0);
        assert!(files::read_selected(&fifo).is_err());
    }
    let big = dir.path().join("big.png");
    std::fs::File::create(&big)
        .unwrap()
        .set_len(input::MAX_FILE as u64 + 1)
        .unwrap();
    assert!(files::read_selected(&big).is_err());
}
#[test]
fn clipboard_closed_ack() {
    for v in [json!({"ok":true}), json!({"ok":true,"message":"done"})] {
        assert!(strict_ack(&v));
    }
    for v in [
        Value::Null,
        json!(true),
        json!(false),
        json!({}),
        json!({"ok":false}),
        json!({"ok":true,"error":"bad"}),
        json!({"ok":true,"message":null}),
        json!({"ok":true,"unknown":1}),
    ] {
        assert!(!strict_ack(&v));
    }
}

struct OwnedServer {
    endpoint: String,
    seen: mpsc::Receiver<Value>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for OwnedServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl OwnedServer {
    async fn new(response: Vec<u8>, hold: bool) -> Self {
        let certs = rustls_pemfile::certs(&mut BufReader::new(
            include_bytes!("../qa/fixtures/tls/server.pem").as_slice(),
        ))
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
        let key = rustls_pemfile::private_key(&mut BufReader::new(
            include_bytes!("../qa/fixtures/tls/server.key").as_slice(),
        ))
        .unwrap()
        .unwrap();
        let config = tokio_rustls::rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .unwrap();
        let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!(
            "https://localhost:{}/image-to-ascii",
            listener.local_addr().unwrap().port()
        );
        let (tx, seen) = mpsc::channel(32);
        let task = tokio::spawn(async move {
            loop {
                let (tcp, _) = listener.accept().await.unwrap();
                let Ok(mut tls) = acceptor.accept(tcp).await else {
                    continue;
                };
                let mut bytes = vec![];
                let mut chunk = [0; 8192];
                let header_end;
                loop {
                    let Ok(n) = tls.read(&mut chunk).await else {
                        return;
                    };
                    if n == 0 {
                        return;
                    }
                    bytes.extend_from_slice(&chunk[..n]);
                    if let Some(i) = bytes.windows(4).position(|s| s == b"\r\n\r\n") {
                        header_end = i + 4;
                        break;
                    }
                    if bytes.len() > 16384 {
                        return;
                    }
                }
                let headers = String::from_utf8(bytes[..header_end].to_vec()).unwrap();
                let len = headers
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length: ")
                            .and_then(|s| s.parse::<usize>().ok())
                    })
                    .unwrap();
                assert!(len < input::MAX_FILE + 16384);
                while bytes.len() - header_end < len {
                    let n = tls.read(&mut chunk).await.unwrap_or(0);
                    if n == 0 {
                        return;
                    }
                    bytes.extend_from_slice(&chunk[..n]);
                }
                let ct = headers
                    .lines()
                    .find_map(|l| l.strip_prefix("content-type: "))
                    .unwrap();
                let boundary = multer::parse_boundary(ct).unwrap();
                let body = bytes[header_end..].to_vec();
                let mut multipart = multer::Multipart::new(
                    stream::once(async { Ok::<_, std::io::Error>(body) }),
                    boundary,
                );
                let mut parts = vec![];
                while let Some(part) = multipart.next_field().await.unwrap() {
                    let name = part.name().unwrap().to_owned();
                    let filename = part.file_name().map(str::to_owned);
                    let mime = part.content_type().map(ToString::to_string);
                    let b = part.bytes().await.unwrap();
                    parts.push(
                        json!({"name":name,"filename":filename,"mime":mime,"bytes":b.to_vec()}),
                    );
                }
                tx.send(json!({"headers":headers,"parts":parts}))
                    .await
                    .unwrap();
                if hold {
                    let _ = tls.read(&mut chunk).await;
                    continue;
                }
                let _ = tls.write_all(&response).await;
                let _ = tls.shutdown().await;
            }
        });
        Self {
            endpoint,
            seen,
            task,
        }
    }
    fn transport(&self, deadline: u64) -> Transport {
        Transport::owned(
            self.endpoint.clone(),
            include_bytes!("../qa/fixtures/tls/ca.pem"),
            Duration::from_millis(deadline),
        )
    }
    async fn next(&mut self) -> Value {
        tokio::time::timeout(Duration::from_secs(3), self.seen.recv())
            .await
            .unwrap()
            .unwrap()
    }
}
fn response(status: &str, body: &[u8]) -> Vec<u8> {
    let mut b=format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\n",body.len()).into_bytes();
    b.extend_from_slice(body);
    b
}
fn upload() -> Upload {
    Upload {
        bytes: vec![0, 1, 2, 255],
        filename: "Fake.PNG".into(),
        complex: true,
        width: Some("16".into()),
        height: Some("1.5".into()),
    }
}
#[tokio::test]
async fn genuine_tls_multipart_and_text_json() {
    for body in [
        b"ASCII\n".as_slice(),
        b"\"quoted\\nASCII\"",
        b"\xef\xbb\xbf\"BOM\"",
    ] {
        let mut server = OwnedServer::new(response("200 OK", body), false).await;
        let t = server.transport(2000);
        assert_eq!(
            t.send(&upload()).await.unwrap(),
            input::decode(body).unwrap()
        );
        let got = server.next().await;
        let headers = got["headers"].as_str().unwrap().to_ascii_lowercase();
        assert!(!headers.contains("authorization"));
        assert!(!headers.contains("cookie"));
        assert!(!headers.contains("proxy"));
        let parts = got["parts"].as_array().unwrap();
        assert_eq!(
            parts
                .iter()
                .map(|p| p["name"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["file", "complex", "width", "height"]
        );
        assert_eq!(parts[0]["filename"], "Fake.PNG");
        assert_eq!(parts[0]["mime"], "image/png");
        assert_eq!(parts[0]["bytes"], json!([0, 1, 2, 255]));
        assert_eq!(parts[1]["bytes"], json!(b"true"));
        assert_eq!(parts[2]["bytes"], json!(b"16"));
        assert_eq!(parts[3]["bytes"], json!(b"1.5"));
        // Compare genuine Rust multipart parts against the original locked transport.
        let cases: Value = serde_json::from_str(include_str!("../qa/oracle-cases.json")).unwrap();
        let sample = &cases["multipartSamples"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["text"] == "0x20")
            .unwrap()["multipart"];
        use base64::Engine;
        let source_body = base64::engine::general_purpose::STANDARD
            .decode(sample["base64"].as_str().unwrap())
            .unwrap();
        let ct = sample["headers"]["Content-Type"].as_str().unwrap();
        let boundary = multer::parse_boundary(ct).unwrap();
        let mut source = multer::Multipart::new(
            stream::once(async { Ok::<_, std::io::Error>(source_body) }),
            boundary,
        );
        let p = source.next_field().await.unwrap().unwrap();
        assert_eq!(p.file_name(), Some("Fake.PNG"));
        assert_eq!(p.bytes().await.unwrap().as_ref(), upload().bytes);
        assert!(server.seen.try_recv().is_err());
    }
    let mut server = OwnedServer::new(response("200 OK", b"OK"), false).await;
    let mut u = upload();
    u.width = None;
    u.height = None;
    u.complex = false;
    server.transport(2000).send(&u).await.unwrap();
    let got = server.next().await;
    assert_eq!(got["parts"].as_array().unwrap().len(), 2);
    assert_eq!(got["parts"][1]["bytes"], json!(b"false"));
}
#[tokio::test]
async fn transport_failures_no_redirect_or_automatic_replay() {
    for wire in [
        response("400 Bad Request", b"private server dump"),
        b"HTTP/1.1 302 Found\r\nLocation: https://127.0.0.1:1/secret\r\nContent-Length: 0\r\n\r\n"
            .to_vec(),
        response("200 OK", b"{\"error\":\"not ASCII\"}"),
        response("200 OK", &[255]),
        b"HTTP/1.1 200 OK\r\nContent-Length: 1000001\r\n\r\n".to_vec(),
        {
            let mut b = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nf4241\r\n".to_vec();
            b.extend(vec![b'a'; input::MAX_RESULT + 1]);
            b.extend_from_slice(b"\r\n0\r\n\r\n");
            b
        },
    ] {
        let mut server = OwnedServer::new(wire, false).await;
        let error = server.transport(2000).send(&upload()).await.unwrap_err();
        assert!(matches!(error, Failure::Known(_)));
        server.next().await;
        assert!(server.seen.try_recv().is_err());
    }
    for wire in [
        b"not an HTTP response\r\n\r\n".to_vec(),
        b"HTTP/1.1 200 OK\r\nContent-Length: 20\r\n\r\nshort".to_vec(),
    ] {
        let mut server = OwnedServer::new(wire, false).await;
        assert_eq!(
            server.transport(2000).send(&upload()).await.unwrap_err(),
            Failure::Unknown
        );
        server.next().await;
        assert!(server.seen.try_recv().is_err());
    }
    // A plaintext peer on the owned TLS socket never receives HTTP multipart.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!(
        "https://localhost:{}/image-to-ascii",
        listener.local_addr().unwrap().port()
    );
    let plain = tokio::spawn(async move {
        let (mut s, _) = listener.accept().await.unwrap();
        s.write_all(b"not TLS").await.unwrap();
    });
    let t = Transport::owned(
        endpoint,
        include_bytes!("../qa/fixtures/tls/ca.pem"),
        Duration::from_secs(1),
    );
    assert_eq!(t.send(&upload()).await.unwrap_err(), Failure::Unknown);
    plain.await.unwrap();
    let mut server = OwnedServer::new(response("200 OK", b"late"), true).await;
    assert_eq!(
        server.transport(50).send(&upload()).await.unwrap_err(),
        Failure::Unknown
    );
    server.next().await;
    assert!(server.seen.try_recv().is_err());
    // Root trust failure: no HTTP request can reach the parser.
    let server = OwnedServer::new(response("200 OK", b"unused"), false).await;
    let mut t = Transport::production().unwrap();
    t.endpoint = server.endpoint.clone();
    assert_eq!(t.send(&upload()).await.unwrap_err(), Failure::Unknown);
    let mut t = Transport::production().unwrap();
    t.endpoint = "http://127.0.0.1:1/image-to-ascii".into();
    assert_eq!(t.send(&upload()).await.unwrap_err(), Failure::Unknown);
}

fn form_values(v: &Value, d: &Draft, revision: u64) -> Value {
    let mut f = serde_json::to_value(d).unwrap();
    f["__formToken"] = v["root"]["props"]["formToken"].clone();
    f["__formRevision"] = json!(revision);
    f
}
fn action_id(v: &Value, suffix: &str) -> String {
    v["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find_map(|a| a["id"].as_str().filter(|s| s.ends_with(suffix)))
        .unwrap()
        .into()
}
async fn call(a: &Adapter, t: &tezbar_extension_sdk::TestContext, op: &str, p: Value) -> Value {
    let mut p = p;
    p["sessionId"] = json!("test");
    t.run(a.handle(&t.context, COMMAND, op, &p, &mut json!({})))
        .await
        .unwrap()
}
#[tokio::test]
async fn state_fresh_actions_aba_back_copy_unknown_repeat_and_drafts() {
    let server = OwnedServer::new(response("200 OK", b"ASCII\n```\n<img src=x>\n"), false).await;
    let a = Adapter {
        transport: Some(server.transport(2000)),
        ..Adapter::default()
    };
    let t = tezbar_extension_sdk::TestContext::new(
        Default::default(),
        &["selection.read", "clipboard.write"],
    );
    t.reply(Ok(json!([])));
    let mut v = call(&a, &t, "extension:run-command", json!({})).await;
    let dir = owned();
    let path = dir.path().join("owned.png");
    std::fs::write(&path, b"fake").unwrap();
    let mut d = Draft {
        image: vec![path.to_str().unwrap().into()],
        ..Draft::default()
    };
    let stale_submit = action_id(&v, ":submit");
    let change = v["root"]["children"][0]["props"]["actionId"]
        .as_str()
        .unwrap()
        .to_owned();
    for (i, w) in ["80", "81", "80"].iter().enumerate() {
        d.width = (*w).into();
        let values = form_values(&v, &d, i as u64 + 1);
        v = call(
            &a,
            &t,
            "extension:invoke-action",
            json!({"actionId":change,"formValues":values}),
        )
        .await;
    }
    assert_eq!(
        call(
            &a,
            &t,
            "extension:invoke-action",
            json!({"actionId":stale_submit,"formValues":form_values(&v,&d,0)})
        )
        .await["mode"],
        "unchanged"
    );
    let id = action_id(&v, ":submit");
    let values = form_values(&v, &d, 3);
    v = call(
        &a,
        &t,
        "extension:invoke-action",
        json!({"actionId":id,"formValues":values}),
    )
    .await;
    assert_eq!(v["root"]["type"], "Detail");
    let copy = action_id(&v, ":copy");
    t.reply(Ok(Value::Null));
    v = call(&a, &t, "extension:invoke-action", json!({"actionId":copy})).await;
    assert!(v["message"].as_str().unwrap().contains("unknown"));
    assert_eq!(
        call(&a, &t, "extension:invoke-action", json!({"actionId":copy})).await["mode"],
        "unchanged"
    );
    for _ in 0..2 {
        let id = action_id(&v, ":copy");
        t.reply(Ok(json!({"ok":true})));
        v = call(&a, &t, "extension:invoke-action", json!({"actionId":id})).await;
        assert_eq!(v["actions"][0]["title"], "Copy Ascii Image");
    }
    let back = action_id(&v, ":back");
    v = call(&a, &t, "extension:invoke-action", json!({"actionId":back})).await;
    assert_eq!(v["root"]["children"][1]["props"]["value"], "80");
    d.width = "bad".into();
    let values = form_values(&v, &d, 1);
    let id = action_id(&v, ":submit");
    v = call(
        &a,
        &t,
        "extension:invoke-action",
        json!({"actionId":id,"formValues":values}),
    )
    .await;
    assert_eq!(v["root"]["children"][1]["props"]["value"], "bad");
    d.width = "80".into();
    let values = form_values(&v, &d, 1);
    let id = action_id(&v, ":submit");
    v = call(
        &a,
        &t,
        "extension:invoke-action",
        json!({"actionId":id,"formValues":values}),
    )
    .await;
    assert_eq!(v["root"]["type"], "Detail");
    let calls = t.calls();
    let copies: Vec<_> = calls
        .iter()
        .filter(|v| v["method"] == "host.clipboard.write")
        .collect();
    assert_eq!(copies.len(), 3);
    for c in copies {
        assert_eq!(c["params"]["text"], "ASCII\n```\n<img src=x>\n");
    }
}
#[tokio::test]
async fn unknown_upload_retry_freezes_file_and_fields() {
    let mut server = OwnedServer::new(response("200 OK", b"unused"), true).await;
    let a = Adapter {
        transport: Some(server.transport(35)),
        ..Adapter::default()
    };
    let t = tezbar_extension_sdk::TestContext::new(Default::default(), &["selection.read"]);
    t.reply(Ok(json!([])));
    let mut v = call(&a, &t, "extension:run-command", json!({})).await;
    let dir = owned();
    let path = dir.path().join("one.png");
    std::fs::write(&path, b"frozen").unwrap();
    let d = Draft {
        image: vec![path.to_str().unwrap().into()],
        width: "0x10".into(),
        height: "1.5".into(),
        complex: true,
    };
    let values = form_values(&v, &d, 1);
    let id = action_id(&v, ":submit");
    v = call(
        &a,
        &t,
        "extension:invoke-action",
        json!({"actionId":id,"formValues":values}),
    )
    .await;
    let first = server.next().await;
    assert!(v["message"].as_str().unwrap().contains("unknown"));
    std::fs::write(&path, b"changed").unwrap();
    let retry = action_id(&v, ":retry");
    let refresh = call(&a, &t, "extension:refresh-session", json!({})).await;
    assert_eq!(refresh, v);
    assert!(server.seen.try_recv().is_err());
    v = call(&a, &t, "extension:invoke-action", json!({"actionId":retry})).await;
    let second = server.next().await;
    assert_eq!(first["parts"], second["parts"]);
    assert_eq!(
        call(&a, &t, "extension:invoke-action", json!({"actionId":retry})).await["mode"],
        "unchanged"
    );
    assert!(v["message"].as_str().unwrap().contains("unknown"));
}

// Compiled only into the unit-test executable. The shipping binary has no origin
// injection, test protocol flags, environment URL override or fixture response.
#[tokio::test]
#[ignore = "owned protocol helper launched explicitly by qa/process.py"]
async fn owned_pipe_worker() {
    let endpoint = std::env::var("OWNED_QA_ENDPOINT").expect("owned TLS endpoint required");
    let url = reqwest::Url::parse(&endpoint).unwrap();
    assert_eq!(url.scheme(), "https");
    assert_eq!(url.host_str(), Some("localhost"));
    assert_eq!(url.path(), "/image-to-ascii");
    assert!(url.port().is_some());
    let adapter = Adapter {
        transport: Some(Transport::owned(
            endpoint,
            include_bytes!("../qa/fixtures/tls/ca.pem"),
            Duration::from_secs(2),
        )),
        ..Adapter::default()
    };
    tezbar_extension_sdk::serve(adapter).await.unwrap();
}

#[test]
fn maximum_output_fits_view_and_copy_protocol_without_truncation() {
    let t = tezbar_extension_sdk::TestContext::new(Default::default(), &[]);
    for text in [
        "\u{0001}".repeat(input::MAX_RESULT),
        "`".repeat(input::MAX_RESULT),
    ] {
        let s = Session {
            opened: t.context.clone(),
            form_token: 1,
            view_token: 2,
            field_revision: 0,
            draft: Draft::default(),
            result: Some(text.clone()),
            frozen: None,
            upload_unknown: false,
            copy_unknown: false,
            notice: String::new(),
        };
        let view = render(&s);
        assert_eq!(view["root"]["props"]["literalText"], text);
        assert!(serde_json::to_vec(&view).unwrap().len() < 16 * 1024 * 1024);
        assert!(serde_json::to_vec(&json!({"type":"request","id":"e:test","method":"host.clipboard.write","params":{"text":text}})).unwrap().len()<16*1024*1024);
    }
}
