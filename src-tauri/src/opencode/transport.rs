use std::fs::OpenOptions;
use std::io::{self, Read};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::{StatusCode, Url};
use serde::Deserialize;

use super::token_usage::{failure, remaining, valid_text, Page, PAGE_MAX_BYTES, SCAN_TIMEOUT};
use crate::error::AppError;
use crate::opencode::binary::opencode_candidates;
use crate::opencode::command::collect_output;
use crate::opencode::target::{
    self, OpenCodeServiceStatus, Selection, SideEffectReservation, Snapshot,
};

const REGISTRATION_MAX_BYTES: u64 = 8192;
const INFO_MAX_BYTES: u64 = 32 * 1024;
const HEALTH_TIMEOUT: Duration = Duration::from_secs(5);
const BOOTSTRAP_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Deserialize)]
struct Registration {
    id: String,
    version: String,
    url: String,
    pid: u32,
    password: Option<String>,
}

#[derive(Deserialize)]
struct Info {
    version: String,
    pid: u32,
}

pub(in crate::opencode) struct ManagedConnection {
    client: Client,
    url: Url,
    registration: Registration,
    snapshot: Snapshot,
}

impl ManagedConnection {
    pub(in crate::opencode) fn into_async(self) -> Result<ManagedAsyncConnection, AppError> {
        self.ensure_current()?;
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .connect_timeout(HEALTH_TIMEOUT)
            .build()
            .map_err(|_| failure("managed API client initialization failed"))?;
        Ok(ManagedAsyncConnection {
            client,
            url: self.url,
            password: self.registration.password,
            pid: self.registration.pid,
            version: self.registration.version,
            snapshot: self.snapshot,
        })
    }

    fn new(client: Client, registration: Registration) -> Result<Self, AppError> {
        let invalid = || failure("invalid managed service registration");
        let port = registration
            .url
            .strip_prefix("http://127.0.0.1:")
            .or_else(|| registration.url.strip_prefix("http://[::1]:"))
            .and_then(|port| port.strip_suffix('/').unwrap_or(port).parse::<u16>().ok())
            .filter(|port| *port != 0)
            .ok_or_else(invalid)?;
        let url = Url::parse(&registration.url).map_err(|_| invalid())?;
        if url.scheme() != "http"
            || !matches!(url.host_str(), Some("127.0.0.1" | "[::1]"))
            || !url.username().is_empty()
            || url.password().is_some()
            || url.port_or_known_default() != Some(port)
            || url.path() != "/"
            || url.query().is_some()
            || url.fragment().is_some()
            || uuid::Uuid::parse_str(&registration.id).is_err()
            || registration.pid == 0
            || !valid_v2_version(&registration.version)
            || !registration
                .password
                .as_deref()
                .is_some_and(|password| valid_text(password, 4096))
        {
            return Err(invalid());
        }
        Ok(Self {
            client,
            url,
            registration,
            snapshot: Snapshot::detached(Selection::default()),
        })
    }

    fn request(
        &self,
        url: Url,
        deadline: Instant,
        timeout: Duration,
    ) -> Result<RequestBuilder, AppError> {
        self.ensure_current()?;
        let request = self
            .client
            .get(url)
            .timeout(remaining(deadline)?.min(timeout));
        Ok(match &self.registration.password {
            Some(password) => request.basic_auth("opencode", Some(password)),
            None => request,
        })
    }

    fn healthy(&self, deadline: Instant) -> Result<bool, AppError> {
        let mut url = self.url.clone();
        url.set_path("/api/info");
        let response = match self.request(url, deadline, HEALTH_TIMEOUT)?.send() {
            Ok(response) => response,
            Err(error) if error.is_connect() || error.is_timeout() => {
                remaining(deadline)?;
                return Ok(false);
            }
            Err(error) => return Err(request_failure(error)),
        };
        let info: super::active::JsonObject<Info> =
            response_json(response, deadline, INFO_MAX_BYTES, "health", "32 KiB")?;
        let info = info.0;
        validate_info(&info)?;
        Ok(info.pid == self.registration.pid && info.version == self.registration.version)
    }

    pub(in crate::opencode) fn ensure_current(&self) -> Result<(), AppError> {
        self.snapshot.ensure_current()
    }

    pub(in crate::opencode) fn directory(&self) -> Option<&Path> {
        self.snapshot.directory()
    }

