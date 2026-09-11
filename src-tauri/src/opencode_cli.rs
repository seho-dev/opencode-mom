use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::AppError;
use crate::paths::ConfigPaths;

/// Raw verbose model JSON as emitted by `opencode models --verbose` (one JSON block per model).
#[derive(Debug, Clone, Deserialize)]
pub struct OpencodeVerboseModel {
    pub id: String,
    #[serde(rename = "providerID")]
    pub provider_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub api: Option<Value>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub headers: Option<Value>,
    #[serde(default)]
    pub options: Option<Value>,
    #[serde(default)]
    pub cost: Option<Value>,
    #[serde(default)]
    pub limit: Option<Value>,
    #[serde(default)]
    pub capabilities: Option<Value>,
    #[serde(default)]
    pub release_date: Option<String>,
    #[serde(default)]
    pub variants: Option<Value>,
}

/// Frontend-facing catalog entry.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCatalogEntry {
    pub provider_id: String,
    pub model_id: String,
    #[serde(rename = "ref")]
    pub model_ref: String,
    pub name: String,
    pub is_custom: bool,
    pub status: Option<String>,
    pub cost: Option<Value>,
    pub limit: Option<Value>,
    pub capabilities: Option<Value>,
    pub variants: Option<Value>,
    pub api: Option<Value>,
    pub release_date: Option<String>,
}

/// Candidate opencode binaries, in priority order:
/// 1. `$OPENCODE_BIN` explicit override (if the file exists)
/// 2. Known install locations — probed directly so the app works even when
///    launched with a minimal PATH (e.g. GUI launch on Windows)
/// 3. Plain `opencode` — resolved through PATH by `Command::new` at spawn time
fn opencode_candidates() -> Vec<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(candidate) = env::var("OPENCODE_BIN") {
        let path = PathBuf::from(candidate.trim());
        if path.exists() {
            candidates.push(path);
        }
    }

    // npm global installs (Windows)
    if let Ok(appdata) = env::var("APPDATA") {
        candidates
            .push(PathBuf::from(appdata).join("npm/node_modules/opencode-ai/bin/opencode.exe"));
    }
    // Bundled CLI with the OpenChamber desktop app (Windows)
    if let Ok(local_appdata) = env::var("LOCALAPPDATA") {
        candidates.push(
            PathBuf::from(local_appdata)
                .join("Programs/@openchamberelectron/resources/opencode-cli/opencode.exe"),
        );
    }
    // Home directories (macOS / Linux)
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".opencode/bin/opencode"));
        candidates.push(home.join(".local/bin/opencode"));
        candidates.push(home.join(".npm-global/bin/opencode"));
        candidates.push(home.join(".cargo/bin/opencode"));
    }
    candidates.push(PathBuf::from("/opt/homebrew/bin/opencode"));
    candidates.push(PathBuf::from("/usr/local/bin/opencode"));

    // Last resort: PATH resolution via `Command::new`.
    candidates.push(PathBuf::from("opencode"));
    candidates
}

/// Resolve the opencode binary: `$OPENCODE_BIN`, then known install locations,
/// otherwise plain `opencode` (resolved through PATH by `Command::new`).
pub fn resolve_opencode_binary() -> PathBuf {
    opencode_candidates()
        .into_iter()
        .find(|p| p.exists())
        .unwrap_or_else(|| PathBuf::from("opencode"))
}

fn run_opencode_models(provider: Option<&str>, verbose: bool) -> Result<String, AppError> {
    let mut last_error: Option<String> = None;
    let candidates = opencode_candidates();
    let total = candidates.len();
    for (index, binary) in candidates.iter().enumerate() {
        // Skip probing non-existent paths, except the final bare `opencode`
        // candidate which resolves through PATH at spawn time.
        if !binary.exists() && index + 1 < total {
            continue;
        }
        match run_with_binary(binary, provider, verbose) {
            Ok(stdout) => return Ok(stdout),
            Err(e) => last_error = Some(format!("{}: {e}", binary.display())),
        }
    }
    Err(AppError::configuration(format!(
        "failed to run opencode CLI — tried {}, none worked. Install the opencode CLI and ensure it is on PATH, or set OPENCODE_BIN.",
        last_error.unwrap_or_else(|| "no candidates found".to_owned())
    )))
}

