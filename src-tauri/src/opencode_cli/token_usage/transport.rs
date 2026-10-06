use std::fs::OpenOptions;
use std::io::{self, Read};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::{StatusCode, Url};

use super::{failure, remaining, valid_text, AppError, Deserialize, Duration, Instant, Page};
use super::{PAGE_MAX_BYTES, SCAN_TIMEOUT};
use crate::opencode_cli::binary::opencode_candidates;
use crate::opencode_cli::command::collect_output;

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
    password: String,
}

#[derive(Deserialize)]
struct Info {
    version: String,
    pid: u32,
}

pub(super) struct ManagedConnection {
    client: Client,
    url: Url,
    registration: Registration,
}

impl ManagedConnection {
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
            || !valid_text(&registration.version, 128)
            || !registration.version.starts_with("2.")
            || !valid_text(&registration.password, 4096)
        {
            return Err(invalid());
        }
        Ok(Self {
            client,
            url,
            registration,
        })
    }

    fn request(
        &self,
        url: Url,
        deadline: Instant,
        timeout: Duration,
    ) -> Result<RequestBuilder, AppError> {
        Ok(self
            .client
            .get(url)
            .basic_auth("opencode", Some(&self.registration.password))
            .timeout(remaining(deadline)?.min(timeout)))
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
        let info: Info = response_json(response, deadline, INFO_MAX_BYTES, "health", "32 KiB")?;
        Ok(info.pid == self.registration.pid && info.version == self.registration.version)
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
        if session_id.is_some() && page.data.len() > 100 {
            return Err(failure("message page item limit exceeded"));
        }
        Ok(page)
    }
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

fn decode_json<T: for<'de> Deserialize<'de>>(bytes: &[u8], operation: &str) -> Result<T, AppError> {
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

fn registration_path() -> Result<PathBuf, AppError> {
    let state = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|home| home.join(".local/state")))
        .ok_or_else(|| failure("managed service state directory unavailable"))?;
    Ok(state.join("opencode/service.json"))
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
        let _: Info = decode_json(&output.stdout, "CLI bootstrap health")?;
        return Ok(());
    }
    Err(failure(&format!(
        "CLI bootstrap spawn failed ({last_error:?})"
    )))
}

pub(super) fn connect(deadline: Instant) -> Result<ManagedConnection, AppError> {
    // Large CLI API responses can be truncated by process.exit; only bootstrap uses stdout.
    connect_at(&registration_path()?, deadline, |deadline| {
        bootstrap_cli(&opencode_candidates(), deadline)
    })
}

#[cfg(test)]
pub(super) fn test_connection(url: &str, deadline: Instant) -> ManagedConnection {
    let connection = ManagedConnection::new(
        client().unwrap(),
        Registration {
            id: uuid::Uuid::new_v4().to_string(),
            version: "2.0.22".into(),
            url: url.into(),
            pid: 42,
            password: "private".into(),
        },
    )
    .unwrap();
    assert!(connection.healthy(deadline).unwrap());
    connection
}

#[cfg(test)]
mod tests;