    pub(in crate::opencode) fn get_json<T: for<'de> Deserialize<'de>>(
        &self,
        segments: &[&str],
        query: &[(&str, &str)],
        deadline: Instant,
        max_bytes: u64,
        operation: &str,
        limit: &str,
    ) -> Result<T, AppError> {
        let mut url = api_url(&self.url, segments)?;
        if !query.is_empty() {
            url.query_pairs_mut().extend_pairs(query.iter().copied());
        }
        let response = self
            .request(url, deadline, HEALTH_TIMEOUT)?
            .send()
            .map_err(request_failure)?;
        let value = response_json(response, deadline, max_bytes, operation, limit)?;
        self.ensure_current()?;
        Ok(value)
    }

    pub(in crate::opencode) fn reload(&self, deadline: Instant) -> Result<(), AppError> {
        let _reservation = self.snapshot.reserve_side_effect(deadline)?;
        if !self.healthy(deadline)? {
            return Err(failure(
                "service identity changed; reconnect before writing",
            ));
        }
        let mut url = api_url(&self.url, &["location", "reload"])?;
        if let Some(directory) = self.directory().and_then(|path| path.to_str()) {
            url.query_pairs_mut()
                .append_pair("location[directory]", directory);
        }
        let request = self
            .client
            .post(url)
            .timeout(remaining(deadline)?.min(HEALTH_TIMEOUT));
        let request = match &self.registration.password {
            Some(password) => request.basic_auth("opencode", Some(password)),
            None => request,
        };
        let response = request
            .send()
            .map_err(|_| failure("reload outcome unknown; do not automatically retry"))?;
        remaining(deadline)?;
        if response.status() != StatusCode::NO_CONTENT {
            return Err(failure(&format!(
                "reload API HTTP {}; do not automatically retry",
                response.status().as_u16()
            )));
        }
        self.ensure_current()
    }

    fn page_url(&self, session_id: Option<&str>, cursor: Option<&str>) -> Result<Url, AppError> {
        let mut url = self.url.clone();
        {
            let mut path = url
                .path_segments_mut()
                .map_err(|_| failure("invalid managed API base URL"))?;
            path.clear().push("api").push("session");
            if let Some(id) = session_id {
                if !valid_text(id, 4096) || matches!(id, "." | "..") {
                    return Err(failure("invalid session identifier"));
                }
                path.push(id).push("message");
            }
        }
        let mut query = url.query_pairs_mut();
        query.append_pair("limit", if session_id.is_some() { "100" } else { "1000" });
        if let Some(cursor) = cursor {
            if !valid_text(cursor, 4096) {
                return Err(failure("invalid page cursor"));
            }
            query.append_pair("cursor", cursor);
        } else {
            query.append_pair("order", "asc");
        }
        drop(query);
        Ok(url)
    }

    pub(super) fn fetch_page<T: for<'de> Deserialize<'de>>(
        &self,
        session_id: Option<&str>,
        cursor: Option<&str>,
        deadline: Instant,
    ) -> Result<Page<T>, AppError> {
        let response = self
            .request(self.page_url(session_id, cursor)?, deadline, SCAN_TIMEOUT)?
            .send()
            .map_err(request_failure)?;
        let page: Page<T> = response_json(response, deadline, PAGE_MAX_BYTES, "page", "32 MiB")?;
        self.ensure_current()?;
        if session_id.is_some() && page.data.len() > 100 {
            return Err(failure("message page item limit exceeded"));
        }
        Ok(page)
    }
}

pub(in crate::opencode) struct ManagedAsyncConnection {
    client: reqwest::Client,
    url: Url,
    password: Option<String>,
    pid: u32,
    version: String,
    snapshot: Snapshot,
}

impl ManagedAsyncConnection {
    pub(in crate::opencode) fn request(
        &self,
        method: reqwest::Method,
        segments: &[&str],
    ) -> Result<reqwest::RequestBuilder, AppError> {
        self.ensure_current()?;
        let url = api_url(&self.url, segments)?;
        let request = self.client.request(method, url);
        Ok(match &self.password {
            Some(password) => request.basic_auth("opencode", Some(password)),
            None => request,
        })
    }

