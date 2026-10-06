use std::process::Command;

use crate::error::AppError;

use super::binary::opencode_candidates;

pub fn reload_via_cli() -> Result<(), AppError> {
    let candidates = opencode_candidates();
    let mut last_error = None;
    for (index, binary) in candidates.iter().enumerate() {
        if !binary.exists() && index + 1 < candidates.len() {
            continue;
        }
        match Command::new(binary).arg("reload").output() {
            Ok(output) => return reload_result(output),
            Err(error) => last_error = Some(format!("{}: {error}", binary.display())),
        }
    }
    Err(AppError::configuration(format!(
        "failed to run opencode CLI — tried {}, none worked. Install the opencode CLI and ensure it is on PATH, or set OPENCODE_BIN.",
        last_error.unwrap_or_else(|| "no candidates found".to_owned())
    )))
}

fn reload_result(output: std::process::Output) -> Result<(), AppError> {
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        let detail = if stderr.is_empty() { stdout } else { stderr };
        return Err(AppError::configuration(format!(
            "opencode reload failed ({}): {}",
            output.status, detail
        )));
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod tests;
