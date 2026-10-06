use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;

use serde_json::{json, Value};

use super::*;

pub(super) fn deadline() -> Instant {
    Instant::now() + SCAN_TIMEOUT
}

fn session(id: &str) -> Value {
    json!({ "id": id })
}

fn fork(id: &str, origin: &str, boundary: &str, kind: &str) -> Value {
    json!({
        "id": id,
        "fork": { "sessionID": origin, "boundary": { "type": kind, "messageID": boundary } }
    })
}

fn usage(id: &str, kind: &str, time: u64, input: u64, output: u64) -> Value {
    json!({
        "id": id, "type": kind, "time": { "created": time, "completed": time + 1 },
        "tokens": { "input": input, "output": output, "reasoning": 999,
            "cache": { "read": 999, "write": 999 } },
        "finish": "stop"
    })
}

fn nonusage(id: &str, kind: &str, time: u64) -> Value {
    json!({ "id": id, "type": kind, "time": { "created": time },
        "content": [{ "text": "PRIVATE_CONTENT" }], "title": "PRIVATE_TITLE" })
}

fn copied(mut message: Value, id: &str) -> Value {
    message["id"] = json!(id);
    message
}

pub(super) fn page(data: Vec<Value>, next: Option<&str>) -> Value {
    json!({ "data": data, "cursor": { "previous": null, "next": next } })
}

fn fixture_pages<T: for<'de> Deserialize<'de>>(
    pages: Vec<Value>,
    inspect: impl Fn(&T) -> Result<(&str, usize), AppError>,
) -> Result<(Vec<T>, Vec<Option<String>>), AppError> {
    let mut pages = pages.into_iter();
    let mut calls = Vec::new();
    let result = read_pages(
        |cursor| {
            calls.push(cursor.map(str::to_owned));
            serde_json::from_value(pages.next().expect("unexpected extra page"))
                .map_err(|_| failure("invalid fixture page"))
        },
        inspect,
        &Budget::default(),
        deadline(),
    )?;
    assert!(pages.next().is_none());
    Ok((result, calls))
}

fn fixture_records(
    sessions: Vec<Value>,
    timelines: Vec<Vec<Value>>,
) -> Result<Vec<TokenUsageRecord>, AppError> {
    let sessions: Vec<Session> = serde_json::from_value(json!(sessions)).unwrap();
    let timelines: Vec<Vec<Message>> = serde_json::from_value(json!(timelines)).unwrap();
    for timeline in &timelines {
        for message in timeline {
            inspect_message(message)?;
        }
    }
    records(&sessions, &timelines, deadline())
}

#[test]
fn session_pages_follow_short_page_cursor_to_empty_and_keep_children_archived_zero() {
    let (sessions, calls) = fixture_pages(
        vec![
            page(
                vec![
                    json!({ "id": "root", "total": 0 }),
                    json!({ "id": "child", "parentID": "root", "time": { "archived": 1 } }),
                ],
                Some("next"),
            ),
            page(vec![], None),
        ],
        inspect_session,
    )
    .unwrap();
    assert_eq!(calls, [None, Some("next".into())]);
    assert_eq!(sessions.len(), 2);
    assert_eq!(sessions[1].id, "child");
    let (sessions, calls) = fixture_pages(
        vec![
            page(vec![session("root")], Some("next")),
            page(vec![], Some("next")),
        ],
        inspect_session,
    )
    .unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(calls.len(), 2);
}