/// Spawn `opencode models [provider] [--verbose]` with the given binary and return stdout.
fn run_with_binary(
    binary: &Path,
    provider: Option<&str>,
    verbose: bool,
) -> Result<String, AppError> {
    // std::process handles the platform specifics of launching `opencode`:
    // Windows searches PATHEXT (skipping extension-less shims) and runs .cmd/.bat
    // through cmd.exe; Unix uses execvp which natively runs shebang scripts/bins.
    let mut cmd = Command::new(binary);
    cmd.arg("models");
    if let Some(p) = provider {
        if !p.trim().is_empty() {
            cmd.arg(p);
        }
    }
    if verbose {
        cmd.arg("--verbose");
    }

    let output = cmd.output().map_err(|e| {
        AppError::configuration(format!(
            "failed to spawn opencode binary at {}: {e}",
            binary.display()
        ))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        let detail = if stderr.is_empty() { stdout } else { stderr };
        return Err(AppError::configuration(format!(
            "opencode models failed ({}): {}",
            output.status, detail
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn parse_verbose_output(stdout: &str) -> Result<Vec<(String, OpencodeVerboseModel)>, AppError> {
    // Verbose output is:
    //   provider/model
    //   { ...json... }
    // JSON is pretty-printed multi-line. We aggregate lines until buffer parses as valid JSON.
    let mut entries: Vec<(String, OpencodeVerboseModel)> = Vec::new();
    let mut current_ref: Option<String> = None;
    let mut json_buffer = String::new();

    for raw_line in stdout.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        let is_ref = !line.starts_with('{')
            && !line.starts_with('}')
            && !line.starts_with('"')
            && line.contains('/')
            && json_buffer.trim().is_empty();

        if is_ref && json_buffer.trim().is_empty() {
            // Header line
            // If previous ref had no JSON (malformed), just overwrite
            if current_ref.is_some() && !json_buffer.is_empty() {
                json_buffer.clear();
            }
            current_ref = Some(line.to_owned());
            continue;
        }

        // JSON part
        if json_buffer.is_empty() && !line.starts_with('{') {
            // Not yet in JSON and line doesn't start JSON — could be stray, treat as possible ref
            if line.contains('/') {
                current_ref = Some(line.to_owned());
            }
            continue;
        }

        if !json_buffer.is_empty() {
            json_buffer.push('\n');
        }
        json_buffer.push_str(raw_line);

        // Try to parse as complete JSON object
        let trimmed = json_buffer.trim();
        if trimmed.starts_with('{') && trimmed.ends_with('}') {
            if let Ok(model) = serde_json::from_str::<OpencodeVerboseModel>(trimmed) {
                if let Some(provider_model) = current_ref.take() {
                    entries.push((provider_model, model));
                }
                json_buffer.clear();
            } else {
                // Check if error is due to incomplete JSON vs truly invalid.
                // If it ends with '}' but fails to parse due to trailing content, keep buffering.
                // We can try parsing and if error indicates EOF, continue buffering.
                // For simplicity, if parsing fails but buffer looks balanced, treat as invalid and reset.
                // Use serde_json error to decide: if error is EOF, continue; else if buffer is large and still fails, keep buffering one more line.
                // Instead, we rely on the fact that valid JSON will parse successfully; incomplete JSON will fail with EOF.
                // We distinguish by checking if error is EOF-like — serde_json reports "EOF while parsing" for incomplete.
                // To avoid complex handling, we only clear on successful parse; otherwise keep buffering.
                let err_str = serde_json::from_str::<Value>(trimmed)
                    .err()
                    .map(|e| e.to_string())
                    .unwrap_or_default();
                if !err_str.contains("EOF")
                    && trimmed.matches('{').count() == trimmed.matches('}').count()
                {
                    // Balanced but still invalid — likely a real parse error for this model; report
                    if let Some(provider_model) = current_ref.clone() {
                        return Err(AppError::validation(format!(
                            "failed to parse opencode models verbose JSON for {}: {}",
                            provider_model, err_str
                        )));
                    }
                }
                // Otherwise keep buffering (incomplete)
            }
        }
    }

    Ok(entries)
}

/// List models via the local opencode binary, tagging each with builtin vs custom.
///
/// - `provider_filter`: optional provider id to filter (`opencode models <provider> --verbose`)
pub fn list_models_via_cli(
    paths: &ConfigPaths,
    provider_filter: Option<&str>,
) -> Result<Vec<ModelCatalogEntry>, AppError> {
    // Always use verbose to get rich metadata; plain mode loses details needed for frontend.
    let stdout = run_opencode_models(provider_filter, true)?;

    let verbose_entries = parse_verbose_output(&stdout)?;

    let custom_ids = crate::providers::custom_provider_ids(&paths.opencode_file())?;

    let mut catalog = Vec::with_capacity(verbose_entries.len());
    for (ref_line, model) in verbose_entries {
        // Prefer provider/model from JSON's providerID/id, but fall back to ref line
        let provider_id = if !model.provider_id.is_empty() {
            model.provider_id.clone()
        } else {
            ref_line.split('/').next().unwrap_or("").to_owned()
        };
        let model_id = if !model.id.is_empty() {
            model.id.clone()
        } else {
            ref_line.split('/').nth(1).unwrap_or("").to_owned()
        };
        let model_ref = if ref_line.contains('/') {
            ref_line.clone()
        } else {
            format!("{}/{}", provider_id, model_id)
        };
        let is_custom = custom_ids.contains(&provider_id);
        let name = if !model.name.is_empty() {
            model.name.clone()
        } else {
            model_id.clone()
        };
        catalog.push(ModelCatalogEntry {
            provider_id: provider_id.clone(),
            model_id: model_id.clone(),
            model_ref,
            name,
            is_custom,
            status: model.status.clone(),
            cost: model.cost.clone(),
            limit: model.limit.clone(),
            capabilities: model.capabilities.clone(),
            variants: model.variants.clone(),
            api: model.api.clone(),
            release_date: model.release_date.clone(),
        });
    }
    catalog.sort_by(|a, b| a.model_ref.cmp(&b.model_ref));
    Ok(catalog)
}
