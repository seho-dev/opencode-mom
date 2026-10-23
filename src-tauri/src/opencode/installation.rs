use std::fs::{self, OpenOptions};
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, ErrorCode};

use super::binary::{opencode_candidates, resolve_path_command};
use super::command::collect_output;

const LATEST_URL: &str = "https://opencode.ai/update/api/latest/cli/npm";
const PACKAGE: &str = "@opencode/cli";
const FORMULA: &str = "anomalyco/tap/opencode-v2";
const LOCAL_TIMEOUT: Duration = Duration::from_secs(15);
const PROCESS_TIMEOUT: Duration = Duration::from_secs(3);
const HTTP_TIMEOUT: Duration = Duration::from_secs(15);
// Verified native updater at 434f7b2a3670dd932794688bee5e34029e546630; earlier V2 builds stay manual.
const MIN_NATIVE_UPGRADE: [u32; 3] = [2, 0, 25];
// Native updater can wait 15 minutes for its flock, then run two five-minute curl operations.
const UPGRADE_TIMEOUT: Duration = Duration::from_secs(26 * 60);
static UPGRADE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenCodeInstallation {
    pub binary_path: Option<String>,
    pub version: Option<String>,
    pub method: String,
    pub can_upgrade: bool,
    pub upgrade_reason: Option<String>,
    pub manual_command: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenCodeUpdateCheck {
    pub installation: OpenCodeInstallation,
    pub latest_version: String,
    pub update_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Version {
    text: String,
    numbers: [u32; 3],
    unstable: bool,
}

fn version(text: &str) -> Option<Version> {
    let text = text.trim().strip_prefix('v').unwrap_or(text.trim());
    if text.is_empty() || text.len() > 96 || !text.is_ascii() {
        return None;
    }
    let (core, suffix) = text
        .split_once('-')
        .map_or((text, None), |(core, suffix)| (core, Some(suffix)));
    let mut numbers = [0; 3];
    let mut parts = core.split('.');
    for number in &mut numbers {
        let part = parts.next()?;
        if part.is_empty()
            || !part.bytes().all(|byte| byte.is_ascii_digit())
            || (part.len() > 1 && part.starts_with('0'))
        {
            return None;
        }
        *number = part.parse().ok()?;
    }
    if parts.next().is_some() {
        return None;
    }
    if let Some(suffix) = suffix {
        if suffix.split('.').any(|part| {
            part.is_empty()
                || !part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                || (part.len() > 1
                    && part.starts_with('0')
                    && part.bytes().all(|byte| byte.is_ascii_digit()))
        }) {
            return None;
        }
    }
    Some(Version {
        text: text.to_owned(),
        numbers,
        unstable: suffix.is_some(),
    })
}

fn stable_v2(text: &str) -> Result<Version, AppError> {
    version(text)
        .filter(|version| version.numbers[0] == 2 && !version.unstable)
        .ok_or_else(|| AppError::validation("A stable OpenCode V2 version is required"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Source {
    Curl {
        bash: PathBuf,
        curl: PathBuf,
        home: PathBuf,
    },
    Manager {
        method: &'static str,
        manager: PathBuf,
        root: PathBuf,
        bin: PathBuf,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Detected {
    info: OpenCodeInstallation,
    logical: Option<PathBuf>,
    canonical: Option<PathBuf>,
    source: Option<Source>,
}

fn manual_command(method: &str) -> Option<String> {
    Some(
        match method {
            "curl" => "curl -fsSL https://opencode.ai/v2/install | bash",
            "npm" => "npm install -g @opencode/cli@2",
            "bun" => "bun install -g --trust @opencode/cli@2",
            "pnpm" => "pnpm add -g --allow-build=@opencode/cli @opencode/cli@2",
            "yarn" => "yarn global add @opencode/cli@2",
            "vite-plus" => "vp install -g @opencode/cli@2",
            "brew" => "brew upgrade anomalyco/tap/opencode-v2",
            _ => return None,
        }
        .to_owned(),
    )
}

fn not_found() -> Detected {
    Detected {
        info: OpenCodeInstallation {
            binary_path: None,
            version: None,
            method: "not-found".to_owned(),
            can_upgrade: false,
            upgrade_reason: Some("not_found".to_owned()),
            manual_command: None,
        },
        logical: None,
        canonical: None,
        source: None,
    }
}

fn selected_binary() -> Option<PathBuf> {
    selected_binary_with(
        std::env::var_os("OPENCODE_BIN").as_deref(),
        opencode_candidates(),
        resolve_path_command,
    )
}

fn selected_binary_with(
    explicit: Option<&std::ffi::OsStr>,
    candidates: impl IntoIterator<Item = PathBuf>,
    mut resolve: impl FnMut(&Path) -> Option<PathBuf>,
) -> Option<PathBuf> {
    if let Some(explicit) = explicit {
        // An invalid override must never redirect an upgrade to another installation.
        return resolve(Path::new(explicit.to_str()?.trim()));
    }
    candidates
        .into_iter()
        .find_map(|candidate| resolve(&candidate))
}

fn sanitized(command: &mut Command) {
    command.env_clear();
    for key in [
        "PATH",
        "HOME",
        "USER",
        "LOGNAME",
        "TMPDIR",
        "TMP",
        "TEMP",
        "LANG",
        "LC_ALL",
        "SystemRoot",
        "COMSPEC",
        "APPDATA",
        "LOCALAPPDATA",
        "NPM_CONFIG_PREFIX",
        "BUN_INSTALL",
        "PNPM_HOME",
    ] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    command
        .env("OPENCODE_DISABLE_AUTOUPDATE", "1")
        .env("HOMEBREW_NO_AUTO_UPDATE", "1")
        .env("HOMEBREW_NO_INSTALL_FROM_API", "1")
        .env("HOMEBREW_NO_ANALYTICS", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
}

fn executable_command(path: &Path) -> Command {
    #[cfg(all(test, unix))]
    if tests::is_fake_executable(path) {
        let mut command = Command::new("/bin/sh");
        command.arg(path);
        return command;
    }
    Command::new(path)
}

fn read_output(
    command: &mut Command,
    deadline: Instant,
    max_bytes: u64,
    operation: &str,
) -> Result<Vec<u8>, AppError> {
    let timeout = deadline
        .checked_duration_since(Instant::now())
        .filter(|duration| !duration.is_zero())
        .ok_or_else(|| AppError::configuration(format!("{operation} timed out")))?
        .min(PROCESS_TIMEOUT);
    sanitized(command);
    let child = command.spawn().map_err(|error| {
        AppError::configuration(format!("{operation} could not start: {:?}", error.kind()))
    })?;
    let output = collect_output(child, timeout, max_bytes, operation, "bounded output")?;
    if !output.status.success() {
        return Err(AppError::configuration(format!(
            "{operation} failed (exit {:?})",
            output.status.code()
        )));
    }
    Ok(output.stdout)
}

fn read_version(path: &Path, deadline: Instant) -> Result<Option<Version>, AppError> {
    let bytes = read_output(
        executable_command(path).arg("--version"),
        deadline,
        1024,
        "OpenCode version query",
    )?;
    Ok(std::str::from_utf8(&bytes).ok().and_then(version))
}

fn query_path(manager: &Path, args: &[&str], deadline: Instant) -> Option<PathBuf> {
    let bytes = read_output(
        executable_command(manager).args(args),
        deadline,
        8192,
        "OpenCode manager ownership query",
    )
    .ok()?;
    let text = std::str::from_utf8(&bytes).ok()?.trim();
    if text.is_empty() || text.contains(['\n', '\r', '\0']) {
        return None;
    }
    let path = PathBuf::from(text);
    path.is_absolute().then_some(path)
}

fn bounded_file(path: &Path, max_bytes: u64) -> Option<Vec<u8>> {
    if !fs::symlink_metadata(path).ok()?.file_type().is_file() {
        return None;
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW);
    }
    let file = options.open(path).ok()?;
    let metadata = file.metadata().ok()?;
    if !metadata.is_file() || metadata.len() > max_bytes {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(max_bytes + 1).read_to_end(&mut bytes).ok()?;
    (bytes.len() as u64 <= max_bytes).then_some(bytes)
}

fn package_bin(root: &Path) -> Option<PathBuf> {
    let root = root.canonicalize().ok()?;
    let package = root.join("@opencode/cli");
    let bytes = bounded_file(&package.join("package.json"), 64 * 1024)?;
    let metadata: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    if metadata.get("name").and_then(|value| value.as_str()) != Some(PACKAGE) {
        return None;
    }
    let bin = metadata.get("bin");
    let bin = bin.and_then(|value| {
        value
            .as_str()
            .or_else(|| value.get("opencode").and_then(|value| value.as_str()))
    })?;
    let relative = Path::new(bin);
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_) | Component::CurDir))
    {
        return None;
    }
    let path = package.join(relative).canonicalize().ok()?;
    path.starts_with(root).then_some(path)
}

fn package_owns(root: &Path, binary: &Path) -> bool {
    package_bin(root).as_deref() == Some(binary)
}

fn global_bin_owns(root: &Path, bin: &Path, logical: &Path, canonical: &Path) -> bool {
    if package_owns(root, canonical)
        && ["opencode", "opencode.exe"]
            .iter()
            .any(|name| bin.join(name).canonicalize().ok().as_deref() == Some(canonical))
    {
        return true;
    }
    // Windows npm shims are readable installations, never automatic upgrade candidates.
    if cfg!(windows) && logical == bin.join("opencode.cmd") && package_bin(root).is_some() {
        return npm_native_shim(logical);
    }
    false
}

fn npm_native_shim(path: &Path) -> bool {
    bounded_file(path, 16 * 1024)
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .is_some_and(|text| {
            text.contains("\"%dp0%\\node_modules\\@opencode\\cli\\bin\\opencode.exe\"")
        })
}

fn bundled(path: &Path) -> bool {
    for ancestor in path.ancestors().skip(1) {
        let Some(name) = ancestor.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name == "Contents"
            && ancestor
                .parent()
                .is_some_and(|parent| parent.extension() == Some(std::ffi::OsStr::new("app")))
        {
            return true;
        }
        if name.eq_ignore_ascii_case("resources")
            && ancestor
                .parent()
                .and_then(|parent| parent.file_name())
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    ["opencode", "openchamber", "@openchamberelectron"]
                        .iter()
                        .any(|app| name.eq_ignore_ascii_case(app))
                })
        {
            return true;
        }
    }
    false
}

fn official_curl_path(logical: &Path, canonical: &Path, home: &Path) -> bool {
    let name = if cfg!(windows) {
        "opencode.exe"
    } else {
        "opencode"
    };
    let expected = home.join(".opencode/bin").join(name);
    logical == expected
        && fs::symlink_metadata(logical)
            .ok()
            .is_some_and(|metadata| metadata.file_type().is_file())
        && home
            .canonicalize()
            .ok()
            .is_some_and(|home| canonical == home.join(".opencode/bin").join(name))
}

fn brew_formula(path: &Path, deadline: Instant) -> Option<bool> {
    let cellar = path.ancestors().find(|ancestor| {
        ancestor.file_name() == Some(std::ffi::OsStr::new("opencode-v2"))
            && ancestor
                .parent()
                .is_some_and(|parent| parent.file_name() == Some(std::ffi::OsStr::new("Cellar")))
    })?;
    let manager = cellar.parent()?.parent()?.join("bin/brew");
    let manager = resolve_path_command(&manager)?;
    let prefix = query_path(&manager, &["--prefix", FORMULA], deadline)?
        .canonicalize()
        .ok()?;
    if prefix.join("bin/opencode").canonicalize().ok().as_deref() != Some(path)
        || !prefix.starts_with(cellar)
    {
        return Some(false);
    }
    let bytes = read_output(
        executable_command(&manager).args(["info", "--json=v2", "--installed", FORMULA]),
        deadline,
        64 * 1024,
        "Homebrew ownership query",
    )
    .ok()?;
    let metadata: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    Some(metadata.get("formulae")?.as_array()?.iter().any(|formula| {
        formula.get("name").and_then(|value| value.as_str()) == Some("opencode-v2")
            && formula.get("full_name").and_then(|value| value.as_str()) == Some(FORMULA)
            && formula
                .get("installed")
                .and_then(|value| value.as_array())
                .is_some_and(|values| !values.is_empty())
    }))
}

fn identify_source(
    logical: &Path,
    canonical: &Path,
    home: Option<&Path>,
    deadline: Instant,
) -> (String, Option<Source>, &'static str) {
    identify_source_with(logical, canonical, home, deadline, resolve_path_command)
}

fn identify_source_with(
    logical: &Path,
    canonical: &Path,
    home: Option<&Path>,
    deadline: Instant,
    mut resolve: impl FnMut(&Path) -> Option<PathBuf>,
) -> (String, Option<Source>, &'static str) {
    if bundled(logical) || bundled(canonical) {
        return ("bundled".to_owned(), None, "bundled");
    }
    if let Some(home) = home.filter(|home| official_curl_path(logical, canonical, home)) {
        #[cfg(unix)]
        if let (Some(bash), Some(curl)) = (resolve(Path::new("bash")), resolve(Path::new("curl"))) {
            return (
                "curl".to_owned(),
                Some(Source::Curl {
                    bash,
                    curl,
                    home: home.to_path_buf(),
                }),
                "",
            );
        }
        return (
            "curl".to_owned(),
            None,
            if cfg!(unix) {
                "manager_unavailable"
            } else {
                "unsupported"
            },
        );
    }
    if let Some(proven) = brew_formula(canonical, deadline) {
        // Homebrew has no verified version-pinned upgrade for this formula.
        return (
            if proven { "brew" } else { "unknown" }.to_owned(),
            None,
            if proven { "unsupported" } else { "ambiguous" },
        );
    }
    if !(canonical
        .ancestors()
        .any(|path| path.file_name() == Some(std::ffi::OsStr::new("node_modules")))
        || (cfg!(windows) && logical.file_name() == Some(std::ffi::OsStr::new("opencode.cmd"))))
    {
        return ("manual".to_owned(), None, "unsupported");
    }
    if let Some(manager) = resolve(Path::new("npm")) {
        if let (Some(root), Some(prefix)) = (
            query_path(&manager, &["root", "-g"], deadline),
            query_path(&manager, &["prefix", "-g"], deadline),
        ) {
            let expected_root = prefix.join(if cfg!(windows) {
                "node_modules"
            } else {
                "lib/node_modules"
            });
            let bin = if cfg!(windows) {
                prefix
            } else {
                prefix.join("bin")
            };
            if root.canonicalize().ok() == expected_root.canonicalize().ok()
                && global_bin_owns(&root, &bin, logical, canonical)
            {
                return (
                    "npm".to_owned(),
                    cfg!(unix).then_some(Source::Manager {
                        method: "npm",
                        manager,
                        root,
                        bin,
                    }),
                    if cfg!(unix) { "" } else { "unsupported" },
                );
            }
        }
    }
    // Other manager shims/layouts must match both their queried global root and package bin.
    for (name, method, root_args, bin_args) in [
        ("pnpm", "pnpm", &["root", "-g"][..], &["bin", "-g"][..]),
        (
            "yarn",
            "yarn",
            &["global", "dir"][..],
            &["global", "bin"][..],
        ),
        ("bun", "bun", &[][..], &["pm", "bin", "-g"][..]),
    ] {
        let Some(manager) = resolve(Path::new(name)) else {
            continue;
        };
        let Some(bin) = query_path(&manager, bin_args, deadline) else {
            continue;
        };
        let root = if method == "bun" {
            bin.parent()
                .map(|parent| parent.join("install/global/node_modules"))
        } else {
            query_path(&manager, root_args, deadline).map(|path| {
                if method == "yarn" {
                    path.join("node_modules")
                } else {
                    path
                }
            })
        };
        if let Some(root) = root.filter(|root| global_bin_owns(root, &bin, logical, canonical)) {
            return (
                method.to_owned(),
                cfg!(unix).then_some(Source::Manager {
                    method,
                    manager,
                    root,
                    bin,
                }),
                if cfg!(unix) { "" } else { "unsupported" },
            );
        }
    }
    ("unknown".to_owned(), None, "ambiguous")
}

fn inspect_path(logical: PathBuf, home: Option<&Path>) -> Result<Detected, AppError> {
    let deadline = Instant::now() + LOCAL_TIMEOUT;
    let canonical = logical.canonicalize().map_err(|error| {
        AppError::configuration(format!(
            "OpenCode binary is unavailable: {:?}",
            error.kind()
        ))
    })?;
    let current = read_version(&logical, deadline)?;
    let (method, source, source_reason) = identify_source(&logical, &canonical, home, deadline);
    let reason = if method == "bundled" {
        "bundled"
    } else {
        match &current {
            None => "unsupported",
            Some(current) if current.numbers[0] != 2 => "not_v2",
            Some(current) if current.unstable => "unstable",
            Some(current) if current.numbers < MIN_NATIVE_UPGRADE => "unsupported",
            Some(_) => source_reason,
        }
    };
    let binary_path = logical
        .to_str()
        .ok_or_else(|| AppError::configuration("OpenCode binary path is not valid UTF-8"))?
        .to_owned();
    Ok(Detected {
        info: OpenCodeInstallation {
            binary_path: Some(binary_path),
            version: current.map(|version| version.text),
            manual_command: manual_command(&method),
            method,
            can_upgrade: reason.is_empty(),
            upgrade_reason: (!reason.is_empty()).then(|| reason.to_owned()),
        },
        logical: Some(logical),
        canonical: Some(canonical),
        source,
    })
}

fn inspect_selected() -> Result<Detected, AppError> {
    match selected_binary() {
        Some(path) => inspect_path(path, dirs::home_dir().as_deref()),
        None => Ok(not_found()),
    }
}

/// Local discovery only; never contacts or starts the OpenCode service.
pub fn get_installation() -> Result<OpenCodeInstallation, AppError> {
    Ok(inspect_selected()?.info)
}

fn http_client() -> Result<Client, AppError> {
    Client::builder()
        .timeout(HTTP_TIMEOUT)
        .connect_timeout(Duration::from_secs(5))
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .build()
        .map_err(|_| AppError::configuration("OpenCode update HTTP initialization failed"))
}

fn fetch_bytes(client: &Client, url: &str, max_bytes: u64) -> Result<Vec<u8>, AppError> {
    let response = client
        .get(url)
        .header(reqwest::header::USER_AGENT, "opencode-mom")
        .send()
        .map_err(|_| AppError::configuration("OpenCode update request failed"))?;
    if !response.status().is_success() {
        return Err(AppError::configuration(format!(
            "OpenCode update request failed (HTTP {})",
            response.status().as_u16()
        )));
    }
    if response
        .content_length()
        .is_some_and(|length| length > max_bytes)
    {
        return Err(AppError::configuration(
            "OpenCode update response exceeds its size limit",
        ));
    }
    let mut bytes = Vec::new();
    response
        .take(max_bytes + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| AppError::configuration("OpenCode update response could not be read"))?;
    if bytes.len() as u64 > max_bytes {
        return Err(AppError::configuration(
            "OpenCode update response exceeds its size limit",
        ));
    }
    Ok(bytes)
}

#[derive(Deserialize)]
struct Latest {
    channel: String,
    name: String,
    distribution: String,
    version: String,
    metadata: LatestMetadata,
    active: bool,
}

#[derive(Deserialize)]
struct LatestMetadata {
    package: String,
}

fn latest_version(bytes: &[u8]) -> Result<Version, AppError> {
    let invalid = || {
        AppError::configuration("OpenCode update metadata is not an active stable V2 CLI release")
    };
    let latest: Latest = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    if latest.channel != "latest"
        || latest.name != "cli"
        || latest.distribution != "npm"
        || !latest.active
        || latest.metadata.package != PACKAGE
    {
        return Err(invalid());
    }
    stable_v2(&latest.version).map_err(|_| invalid())
}

fn fetch_latest() -> Result<Version, AppError> {
    latest_version(&fetch_bytes(&http_client()?, LATEST_URL, 16 * 1024)?)
}

pub fn check_for_update() -> Result<OpenCodeUpdateCheck, AppError> {
    let installation = get_installation()?;
    let latest = fetch_latest()?;
    let update_available = installation
        .version
        .as_deref()
        .and_then(version)
        .is_some_and(|current| {
            current.numbers[0] == 2 && !current.unstable && latest.numbers > current.numbers
        });
    Ok(OpenCodeUpdateCheck {
        installation,
        latest_version: latest.text,
        update_available,
    })
}

fn upgrade_command(detected: &Detected, target: &str) -> Result<Command, AppError> {
    let target = stable_v2(target)?.text;
    let source = detected
        .source
        .as_ref()
        .ok_or_else(|| AppError::validation("OpenCode installation ownership is unproven"))?;
    let binary = detected
        .logical
        .as_ref()
        .ok_or_else(|| AppError::validation("OpenCode binary is unavailable"))?;
    let method = match source {
        Source::Curl { .. } => "curl",
        Source::Manager { method, .. } => *method,
    };
    let mut command = executable_command(binary);
    command.args(["upgrade", &target, "--method", method]);
    sanitized(&mut command);
    command.stdout(Stdio::null());
    if let Source::Curl { home, .. } = source {
        command.env("HOME", home);
    }
    Ok(command)
}

#[cfg(unix)]
fn run_upgrade(command: &mut Command, timeout: Duration) -> Result<(), AppError> {
    use std::os::unix::process::CommandExt;

    command.process_group(0);
    let mut child = command.spawn().map_err(|error| {
        AppError::configuration(format!(
            "OpenCode upgrade could not start: {:?}",
            error.kind()
        ))
    })?;
    let group = child.id() as libc::pid_t;
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(status)) => {
                return Err(AppError::configuration(format!(
                    "OpenCode upgrade failed (exit {:?}); use the installation's manual command",
                    status.code()
                )))
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(AppError::configuration(format!(
                    "OpenCode upgrade wait failed: {:?}",
                    error.kind()
                )));
            }
            Ok(None) => {}
        }
        if Instant::now() >= deadline {
            // SAFETY: kill takes scalar arguments; the negative spawned PID selects the group requested
            // by process_group(0). This Child is unreaped here, but external reaping/PID reuse is not excluded.
            let _ = unsafe { libc::kill(-group, libc::SIGKILL) };
            let _ = child.kill();
            let _ = child.wait();
            return Err(AppError::configuration(
                "OpenCode upgrade timed out; check the selected installation before trying again",
            ));
        }
        std::thread::sleep(
            Duration::from_millis(20).min(deadline.saturating_duration_since(Instant::now())),
        );
    }
}