#[test]
fn timelines_paginate_without_type_filter_and_only_serialize_root_input_output() {
    let (messages, calls) = fixture_pages(
        vec![
            page(
                vec![
                    nonusage("user", "user", 10),
                    usage("a", "assistant", 11, 100, 20),
                ],
                Some("p2"),
            ),
            page(
                vec![
                    usage("c", "compaction", 12, 30, 4),
                    nonusage("x", "future_type", 13),
                ],
                Some("p3"),
            ),
            page(vec![], None),
        ],
        inspect_message,
    )
    .unwrap();
    assert_eq!(calls, [None, Some("p2".into()), Some("p3".into())]);
    let sessions = vec![serde_json::from_value(session("root")).unwrap()];
    let result = records(&sessions, &[messages], deadline()).unwrap();
    assert_eq!(
        serde_json::to_value(result).unwrap(),
        json!([
            { "time": 11, "input": 100, "output": 20 },
            { "time": 12, "input": 30, "output": 4 }
        ])
    );
}

#[test]
fn missing_recorded_usage_is_excluded_and_cancelled_usage_is_retained() {
    let mut pending_compaction = nonusage("pc", "compaction", 1);
    pending_compaction["status"] = json!("running");
    let pending_assistant = nonusage("pa", "assistant", 2);
    let mut aborted = usage("abort", "assistant", 3, 12, 4);
    aborted["finish"] = json!("abort");
    aborted["error"] = json!({ "message": "PRIVATE_ERROR" });
    let result = fixture_records(
        vec![session("root")],
        vec![vec![pending_compaction, pending_assistant, aborted]],
    )
    .unwrap();
    assert_eq!(
        result,
        [TokenUsageRecord {
            time: 3,
            input: 12,
            output: 4
        }]
    );
    let mut false_error = usage("false-error", "assistant", 4, 1, 1);
    false_error["error"] = json!(false);
    assert_eq!(
        fixture_records(vec![session("root")], vec![vec![false_error]])
            .unwrap()
            .len(),
        1
    );
    for (kind, field, value) in [
        ("assistant", "finish", json!("stop")),
        ("assistant", "error", json!({ "message": "PRIVATE" })),
        ("assistant", "status", json!("failed")),
        ("compaction", "status", json!("completed")),
        ("compaction", "status", json!("failed")),
        ("compaction", "status", json!("future_status")),
    ] {
        let mut message = nonusage("missing", kind, 1);
        message[field] = value;
        assert!(fixture_records(vec![session("root")], vec![vec![message]])
            .unwrap()
            .is_empty());
    }
    let mut completed = nonusage("missing", "assistant", 1);
    completed["time"]["completed"] = json!(2);
    assert!(
        fixture_records(vec![session("root")], vec![vec![completed]])
            .unwrap()
            .is_empty()
    );
}

#[test]
fn terminal_absent_usage_and_pending_stale_tokens_do_not_create_records() {
    let mut failed = nonusage("failed", "assistant", 2);
    failed["finish"] = json!("abort");
    failed["error"] = json!({ "message": "PRIVATE_ERROR" });
    failed["tokens"] = Value::Null;
    let mut compaction = nonusage("summary", "compaction", 3);
    compaction["status"] = json!("completed");
    compaction["time"]["completed"] = json!(4);
    let mut pending = nonusage("retry", "assistant", 4);
    pending["tokens"] = json!({ "input": 100, "output": 10 });
    let result = fixture_records(
        vec![session("root")],
        vec![vec![
            usage("a", "assistant", 1, 10, 2),
            failed,
            compaction,
            pending,
        ]],
    )
    .unwrap();
    assert_eq!(
        result,
        [TokenUsageRecord {
            time: 1,
            input: 10,
            output: 2,
        }]
    );
}

#[test]
fn pending_retries_exclude_stale_tokens_until_terminal_metadata_is_present() {
    let mut pending = nonusage("retry", "assistant", 2);
    pending["tokens"] = json!({ "input": 12, "output": 4 });
    let mut running_assistant = pending.clone();
    running_assistant["status"] = json!("running");
    let mut running_compaction = running_assistant.clone();
    running_compaction["type"] = json!("compaction");
    for pending in [pending, running_assistant, running_compaction] {
        assert!(
            fixture_records(vec![session("root")], vec![vec![pending.clone()]])
                .unwrap()
                .is_empty()
        );
        for (field, value) in [
            ("finish", json!("stop")),
            ("finish", json!("abort")),
            ("time", json!({ "created": 2, "completed": 3 })),
            ("error", json!({ "message": "PRIVATE_ERROR" })),
            ("status", json!("failed")),
        ] {
            let mut terminal = pending.clone();
            terminal[field] = value;
            assert_eq!(
                fixture_records(vec![session("root")], vec![vec![terminal]]).unwrap(),
                [TokenUsageRecord {
                    time: 2,
                    input: 12,
                    output: 4,
                }]
            );
        }
    }
}

