use std::future::Future;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread;

use serde_json::json;

use super::*;

struct FakeApi {
    address: SocketAddr,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl FakeApi {
    fn new(handler: impl Fn(&str, &str, Value, &mut TcpStream) + Send + Sync + 'static) -> Self {
        Self::with_info(move |method, target, body, stream| {
            if target == "/api/info" {
                respond(stream, 200, &json!({"pid":42,"version":"2.0.22"}));
            } else {
                handler(method, target, body, stream);
            }
        })
    }

    fn with_info(
        handler: impl Fn(&str, &str, Value, &mut TcpStream) + Send + Sync + 'static,
    ) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let handler = Arc::new(handler);
        let thread = thread::spawn(move || {
            let mut workers = Vec::new();
            for incoming in listener.incoming() {
                if stopping.load(Ordering::Relaxed) {
                    break;
                }
                let mut stream = incoming.unwrap();
                let handler = handler.clone();
                workers.push(thread::spawn(move || {
                    stream.set_read_timeout(Some(REQUEST_TIMEOUT)).unwrap();
                    let mut bytes = Vec::new();
                    let end = loop {
                        let mut byte = [0];
                        stream.read_exact(&mut byte).unwrap();
                        bytes.push(byte[0]);
                        assert!(bytes.len() < 8192);
                        if bytes.ends_with(b"\r\n\r\n") {
                            break bytes.len();
                        }
                    };
                    let headers = String::from_utf8(bytes.clone()).unwrap();
                    assert!(headers
                        .to_ascii_lowercase()
                        .contains("authorization: basic "));
                    assert!(!headers.to_ascii_lowercase().contains("last-event-id"));
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            let (key, value) = line.split_once(':')?;
                            key.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    assert!(length < MAX_BYTES);
                    bytes.resize(end + length, 0);
                    stream.read_exact(&mut bytes[end..]).unwrap();
                    let mut request = headers.lines().next().unwrap().split_whitespace();
                    let method = request.next().unwrap();
                    let target = request.next().unwrap();
                    let body = if length == 0 {
                        Value::Null
                    } else {
                        serde_json::from_slice(&bytes[end..]).unwrap()
                    };
                    handler(method, target, body, &mut stream);
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

    fn api(&self) -> IslandApi {
        let url = format!("http://{}", self.address);
        let connection = transport::test_connection(&url, Instant::now() + REQUEST_TIMEOUT)
            .into_async()
            .unwrap();
        IslandApi {
            connection: Arc::new(connection),
        }
    }
}

impl Drop for FakeApi {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = TcpStream::connect(self.address);
        self.thread.take().unwrap().join().unwrap();
    }
}

fn respond(stream: &mut TcpStream, status: u16, body: &Value) {
    raw(stream, status, "", &serde_json::to_vec(body).unwrap());
}

fn raw(stream: &mut TcpStream, status: u16, headers: &str, body: &[u8]) {
    let _ = write!(
        stream,
        "HTTP/1.1 {status} Fixture\r\nConnection: close\r\nContent-Length: {}\r\n{headers}\r\n",
        body.len()
    );
    let _ = stream.write_all(body);
}

fn run<T>(future: impl std::future::Future<Output = T>) -> T {
    tauri::async_runtime::block_on(future)
}

#[test]
fn api_is_clone_send_sync_and_dtos_are_camel_case() {
    fn traits<T: Clone + Send + Sync>() {}
    traits::<IslandApi>();
    let message = IslandMessage {
        id: "msg_a".into(),
        role: "user".into(),
        text: "public".into(),
        time: 1,
        truncated: false,
    };
    assert_eq!(serde_json::to_value(message).unwrap()["truncated"], false);
    assert_eq!(
        serde_json::to_value(IslandSnapshot::default()).unwrap(),
        json!({"sessions":[],"permissions":[],"questions":[]})
    );
}

#[test]
fn public_text_excludes_reasoning_tools_synthetic_and_hidden() {
    let result = projection::public_messages(vec![
        json!({"id":"msg_a","type":"assistant","time":{"created":2},"content":[
            {"type":"reasoning","text":"PRIVATE_REASONING"},
            {"type":"tool","text":"PRIVATE_TOOL"},
            {"type":"text","text":"PRIVATE_SYNTHETIC","synthetic":true},
            {"type":"text","text":"PRIVATE_HIDDEN","hidden":true},
            {"type":"text","text":"public reply"}
        ]}),
        json!({"id":"msg_u","type":"user","time":{"created":1},"text":"public question"}),
        json!({"id":"msg_c","type":"compaction","text":"PRIVATE_INTERNAL"}),
    ])
    .unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].role, "user");
    assert_eq!(result[1].text, "public reply");
    assert!(!serde_json::to_string(&result).unwrap().contains("PRIVATE"));
}

#[test]
fn public_text_is_utf8_bounded_and_time_is_safe() {
    let text = "界".repeat(MAX_TEXT);
    let mut message = json!({"id":"msg_a","type":"user","time":{"created":1},"text":text});
    let result = projection::public_messages(vec![message.clone()]).unwrap();
    assert!(result[0].truncated);
    assert!(result[0].text.len() <= MAX_TEXT);
    message["time"]["created"] = json!(JS_MAX_DATE + 1);
    assert!(projection::public_messages(vec![message]).is_err());
}

#[test]
fn write_urls_bodies_and_decisions_are_exact_and_not_retried() {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let capture = requests.clone();
    let server = FakeApi::new(move |method, target, body, stream| {
        capture
            .lock()
            .unwrap()
            .push((method.to_owned(), target.to_owned(), body));
        respond(stream, 200, &Value::Null);
    });
    let api = server.api();
    run(async {
        api.send("ses_a", "hello", "msg_a").await.unwrap();
        api.reply_permission("ses_a", "per_a", "once")
            .await
            .unwrap();
        api.reply_permission("ses_a", "per_a", "reject")
            .await
            .unwrap();
        api.cancel_question("ses_a", "frm_a").await.unwrap();
        api.interrupt("ses_a").await.unwrap();
        assert!(api
            .reply_permission("ses_a", "per_a", "allow")
            .await
            .is_err());
        assert!(api.send("ses_a/secret", "hello", "msg_a").await.is_err());
        assert!(api.send("ses_a", "hello", "bad_a").await.is_err());
    });
    let requests = requests.lock().unwrap();
    assert_eq!(
        requests.as_slice(),
        [
            (
                "POST".into(),
                "/api/session/ses_a/prompt".into(),
                json!({"id":"msg_a","text":"hello","delivery":"queue"})
            ),
            (
                "POST".into(),
                "/api/session/ses_a/permission/per_a/reply".into(),
                json!({"decision":"once"})
            ),
            (
                "POST".into(),
                "/api/session/ses_a/permission/per_a/reply".into(),
                json!({"decision":"reject"})
            ),
            (
                "DELETE".into(),
                "/api/session/ses_a/form/frm_a".into(),
                Value::Null
            ),
            (
                "POST".into(),
                "/api/session/ses_a/interrupt?resume=false".into(),
                Value::Null
            ),
        ]
    );
}

#[test]
fn post_reserves_target_until_completion_or_local_cancellation_without_transition_lock() {
    use crate::error::ErrorCode;
    use crate::opencode::target::{self, TargetRuntime};

    for cancel in [false, true] {
        let directory =
            std::env::temp_dir().join(format!("mom-island-write-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        let runtime = TargetRuntime::new(
            directory.join("target.json"),
            directory.join("service.json"),
            directory.clone(),
        );
        let (entered_tx, entered_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let release_rx = Mutex::new(release_rx);
        let server = FakeApi::new(move |method, _, _, stream| {
            assert_eq!(method, "POST");
            entered_tx.send(()).unwrap();
            release_rx
                .lock()
                .unwrap()
                .recv_timeout(REQUEST_TIMEOUT)
                .unwrap();
            respond(stream, 200, &Value::Null);
        });
        std::fs::write(&runtime.registration, serde_json::to_vec(&json!({
            "id":uuid::Uuid::new_v4(), "version":"2.0.22", "url":format!("http://{}", server.address), "pid":42, "password":"private"
        })).unwrap()).unwrap();
        let connection = transport::connect_target(
            runtime.snapshot().unwrap(),
            &runtime.registration,
            Instant::now() + REQUEST_TIMEOUT,
            |_| panic!("no bootstrap"),
        )
        .unwrap()
        .into_async()
        .unwrap();
        let api = IslandApi {
            connection: Arc::new(connection),
        };
        let task =
            tauri::async_runtime::spawn(async move { api.send("ses_a", "hello", "msg_a").await });
        entered_rx.recv_timeout(REQUEST_TIMEOUT).unwrap();
        drop(target::transition().unwrap());
        let error = runtime.set(None, None, || Ok(())).err().unwrap();
        assert_eq!(error.code, ErrorCode::Busy);
        if cancel {
            task.abort();
            assert!(run(task).is_err());
            runtime.set(None, None, || Ok(())).unwrap();
            // Dropping the client cannot withdraw the POST already received by the server.
            release_tx.send(()).unwrap();
        } else {
            release_tx.send(()).unwrap();
            run(task).unwrap().unwrap();
            runtime.set(None, None, || Ok(())).unwrap();
        }
        drop(server);
        std::fs::remove_dir_all(directory).unwrap();
    }
}

#[test]
fn server_restart_at_same_endpoint_rejects_write_before_post() {
    let restarted = Arc::new(AtomicBool::new(false));
    let flag = restarted.clone();
    let posts = Arc::new(Mutex::new(0));
    let count = posts.clone();
    let server = FakeApi::with_info(move |_, target, _, stream| {
        if target == "/api/info" {
            respond(
                stream,
                200,
                &json!({"pid":if flag.load(Ordering::SeqCst) {43} else {42}, "version":"2.0.22"}),
            );
        } else {
            *count.lock().unwrap() += 1;
            respond(stream, 200, &Value::Null);
        }
    });
    let api = server.api();
    restarted.store(true, Ordering::SeqCst);
    let error = run(api.send("ses_a", "hello", "msg_a")).unwrap_err();
    assert!(error.message.contains("identity changed"));
    assert_eq!(*posts.lock().unwrap(), 0);
}

#[test]
fn malformed_oversized_redirect_and_write_errors_are_private() {
    for (status, bytes) in [
        (200, b"PRIVATE_MALFORMED".to_vec()),
        (200, vec![b' '; MAX_BYTES + 1]),
        (302, b"PRIVATE_REDIRECT".to_vec()),
        (401, b"PRIVATE_AUTH_ERROR".to_vec()),
    ] {
        let server = FakeApi::new(move |_, _, _, stream| {
            raw(
                stream,
                status,
                "Location: http://example.com/PRIVATE\r\n",
                &bytes,
            )
        });
        let api = server.api();
        let error = run(api.messages("ses_a")).unwrap_err();
        assert!(!error.message.contains("PRIVATE"));
        assert!(error.detail.is_none());
    }
    let calls = Arc::new(Mutex::new(0));
    let count = calls.clone();
    let server = FakeApi::new(move |_, _, _, stream| {
        *count.lock().unwrap() += 1;
        raw(stream, 500, "", b"PRIVATE_WRITE_ERROR");
    });
    let api = server.api();
    let error = run(api.send("ses_a", "hello", "msg_a")).unwrap_err();
    assert!(error.message.contains("may have been accepted"));
    assert!(!error.message.contains("PRIVATE"));
    assert_eq!(*calls.lock().unwrap(), 1);
    let server = FakeApi::new(|_, _, _, stream| raw(stream, 403, "", b"PRIVATE_REJECTED"));
    let api = server.api();
    let error = run(api.send("ses_a", "hello", "msg_a")).unwrap_err();
    assert!(error.message.contains("write rejected"));
    assert!(!error.message.contains("may have been accepted"));
}

#[test]
fn duplicate_message_id_is_rejected_once_without_retry_or_id_replacement() {
    let calls = Arc::new(Mutex::new(0));
    let count = calls.clone();
    let server = FakeApi::new(move |method, target, body, stream| {
        assert_eq!(method, "POST");
        assert_eq!(target, "/api/session/ses_a/prompt");
        assert_eq!(body["id"], "msg_stable");
        *count.lock().unwrap() += 1;
        raw(stream, 409, "", b"PRIVATE_DUPLICATE_ID");
    });
    let api = server.api();
    let error = run(api.send("ses_a", "hello", "msg_stable")).unwrap_err();
    assert!(error.message.contains("HTTP 409; write rejected"));
    assert!(!error.message.contains("PRIVATE"));
    assert!(!error.message.contains("may have been accepted"));
    assert_eq!(*calls.lock().unwrap(), 1);
}

#[test]
fn v2_prompt_write_and_read_only_message_routes_follow_protocol_contract() {
    // V2 protocol: session.prompt is POST; session.messages is GET only.
    let server = FakeApi::new(|method, target, body, stream| match (method, target) {
        ("POST", "/api/session/ses_a/prompt") => {
            assert_eq!(
                body,
                json!({"id":"msg_stable","text":"hello","delivery":"queue"})
            );
            respond(
                stream,
                200,
                &json!({"data":{"id":"msg_stable","type":"user"}}),
            );
        }
        ("GET", "/api/session/ses_a/message?limit=20&order=desc") => {
            respond(stream, 200, &json!({"data":[],"cursor":{}}));
        }
        ("POST", "/api/session/ses_a/message") => raw(stream, 405, "", b""),
        _ => raw(stream, 404, "", b""),
    });
    let api = server.api();
    run(async {
        api.send("ses_a", "hello", "msg_stable").await.unwrap();
        assert!(api.messages("ses_a").await.unwrap().is_empty());
    });
}

#[test]
fn sse_frames_handle_fragmentation_multiline_crlf_comments_and_bounds() {
    let mut frame = events::Frame::default();
    let source = b": heartbeat\r\nid: ignored\r\ndata: {\r\ndata: \"type\":\"unknown\"}\r\n\r\n";
    let mut result = Vec::new();
    for piece in source.chunks(3) {
        for byte in piece {
            if let Some(data) = frame.byte(*byte).unwrap() {
                result.push(data);
            }
        }
    }
    assert_eq!(result, vec![b"{\n\"type\":\"unknown\"}".to_vec()]);
    assert!(events::classify(&result[0]).unwrap().is_none());
    for _ in 0..65536 {
        frame.byte(b'x').unwrap();
    }
    assert!(frame.byte(b'x').is_err());
}

#[test]
fn messages_fetches_only_newest_bounded_page() {
    let server = FakeApi::new(|method, target, _, stream| {
        assert_eq!(method, "GET");
        assert_eq!(target, "/api/session/ses_a/message?limit=20&order=desc");
        respond(
            stream,
            200,
            &json!({"data":[
            {"id":"msg_b","type":"assistant","time":{"created":2},"content":[{"type":"text","text":"reply"}]},
            {"id":"msg_a","type":"user","time":{"created":1},"text":"hello"}
        ],"cursor":{"next":"DO_NOT_FETCH_HISTORY"}}),
        );
    });
    let api = server.api();
    let messages = run(api.messages("ses_a")).unwrap();
    assert_eq!(messages[0].id, "msg_a");
    assert_eq!(messages[1].text, "reply");
}

#[test]
fn path_segments_are_encoded_by_shared_transport() {
    let server = FakeApi::new(|_, target, _, stream| {
        assert_eq!(target, "/api/session/ses_%2F%3F%23%25%20x/message");
        respond(stream, 200, &json!({"data":[]}));
    });
    let api = server.api();
    run(async {
        let request = api
            .connection
            .request(Method::GET, &["session", "ses_/?#% x", "message"])
            .unwrap();
        let response = request.timeout(REQUEST_TIMEOUT).send().await.unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
    });
}

#[test]
fn snapshot_rejects_disconnected_malformed_or_over_limit_active_map() {
    for active in [
        json!({"data":{"ses_a":{"connected":false}}}),
        json!({"data":{"ses_a":{"type":"idle","connected":false}}}),
        json!({"data":{"PRIVATE_INVALID_ID":{"type":"running"}}}),
        json!({"data":(0..101).map(|index| (format!("ses_{index}"), json!({"type":"running"}))).collect::<Map<_,_>>()}),
    ] {
        let server = FakeApi::new(move |_, target, _, stream| match target {
            "/api/session?limit=20&order=desc&parentID=null" => {
                respond(stream, 200, &json!({"data":[]}))
            }
            "/api/session/active" => respond(stream, 200, &active),
            _ => panic!("must fail before pending or detail fetch"),
        });
        let api = server.api();
        let error = run(api.snapshot()).unwrap_err();
        assert!(!error.message.contains("PRIVATE"));
        assert!(error.detail.is_none());
    }
}

#[test]
fn active_map_duplicate_ids_are_rejected_by_shared_parser() {
    let server = FakeApi::new(|_, target, _, stream| match target {
        "/api/session?limit=20&order=desc&parentID=null" => {
            respond(stream, 200, &json!({"data":[]}))
        }
        "/api/session/active" => raw(
            stream,
            200,
            "",
            br#"{"data":{"ses_a":{"type":"running"},"ses_a":{"type":"running"}}}"#,
        ),
        _ => panic!("must fail before pending or detail fetch"),
    });
    let api = server.api();
    assert!(run(api.snapshot()).is_err());
}

fn session(id: &str, directory: &str, parent: Option<&str>, time: u64) -> Value {
    json!({"id":id,"title":id,"location":{"directory":directory},"parentID":parent,"time":{"updated":time}})
}

fn permission(session_id: &str) -> Value {
    json!({"id":"per_a","sessionID":session_id,"action":"edit","resources":["src/file"],"save":["src/*"]})
}

fn question() -> Value {
    json!({"id":"frm_a","sessionID":"ses_child","title":"Choose","metadata":{"kind":"question"},"fields":[
        {"key":"original-key","type":"string","title":"Choice","required":true,"custom":false,
            "options":[{"value":"wire-value","label":"Display label"}]},
        {"key":"many","type":"multiselect","required":true,"custom":false,
            "options":[{"value":"one","label":"One"},{"value":"two","label":"Two"}]},
        {"key":"custom","type":"string","custom":true,"options":[]}
    ]})
}

#[test]
fn snapshot_covers_multiple_directories_active_children_and_recent_root_pending() {
    let server = FakeApi::new(|method, target, _, stream| {
        assert_eq!(method, "GET");
        let result = match target {
            "/api/session?limit=20&order=desc&parentID=null" => {
                json!({"data":[session("ses_recent","/one",None,30),session("ses_other","/two",None,20)]})
            }
            "/api/session/active" => {
                json!({"data":{"ses_child":{"type":"running","connected":false},"ses_old":{"type":"running"}}})
            }
            "/api/session/ses_child" => {
                json!({"data":session("ses_child","/three",Some("ses_old"),10)})
            }
            "/api/session/ses_old" => json!({"data":session("ses_old","/three",None,5)}),
            "/api/session/ses_child/permission" => {
                json!({"data":[{"id":"per_child","sessionID":"ses_child","action":"edit","resources":["src/file"],"save":["src/*"],"message":"approval","metadata":{"private":"PRIVATE_METADATA"}}]})
            }
            "/api/session/ses_recent/permission" => {
                json!({"data":[{"id":"per_root","sessionID":"ses_recent","action":"bash","resources":["command"]}]})
            }
            "/api/session/ses_child/form" => json!({"data":[question()]}),
            "/api/session/ses_recent/form"
            | "/api/session/ses_other/form"
            | "/api/session/ses_old/form"
            | "/api/session/ses_other/permission"
            | "/api/session/ses_old/permission" => json!({"data":[]}),
            _ => panic!("unexpected snapshot request: {target}"),
        };
        respond(stream, 200, &result);
    });
    let api = server.api();
    let result = run(api.snapshot()).unwrap();
    assert_eq!(result.sessions.len(), 4);
    assert_eq!(result.sessions[0].id, "ses_recent");
    assert_eq!(result.sessions[1].id, "ses_child");
    assert_eq!(result.sessions[1].parent_id.as_deref(), Some("ses_old"));
    assert_eq!(result.sessions[1].directory, "/three");
    assert_eq!(result.sessions[1].status, "running");
    assert_eq!(
        result
            .permissions
            .iter()
            .find(|request| request.id == "per_child")
            .unwrap()
            .session_id,
        "ses_child"
    );
    assert_eq!(result.questions[0].fields[0].key, "original-key");
    let public = serde_json::to_value(&result).unwrap();
    for request in public["permissions"].as_array().unwrap() {
        assert_eq!(
            request["save"],
            if request["id"] == "per_child" {
                json!(["src/*"])
            } else {
                json!([])
            }
        );
        assert!(request.get("sessionId").is_some());
        assert!(request.get("sessionID").is_none());
    }
    assert!(!serde_json::to_string(&result).unwrap().contains("PRIVATE"));
}

#[test]
fn permission_save_is_optional_bounded_plaintext_and_errors_are_private() {
    for (save, accepted) in [
        (None, true),
        (Some(json!([])), true),
        (Some(json!(["src/*", "command\nwith\ttabs"])), true),
        (Some(json!(vec!["x".repeat(4096); MAX_OPTIONS])), true),
        (Some(Value::Null), false),
        (Some(json!("PRIVATE_NOT_A_LIST")), false),
        (Some(json!({"PRIVATE_RULE":true})), false),
        (Some(json!(["src/*", 1])), false),
        (Some(json!(["PRIVATE\u{0}RULE"])), false),
        (Some(json!(["PRIVATE\u{7f}RULE"])), false),
        (Some(json!(["PRIVATE".repeat(586)])), false),
        (Some(json!(vec!["PRIVATE_RULE"; MAX_OPTIONS + 1])), false),
    ] {
        let mut pending = permission("ses_a");
        match &save {
            Some(save) => pending["save"] = save.clone(),
            None => {
                pending.as_object_mut().unwrap().remove("save");
            }
        }
        let calls = Arc::new(Mutex::new(0));
        let count = calls.clone();
        let server = FakeApi::new(move |method, target, _, stream| {
            assert_eq!(method, "GET");
            assert_eq!(target, "/api/session/ses_a/permission");
            *count.lock().unwrap() += 1;
            respond(stream, 200, &json!({"data":[pending]}));
        });
        let api = server.api();
        let result = run(api.permissions("ses_a"));
        assert_eq!(result.is_ok(), accepted);
        match result {
            Ok(requests) => {
                assert_eq!(
                    serde_json::to_value(&requests).unwrap()[0]["save"],
                    save.unwrap_or_else(|| json!([]))
                );
            }
            Err(error) => {
                assert!(!error.message.contains("PRIVATE"));
                assert!(error.detail.is_none());
            }
        }
        assert_eq!(*calls.lock().unwrap(), 1);
    }
}

#[test]
fn always_rereads_exact_pending_identity_and_posts_only_the_decision_including_children() {
    for session_id in ["ses_a", "ses_child"] {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let capture = requests.clone();
        let server = FakeApi::new(move |method, target, body, stream| {
            capture
                .lock()
                .unwrap()
                .push((method.to_owned(), target.to_owned(), body));
            match method {
                "GET" => {
                    assert_eq!(target, format!("/api/session/{session_id}/permission"));
                    respond(stream, 200, &json!({"data":[permission(session_id)]}));
                }
                "POST" => raw(stream, 204, "", b""),
                _ => panic!("unexpected Always method"),
            }
        });
        let api = server.api();
        run(api.reply_permission(session_id, "per_a", "always")).unwrap();
        assert_eq!(
            requests.lock().unwrap().as_slice(),
            [
                (
                    "GET".into(),
                    format!("/api/session/{session_id}/permission"),
                    Value::Null
                ),
                (
                    "POST".into(),
                    format!("/api/session/{session_id}/permission/per_a/reply"),
                    json!({"decision":"always"})
                ),
            ]
        );
    }
}

#[test]
fn stale_missing_unsavable_or_invalid_always_never_posts() {
    for cause in [
        "missing",
        "no-save",
        "empty-save",
        "wrong-request",
        "wrong-owner",
        "null-save",
        "control-save",
        "oversized-save",
        "too-many-save",
        "invalid-response",
    ] {
        let mut pending = permission("ses_a");
        match cause {
            "no-save" => {
                pending.as_object_mut().unwrap().remove("save");
            }
            "empty-save" => pending["save"] = json!([]),
            "wrong-request" => pending["id"] = json!("per_other"),
            "wrong-owner" => pending["sessionID"] = json!("ses_other"),
            "null-save" => pending["save"] = Value::Null,
            "control-save" => pending["save"] = json!(["PRIVATE\u{0}RULE"]),
            "oversized-save" => pending["save"] = json!(["PRIVATE".repeat(586)]),
            "too-many-save" => pending["save"] = json!(vec!["PRIVATE_RULE"; MAX_OPTIONS + 1]),
            _ => {}
        }
        let calls = Arc::new(Mutex::new(0));
        let count = calls.clone();
        let server = FakeApi::new(move |method, target, _, stream| {
            assert_eq!(method, "GET");
            assert_eq!(target, "/api/session/ses_a/permission");
            *count.lock().unwrap() += 1;
            if cause == "invalid-response" {
                raw(stream, 200, "", b"PRIVATE_INVALID");
            } else {
                respond(
                    stream,
                    200,
                    &json!({"data":if cause == "missing" { vec![] } else { vec![pending.clone()] }}),
                );
            }
        });
        let api = server.api();
        let error = run(api.reply_permission("ses_a", "per_a", "always")).unwrap_err();
        assert_eq!(error.code, crate::error::ErrorCode::ValidationFailed);
        assert!(!error.message.contains("PRIVATE"));
        assert!(error.detail.is_none());
        assert_eq!(*calls.lock().unwrap(), 1);
    }
}

#[test]
fn always_rejects_an_externally_replied_request_even_after_a_savable_snapshot() {
    let reads = Arc::new(Mutex::new(0));
    let count = reads.clone();
    let server = FakeApi::new(move |method, target, _, stream| {
        assert_eq!(method, "GET");
        let result = match target {
            "/api/session?limit=20&order=desc&parentID=null" => {
                json!({"data":[session("ses_a","/project",None,1)]})
            }
            "/api/session/active" => json!({"data":{}}),
            "/api/session/ses_a/form" => json!({"data":[]}),
            "/api/session/ses_a/permission" => {
                let mut reads = count.lock().unwrap();
                *reads += 1;
                json!({"data":if *reads == 1 { vec![permission("ses_a")] } else { vec![] }})
            }
            _ => panic!("unexpected stale Always route"),
        };
        respond(stream, 200, &result);
    });
    let api = server.api();
    run(async {
        let snapshot = api.snapshot().await.unwrap();
        assert!(!snapshot.permissions[0].save.is_empty());
        let error = api
            .reply_permission("ses_a", "per_a", "always")
            .await
            .unwrap_err();
        assert_eq!(error.code, crate::error::ErrorCode::ValidationFailed);
    });
    assert_eq!(*reads.lock().unwrap(), 2);
}

#[test]
fn permission_conflicts_are_never_retried_or_downgraded() {
    for decision in ["once", "always"] {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let capture = requests.clone();
        let server = FakeApi::new(move |method, target, body, stream| {
            capture
                .lock()
                .unwrap()
                .push((method.to_owned(), target.to_owned(), body));
            match method {
                "GET" => respond(stream, 200, &json!({"data":[permission("ses_child")]})),
                "POST" => raw(stream, 409, "", b"PRIVATE_ALREADY_REPLIED"),
                _ => panic!("unexpected permission conflict method"),
            }
        });
        let api = server.api();
        let error = run(api.reply_permission("ses_child", "per_a", decision)).unwrap_err();
        assert!(error.message.contains("HTTP 409; write rejected"));
        assert!(!error.message.contains("PRIVATE"));
        assert!(!error.message.contains("may have been accepted"));
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), if decision == "always" { 2 } else { 1 });
        if decision == "always" {
            assert_eq!(
                requests[0],
                (
                    "GET".into(),
                    "/api/session/ses_child/permission".into(),
                    Value::Null
                )
            );
        }
        assert_eq!(
            requests.last().unwrap(),
            &(
                "POST".into(),
                "/api/session/ses_child/permission/per_a/reply".into(),
                json!({"decision":decision})
            )
        );
    }
}

#[test]
fn cancelling_the_always_pending_read_leaves_no_background_post() {
    let (started, reading) = tokio::sync::oneshot::channel();
    let started = Mutex::new(Some(started));
    let (release, released) = std::sync::mpsc::channel();
    let released = Mutex::new(released);
    let calls = Arc::new(Mutex::new(0));
    let count = calls.clone();
    let server = FakeApi::new(move |method, target, _, stream| {
        assert_eq!(method, "GET");
        assert_eq!(target, "/api/session/ses_a/permission");
        *count.lock().unwrap() += 1;
        started.lock().unwrap().take().unwrap().send(()).unwrap();
        released
            .lock()
            .unwrap()
            .recv_timeout(REQUEST_TIMEOUT)
            .unwrap();
        respond(stream, 200, &json!({"data":[permission("ses_a")]}));
    });
    let api = server.api();
    run(async {
        {
            let mut pending = std::pin::pin!(api.reply_permission("ses_a", "per_a", "always"));
            let mut reading = std::pin::pin!(reading);
            std::future::poll_fn(|context| {
                if let std::task::Poll::Ready(result) = reading.as_mut().poll(context) {
                    result.unwrap();
                    return std::task::Poll::Ready(());
                }
                assert!(pending.as_mut().poll(context).is_pending());
                std::task::Poll::Pending
            })
            .await;
        }
        release.send(()).unwrap();
    });
    drop(api);
    drop(server);
    assert_eq!(*calls.lock().unwrap(), 1);
}

#[test]
fn form_reply_rereads_pending_state_and_posts_values_under_original_keys() {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let capture = requests.clone();
    let server = FakeApi::new(move |method, target, body, stream| {
        capture
            .lock()
            .unwrap()
            .push((method.to_owned(), target.to_owned(), body));
        match (method, target) {
            ("GET", "/api/session/ses_child/form/frm_a") => {
                let mut form = question();
                form["state"] = json!({"status":"pending"});
                respond(stream, 200, &json!({"data":form}));
            }
            ("POST", "/api/session/ses_child/form/frm_a/reply") => raw(stream, 204, "", b""),
            _ => panic!("unexpected form request"),
        }
    });
    let api = server.api();
    let answer = json!({"original-key":"wire-value","many":["one","two"],"custom":"free text"})
        .as_object()
        .unwrap()
        .clone();
    run(api.reply_question("ses_child", "frm_a", answer.clone())).unwrap();
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[1].2, json!({"answer":answer}));
}

#[test]
fn v2_form_detail_uses_state_status_and_top_level_metadata_only() {
    let detail = json!({
        "id":"frm_a","sessionID":"ses_child","title":"Choose",
        "metadata":{"kind":"question"},
        "fields":[{"key":"choice","type":"string","required":true,"custom":false,
            "options":[{"value":"wire-value","label":"Display label"}]}],
        "state":{"status":"pending"}
    });
    for variant in [
        "valid",
        "type-discriminator",
        "missing-state",
        "state-metadata",
        "envelope-metadata",
        "missing-metadata",
        "cancelled",
        "hidden",
        "when",
        "pattern",
        "minItems",
    ] {
        let mut form = detail.clone();
        let mut envelope = json!({});
        match variant {
            "type-discriminator" => form["state"] = json!({"type":"pending"}),
            "missing-state" => {
                form.as_object_mut().unwrap().remove("state");
            }
            "state-metadata" => {
                form.as_object_mut().unwrap().remove("metadata");
                form["state"]["metadata"] = json!({"kind":"question"});
            }
            "envelope-metadata" => {
                form.as_object_mut().unwrap().remove("metadata");
                envelope["metadata"] = json!({"kind":"question"});
            }
            "missing-metadata" => {
                form.as_object_mut().unwrap().remove("metadata");
            }
            "cancelled" => form["state"] = json!({"status":"cancelled"}),
            "hidden" => form["fields"][0]["hidden"] = json!(true),
            "when" => form["fields"][0]["when"] = json!([{"key":"earlier","op":"eq","value":"x"}]),
            "pattern" => form["fields"][0]["pattern"] = json!(".*"),
            "minItems" => form["fields"][0]["minItems"] = json!(1),
            _ => {}
        }
        envelope["data"] = form;
        let writes = Arc::new(Mutex::new(0));
        let count = writes.clone();
        let server = FakeApi::new(move |method, target, body, stream| match (method, target) {
            ("GET", "/api/session/ses_child/form/frm_a") => respond(stream, 200, &envelope),
            ("POST", "/api/session/ses_child/form/frm_a/reply") => {
                assert_eq!(body, json!({"answer":{"choice":"wire-value"}}));
                *count.lock().unwrap() += 1;
                raw(stream, 204, "", b"");
            }
            _ => panic!("unexpected V2 form route"),
        });
        let api = server.api();
        let answer = json!({"choice":"wire-value"}).as_object().unwrap().clone();
        assert_eq!(
            run(api.reply_question("ses_child", "frm_a", answer)).is_ok(),
            variant == "valid"
        );
        assert_eq!(*writes.lock().unwrap(), usize::from(variant == "valid"));
    }
}

#[test]
fn inert_form_defaults_and_optional_empty_answers_preserve_required_option_guards() {
    let detail = json!({
        "id":"frm_a","sessionID":"ses_child","title":"Choose","metadata":{"kind":"question"},
        "fields":[
            {"key":"required","type":"string","required":true,"hidden":false,"when":[],"custom":false,"options":[{"value":"value","label":"Label"}]},
            {"key":"optional","type":"string","hidden":false,"when":[],"custom":false,"options":[{"value":"value","label":"Label"}]},
            {"key":"optional-many","type":"multiselect","minItems":0,"hidden":false,"when":[],"custom":false,"options":[{"value":"value","label":"Label"}]},
            {"key":"at-least-one","type":"multiselect","minItems":1,"custom":false,"options":[{"value":"value","label":"Label"}]}
        ],"state":{"status":"pending"}
    });
    let question = projection::public_question(&detail, "ses_child").unwrap();
    assert!(question.answerable);
    assert!(question.fields[3].required);
    for (answer, accepted) in [
        (
            json!({"required":"value","optional":"","optional-many":[],"at-least-one":["value"]}),
            true,
        ),
        (
            json!({"required":"","optional":"","at-least-one":["value"]}),
            false,
        ),
        (json!({"required":"Label","at-least-one":["value"]}), false),
        (
            json!({"required":"value","optional":"Label","at-least-one":["value"]}),
            false,
        ),
        (json!({"required":"value","at-least-one":[]}), false),
        (json!({"required":"value"}), false),
    ] {
        let detail = detail.clone();
        let writes = Arc::new(Mutex::new(0));
        let count = writes.clone();
        let server = FakeApi::new(move |method, target, body, stream| match (method, target) {
            ("GET", "/api/session/ses_child/form/frm_a") => {
                respond(stream, 200, &json!({"data":detail}))
            }
            ("POST", "/api/session/ses_child/form/frm_a/reply") => {
                assert_eq!(
                    body,
                    json!({"answer":{"required":"value","at-least-one":["value"]}})
                );
                *count.lock().unwrap() += 1;
                raw(stream, 204, "", b"");
            }
            _ => panic!("unexpected optional-answer route"),
        });
        let api = server.api();
        assert_eq!(
            run(api.reply_question("ses_child", "frm_a", answer.as_object().unwrap().clone()))
                .is_ok(),
            accepted
        );
        assert_eq!(*writes.lock().unwrap(), usize::from(accepted));
    }
    let mut constrained = detail;
    constrained["fields"][3]["minItems"] = json!(2);
    assert!(
        !projection::public_question(&constrained, "ses_child")
            .unwrap()
            .answerable
    );
}

#[test]
fn fieldless_non_question_form_does_not_fail_the_snapshot() {
    let server = FakeApi::new(|_, target, _, stream| {
        let value = match target {
            "/api/session?limit=20&order=desc&parentID=null" => {
                json!({"data":[session("ses_a","/project",None,1)]})
            }
            "/api/session/active" => json!({"data":{}}),
            "/api/session/ses_a/permission" => json!({"data":[]}),
            "/api/session/ses_a/form" => {
                json!({"data":[{"id":"frm_a","sessionID":"ses_a","title":"External form","metadata":{"kind":"authentication"},"fields":[]}]})
            }
            _ => panic!("unexpected fieldless-form route"),
        };
        respond(stream, 200, &value);
    });
    let api = server.api();
    let snapshot = run(api.snapshot()).unwrap();
    assert_eq!(snapshot.questions.len(), 1);
    assert!(!snapshot.questions[0].answerable);
    assert!(snapshot.questions[0].fields.is_empty());
}

#[test]
fn metadata_private_flags_never_enter_public_message_projection() {
    let mut messages = Vec::new();
    for (index, key) in ["hidden", "synthetic", "internal"].into_iter().enumerate() {
        let mut metadata = Map::new();
        metadata.insert(key.into(), json!(true));
        messages.push(json!({"id":format!("msg_{index}"),"type":"user","time":{"created":1},"metadata":metadata,"text":"PRIVATE_MESSAGE"}));
    }
    messages.push(
        json!({"id":"msg_public","type":"assistant","time":{"created":2},"content":[
            {"type":"text","metadata":{"hidden":true},"text":"PRIVATE_PART"},
            {"type":"text","metadata":{"hidden":false},"text":"public"},
            {"type":"reasoning","text":"PRIVATE_REASONING"},
            {"type":"tool","state":{"content":[{"type":"text","text":"PRIVATE_TOOL"}]}}
        ]}),
    );
    let result = projection::public_messages(messages).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].text, "public");
    assert!(!serde_json::to_string(&result).unwrap().contains("PRIVATE"));
}

#[test]
fn stale_unknown_unsupported_and_invalid_answers_never_write() {
    for (status, answer, unsupported) in [
        (
            "answered",
            json!({"original-key":"wire-value","many":["one"]}),
            false,
        ),
        (
            "pending",
            json!({"original-key":"Display label","many":["one"]}),
            false,
        ),
        (
            "pending",
            json!({"original-key":"wire-value","many":["one","one"]}),
            false,
        ),
        ("pending", json!({"original-key":"wire-value"}), false),
        (
            "pending",
            json!({"original-key":"wire-value","many":["one"],"unknown":"private"}),
            false,
        ),
        (
            "pending",
            json!({"original-key":"wire-value","many":["one"]}),
            true,
        ),
    ] {
        let server = FakeApi::new(move |method, _, _, stream| {
            assert_eq!(method, "GET");
            let mut form = question();
            form["state"] = json!({"status":status});
            if unsupported {
                form["metadata"]["kind"] = json!("authentication");
            }
            respond(stream, 200, &json!({"data":form}));
        });
        let api = server.api();
        assert!(
            run(api.reply_question("ses_child", "frm_a", answer.as_object().unwrap().clone()))
                .is_err()
        );
    }
    for change in [
        json!({"type":"number"}),
        json!({"hidden":true}),
        json!({"when":[{"key":"first","op":"eq","value":"x"}]}),
        json!({"pattern":".*"}),
        json!({"minItems":1}),
        json!({"maxItems":2}),
    ] {
        let mut form = question();
        for (key, value) in change.as_object().unwrap() {
            form["fields"][0][key] = value.clone();
        }
        let public = projection::public_question(&form, "ses_child").unwrap();
        assert!(!public.answerable);
        assert!(public.fields.is_empty());
    }
}

#[test]
fn sse_classification_ignores_private_and_unrelated_events_without_token_snapshots() {
    for kind in [
        "session.status",
        "session.idle",
        "session.execution.started",
        "session.execution.succeeded",
        "session.execution.failed",
        "session.execution.interrupted",
        "session.deleted",
        "permission.asked",
        "permission.replied",
        "form.replied",
        "form.cancelled",
    ] {
        assert_eq!(
            events::classify(
                &serde_json::to_vec(&json!({"type":kind,"data":{"sessionID":"ses_a"}})).unwrap()
            )
            .unwrap(),
            Some(IslandChange::Snapshot)
        );
    }
    assert_eq!(
        events::classify(
            &serde_json::to_vec(&json!({"id":"evt_shutdown","type":"location.shutdown","created":1,"location":{"directory":"/project"},"data":{}})).unwrap()
        ).unwrap(),
        Some(IslandChange::Snapshot)
    );
    assert_eq!(
        events::classify(br#"{"id":"evt_connected","type":"server.connected","data":{}}"#).unwrap(),
        Some(IslandChange::Snapshot)
    );
    assert_eq!(
        events::classify(
            &serde_json::to_vec(&json!({"type":"form.created","data":{"form":question()}}))
                .unwrap()
        )
        .unwrap(),
        Some(IslandChange::Snapshot)
    );
    for kind in [
        "session.text.delta",
        "session.text.started",
        "session.text.ended",
        "session.message.content.updated",
        "session.inbox.delivered",
    ] {
        assert_eq!(
            events::classify(
                &serde_json::to_vec(
                    &json!({"type":kind,"data":{"sessionID":"ses_a","delta":"public"}})
                )
                .unwrap()
            )
            .unwrap(),
            Some(IslandChange::Messages {
                session_id: "ses_a".into()
            })
        );
    }
    for kind in [
        "server.heartbeat",
        "provider.updated",
        "config.updated",
        "rpc.private",
        "session.reasoning.delta",
        "session.tool.progress",
        "session.synthetic",
        "session.compaction.delta",
        "session.step.ended",
        "session.usage.updated",
    ] {
        assert!(events::classify(
            &serde_json::to_vec(
                &json!({"type":kind,"data":{"sessionID":"PRIVATE_ID","text":"PRIVATE_CONTENT"}})
            )
            .unwrap()
        )
        .unwrap()
        .is_none());
    }
    assert!(events::classify(
        br#"{"type":"session.text.delta","properties":{"sessionID":"ses_a"}}"#
    )
    .is_err());
    assert!(events::classify(
        br#"{"type":"session.text.delta","data":"{\"sessionID\":\"ses_a\"}"}"#
    )
    .is_err());
}

#[test]
fn session_snapshot_lifecycles_do_not_require_a_shared_session_id_shape() {
    for (kind, data) in [
        (
            "session.created",
            json!({"session":{"id":"ses_child","parentID":"ses_root","location":{"directory":"/project"}}}),
        ),
        (
            "session.forked",
            json!({"sessionID":"ses_child","parentID":"ses_root","boundary":{"type":"through","messageID":"msg_a"}}),
        ),
        (
            "session.moved",
            json!({"sessionID":"ses_child","location":{"directory":"/other"}}),
        ),
        (
            "session.renamed",
            json!({"sessionID":"ses_child","title":"new title"}),
        ),
        ("session.deleted", json!({"sessionID":"ses_child"})),
    ] {
        let event = json!({"id":"evt_a","type":kind,"created":1,"location":{"directory":"/project"},"data":data});
        assert_eq!(
            events::classify(&serde_json::to_vec(&event).unwrap()).unwrap(),
            Some(IslandChange::Snapshot)
        );
    }
}

#[test]
fn nested_session_created_does_not_end_the_live_stream() {
    let server = FakeApi::new(|_, target, _, stream| {
        assert_eq!(target, "/api/event");
        let created = json!({"id":"evt_created","type":"session.created","created":1,"location":{"directory":"/project"},"data":{"session":{"id":"ses_a"}}});
        let text = json!({"id":"evt_text","type":"session.text.delta","created":2,"location":{"directory":"/project"},"data":{"sessionID":"ses_a","delta":"public"}});
        let frames = format!("data: {created}\n\ndata: {text}\n\n");
        raw(
            stream,
            200,
            "Content-Type: text/event-stream\r\n",
            frames.as_bytes(),
        );
    });
    let api = server.api();
    run(async {
        let mut events = api.events().await.unwrap();
        assert_eq!(events.next().await.unwrap(), IslandChange::Snapshot);
        assert_eq!(
            events.next().await.unwrap(),
            IslandChange::Messages {
                session_id: "ses_a".into()
            }
        );
    });
}

#[test]
fn sse_http_fragmentation_and_eof_are_bounded_without_replay() {
    let server = FakeApi::new(|method, target, _, stream| {
        assert_eq!(method, "GET");
        assert_eq!(target, "/api/event");
        let _ = write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n");
        for piece in b": heartbeat\n\ndata: {\ndata: \"type\":\"session.text.delta\",\"data\":{\"sessionID\":\"ses_a\",\"delta\":\"public\"}}\n\ndata: {\"type\":\"session.execution.succeeded\",\"data\":{\"sessionID\":\"ses_a\"}}\n\n".chunks(7) {
            let _ = write!(stream,"{:x}\r\n",piece.len());
            let _ = stream.write_all(piece);
            let _ = stream.write_all(b"\r\n");
        }
        let _ = stream.write_all(b"0\r\n\r\n");
    });
    let api = server.api();
    run(async {
        let mut events = api.events().await.unwrap();
        assert_eq!(
            events.next().await.unwrap(),
            IslandChange::Messages {
                session_id: "ses_a".into()
            }
        );
        assert_eq!(events.next().await.unwrap(), IslandChange::Snapshot);
        assert!(events.next().await.is_err());
    });
}

#[test]
fn pending_limits_owner_mismatch_and_unsafe_plaintext_fail_without_partial_snapshot() {
    for cause in ["too-many", "wrong-owner", "control-text"] {
        let server = FakeApi::new(move |_, target, _, stream| {
            let result = match target {
                "/api/session?limit=20&order=desc&parentID=null" => {
                    json!({"data":[session("ses_a","/one",None,1)]})
                }
                "/api/session/active" => json!({"data":{}}),
                "/api/session/ses_a/permission" => {
                    let request = json!({"id":"per_a","sessionID":if cause == "wrong-owner" { "ses_wrong" } else { "ses_a" },"action":if cause == "control-text" { "PRIVATE\u{0}ACTION" } else { "edit" },"resources":[]});
                    json!({"data":if cause == "too-many" { vec![request;101] } else { vec![request] }})
                }
                _ => panic!("invalid pending request must fail before any form reads"),
            };
            respond(stream, 200, &result);
        });
        let api = server.api();
        let error = run(api.snapshot()).unwrap_err();
        assert!(!error.message.contains("PRIVATE"));
    }
}

#[test]
fn optional_form_custom_and_required_flags_are_validated_without_guessing_labels() {
    let form = json!({"id":"frm_a","sessionID":"ses_a","title":"Question","metadata":{"kind":"question"},"fields":[
        {"key":"free","type":"string"},
        {"key":"selection","type":"string","options":[{"value":"v","label":"Label"}]}
    ]});
    let question = projection::public_question(&form, "ses_a").unwrap();
    assert!(question.answerable);
    assert!(question.fields[0].custom);
    assert!(!question.fields[0].required);
    assert!(!question.fields[1].custom);
    let mut malformed = form;
    malformed["fields"][0]["required"] = json!("PRIVATE_FLAG");
    let error = projection::public_question(&malformed, "ses_a").unwrap_err();
    assert!(!error.message.contains("PRIVATE"));
}

#[test]
fn quiet_sse_reads_are_async_and_cancel_safe() {
    let server = FakeApi::new(|_, _, _, stream| {
        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n").unwrap();
        thread::sleep(Duration::from_millis(80));
        let event = b"data: {\"type\":\"session.execution.started\",\"data\":{\"sessionID\":\"ses_a\"}}\n\n";
        let _ = write!(stream, "{:x}\r\n", event.len());
        let _ = stream.write_all(event);
        let _ = stream.write_all(b"\r\n0\r\n\r\n");
    });
    let api = server.api();
    run(async {
        let mut events = api.events().await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(10), events.next())
                .await
                .is_err()
        );
        assert_eq!(events.next().await.unwrap(), IslandChange::Snapshot);
    });
}