    pub(in crate::opencode) async fn prepare_write(
        &self,
        deadline: Instant,
    ) -> Result<SideEffectReservation, AppError> {
        let snapshot = self.snapshot.clone();
        let reservation =
            tauri::async_runtime::spawn_blocking(move || snapshot.reserve_side_effect(deadline))
                .await
                .map_err(|_| failure("write reservation worker failed"))??;
        let mut response = self
            .request(reqwest::Method::GET, &["info"])?
            .timeout(remaining(deadline)?.min(HEALTH_TIMEOUT))
            .send()
            .await
            .map_err(request_failure)?;
        if response.status() != StatusCode::OK {
            return Err(failure(&format!("API HTTP {}", response.status().as_u16())));
        }
        if response
            .content_length()
            .is_some_and(|length| length > INFO_MAX_BYTES)
        {
            return Err(failure("health response exceeds the 32 KiB limit"));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(request_failure)? {
            if bytes.len() + chunk.len() > INFO_MAX_BYTES as usize {
                return Err(failure("health response exceeds the 32 KiB limit"));
            }
            bytes.extend_from_slice(&chunk);
        }
        let info: super::active::JsonObject<Info> = decode_json(&bytes, "health")?;
        validate_info(&info.0)?;
        if info.0.pid != self.pid || info.0.version != self.version {
            return Err(failure(
                "service identity changed; reconnect before writing",
            ));
        }
        remaining(deadline)?;
        self.ensure_current()?;
        Ok(reservation)
    }

    pub(in crate::opencode) fn ensure_current(&self) -> Result<(), AppError> {
        // PID/version are the identity verified at connect; all async requests pin this connection.
        if self.pid == 0 || !valid_v2_version(&self.version) {
            return Err(failure("invalid pinned service identity"));
        }
        self.snapshot.ensure_current()
    }
}

fn api_url(base: &Url, segments: &[&str]) -> Result<Url, AppError> {
    let mut url = base.clone();
    let mut path = url
        .path_segments_mut()
        .map_err(|_| failure("invalid managed API base URL"))?;
    path.clear().push("api");
    for segment in segments {
        if !valid_text(segment, 4096) || matches!(*segment, "." | "..") {
            return Err(failure("invalid managed API path segment"));
        }
        path.push(segment);
    }
    drop(path);
    Ok(url)
}

fn validate_info(info: &Info) -> Result<(), AppError> {
    if info.pid == 0 || !valid_v2_version(&info.version) {
        return Err(failure("invalid V2 service identity"));
    }
    Ok(())
}

fn valid_v2_version(version: &str) -> bool {
    if version.len() > 128 {
        return false;
    }
    let identifiers = |value: &str, prerelease: bool| {
        value.split('.').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                && !(prerelease
                    && part.len() > 1
                    && part.starts_with('0')
                    && part.bytes().all(|byte| byte.is_ascii_digit()))
        })
    };
    let (version, build) = version
        .split_once('+')
        .map_or((version, None), |(version, build)| (version, Some(build)));
    if build.is_some_and(|build| !identifiers(build, false)) {
        return false;
    }
    let (version, prerelease) = version
        .split_once('-')
        .map_or((version, None), |(version, prerelease)| {
            (version, Some(prerelease))
        });
    if prerelease.is_some_and(|prerelease| !identifiers(prerelease, true)) {
        return false;
    }
    let parts: Vec<_> = version.split('.').collect();
    parts.len() == 3
        && parts[0] == "2"
        && parts.iter().all(|part| {
            !part.is_empty()
                && (part.len() == 1 || !part.starts_with('0'))
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && part.parse::<u32>().is_ok()
        })
}

fn request_failure(error: reqwest::Error) -> AppError {
    failure(if error.is_timeout() {
        "API request timed out"
    } else if error.is_connect() {
        "API connection failed"
    } else {
        "API request failed"
    })
}

pub(in crate::opencode) fn decode_json<T: for<'de> Deserialize<'de>>(
    bytes: &[u8],
    operation: &str,
) -> Result<T, AppError> {
    serde_json::from_slice(bytes).map_err(|error| {
        // Serde and HTTP errors can include private values; retain only category/location.
        failure(&format!(
            "invalid {operation} ({:?} at line {}, column {})",
            error.classify(),
            error.line(),
            error.column()
        ))
    })
}