#[test]
fn fork_before_through_changed_ids_and_fork_chain_count_only_owned_usage() {
    let user = nonusage("u", "user", 1);
    let a = usage("a", "assistant", 2, 10, 1);
    let c = usage("c", "compaction", 3, 20, 2);
    let own_before = usage("b-own", "assistant", 4, 30, 3);
    let own_through = usage("t-own", "assistant", 5, 40, 4);
    let own_chain = usage("chain-own", "assistant", 6, 50, 5);
    let result = fixture_records(
        vec![
            session("root"),
            fork("before", "root", "a", "before"),
            fork("through", "root", "c", "through"),
            fork("chain", "through", "t-own", "through"),
        ],
        vec![
            vec![user.clone(), a.clone(), c.clone()],
            vec![copied(user.clone(), "b-u"), own_before],
            vec![
                copied(user.clone(), "t-u"),
                copied(a.clone(), "t-a"),
                copied(c.clone(), "t-c"),
                own_through.clone(),
            ],
            vec![
                copied(user, "chain-u"),
                copied(a, "chain-a"),
                copied(c, "chain-c"),
                copied(own_through, "chain-t"),
                own_chain,
            ],
        ],
    )
    .unwrap();
    assert_eq!(result.len(), 5);
    assert_eq!(result.iter().map(|record| record.input).sum::<u64>(), 150);
    assert_eq!(result.iter().map(|record| record.output).sum::<u64>(), 15);
}

#[test]
fn absent_usage_remains_in_fork_prefix_validation() {
    for kind in ["assistant", "compaction"] {
        let mut absent = nonusage("missing", kind, 2);
        absent["time"]["completed"] = json!(3);
        if kind == "compaction" {
            absent["status"] = json!("completed");
        }
        let original = usage("a", "assistant", 1, 10, 2);
        let sessions = vec![session("root"), fork("child", "root", "missing", "through")];
        let origin = vec![original.clone(), absent.clone()];
        let child = vec![
            copied(original, "copy-a"),
            copied(absent, "copy-missing"),
            usage("own", "assistant", 4, 20, 3),
        ];
        let result =
            fixture_records(sessions.clone(), vec![origin.clone(), child.clone()]).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result.iter().map(|record| record.input).sum::<u64>(), 30);
        assert_eq!(result.iter().map(|record| record.output).sum::<u64>(), 5);

        let mut changed = child;
        changed[1]["tokens"] = json!({ "input": 0, "output": 0 });
        let error = fixture_records(sessions, vec![origin, changed]).unwrap_err();
        assert!(error.message.contains("copied prefix mismatch"));
    }
}

