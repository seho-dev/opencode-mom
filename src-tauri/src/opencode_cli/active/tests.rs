use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Output};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::{
    active_sessions_command, active_sessions_result, collect_active_sessions,
    ACTIVE_SESSIONS_MAX_BYTES, ACTIVE_SESSIONS_TIMEOUT,
};

struct FakeCli(PathBuf);

impl FakeCli {
    fn new(script: &str) -> Self {
        let directory =
            std::env::temp_dir().join(format!("opencode-mom-active-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&directory).unwrap();
        let path = directory.join("opencode");
        fs::write(
            &path,
            format!(
                "#!/bin/sh\n[ \"$#\" -eq 2 ] && [ \"$1\" = api ] && [ \"$2\" = session.active ] || exit 99\n{script}\n"
            ),
        )
        .unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        Self(path)
    }

    fn collect(&self, timeout: Duration) -> Result<usize, crate::error::AppError> {
        collect_active_sessions(active_sessions_command(&self.0).spawn().unwrap(), timeout)
    }
}

impl Drop for FakeCli {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(self.0.parent().unwrap());
    }
}

fn successful_output(value: Value) -> Output {
    Output {
        status: ExitStatus::from_raw(0),
        stdout: serde_json::to_vec(&value).unwrap(),
        stderr: Vec::new(),
    }
}

#[test]
fn active_sessions_counts_every_running_session_including_children() {
    let cli = FakeCli::new(
        "printf '%s' '{\"data\":{\"ses_parent\":{\"type\":\"running\"},\"ses_child\":{\"type\":\"running\",\"parentID\":\"ses_parent\"},\"ses_other\":{\"type\":\"running\"}}}'",
    );
    assert_eq!(cli.collect(ACTIVE_SESSIONS_TIMEOUT).unwrap(), 3);
    assert_eq!(
        active_sessions_command(Path::new("fake-opencode"))
            .get_args()
            .collect::<Vec<_>>(),
        ["api", "session.active"]
    );
}

#[test]
fn active_sessions_empty_data_map_is_idle() {
    let cli = FakeCli::new("printf '%s' '{\"data\":{}}'");
    assert_eq!(cli.collect(ACTIVE_SESSIONS_TIMEOUT).unwrap(), 0);
}

#[test]
fn active_sessions_rejects_missing_malformed_and_unknown_state_without_leaking() {
    for value in [
        json!({}),
        json!([{}]),
        json!([{"ses_SECRET": {"type": "running"}}]),
        json!({ "data": null }),
        json!({ "data": [] }),
        json!({ "data": 0 }),
        json!({ "data": { "ses_SECRET": {} } }),
        json!({ "data": { "ses_SECRET": null } }),
        json!({ "data": { "ses_SECRET": "running" } }),
        json!({ "data": { "ses_SECRET": ["running"] } }),
        json!({ "data": { "ses_SECRET": { "type": "idle" } } }),
        json!({ "data": { "ses_SECRET": { "type": "retry" } } }),
        json!({ "data": { "ses_SECRET": { "type": "SECRET_UNKNOWN" } } }),
        json!({ "data": { "ses_SECRET": { "type": 1 } } }),
        json!({ "data": { "ses_good": { "type": "running" }, "ses_SECRET": { "type": "busy" } } }),
    ] {
        let error = active_sessions_result(successful_output(value)).unwrap_err();
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
        let error = active_sessions_result(Output {
            status: ExitStatus::from_raw(0),
            stdout: payload.as_bytes().to_vec(),
            stderr: Vec::new(),
        })
        .unwrap_err();
        assert!(!error.message.contains("SECRET"));
    }
}

#[test]
fn active_sessions_rejects_unsafe_session_ids_without_leaking() {
    for id in [
        "SECRET_wrong_prefix",
        "ses_",
        "ses_SECRET/../path",
        "ses_SECRET\nline",
        "ses_SECRET space",
        "ses_SECRET\u{1b}[31m",
        "ses_SECRETé",
    ] {
        let value = json!({ "data": { id: { "type": "running" } } });
        let error = active_sessions_result(successful_output(value)).unwrap_err();
        assert!(error.message.contains("invalid opencode session.active"));
        assert!(!error.message.contains("SECRET"));
    }
}

#[test]
fn active_sessions_nonzero_is_error_even_with_idle_output_and_redacts_output() {
    let cli = FakeCli::new(
        "printf '%s' '{\"data\":{},\"SECRET_STDOUT\":true}'; printf '%s' 'SECRET_STDERR' >&2; exit 7",
    );
    let error = cli.collect(ACTIVE_SESSIONS_TIMEOUT).unwrap_err();
    assert!(error.message.contains("session.active failed"));
    assert!(error.message.contains('7'));
    assert!(!error.message.contains("SECRET"));
}

#[test]
fn active_sessions_process_timeout_is_bounded() {
    let cli = FakeCli::new("printf '%s' '{\"data\":{}}'; exec sleep 5");
    let started = Instant::now();
    let error = cli.collect(Duration::from_millis(100)).unwrap_err();
    assert!(error.message.contains("session.active timed out"));
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn active_sessions_oversized_stdout_is_error() {
    let cli = FakeCli::new(&format!(
        "exec /usr/bin/head -c {} /dev/zero",
        ACTIVE_SESSIONS_MAX_BYTES + 1
    ));
    let started = Instant::now();
    let error = cli.collect(ACTIVE_SESSIONS_TIMEOUT).unwrap_err();
    assert!(
        error.message.contains("response exceeds the 1 MiB limit"),
        "{}",
        error.message
    );
    assert!(started.elapsed() < Duration::from_secs(2));
}