fn response_json<T: for<'de> Deserialize<'de>>(
    mut response: Response,
    deadline: Instant,
    max_bytes: u64,
    operation: &str,
    limit: &str,
) -> Result<T, AppError> {
    remaining(deadline)?;
    if response.status() != StatusCode::OK {
        return Err(failure(&format!("API HTTP {}", response.status().as_u16())));
    }
    let oversized = || failure(&format!("{operation} response exceeds the {limit} limit"));
    if response
        .content_length()
        .is_some_and(|size| size > max_bytes)
    {
        return Err(oversized());
    }
    let mut bytes = Vec::new();
    let mut buffer = [0; 8192];
    loop {
        remaining(deadline)?;
        let available = (max_bytes - bytes.len() as u64 + 1).min(buffer.len() as u64) as usize;
        let count = response
            .read(&mut buffer[..available])
            .map_err(|error| failure(&format!("API body read failed ({:?})", error.kind())))?;
        if count == 0 {
            break;
        }
        if bytes.len() as u64 + count as u64 > max_bytes {
            return Err(oversized());
        }
        bytes.extend_from_slice(&buffer[..count]);
    }
    remaining(deadline)?;
    let result = decode_json(&bytes, operation);
    drop(bytes);
    remaining(deadline)?;
    result
}

fn read_registration(path: &Path, deadline: Instant) -> Result<Option<Registration>, AppError> {
    remaining(deadline)?;
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    // Do not wait for a FIFO writer before inspecting the opened handle.
    options.custom_flags(libc::O_NONBLOCK);
    let file = match options.open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(failure(&format!(
                "registration read failed ({:?})",
                error.kind()
            )))
        }
    };
    if !file
        .metadata()
        .map_err(|error| failure(&format!("registration read failed ({:?})", error.kind())))?
        .is_file()
    {
        return Err(failure("invalid managed service registration file type"));
    }
    let mut bytes = Vec::new();
    file.take(REGISTRATION_MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| failure(&format!("registration read failed ({:?})", error.kind())))?;
    remaining(deadline)?;
    if bytes.len() as u64 > REGISTRATION_MAX_BYTES {
        return Err(failure("registration exceeds the 8 KiB limit"));
    }
    let result = decode_json(&bytes, "managed service registration");
    drop(bytes);
    result.map(Some)
}

fn connect_at(
    path: &Path,
    deadline: Instant,
    bootstrap: impl FnOnce(Instant) -> Result<(), AppError>,
) -> Result<ManagedConnection, AppError> {
    remaining(deadline)?;
    let client = client()?;
    if let Some(registration) = read_registration(path, deadline)? {
        let connection = ManagedConnection::new(client.clone(), registration)?;
        if connection.healthy(deadline)? {
            return Ok(connection);
        }
    }
    bootstrap(deadline)?;
    let registration = read_registration(path, deadline)?.ok_or_else(|| {
        failure("managed local service registration unavailable after CLI bootstrap")
    })?;
    let connection = ManagedConnection::new(client, registration)?;
    if !connection.healthy(deadline)? {
        return Err(failure(
            "managed API unavailable or registration identity mismatch",
        ));
    }
    Ok(connection)
}

fn client() -> Result<Client, AppError> {
    Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .retry(reqwest::retry::never())
        .connect_timeout(HEALTH_TIMEOUT)
        .build()
        .map_err(|_| failure("managed API client initialization failed"))
}

