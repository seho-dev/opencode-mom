use std::os::unix::process::ExitStatusExt;
use std::process::{ExitStatus, Output};

use super::reload_result;

#[test]
fn reload_nonzero_reports_failure() {
    let error = reload_result(Output {
        status: ExitStatus::from_raw(7 << 8),
        stdout: Vec::new(),
        stderr: b"reload failed".to_vec(),
    })
    .unwrap_err();
    assert!(error.message.contains("reload failed"));
}
