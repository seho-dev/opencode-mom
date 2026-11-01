use std::time::{Duration, Instant};

use crate::opencode::token_usage::tests::{connection, healthy_response, respond, FakeApi};

#[test]
fn reload_accepts_204_and_never_retries_or_exposes_errors() {
    for status in [204, 401, 500] {
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let requested = calls.clone();
        let api = FakeApi::new(move |target, stream| {
            if target == "/api/info" {
                healthy_response(stream);
            } else {
                assert_eq!(target, "/api/location/reload");
                requested.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                respond(
                    stream,
                    status,
                    "",
                    if status == 204 { b"" } else { b"PRIVATE_ERROR" },
                );
            }
        });
        let result = connection(&api).reload(Instant::now() + Duration::from_secs(1));
        if status == 204 {
            result.unwrap();
        } else {
            assert!(!result.unwrap_err().message.contains("PRIVATE"));
        }
        assert_eq!(calls.load(std::sync::atomic::Ordering::Relaxed), 1);
    }
}
