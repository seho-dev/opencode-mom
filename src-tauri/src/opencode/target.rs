use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, RwLock};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, ErrorCode};
use crate::paths::ConfigPaths;
use crate::private_file::{self, PublishError, PublishOutcome, ReadError};
use crate::protected_data::SecretBytes;

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
    let bytes = private_file::read(path, MAX_BYTES).map_err(|error| match error {
        ReadError::Unreadable => failure("private configuration unreadable"),
        ReadError::NotRegular | ReadError::MetadataTooLarge => {
            failure("configuration must be a bounded regular file")
        }
        ReadError::TooLarge => failure("configuration byte limit exceeded"),
        ReadError::UnsafePermissions => {
            failure("configuration must be account-owned with mode 0600")
        }
        #[cfg(windows)]
        ReadError::ReparsePoint => failure("configuration reparse points are not supported"),
    })?;
    let Some(bytes) = bytes else {
        return Ok(Selection::default());
    };
    let bytes = SecretBytes(bytes);
    #[cfg(windows)]
    return windows::decode(&bytes.0, crate::protected_data::unprotect);
    #[cfg(not(windows))]
    {
        let selection: super::active::JsonObject<Selection> = serde_json::from_slice(&bytes.0)
            .map_err(|_| failure("invalid private configuration"))?;
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
    #[cfg(not(windows))]
    let bytes = SecretBytes(
        serde_json::to_vec(selection).map_err(|_| failure("configuration serialization failed"))?,
    );
    #[cfg(windows)]
    let bytes = SecretBytes(windows::encode(selection, crate::protected_data::protect)?);
    if bytes.0.len() as u64 > MAX_BYTES {
        return Err(failure("configuration byte limit exceeded"));
    }
    match private_file::publish(path, &bytes.0).map_err(|error| {
        failure(match error {
            PublishError::DirectoryUnavailable => "configuration directory unavailable",
            PublishError::TemporaryCreate => "private temporary file creation failed",
            PublishError::TemporaryWrite => "private configuration write failed",
            PublishError::Publication => "private configuration publication failed",
        })
    })? {
        PublishOutcome::Committed => {}
        PublishOutcome::CommittedWithWarning => {
            eprintln!("OpenCode service target saved, but directory sync failed");
        }
    }
    Ok(())
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
    super::transport::service_status(
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
    super::transport::service_status(
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
