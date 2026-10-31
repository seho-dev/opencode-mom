use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use serde_json::{json, Value};

use super::*;
use crate::error::ErrorCode;
use crate::opencode::target::TargetRuntime;
use crate::opencode::token_usage::tests::respond;

struct FakeApi {
    address: SocketAddr,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl FakeApi {
    fn new(handler: impl Fn(&str, &str, &Value, &mut TcpStream) + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let handler = Arc::new(handler);
        let thread = thread::spawn(move || {
            let mut workers = Vec::new();
            for stream in listener.incoming() {
                let mut stream = stream.unwrap();
                if stopping.load(Ordering::SeqCst) {
                    break;
                }
                let handler = handler.clone();
                workers.push(thread::spawn(move || {
                    stream
                        .set_read_timeout(Some(Duration::from_secs(5)))
                        .unwrap();
                    stream
                        .set_write_timeout(Some(Duration::from_secs(5)))
                        .unwrap();
                    let mut bytes = Vec::new();
                    while !bytes.ends_with(b"\r\n\r\n") {
                        let mut byte = [0];
                        if stream.read(&mut byte).unwrap_or(0) == 0 {
                            return;
                        }
                        bytes.push(byte[0]);
                        assert!(bytes.len() <= 8192);
                    }
                    let headers = String::from_utf8(bytes).unwrap();
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            let (key, value) = line.split_once(':')?;
                            key.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    assert!(length <= MAX_RESPONSE_BYTES);
                    let mut body = vec![0; length];
                    stream.read_exact(&mut body).unwrap();
                    let body = if body.is_empty() {
                        Value::Null
                    } else {
                        serde_json::from_slice(&body).unwrap()
                    };
                    let path = headers.split_whitespace().nth(1).unwrap();
                    handler(path, &headers, &body, &mut stream);
                }));
            }
            for worker in workers {
                worker.join().unwrap();
            }
        });
        Self {
            address,
            stop,
            thread: Some(thread),
        }
    }

    fn url(&self) -> String {
        format!("http://{}", self.address)
    }
}

impl Drop for FakeApi {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.address);
        self.thread.take().unwrap().join().unwrap();
    }
}

fn request() -> PromptGenerationRequest {
    serde_json::from_value(json!({
        "kind": "agent", "preset": "generate", "instructions": "PRIVATE_INSTRUCTIONS",
        "currentPrompt": "PRIVATE_CURRENT"
    }))
    .unwrap()
}

fn runtime() -> (tempfile::TempDir, Arc<TargetRuntime>) {
    let directory = tempfile::tempdir().unwrap();
    let runtime = TargetRuntime::new(
        directory.path().join("target.json"),
        directory.path().join("service.json"),
        directory.path().to_owned(),
    );
    (directory, runtime)
}