fn bootstrap_command(binary: &Path) -> Command {
    let mut command = Command::new(binary);
    command
        .args(["api", "get", "/api/info"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    command
}

fn bootstrap_cli(candidates: &[PathBuf], deadline: Instant) -> Result<(), AppError> {
    let mut last_error = None;
    for (index, binary) in candidates.iter().enumerate() {
        remaining(deadline)?;
        if !binary.exists() && index + 1 < candidates.len() {
            continue;
        }
        let child = match bootstrap_command(binary).spawn() {
            Ok(child) => child,
            Err(error) => {
                last_error = Some(error.kind());
                continue;
            }
        };
        let output = collect_output(
            child,
            deadline
                .saturating_duration_since(Instant::now())
                .min(BOOTSTRAP_TIMEOUT),
            INFO_MAX_BYTES,
            "opencode managed API bootstrap",
            "32 KiB",
        )?;
        remaining(deadline)?;
        if !output.status.success() {
            return Err(failure(
                "CLI bootstrap failed; managed OpenCode V2 service is required",
            ));
        }
        let info: super::active::JsonObject<Info> =
            decode_json(&output.stdout, "CLI bootstrap health")?;
        validate_info(&info.0)?;
        return Ok(());
    }
    Err(failure(&format!(
        "CLI bootstrap spawn failed ({last_error:?})"
    )))
}

pub(in crate::opencode) fn connect(deadline: Instant) -> Result<ManagedConnection, AppError> {
    connect_snapshot(target::snapshot()?, deadline)
}

pub(in crate::opencode) fn connect_snapshot(
    snapshot: Snapshot,
    deadline: Instant,
) -> Result<ManagedConnection, AppError> {
    let registration = snapshot.registration()?.to_owned();
    connect_target(snapshot, &registration, deadline, |deadline| {
        bootstrap_cli(&opencode_candidates(), deadline)
    })
}

pub(in crate::opencode) fn connect_target(
    snapshot: Snapshot,
    registration: &Path,
    deadline: Instant,
    bootstrap: impl FnOnce(Instant) -> Result<(), AppError>,
) -> Result<ManagedConnection, AppError> {
    let mut connection = if snapshot.selection.url.is_some() {
        explicit_connection(&snapshot.selection, deadline)?
    } else {
        connect_at(registration, deadline, |deadline| {
            let _reservation = snapshot.reserve_side_effect(deadline)?;
            bootstrap(deadline)
        })?
    };
    snapshot.ensure_current()?;
    connection.snapshot = snapshot;
    Ok(connection)
}

fn explicit_connection(
    selection: &Selection,
    deadline: Instant,
) -> Result<ManagedConnection, AppError> {
    let url = selection
        .url
        .as_deref()
        .ok_or_else(|| failure("explicit endpoint unavailable"))?;
    let url = target::normalize_url(url).map_err(|_| failure("invalid explicit endpoint"))?;
    let url = Url::parse(&url).map_err(|_| failure("invalid explicit endpoint"))?;
    let client = client()?;
    let request = client
        .get(api_url(&url, &["info"])?)
        .timeout(remaining(deadline)?.min(HEALTH_TIMEOUT));
    let request = match &selection.password {
        Some(password) => request.basic_auth("opencode", Some(password)),
        None => request,
    };
    let response = request.send().map_err(request_failure)?;
    let info: super::active::JsonObject<Info> =
        response_json(response, deadline, INFO_MAX_BYTES, "health", "32 KiB")?;
    validate_info(&info.0)?;
    Ok(ManagedConnection {
        client,
        url: url.clone(),
        registration: Registration {
            id: uuid::Uuid::new_v4().to_string(),
            url: url.to_string(),
            pid: info.0.pid,
            version: info.0.version,
            password: selection.password.clone(),
        },
        snapshot: Snapshot::detached(selection.clone()),
    })
}

pub(super) fn service_status(
    snapshot: Snapshot,
    deadline: Instant,
) -> Result<OpenCodeServiceStatus, AppError> {
    let target = snapshot.selection.public();
    let result = (|| {
        if snapshot.selection.url.is_some() {
            explicit_connection(&snapshot.selection, deadline)
        } else {
            let registration = read_registration(snapshot.registration()?, deadline)?;
            match registration {
                Some(registration) => {
                    let connection = ManagedConnection::new(client()?, registration)?;
                    if connection.healthy(deadline)? {
                        Ok(connection)
                    } else {
                        Err(failure("shared service unavailable or identity mismatch"))
                    }
                }
                None => Err(failure("shared service registration unavailable")),
            }
        }
    })();
    snapshot.ensure_current()?;
    Ok(match result {
        Ok(connection) => OpenCodeServiceStatus {
            target,
            connected: true,
            url: Some(connection.url.as_str().trim_end_matches('/').into()),
            pid: Some(connection.registration.pid),
            version: Some(connection.registration.version),
            error: None,
        },
        Err(error) => OpenCodeServiceStatus {
            target,
            connected: false,
            url: None,
            pid: None,
            version: None,
            error: Some(error.message),
        },
    })
}

#[cfg(test)]
pub(in crate::opencode) fn test_connection(url: &str, deadline: Instant) -> ManagedConnection {
    let connection = ManagedConnection::new(
        client().unwrap(),
        Registration {
            id: uuid::Uuid::new_v4().to_string(),
            version: "2.0.22".into(),
            url: url.into(),
            pid: 42,
            password: Some("private".into()),
        },
    )
    .unwrap();
    assert!(connection.healthy(deadline).unwrap());
    connection
}

#[cfg(test)]
mod tests;
