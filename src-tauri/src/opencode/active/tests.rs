use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::*;
use crate::opencode::token_usage::tests::{connection, healthy_response, respond, FakeApi};

fn parse(bytes: &[u8]) -> Result<usize, AppError> {
    transport::decode_json::<JsonObject<ActiveSessionsEnvelope>>(bytes, "opencode session.active")
        .map(|value| value.0.data)
}

#[test]
fn active_sessions_counts_children_and_accepts_empty_map_only_as_idle() {
    let api = FakeApi::new(|target, stream| {
        if target == "/api/info" {
            healthy_response(stream);
        } else {
            assert_eq!(target, "/api/session/active");
            respond(stream, 200, "", br#"{"data":{"ses_parent":{"type":"running"},"ses_child":{"type":"running","parentID":"ses_parent"}}}"#);
        }
    });
    assert_eq!(
        active_sessions(&connection(&api), Instant::now() + ACTIVE_SESSIONS_TIMEOUT).unwrap(),
        2
    );
    assert_eq!(parse(br#"{"data":{}}"#).unwrap(), 0);
}

#[test]
fn active_sessions_rejects_unverified_shapes_ids_duplicates_and_states_without_leaking() {
    for value in [
        json!({}),
        json!([{}]),
        json!({"data": null}),
        json!({"data": []}),
        json!({"data": 0}),
        json!({"data":{"ses_SECRET":{}}}),
        json!({"data":{"ses_SECRET":null}}),
        json!({"data":{"ses_SECRET":"running"}}),
        json!({"data":{"ses_SECRET":["running"]}}),
        json!({"data":{"ses_SECRET":{"type":"idle"}}}),
        json!({"data":{"ses_SECRET":{"type":"retry"}}}),
        json!({"data":{"ses_SECRET":{"type":"SECRET_UNKNOWN"}}}),
        json!({"data":{"ses_SECRET":{"type":1}}}),
    ] {
        let error = parse(&serde_json::to_vec(&value).unwrap()).unwrap_err();
        assert!(error.message.contains("invalid opencode session.active"));
        assert!(!error.message.contains("SECRET"));
        assert!(error.detail.is_none());
    }
    for payload in [
        "SECRET_NOT_JSON",
        "{\"data\":{",
        "{\"data\":{}} trailing_SECRET",
        "{\"data\":{},\"data\":{}}",
        "{\"data\":{\"ses_SECRET\":{\"type\":\"running\"},\"ses_SECRET\":{\"type\":\"running\"}}}",
        "{\"data\":{\"ses_SECRET\":{\"type\":\"running\",\"type\":\"running\"}}}",
    ] {
        assert!(!parse(payload.as_bytes())
            .unwrap_err()
            .message
            .contains("SECRET"));
    }
    for id in [
        "SECRET_prefix",
        "ses_",
        "ses_SECRET/../path",
        "ses_SECRET\nline",
        "ses_SECRET space",
        "ses_SECRET\u{1b}[31m",
        "ses_SECRETé",
    ] {
        let value: Value = json!({"data":{id:{"type":"running"}}});
        assert!(!parse(&serde_json::to_vec(&value).unwrap())
            .unwrap_err()
            .message
            .contains("SECRET"));
    }
}

#[test]
fn readonly_http_errors_are_not_idle_and_responses_are_bounded() {
    for status in [401, 403, 500] {
        let api = FakeApi::new(move |target, stream| {
            if target == "/api/info" {
                healthy_response(stream);
            } else {
                respond(stream, status, "", br#"{"data":{},"SECRET":true}"#);
            }
        });
        let error = active_sessions(&connection(&api), Instant::now() + ACTIVE_SESSIONS_TIMEOUT)
            .unwrap_err();
        assert!(error.message.contains(&status.to_string()));
        assert!(!error.message.contains("SECRET"));
    }
    let api = FakeApi::new(|target, stream| {
        if target == "/api/info" {
            healthy_response(stream);
        } else {
            respond(
                stream,
                200,
                "",
                &vec![b' '; ACTIVE_SESSIONS_MAX_BYTES as usize + 1],
            );
        }
    });
    assert!(
        active_sessions(&connection(&api), Instant::now() + ACTIVE_SESSIONS_TIMEOUT)
            .unwrap_err()
            .message
            .contains("1 MiB")
    );
    let api = FakeApi::new(|target, stream| {
        if target == "/api/info" {
            healthy_response(stream);
        } else {
            std::thread::sleep(Duration::from_millis(250));
            respond(stream, 200, "", br#"{"data":{}}"#);
        }
    });
    let connection = connection(&api);
    let now = Instant::now();
    assert!(active_sessions(&connection, now + Duration::from_millis(50)).is_err());
    assert!(now.elapsed() < Duration::from_millis(200));
}
