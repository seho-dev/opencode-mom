use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

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

    // npm's Windows shim also supports the modern @opencode/cli package.
    #[cfg(windows)]
    if let Ok(appdata) = env::var("APPDATA") {
        let npm = PathBuf::from(appdata).join("npm");
        candidates.push(npm.join("opencode.exe"));
        candidates.push(npm.join("opencode.cmd"));
        candidates.push(npm.join("node_modules/opencode-ai/bin/opencode.exe"));
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
        let name = if cfg!(windows) {
            "opencode.exe"
        } else {
            "opencode"
        };
        for directory in [
            ".opencode/bin",
            ".local/bin",
            ".npm-global/bin",
            ".bun/bin",
            ".cargo/bin",
        ] {
            candidates.push(home.join(directory).join(name));
        }
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

/// Resolve the exact executable without canonicalizing its logical installation path.
pub(super) fn resolve_path_command(command: &Path) -> Option<PathBuf> {
    resolve_path_command_in(
        command,
        env::var_os("PATH").as_deref(),
        &env::current_dir().ok()?,
    )
}

pub(super) fn resolve_path_command_in(
    command: &Path,
    path: Option<&OsStr>,
    cwd: &Path,
) -> Option<PathBuf> {
    if command.as_os_str().is_empty() {
        return None;
    }
    if command.is_absolute() || command.components().count() > 1 {
        let absolute = if command.is_absolute() {
            command.to_path_buf()
        } else {
            cwd.join(command)
        };
        return executable(&absolute).then_some(absolute);
    }
    for directory in env::split_paths(path?) {
        let directory = if directory.is_absolute() {
            directory
        } else {
            cwd.join(directory)
        };
        let candidate = directory.join(command);
        #[cfg(not(windows))]
        if executable(&candidate) {
            return Some(candidate);
        }
        #[cfg(windows)]
        {
            if candidate.extension().is_some() {
                if executable(&candidate) {
                    return Some(candidate);
                }
                continue;
            }
            // These are the native/shim extensions supported by Command on Windows.
            for extension in ["exe", "com", "cmd", "bat"] {
                let candidate = candidate.with_extension(extension);
                if executable(&candidate) {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

fn executable(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}