#[test]
fn forks_filter_native_copy_eligibility_before_boundaries_and_nested_offsets() {
    let user = nonusage("u", "user", 1);
    let mut pending = nonusage("pending", "assistant", 2);
    pending["tokens"] = json!({ "input": 100, "output": 10 });
    let mut shell = nonusage("shell", "shell", 3);
    shell["status"] = json!("running");
    let mut compaction = nonusage("compaction", "compaction", 4);
    compaction["status"] = json!("running");
    compaction["tokens"] = json!({ "input": 200, "output": 20 });
    let mut aborted = usage("abort", "assistant", 5, 5, 1);
    aborted["finish"] = json!("abort");
    aborted["error"] = json!({ "message": "PRIVATE_ERROR" });
    aborted["time"]["completed"] = Value::Null;
    let completed = usage("a", "assistant", 6, 10, 2);
    let origin = vec![
        user.clone(),
        pending,
        shell,
        compaction,
        aborted,
        completed.clone(),
    ];
    for boundary in ["pending", "shell", "compaction", "abort", "a"] {
        for kind in ["before", "through"] {
            let mut child = vec![copied(user.clone(), "child-u")];
            if boundary == "a" && kind == "through" {
                child.push(copied(completed.clone(), "child-a"));
            }
            child.push(usage("child-own", "assistant", 7, 20, 3));
            let mut nested = child
                .iter()
                .enumerate()
                .map(|(index, message)| copied(message.clone(), &format!("nested-{index}")))
                .collect::<Vec<_>>();
            nested.push(usage("nested-own", "assistant", 8, 30, 4));
            let result = fixture_records(
                vec![
                    session("root"),
                    fork("child", "root", boundary, kind),
                    fork("nested", "child", "child-own", "through"),
                ],
                vec![origin.clone(), child, nested],
            )
            .unwrap();
            assert_eq!(result.len(), 4);
            assert_eq!(result.iter().map(|record| record.input).sum::<u64>(), 65);
            assert_eq!(result.iter().map(|record| record.output).sum::<u64>(), 10);
        }
    }
    let error = fixture_records(
        vec![session("root"), fork("child", "root", "a", "through")],
        vec![
            origin,
            vec![
                copied(user, "child-u"),
                usage("child-own", "assistant", 7, 20, 3),
            ],
        ],
    )
    .unwrap_err();
    assert!(error.message.contains("copied prefix mismatch"));
}

#[test]
fn fork_zero_owned_prefix_and_child_with_old_timestamp_are_not_time_filtered() {
    let a = usage("a", "assistant", 10, 2, 1);
    let result = fixture_records(
        vec![
            session("root"),
            fork("empty", "root", "a", "before"),
            fork("nested", "empty", "b", "before"),
        ],
        vec![
            vec![a],
            vec![usage("b", "assistant", 1, 3, 1)],
            vec![usage("own", "assistant", 0, 4, 1)],
        ],
    )
    .unwrap();
    assert_eq!(result.iter().map(|record| record.input).sum::<u64>(), 9);

    let original = usage("a", "assistant", 10, 2, 1);
    let result = fixture_records(
        vec![
            session("root"),
            fork("no-own", "root", "a", "through"),
            fork("nested", "no-own", "copied-a", "through"),
        ],
        vec![
            vec![original.clone()],
            vec![copied(original.clone(), "copied-a")],
            vec![
                copied(original, "copied-again"),
                usage("own", "assistant", 0, 4, 1),
            ],
        ],
    )
    .unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result.iter().map(|record| record.input).sum::<u64>(), 6);
}

#[test]
fn fork_missing_origin_boundary_prefix_mismatch_and_cycle_fail_closed() {
    let a = usage("a", "assistant", 2, 10, 1);
    for (sessions, timelines) in [
        (
            vec![fork("fork", "missing", "a", "through")],
            vec![vec![copied(a.clone(), "copy")]],
        ),
        (
            vec![session("root"), fork("fork", "root", "missing", "before")],
            vec![vec![a.clone()], vec![]],
        ),
        (
            vec![session("root"), fork("fork", "root", "a", "through")],
            vec![vec![a.clone()], vec![]],
        ),
        (
            vec![
                fork("a", "b", "b-msg", "through"),
                fork("b", "a", "a-msg", "through"),
            ],
            vec![
                vec![copied(a.clone(), "a-msg")],
                vec![copied(a.clone(), "b-msg")],
            ],
        ),
    ] {
        let error = fixture_records(sessions, timelines).unwrap_err();
        assert!(error.message.contains("fork provenance unavailable"));
        assert!(!error.message.contains("missing\""));
    }
    for field in ["type", "time", "tokens", "finish", "status", "error"] {
        let mut copy = copied(a.clone(), "copy");
        copy[field] = match field {
            "type" => json!("compaction"),
            "time" => json!({ "created": 3, "completed": 4 }),
            "tokens" => json!({ "input": 11, "output": 1 }),
            "finish" => json!("abort"),
            "status" => json!("failed"),
            _ => json!({ "message": "PRIVATE" }),
        };
        let error = fixture_records(
            vec![session("root"), fork("fork", "root", "a", "through")],
            vec![vec![a.clone()], vec![copy]],
        )
        .unwrap_err();
        assert!(error.message.contains("copied prefix mismatch"));
    }
}