fn health(stream: &mut TcpStream) {
    respond(stream, 200, "", br#"{"pid":42,"version":"2.0.22"}"#);
}

fn run<T>(future: impl std::future::Future<Output = T>) -> T {
    tauri::async_runtime::block_on(future)
}

fn generate(request: PromptGenerationRequest, snapshot: Snapshot) -> Result<String, AppError> {
    run(generate_snapshot(
        request.prepare().unwrap(),
        snapshot,
        Instant::now() + Duration::from_secs(5),
    ))
}

fn redacted(error: &AppError) {
    assert!(!serde_json::to_string(error).unwrap().contains("PRIVATE"));
    assert!(error.detail.is_none());
}

#[test]
fn presets_and_kind_have_fixed_order_and_preserve_source_material() {
    for (preset, guide) in [
        (PromptGenerationPreset::Generate, "Draft a prompt"),
        (
            PromptGenerationPreset::Improve,
            "Improve the existing prompt",
        ),
        (PromptGenerationPreset::Shorten, "Shorten the prompt"),
    ] {
        for kind in [PromptGenerationKind::Agent, PromptGenerationKind::Command] {
            let mut input = request();
            input.preset = preset;
            input.kind = kind;
            input.current_prompt =
                "---\r\ndescription: existing\r\n---\r\n$ARGUMENTS $1 $2 @file.md".into();
            let current = input.current_prompt.clone();
            let body = input.prepare().unwrap();
            assert!(body.prompt.starts_with(COMMON_GUIDE));
            let kind_guide = match kind {
                PromptGenerationKind::Agent => "body of an agent system prompt",
                PromptGenerationKind::Command => {
                    assert!(body
                        .prompt
                        .contains("Preserve any existing YAML frontmatter exactly"));
                    assert!(body.prompt.contains("never invent metadata"));
                    assert!(body.prompt.contains("$ARGUMENTS, $1, $2 and @file"));
                    "complete command Markdown document"
                }
            };
            assert!(body.prompt.find(kind_guide).unwrap() < body.prompt.find(guide).unwrap());
            assert!(
                body.prompt.find(guide).unwrap() < body.prompt.find("Existing prompt:").unwrap()
            );
            assert!(body.prompt.ends_with(&format!(
                "User instructions:\nPRIVATE_INSTRUCTIONS\n\nExisting prompt:\n{current}"
            )));
        }
    }
    for field in ["kind", "preset"] {
        let mut value =
            json!({"kind":"agent","preset":"generate","instructions":"x","currentPrompt":""});
        value[field] = json!("unknown");
        assert!(serde_json::from_value::<PromptGenerationRequest>(value).is_err());
    }
}

#[test]
fn input_boundaries_are_checked_before_any_connection_or_bootstrap() {
    let mut valid = request();
    valid.instructions = "x".repeat(MAX_INSTRUCTIONS_BYTES);
    valid.current_prompt = "x".repeat(MAX_PROMPT_BYTES);
    assert!(valid.prepare().is_ok());
    let mut current_only = request();
    current_only.instructions.clear();
    assert!(current_only.prepare().is_ok());
    let mut instructions_only = request();
    instructions_only.current_prompt.clear();
    assert!(instructions_only.prepare().is_ok());
    for (instructions, current) in [
        (" \t\n".into(), " \r\n".into()),
        ("x".repeat(MAX_INSTRUCTIONS_BYTES + 1), "".into()),
        ("".into(), "x".repeat(MAX_PROMPT_BYTES + 1)),
        ("PRIVATE\0TOKEN".into(), "".into()),
        ("".into(), "PRIVATE\u{1b}PROMPT".into()),
    ] {
        let mut input = request();
        input.instructions = instructions;
        input.current_prompt = current;
        let error = run(generate_prompt(input)).unwrap_err();
        assert_eq!(error.code, ErrorCode::ValidationFailed);
        redacted(&error);
    }
    for model in [
        "".into(),
        "provider".into(),
        "/model".into(),
        "provider/".into(),
        "provider/model#high".into(),
        "PRIVATE#provider/model".into(),
        "provider/mo del".into(),
        "provider/model\nPRIVATE".into(),
        "provider/model\0PRIVATE".into(),
        "provider/model\u{2003}PRIVATE".into(),
        format!("{}/model", "x".repeat(257)),
        "x".repeat(MAX_MODEL_BYTES + 1),
    ] {
        let mut input = request();
        input.model = Some(model);
        let error = run(generate_prompt(input)).unwrap_err();
        assert_eq!(error.code, ErrorCode::ValidationFailed);
        redacted(&error);
    }
    for variant in [
        "".into(),
        "PRIVATE\nTOKEN".into(),
        "PRIVATE\0TOKEN".into(),
        "PRIVATE high".into(),
        "PRIVATE\u{2003}high".into(),
        "#high".into(),
        "x".repeat(MAX_VARIANT_BYTES + 1),
    ] {
        let mut input = request();
        input.model = Some("provider/model".into());
        input.variant = Some(variant);
        let error = run(generate_prompt(input)).unwrap_err();
        assert_eq!(error.code, ErrorCode::ValidationFailed);
        redacted(&error);
    }
    let mut input = request();
    input.variant = Some("high".into());
    assert_eq!(
        run(generate_prompt(input)).unwrap_err().code,
        ErrorCode::ValidationFailed
    );
}

#[test]
fn custom_selectors_allow_unicode_plus_and_non_path_model_ids() {
    for id in ["组织/模型+v2?tag", "/model", "../model"] {
        let mut input = request();
        input.model = Some(format!("自定义+provider/{id}"));
        input.variant = Some("思考+high".into());
        assert_eq!(
            serde_json::to_value(input.prepare().unwrap()).unwrap()["model"],
            json!({"providerID":"自定义+provider","id":id,"variant":"思考+high"})
        );
    }
}

#[test]
fn only_prompt_and_optional_model_are_serialized_and_model_splits_once() {
    let mut input = request();
    input.model = Some("provider/org/model:latest@tag".into());
    input.variant = Some("high".into());
    let body = serde_json::to_value(input.prepare().unwrap()).unwrap();
    assert_eq!(body.as_object().unwrap().len(), 2);
    assert_eq!(
        body["model"],
        json!({"providerID":"provider","id":"org/model:latest@tag","variant":"high"})
    );
    let mut input = request();
    input.model = Some("provider/model".into());
    assert_eq!(
        serde_json::to_value(input.prepare().unwrap()).unwrap()["model"],
        json!({"providerID":"provider","id":"model"})
    );
    assert_eq!(
        serde_json::to_value(request().prepare().unwrap())
            .unwrap()
            .as_object()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn success_pins_selected_target_and_reserves_connect_health_and_post_without_island() {
    for password in [None, Some("private".to_owned())] {
        let (_directory, runtime) = runtime();
        let selected = runtime.clone();
        let authenticated = password.is_some();
        let calls = Arc::new(AtomicUsize::new(0));
        let counted = calls.clone();
        let server = FakeApi::new(move |path, headers, body, stream| {
            assert_eq!(
                selected.set(None, None, || Ok(())).err().unwrap().code,
                ErrorCode::Busy
            );
            let headers = headers.to_ascii_lowercase();
            assert_eq!(headers.contains("authorization:"), authenticated);
            if authenticated {
                assert!(headers.contains("authorization: basic b3blbmnvzgu6chjpdmf0zq=="));
            }
            match path {
                "/api/info" => {
                    assert!(headers.starts_with("get "));
                    health(stream);
                }
                "/api/experimental/generate" => {
                    assert!(headers.starts_with("post "));
                    assert_eq!(body.as_object().unwrap().len(), 1);
                    assert!(body["prompt"]
                        .as_str()
                        .unwrap()
                        .contains("PRIVATE_INSTRUCTIONS"));
                    counted.fetch_add(1, Ordering::SeqCst);
                    respond(
                        stream,
                        200,
                        "",
                        br#"{"data":{"text":" # Final\n\nText\n"}}"#,
                    );
                }
                _ => panic!("unexpected route: {path}"),
            }
        });
        let unselected = TcpListener::bind("127.0.0.1:0").unwrap();
        unselected.set_nonblocking(true).unwrap();
        std::fs::write(
            &runtime.registration,
            format!(
                "{{\"PRIVATE\":\"http://{}\"}}",
                unselected.local_addr().unwrap()
            ),
        )
        .unwrap();
        let snapshot = runtime
            .set(Some(server.url()), password, || Ok(()))
            .unwrap();
        assert_eq!(generate(request(), snapshot).unwrap(), " # Final\n\nText\n");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(
            matches!(unselected.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
        );
        runtime.set(None, None, || Ok(())).unwrap();
    }
}

#[test]
fn shared_registration_uses_pinned_credentials_and_selected_model() {
    let (_directory, runtime) = runtime();
    let selected = runtime.clone();
    let server = FakeApi::new(move |path, headers, body, stream| {
        assert_eq!(
            selected.set(None, None, || Ok(())).err().unwrap().code,
            ErrorCode::Busy
        );
        assert!(headers.contains("Basic b3BlbmNvZGU6cHJpdmF0ZQ=="));
        match path {
            "/api/info" => health(stream),
            "/api/experimental/generate" => {
                assert_eq!(
                    body["model"],
                    json!({"providerID":"provider","id":"org/model","variant":"high"})
                );
                respond(stream, 200, "", br#"{"data":{"text":"Generated"}}"#);
            }
            _ => panic!("unexpected route: {path}"),
        }
    });
    std::fs::write(
        &runtime.registration,
        serde_json::to_vec(&json!({
            "id": uuid::Uuid::new_v4().to_string(), "version":"2.0.22",
            "url":server.url(), "pid":42, "password":"private"
        }))
        .unwrap(),
    )
    .unwrap();
    let mut input = request();
    input.model = Some("provider/org/model".into());
    input.variant = Some("high".into());
    assert_eq!(
        generate(input, runtime.snapshot().unwrap()).unwrap(),
        "Generated"
    );
    runtime.set(None, None, || Ok(())).unwrap();
}

#[test]
fn identity_change_or_failed_write_health_never_posts() {
    for status in [200, 401] {
        let infos = Arc::new(AtomicUsize::new(0));
        let counted = infos.clone();
        let server = FakeApi::new(move |path, _, _, stream| {
            assert_eq!(path, "/api/info");
            if counted.fetch_add(1, Ordering::SeqCst) == 0 {
                health(stream);
            } else {
                respond(
                    stream,
                    status,
                    "",
                    br#"{"pid":43,"version":"2.0.22","PRIVATE":"TOKEN"}"#,
                );
            }
        });
        let (_directory, runtime) = runtime();
        let snapshot = runtime.set(Some(server.url()), None, || Ok(())).unwrap();
        let error = generate(request(), snapshot).unwrap_err();
        redacted(&error);
        assert_eq!(infos.load(Ordering::SeqCst), 2);
        runtime.set(None, None, || Ok(())).unwrap();
    }
}

#[test]
fn stale_snapshot_is_rejected_before_connect() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let (_directory, runtime) = runtime();
    let snapshot = runtime
        .set(
            Some(format!("http://{}", listener.local_addr().unwrap())),
            None,
            || Ok(()),
        )
        .unwrap();
    runtime.set(None, None, || Ok(())).unwrap();
    assert!(generate(request(), snapshot)
        .unwrap_err()
        .message
        .contains("target changed"));
    assert!(
        matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
    );
}

#[test]
fn errors_are_redacted_and_http_failures_never_retry_or_redirect() {
    let destination = TcpListener::bind("127.0.0.1:0").unwrap();
    destination.set_nonblocking(true).unwrap();
    for status in [302, 401, 403, 404, 405, 408, 429, 500, 503] {
        let calls = Arc::new(AtomicUsize::new(0));
        let counted = calls.clone();
        let location = format!(
            "Location: http://{}/PRIVATE_REDIRECT\r\n",
            destination.local_addr().unwrap()
        );
        let server = FakeApi::new(move |path, _, _, stream| {
            if path == "/api/info" {
                health(stream);
            } else {
                counted.fetch_add(1, Ordering::SeqCst);
                respond(stream, status, &location, b"PRIVATE_RESPONSE_TOKEN");
            }
        });
        let (_directory, runtime) = runtime();
        let snapshot = runtime
            .set(Some(server.url()), Some("PRIVATE_TOKEN".into()), || Ok(()))
            .unwrap();
        let error = generate(request(), snapshot).unwrap_err();
        redacted(&error);
        assert!(error.message.contains("may have executed"));
        assert!(error.message.contains("do not automatically retry"));
        assert_eq!(
            error.message.contains("not supported"),
            matches!(status, 404 | 405)
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(
            matches!(destination.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
        );
    }
}

#[test]
fn empty_non_text_malformed_and_oversized_results_are_rejected_once() {
    for body in [
        Vec::new(),
        b"PRIVATE_MALFORMED".to_vec(),
        b"null".to_vec(),
        br#"[{"data":{"text":"PRIVATE"}}]"#.to_vec(),
        br#"{"data":["PRIVATE"]}"#.to_vec(),
        br#"{"data":{"text":null}}"#.to_vec(),
        br#"{"data":{"text":42}}"#.to_vec(),
        br#"{"data":{"text":""}}"#.to_vec(),
        br#"{"data":{"text":" \n\t"}}"#.to_vec(),
        br#"{"data":{"text":"PRIVATE\u0000"}}"#.to_vec(),
        vec![b'x'; MAX_RESPONSE_BYTES + 1],
    ] {
        let calls = Arc::new(AtomicUsize::new(0));
        let counted = calls.clone();
        let server = FakeApi::new(move |path, _, _, stream| {
            if path == "/api/info" {
                health(stream);
            } else {
                counted.fetch_add(1, Ordering::SeqCst);
                respond(stream, 200, "", &body);
            }
        });
        let (_directory, runtime) = runtime();
        let snapshot = runtime.set(Some(server.url()), None, || Ok(())).unwrap();
        let error = generate(request(), snapshot).unwrap_err();
        redacted(&error);
        assert!(error.message.contains("may have executed"));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
    let server = FakeApi::new(|path, _, _, stream| {
        if path == "/api/info" {
            health(stream);
        } else {
            write!(stream, "HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n").unwrap();
            let _ = stream.write_all(&vec![b'x'; MAX_RESPONSE_BYTES + 1]);
        }
    });
    let (_directory, runtime) = runtime();
    let snapshot = runtime.set(Some(server.url()), None, || Ok(())).unwrap();
    assert!(generate(request(), snapshot)
        .unwrap_err()
        .message
        .contains("2 MiB"));
}

#[test]
fn timeout_or_connection_loss_has_unknown_outcome_and_is_not_retried() {
    for timeout in [false, true] {
        let calls = Arc::new(AtomicUsize::new(0));
        let counted = calls.clone();
        let server = FakeApi::new(move |path, _, _, stream| {
            if path == "/api/info" {
                health(stream);
            } else {
                counted.fetch_add(1, Ordering::SeqCst);
                if timeout {
                    thread::sleep(Duration::from_millis(1250));
                }
            }
        });
        let connection =
            transport::test_connection(&server.url(), Instant::now() + Duration::from_secs(5))
                .into_async()
                .unwrap();
        let error = run(send(
            &connection,
            &request().prepare().unwrap(),
            Instant::now() + Duration::from_secs(1),
        ))
        .unwrap_err();
        redacted(&error);
        assert!(error.message.contains("may have executed"));
        assert_eq!(error.message.contains("timed out"), timeout);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn cancelled_connect_worker_keeps_reservation_until_it_finishes() {
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let release_rx = Mutex::new(release_rx);
    let server = FakeApi::new(move |path, _, _, stream| {
        assert_eq!(path, "/api/info");
        entered_tx.send(()).unwrap();
        release_rx
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        health(stream);
    });
    let (_directory, runtime) = runtime();
    let snapshot = runtime.set(Some(server.url()), None, || Ok(())).unwrap();
    run(async {
        let task = tauri::async_runtime::spawn(generate_snapshot(
            request().prepare().unwrap(),
            snapshot,
            Instant::now() + Duration::from_secs(5),
        ));
        entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(
            runtime.set(None, None, || Ok(())).err().unwrap().code,
            ErrorCode::Busy
        );
        task.abort();
        let _ = task.await;
        assert_eq!(
            runtime.set(None, None, || Ok(())).err().unwrap().code,
            ErrorCode::Busy
        );
        release_tx.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match runtime.set(None, None, || Ok(())) {
                Ok(_) => break,
                Err(error) => assert_eq!(error.code, ErrorCode::Busy),
            }
            assert!(Instant::now() < deadline);
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    });
}
