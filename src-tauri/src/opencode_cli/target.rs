use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, RwLock};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, ErrorCode};
use crate::paths::ConfigPaths;

#[cfg(not(windows))]
const MAX_BYTES: u64 = 8192;
#[cfg(windows)]
const MAX_BYTES: u64 = windows::MAX_FILE_BYTES as u64;

#[cfg(any(windows, test))]
mod windows;
static RUNTIME: OnceLock<Arc<TargetRuntime>> = OnceLock::new();
static TRANSITION: Mutex<()> = Mutex::new(());

#[derive(Clone, Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]
pub(super) struct Selection {
    #[serde(deserialize_with = "nullable_string")]
    pub(super) url: Option<String>,
    #[serde(deserialize_with = "nullable_string")]
    pub(super) password: Option<String>,
}

fn nullable_string<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    Option::deserialize(deserializer)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenCodeServiceTarget {
    pub mode: String,
    pub url: Option<String>,
    pub has_password: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenCodeServiceStatus {
    pub target: OpenCodeServiceTarget,
    pub connected: bool,
    pub url: Option<String>,
    pub pid: Option<u32>,
    pub version: Option<String>,
    pub error: Option<String>,
}

impl Selection {
    pub(super) fn public(&self) -> OpenCodeServiceTarget {
        OpenCodeServiceTarget {
            mode: if self.url.is_some() {
                "explicit"
            } else {
                "shared"
            }
            .into(),
            url: self.url.clone(),
            has_password: self.password.is_some(),
        }
    }
}

struct State {
    selection: Result<Selection, AppError>,
    revision: u64,
}

pub(crate) struct TargetRuntime {
    path: PathBuf,
    pub(super) registration: PathBuf,
    pub(super) directory: PathBuf,
    state: RwLock<State>,
    in_flight: AtomicUsize,
}

pub(super) struct SideEffectReservation(Option<Arc<TargetRuntime>>);

impl Drop for SideEffectReservation {
    fn drop(&mut self) {
        if let Some(runtime) = &self.0 {
            runtime.in_flight.fetch_sub(1, Ordering::SeqCst);
        }
    }
}

#[derive(Clone)]
pub(super) struct Snapshot {
    pub(super) selection: Selection,
    pub(super) revision: u64,
    runtime: Option<Arc<TargetRuntime>>,
}

impl Snapshot {
    pub(super) fn reserve_side_effect(
        &self,
        deadline: std::time::Instant,
    ) -> Result<SideEffectReservation, AppError> {
        let _transition = transition_until(deadline)?;
        self.ensure_current()?;
        if let Some(runtime) = &self.runtime {
            runtime.in_flight.fetch_add(1, Ordering::SeqCst);
        }
        Ok(SideEffectReservation(self.runtime.clone()))
    }

    pub(super) fn ensure_current(&self) -> Result<(), AppError> {
        if let Some(runtime) = &self.runtime {
            let state = runtime
                .state
                .read()
                .map_err(|_| failure("state unavailable"))?;
            if state.revision != self.revision {
                return Err(failure("target changed; retry the operation"));
            }
        }
        Ok(())
    }

    pub(super) fn directory(&self) -> Option<&Path> {
        self.runtime
            .as_ref()
            .map(|runtime| runtime.directory.as_path())
    }

    pub(super) fn registration(&self) -> Result<&Path, AppError> {
        self.runtime
            .as_ref()
            .map(|runtime| runtime.registration.as_path())
            .ok_or_else(|| failure("registration path unavailable"))
    }

    pub(super) fn detached(selection: Selection) -> Self {
        Self {
            selection,
            revision: 0,
            runtime: None,
        }
    }
}

pub(crate) fn transition() -> Result<MutexGuard<'static, ()>, AppError> {
    TRANSITION
        .lock()
        .map_err(|_| failure("transition unavailable"))
}

pub(super) fn transition_until(
    deadline: std::time::Instant,
) -> Result<MutexGuard<'static, ()>, AppError> {
    loop {
        if std::time::Instant::now() >= deadline {
            return Err(failure("transition timed out"));
        }
        match TRANSITION.try_lock() {
            Ok(guard) => return Ok(guard),
            Err(std::sync::TryLockError::Poisoned(_)) => {
                return Err(failure("transition unavailable"))
            }
            Err(std::sync::TryLockError::WouldBlock) => {
                std::thread::sleep(std::time::Duration::from_millis(5))
            }
        }
    }
}

fn failure(reason: &str) -> AppError {
    AppError::configuration(format!("OpenCode service target: {reason}."))
}