#[test]
fn malformed_counts_timestamps_identifiers_and_safe_integer_sums_are_rejected() {
    for tokens in [json!({ "input": 1 }), json!({ "output": 1 }), json!({})] {
        let mut message = usage("partial", "assistant", 2, 1, 1);
        message["tokens"] = tokens;
        assert!(serde_json::from_value::<Message>(message.clone()).is_err());
        assert!(fixture_pages::<Message>(
            vec![
                page(vec![usage("valid", "assistant", 1, 10, 2)], Some("next")),
                page(vec![message], None),
            ],
            inspect_message,
        )
        .is_err());
    }
    for value in [
        json!(-1),
        json!(1.5),
        json!(JS_MAX_INTEGER + 1),
        json!(u64::MAX),
        json!("1"),
    ] {
        for field in ["input", "output"] {
            let mut pending = nonusage("a", "assistant", 1);
            pending["tokens"] = json!({ "input": 0, "output": 0 });
            let mut compaction = pending.clone();
            compaction["type"] = json!("compaction");
            compaction["status"] = json!("running");
            for mut message in [usage("a", "assistant", 1, 0, 0), pending, compaction] {
                message["tokens"][field] = value.clone();
                let parsed = serde_json::from_value::<Message>(message);
                assert!(parsed.is_err() || inspect_message(&parsed.unwrap()).is_err());
            }
        }
    }
    for value in [json!(-1), json!(1.5), json!(JS_MAX_DATE + 1)] {
        for field in ["created", "completed", "streamed"] {
            let mut message = usage("a", "assistant", 1, 0, 0);
            message["time"][field] = value.clone();
            let parsed = serde_json::from_value::<Message>(message);
            assert!(parsed.is_err() || inspect_message(&parsed.unwrap()).is_err());
        }
    }
    let mut zero = usage("a", "assistant", 0, 0, 0);
    zero["time"]["created"] = json!(JS_MAX_DATE);
    assert!(fixture_records(vec![session("root")], vec![vec![zero]]).is_ok());
    for (input, output) in [(JS_MAX_INTEGER, 1), (u64::MAX, 1)] {
        assert!(fixture_records(
            vec![session("root")],
            vec![vec![usage("a", "assistant", 1, input, output)]]
        )
        .is_err());
    }
    for second in [
        usage("b", "assistant", 2, 1, 0),
        usage("b", "assistant", 2, 0, 1),
    ] {
        assert!(fixture_records(
            vec![session("root")],
            vec![vec![usage("a", "assistant", 1, JS_MAX_INTEGER, 0), second]],
        )
        .is_err());
    }
    for (field, value) in [("id", ""), ("type", ""), ("id", "bad\nidentifier")] {
        let mut message = usage("a", "assistant", 1, 1, 1);
        message[field] = json!(value);
        assert!(fixture_records(vec![session("root")], vec![vec![message]]).is_err());
    }
}