#[cfg(not(unix))]
fn run_upgrade(_: &mut Command, _: Duration) -> Result<(), AppError> {
    Err(AppError::configuration(
        "Automatic OpenCode upgrade is unsupported on this platform",
    ))
}

fn execute_upgrade(detected: &Detected, target: &str) -> Result<(), AppError> {
    run_upgrade(&mut upgrade_command(detected, target)?, UPGRADE_TIMEOUT)
}

fn verify_snapshot(
    detected: &Detected,
    expected_path: &str,
    expected_version: &str,
) -> Result<(), AppError> {
    if detected.info.binary_path.as_deref() != Some(expected_path)
        || detected.info.version.as_deref() != Some(expected_version)
    {
        return Err(AppError::validation(
            "OpenCode installation changed; refresh before upgrading",
        ));
    }
    if !detected.info.can_upgrade || detected.source.is_none() {
        return Err(AppError::validation(format!(
            "OpenCode automatic upgrade is unavailable ({}); use the installation's manual command",
            detected
                .info
                .upgrade_reason
                .as_deref()
                .unwrap_or("unsupported")
        )));
    }
    Ok(())
}

fn guarded_upgrade(
    expected_path: &str,
    expected_version: &str,
    target: &str,
    mut inspect: impl FnMut(Option<&Path>) -> Result<Detected, AppError>,
    latest: impl FnOnce() -> Result<Version, AppError>,
    execute: impl FnOnce(&Detected, &str) -> Result<(), AppError>,
) -> Result<OpenCodeInstallation, AppError> {
    let target = stable_v2(target)?;
    let current = stable_v2(expected_version)?;
    if current.numbers < MIN_NATIVE_UPGRADE {
        return Err(AppError::validation(
            "This OpenCode V2 version requires a manual upgrade",
        ));
    }
    if target.numbers <= current.numbers {
        return Err(AppError::validation(
            "OpenCode upgrades cannot downgrade or reinstall the current version",
        ));
    }
    let before = inspect(None)?;
    verify_snapshot(&before, expected_path, expected_version)?;
    if stable_v2(&latest()?.text)?.text != target.text {
        return Err(AppError::validation(
            "OpenCode target is no longer the latest stable V2 release; check again",
        ));
    }
    let refreshed = inspect(None)?;
    verify_snapshot(&refreshed, expected_path, expected_version)?;
    if before.logical != refreshed.logical
        || before.canonical != refreshed.canonical
        || before.source != refreshed.source
    {
        return Err(AppError::validation(
            "OpenCode installation source changed; refresh before upgrading",
        ));
    }
    execute(&refreshed, &target.text)?;
    let after = inspect(before.logical.as_deref())?;
    if after.logical != before.logical || after.info.version.as_deref() != Some(&target.text) {
        return Err(AppError::configuration("OpenCode upgrade finished, but the selected binary does not report the target version; no fallback installation was used"));
    }
    Ok(after.info)
}

pub fn upgrade(
    expected_binary_path: &str,
    expected_version: &str,
    target_version: &str,
) -> Result<OpenCodeInstallation, AppError> {
    let _guard = UPGRADE_LOCK
        .try_lock()
        .map_err(|_| AppError::new(ErrorCode::Busy, "OpenCode upgrade is already in progress"))?;
    guarded_upgrade(
        expected_binary_path,
        expected_version,
        target_version,
        |path| match path {
            Some(path) => inspect_path(path.to_path_buf(), dirs::home_dir().as_deref()),
            None => inspect_selected(),
        },
        fetch_latest,
        execute_upgrade,
    )
}

#[cfg(test)]
mod tests;