pub(super) fn normalize_url(value: &str) -> Result<String, AppError> {
    let invalid = || {
        AppError::validation("Use HTTP localhost, 127.0.0.1 or [::1] with an explicit positive port and no path, credentials, query or fragment")
    };
    if value.len() > 256 || value.chars().any(char::is_control) {
        return Err(invalid());
    }
    let authority = value.strip_prefix("http://").ok_or_else(invalid)?;
    let authority = authority.strip_suffix('/').unwrap_or(authority);
    let (host, port) = authority.rsplit_once(':').ok_or_else(invalid)?;
    if !matches!(host, "localhost" | "127.0.0.1" | "[::1]")
        || port.is_empty()
        || !port.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(invalid());
    }
    let port = port
        .parse::<u16>()
        .ok()
        .filter(|port| *port != 0)
        .ok_or_else(invalid)?;
    Ok(format!(
        "http://{}:{port}",
        if host == "localhost" {
            "127.0.0.1"
        } else {
            host
        }
    ))
}

fn password_valid(password: &str) -> bool {
    password.len() <= 4096 && !password.chars().any(char::is_control)
}

fn updated(
    old: Option<&Selection>,
    url: Option<String>,
    password: Option<String>,
) -> Result<Selection, AppError> {
    let Some(url) = url else {
        return Ok(Selection::default());
    };
    let url = normalize_url(&url)?;
    if password
        .as_deref()
        .is_some_and(|password| !password_valid(password))
    {
        return Err(AppError::validation(
            "Service password must be at most 4096 bytes without control characters",
        ));
    }
    let password = match password {
        Some(password) => (!password.is_empty()).then_some(password),
        None => old
            .filter(|old| old.url.as_deref() == Some(&url))
            .and_then(|old| old.password.clone()),
    };
    Ok(Selection {
        url: Some(url),
        password,
    })
}

fn load(path: &Path) -> Result<Selection, AppError> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x00200000);
    }
    let file = match options.open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Selection::default())
        }
        Err(_) => return Err(failure("private configuration unreadable")),
    };
    let metadata = file
        .metadata()
        .map_err(|_| failure("private configuration unreadable"))?;
    if !metadata.is_file() || metadata.len() > MAX_BYTES {
        return Err(failure("configuration must be a bounded regular file"));
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(failure("configuration reparse points are not supported"));
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.uid() != unsafe { libc::geteuid() } || metadata.mode() & 0o777 != 0o600 {
            return Err(failure(
                "configuration must be account-owned with mode 0600",
            ));
        }
    }
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| failure("private configuration unreadable"))?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(failure("configuration byte limit exceeded"));
    }
    #[cfg(windows)]
    return windows::decode(&bytes, windows::unprotect);
    #[cfg(not(windows))]
    {
        let selection: super::active::JsonObject<Selection> =
            serde_json::from_slice(&bytes).map_err(|_| failure("invalid private configuration"))?;
        validate_selection(selection.0)
    }
}

fn validate_selection(selection: Selection) -> Result<Selection, AppError> {
    match &selection.url {
        Some(url) => {
            let url = normalize_url(url).map_err(|_| failure("invalid configured endpoint"))?;
            if selection
                .password
                .as_deref()
                .is_some_and(|password| password.is_empty() || !password_valid(password))
            {
                return Err(failure("invalid configured password"));
            }
            Ok(Selection {
                url: Some(url),
                password: selection.password,
            })
        }
        None if selection.password.is_none() => Ok(selection),
        None => Err(failure("shared target cannot contain a password")),
    }
}

