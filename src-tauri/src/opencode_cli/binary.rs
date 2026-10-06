use std::env;
use std::path::PathBuf;

/// Candidate opencode binaries, in priority order:
/// 1. `$OPENCODE_BIN` explicit override (if the file exists)
/// 2. Known install locations — probed directly so the app works even when
///    launched with a minimal PATH (e.g. GUI launch on Windows)
/// 3. Plain `opencode` — resolved through PATH by `Command::new` at spawn time
pub(super) fn opencode_candidates() -> Vec<PathBuf> {
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