#[test]
fn cursor_loops_duplicate_ids_failed_pages_and_metadata_limits_never_return_partial_data() {
    for pages in [
        vec![
            page(vec![session("a")], Some("loop")),
            page(vec![session("b")], Some("loop")),
        ],
        vec![
            page(vec![session("a")], Some("next")),
            page(vec![session("a")], None),
        ],
        vec![page(vec![session("a"), session("a")], None)],
    ] {
        assert!(fixture_pages(pages, inspect_session).is_err());
    }
    let mut calls = 0;
    let result = read_pages(
        |_| {
            calls += 1;
            if calls == 1 {
                Ok(serde_json::from_value(page(vec![session("a")], Some("next"))).unwrap())
            } else {
                Err(failure("PRIVATE page failed"))
            }
        },
        inspect_session,
        &Budget::default(),
        deadline(),
    );
    assert!(result.is_err());
    assert_eq!(calls, 2);
    for budget in [
        Budget {
            items: AtomicUsize::new(MAX_METADATA_ITEMS),
            bytes: AtomicUsize::new(0),
        },
        Budget {
            items: AtomicUsize::new(0),
            bytes: AtomicUsize::new(MAX_METADATA_BYTES),
        },
    ] {
        assert!(read_pages(
            |_| Ok(serde_json::from_value(page(vec![session("a")], None)).unwrap()),
            inspect_session,
            &budget,
            deadline(),
        )
        .is_err());
    }
    let mut fetched = false;
    assert!(read_pages(
        |_| {
            fetched = true;
            Err::<Page<Session>, _>(failure("unexpected"))
        },
        inspect_session,
        &Budget::default(),
        Instant::now(),
    )
    .is_err());
    assert!(!fetched);
}

#[test]
fn record_limit_accepts_exact_bound_and_rejects_one_more() {
    let sessions = vec![serde_json::from_value(session("root")).unwrap()];
    let mut timeline: Vec<Message> = (0..MAX_RECORDS)
        .map(|index| {
            serde_json::from_value(usage(&index.to_string(), "assistant", 1, 1, 1)).unwrap()
        })
        .collect();
    assert_eq!(
        records(&sessions, std::slice::from_ref(&timeline), deadline())
            .unwrap()
            .len(),
        MAX_RECORDS
    );
    timeline.push(serde_json::from_value(usage("extra", "assistant", 1, 1, 1)).unwrap());
    assert!(records(&sessions, &[timeline], deadline())
        .unwrap_err()
        .message
        .contains("record limit"));
}

#[test]
fn oversized_page_metadata_is_rejected_before_unbounded_allocation() {
    let items = (0..1001).map(|index| session(&index.to_string())).collect();
    assert!(serde_json::from_value::<Page<Session>>(page(items, None)).is_err());
}

pub(super) struct FakeApi {
    pub(super) address: SocketAddr,
    stopping: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl FakeApi {
    pub(super) fn new(handler: impl Fn(&str, &mut TcpStream) + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let stopping = Arc::new(AtomicBool::new(false));
        let stop = stopping.clone();
        let handler = Arc::new(handler);
        let thread = thread::spawn(move || {
            let mut requests = Vec::new();
            for stream in listener.incoming() {
                let mut stream = stream.unwrap();
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                let handler = handler.clone();
                requests.push(thread::spawn(move || {
                    stream
                        .set_read_timeout(Some(Duration::from_secs(2)))
                        .unwrap();
                    stream
                        .set_write_timeout(Some(Duration::from_secs(2)))
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
                    let request = String::from_utf8(bytes).unwrap();
                    assert!(
                        request.lines().filter_map(|line| line.split_once(':')).any(
                            |(key, value)| {
                                key.eq_ignore_ascii_case("authorization")
                                    && value.trim() == "Basic b3BlbmNvZGU6cHJpdmF0ZQ=="
                            }
                        ),
                        "missing synthetic Basic auth"
                    );
                    let target = request.split_whitespace().nth(1).unwrap();
                    handler(target, &mut stream);
                }));
            }
            for request in requests {
                request.join().unwrap();
            }
        });
        Self {
            address,
            stopping,
            thread: Some(thread),
        }
    }

    pub(super) fn url(&self) -> String {
        format!("http://{}", self.address)
    }
}

impl Drop for FakeApi {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::Relaxed);
        let _ = TcpStream::connect(self.address);
        self.thread.take().unwrap().join().unwrap();
    }
}

