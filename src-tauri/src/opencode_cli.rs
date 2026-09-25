use std::collections::BTreeSet;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;
use serde_json::Value;

use crate::error::AppError;
use crate::paths::ConfigPaths;

/// Frontend-facing catalog entry. V2's `opencode models` prints one `provider/model`
/// ref per line; rich metadata comes from the local config's `providers` block.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCatalogEntry {
    pub provider_id: String,
    pub model_id: String,
    #[serde(rename = "ref")]
    pub model_ref: String,
    pub name: String,
    pub is_custom: bool,
    pub limit: Option<Value>,
    pub variants: Option<Value>,
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

fn run_opencode_models() -> Result<String, AppError> {
    let mut last_error: Option<String> = None;
    let candidates = opencode_candidates();
    let total = candidates.len();
    for (index, binary) in candidates.iter().enumerate() {
        // Skip probing non-existent paths, except the final bare `opencode`
        // candidate which resolves through PATH at spawn time.
        if !binary.exists() && index + 1 < total {
            continue;
        }
        match run_with_binary(binary) {
            Ok(stdout) => return Ok(stdout),
            Err(e) => last_error = Some(format!("{}: {e}", binary.display())),
        }
    }
    Err(AppError::configuration(format!(
        "failed to run opencode CLI — tried {}, none worked. Install the opencode CLI and ensure it is on PATH, or set OPENCODE_BIN.",
        last_error.unwrap_or_else(|| "no candidates found".to_owned())
    )))
}

/// Spawn `opencode models` with the given binary and return stdout.
fn run_with_binary(binary: &Path) -> Result<String, AppError> {
    // std::process handles the platform specifics of launching `opencode`:
    // Windows searches PATHEXT (skipping extension-less shims) and runs .cmd/.bat
    // through cmd.exe; Unix uses execvp which natively runs shebang scripts/bins.
    let mut cmd = Command::new(binary);
    cmd.arg("models");

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

/// Parses plain `opencode models` output: one `provider/model` ref per line.
fn parse_model_refs(stdout: &str) -> BTreeSet<String> {
    stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.contains(char::is_whitespace))
        .map(str::to_owned)
        .collect()
}

/// Lists models via the local opencode binary, merged with the local config.
///
/// The CLI provides the enabled catalog refs; config-provided models (including
/// disabled ones) are always present so they stay editable. `provider_filter`
/// narrows the result client-side because V2's CLI takes no positional argument.
pub fn list_models_via_cli(
    paths: &ConfigPaths,
    provider_filter: Option<&str>,
) -> Result<Vec<ModelCatalogEntry>, AppError> {
    let stdout = run_opencode_models()?;
    let mut refs: BTreeSet<String> = parse_model_refs(&stdout);

    let config_providers = crate::providers::list_providers(&paths.opencode_file())?;
    let custom_ids = crate::providers::custom_provider_ids(&paths.opencode_file())?;

    // Config models are authoritative for metadata and must stay listed.
    for provider in &config_providers {
        for model_id in provider.models.keys() {
            refs.insert(format!("{}/{}", provider.name, model_id));
        }
    }

    let mut catalog = Vec::with_capacity(refs.len());
    for model_ref in refs {
        let Some((provider_id, model_id)) = model_ref.split_once('/') else {
            continue;
        };
        if provider_filter.is_some_and(|filter| provider_id != filter) {
            continue;
        }
        let config_model = config_providers
            .iter()
            .find(|provider| provider.name == provider_id)
            .and_then(|provider| provider.models.get(model_id));
        let name = config_model
            .and_then(|model| model.name.clone())
            .unwrap_or_else(|| model_id.to_owned());
        let is_custom = custom_ids.contains(provider_id);
        catalog.push(ModelCatalogEntry {
            provider_id: provider_id.to_owned(),
            model_id: model_id.to_owned(),
            model_ref,
            name,
            is_custom,
            limit: config_model
                .and_then(|model| model.limit.clone())
                .and_then(|limit| serde_json::to_value(limit).ok()),
            variants: config_model
                .and_then(|model| model.variants.clone())
                .and_then(|variants| serde_json::to_value(variants).ok()),
        });
    }
    Ok(catalog)
}