fn persist(path: &Path, selection: &Selection) -> Result<(), AppError> {
    let directory = path
        .parent()
        .ok_or_else(|| failure("configuration directory unavailable"))?;
    fs::create_dir_all(directory).map_err(|_| failure("configuration directory unavailable"))?;
    #[cfg(unix)]
    let directory_handle =
        File::open(directory).map_err(|_| failure("configuration directory unavailable"))?;
    #[cfg(not(windows))]
    let bytes =
        serde_json::to_vec(selection).map_err(|_| failure("configuration serialization failed"))?;
    #[cfg(windows)]
    let bytes = windows::encode(selection, windows::protect)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(failure("configuration byte limit exceeded"));
    }
    let temporary = directory.join(format!(
        ".opencode-service-{}.pending",
        uuid::Uuid::new_v4()
    ));
    let mut owned = false;
    let result = (|| {
        let mut file = private_file(&temporary)?;
        owned = true;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| failure("private configuration write failed"))?;
        drop(file);
        #[cfg(not(windows))]
        fs::rename(&temporary, path)
            .map_err(|_| failure("private configuration publication failed"))?;
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            use windows_sys::Win32::Storage::FileSystem::{
                MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
            };
            let source: Vec<u16> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
            let destination: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            if unsafe {
                MoveFileExW(
                    source.as_ptr(),
                    destination.as_ptr(),
                    MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
                )
            } == 0
            {
                return Err(failure("private configuration publication failed"));
            }
        }
        #[cfg(unix)]
        if directory_handle.sync_all().is_err() {
            // Publication already committed; keep disk and runtime selection consistent.
            eprintln!("OpenCode service target saved, but directory sync failed");
        }
        Ok(())
    })();
    // Only this create_new UUID path is ours; never remove somebody else's temporary file.
    if owned {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn private_file(path: &Path) -> Result<File, AppError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options
        .open(path)
        .map_err(|_| failure("private temporary file creation failed"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if file
            .set_permissions(fs::Permissions::from_mode(0o600))
            .is_err()
        {
            let _ = fs::remove_file(path);
            return Err(failure("private file permissions failed"));
        }
    }
    Ok(file)
}

impl TargetRuntime {
    pub(super) fn new(path: PathBuf, registration: PathBuf, directory: PathBuf) -> Arc<Self> {
        Arc::new(Self {
            state: RwLock::new(State {
                selection: load(&path),
                revision: 1,
            }),
            in_flight: AtomicUsize::new(0),
            path,
            registration,
            directory,
        })
    }

    pub(super) fn snapshot(self: &Arc<Self>) -> Result<Snapshot, AppError> {
        let state = self
            .state
            .read()
            .map_err(|_| failure("state unavailable"))?;
        Ok(Snapshot {
            selection: state.selection.clone()?,
            revision: state.revision,
            runtime: Some(self.clone()),
        })
    }

    #[cfg(test)]
    pub(super) fn set(
        self: &Arc<Self>,
        url: Option<String>,
        password: Option<String>,
        preflight: impl FnOnce() -> Result<(), AppError>,
    ) -> Result<Snapshot, AppError> {
        self.set_with_refresh(url, password, preflight, |()| ())
            .map(|(snapshot, ())| snapshot)
    }

    pub(super) fn set_with_refresh<G, R>(
        self: &Arc<Self>,
        url: Option<String>,
        password: Option<String>,
        prepare: impl FnOnce() -> Result<G, AppError>,
        commit: impl FnOnce(G) -> R,
    ) -> Result<(Snapshot, R), AppError> {
        let _transition = transition()?;
        if self.in_flight.load(Ordering::SeqCst) != 0 {
            return Err(AppError::new(
                ErrorCode::Busy,
                "OpenCode service operation still in flight; wait before changing the target",
            ));
        }
        let selection = {
            let state = self
                .state
                .read()
                .map_err(|_| failure("state unavailable"))?;
            updated(state.selection.as_ref().ok(), url, password)?
        };
        let guards = prepare()?;
        let mut state = self
            .state
            .write()
            .map_err(|_| failure("state unavailable"))?;
        persist(&self.path, &selection)?;
        state.selection = Ok(selection.clone());
        state.revision += 1;
        let snapshot = Snapshot {
            selection,
            revision: state.revision,
            runtime: Some(self.clone()),
        };
        drop(state);
        // Only local invalidation is allowed here; native work/readers wait until finish.
        let refresh = commit(guards);
        Ok((snapshot, refresh))
    }
}

pub(crate) fn initialize(paths: &ConfigPaths) -> Result<(), AppError> {
    let registration =
        registration_path(paths, std::env::var_os("XDG_STATE_HOME").map(PathBuf::from));
    let directory =
        std::env::current_dir().map_err(|_| failure("working directory unavailable"))?;
    RUNTIME
        .set(TargetRuntime::new(
            paths.app_config_dir().join("opencode-service.json"),
            registration,
            directory,
        ))
        .map_err(|_| failure("already initialized"))
}

fn registration_path(paths: &ConfigPaths, state_home: Option<PathBuf>) -> PathBuf {
    state_home
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| paths.home_dir().join(".local/state"))
        .join("opencode/service.json")
}

pub(crate) fn runtime() -> Result<&'static Arc<TargetRuntime>, AppError> {
    RUNTIME.get().ok_or_else(|| failure("not initialized"))
}

pub(super) fn snapshot() -> Result<Snapshot, AppError> {
    runtime()?.snapshot()
}

pub(crate) fn get_status() -> Result<OpenCodeServiceStatus, AppError> {
    super::token_usage::transport::service_status(
        snapshot()?,
        std::time::Instant::now() + std::time::Duration::from_secs(5),
    )
}

pub(crate) fn set_target<G, R>(
    url: Option<String>,
    password: Option<String>,
    prepare: impl FnOnce() -> Result<G, AppError>,
    commit: impl FnOnce(G) -> R,
    finish: impl FnOnce(R),
) -> Result<OpenCodeServiceStatus, AppError> {
    let (snapshot, refresh) = runtime()?.set_with_refresh(url, password, prepare, commit)?;
    Ok(finish_target(snapshot, refresh, finish))
}

fn finish_target<R>(
    snapshot: Snapshot,
    refresh: R,
    finish: impl FnOnce(R),
) -> OpenCodeServiceStatus {
    let target = snapshot.selection.public();
    finish(refresh);
    super::token_usage::transport::service_status(
        snapshot,
        std::time::Instant::now() + std::time::Duration::from_secs(5),
    )
    .unwrap_or_else(|_| OpenCodeServiceStatus {
        target,
        connected: false,
        url: None,
        pid: None,
        version: None,
        error: Some("OpenCode service target changed or connection status unavailable".into()),
    })
}

#[cfg(test)]
mod tests;