pub(super) fn respond(stream: &mut TcpStream, status: u16, headers: &str, body: &[u8]) {
    let _ = write!(
        stream,
        "HTTP/1.1 {status} Fixture\r\nConnection: close\r\nContent-Length: {}\r\n{headers}\r\n",
        body.len()
    );
    let _ = stream.write_all(body);
}

pub(super) fn healthy_response(stream: &mut TcpStream) {
    respond(stream, 200, "", br#"{"version":"2.0.22","pid":42}"#);
}

pub(super) fn connection(api: &FakeApi) -> ManagedConnection {
    transport::test_connection(&api.url(), deadline())
}

#[test]
fn official_http_paging_workers_large_message_and_fork_aggregation() {
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let api = FakeApi::new(move |target, stream| {
        count.fetch_add(1, Ordering::Relaxed);
        if target == "/api/info" {
            healthy_response(stream);
            return;
        }
        let user = nonusage("u", "user", 1);
        let mut a = usage("a", "assistant", 2, 10, 1);
        a["content"] = json!("x".repeat(2 * 65536));
        let result = match target {
            "/api/session?limit=1000&order=asc" => page(
                vec![
                    json!({ "id": "root", "total": 0 }),
                    json!({ "id": "child", "parentID": "root", "time": { "archived": 1 } }),
                    fork("fork", "root", "a", "through"),
                ],
                Some("sessions-next"),
            ),
            "/api/session?limit=1000&cursor=sessions-next" => page(vec![], None),
            "/api/session/root/message?limit=100&order=asc" => {
                page(vec![user.clone(), a.clone()], Some("root-next"))
            }
            "/api/session/root/message?limit=100&cursor=root-next" => {
                page(vec![usage("c", "compaction", 3, 20, 2)], None)
            }
            "/api/session/child/message?limit=100&order=asc" => {
                page(vec![usage("sub", "assistant", 4, 30, 3)], None)
            }
            "/api/session/fork/message?limit=100&order=asc" => page(
                vec![
                    copied(user, "copy-u"),
                    copied(a, "copy-a"),
                    usage("own", "assistant", 5, 40, 4),
                ],
                Some("fork-next"),
            ),
            "/api/session/fork/message?limit=100&cursor=fork-next" => page(vec![], None),
            _ => panic!("unexpected synthetic API target"),
        };
        respond(stream, 200, "", &serde_json::to_vec(&result).unwrap());
    });
    let connection = connection(&api);
    let budget = Budget::default();
    let deadline = deadline();
    let sessions = read_pages(
        |cursor| connection.fetch_page(None, cursor, deadline),
        inspect_session,
        &budget,
        deadline,
    )
    .unwrap();
    let timelines = read_timelines(&connection, &sessions, &budget, deadline).unwrap();
    let result = records(&sessions, &timelines, deadline).unwrap();
    assert_eq!(result.len(), 4);
    assert_eq!(result.iter().map(|record| record.input).sum::<u64>(), 100);
    assert_eq!(result.iter().map(|record| record.output).sum::<u64>(), 10);
    assert_eq!(calls.load(Ordering::Relaxed), 8);
    assert!(serde_json::to_value(result)
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .all(|record| {
            record
                .as_object()
                .unwrap()
                .keys()
                .all(|key| matches!(key.as_str(), "time" | "input" | "output"))
        }));
}

#[test]
fn http_path_and_cursor_are_encoded_as_one_segment_and_query_value() {
    let api = FakeApi::new(|target, stream| {
        if target == "/api/info" {
            healthy_response(stream);
            return;
        }
        assert!(
            target
                == "/api/session/s%2F%3F%23%25%20x/message?limit=100&cursor=c%2F%3F%23%25+%26%3D"
        );
        respond(
            stream,
            200,
            "",
            &serde_json::to_vec(&page(vec![], None)).unwrap(),
        );
    });
    let connection = connection(&api);
    assert!(connection
        .fetch_page::<Message>(Some("s/?#% x"), Some("c/?#% &="), deadline())
        .unwrap()
        .data
        .is_empty());
}

#[test]
fn http_invalid_truncated_and_oversized_metadata_never_expose_private_values() {
    for body in [
        b"PRIVATE_INVALID_JSON".to_vec(),
        br#"{"data":[{"id":"PRIVATE_ID","type":"assistant","time":{"created":"PRIVATE_TIME"},"content":"PRIVATE_CONTENT"}],"cursor":{"next":null}}"#.to_vec(),
        serde_json::to_vec(&page((0..101).map(|index| usage(&index.to_string(), "assistant", 1, 1, 1)).collect(), None)).unwrap(),
        {
            let mut bytes = format!(r#"{{"data":[{}],"cursor":{{"next":"#, usage("PRIVATE_ID", "assistant", 1, 10, 2)).into_bytes();
            bytes.resize(65536, b' ');
            bytes
        },
    ] {
        let api = FakeApi::new(move |target, stream| {
            if target == "/api/info" { healthy_response(stream); }
            else { respond(stream, 200, "", &body); }
        });
        let error = connection(&api).fetch_page::<Message>(Some("s"), None, deadline()).err().unwrap();
        assert!(!error.message.contains("PRIVATE"));
        assert!(error.detail.is_none());
    }
}

#[test]
fn http_header_and_body_stalls_share_deadline_and_bound_all_worker_joins() {
    for body_stall in [false, true] {
        let api = FakeApi::new(move |target, stream| {
            if target == "/api/info" {
                healthy_response(stream);
                return;
            }
            if body_stall {
                let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\n{");
            }
            thread::sleep(Duration::from_millis(400));
        });
        let connection = connection(&api);
        let sessions = (0..8)
            .map(|index| serde_json::from_value(session(&index.to_string())).unwrap())
            .collect::<Vec<_>>();
        let started = Instant::now();
        let error = read_timelines(
            &connection,
            &sessions,
            &Budget::default(),
            started + Duration::from_millis(100),
        )
        .err()
        .unwrap();
        assert!(started.elapsed() < Duration::from_millis(350));
        assert!(!error.message.contains("PRIVATE"));
    }
}

#[test]
fn worker_failure_does_not_return_partial_records_or_mask_the_original_error() {
    let api = FakeApi::new(|target, stream| {
        if target == "/api/info" {
            healthy_response(stream);
            return;
        }
        if target.contains("/bad/") {
            respond(stream, 403, "", b"PRIVATE_ERROR");
        } else {
            thread::sleep(Duration::from_millis(30));
            respond(
                stream,
                200,
                "",
                &serde_json::to_vec(&page(vec![usage("a", "assistant", 1, 10, 2)], Some("next")))
                    .unwrap(),
            );
        }
    });
    let connection = connection(&api);
    let sessions =
        ["slow", "bad", "slow2", "slow3"].map(|id| serde_json::from_value(session(id)).unwrap());
    let error = read_timelines(&connection, &sessions, &Budget::default(), deadline())
        .err()
        .unwrap();
    assert_eq!(error.message, "opencode token usage: API HTTP 403.");
}

mod process {
    use super::*;

    #[test]
    #[ignore = "Read-only live OpenCode scan; run explicitly after reviewing the managed CLI target"]
    fn read_only_live_scan() {
        let started = Instant::now();
        let result = token_usage_records_via_cli().unwrap();
        let input: u64 = result.iter().map(|record| record.input).sum();
        let output: u64 = result.iter().map(|record| record.output).sum();
        println!(
            "records={} input={input} output={output} elapsed={:?}",
            result.len(),
            started.elapsed()
        );
    }
}
