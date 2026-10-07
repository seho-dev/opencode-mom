use std::io::Read;
use std::process::{Command, Stdio};
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, USER_AGENT};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::error::AppError;

const LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/seho-dev/opencode-mom/releases/latest";
const REPOSITORY_URL: &str = "https://github.com/seho-dev/opencode-mom/";
const RELEASES_URL: &str = "https://github.com/seho-dev/opencode-mom/releases/latest";
const MAX_RESPONSE_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Macos,
    Windows,
    Linux,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub version: String,
    pub platform: Platform,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheck {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub update_available: bool,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    prerelease: bool,
    draft: bool,
}

pub fn app_info(version: String) -> AppInfo {
    AppInfo {
        version,
        platform: match std::env::consts::OS {
            "macos" => Platform::Macos,
            "windows" => Platform::Windows,
            "linux" => Platform::Linux,
            _ => Platform::Other,
        },
    }
}

fn stable_version(tag: &str) -> Result<[u32; 3], AppError> {
    let invalid =
        || AppError::configuration("Update check requires a stable X.Y.Z release version");
    if tag.len() > 33 {
        return Err(invalid());
    }
    let mut parts = tag.strip_prefix('v').unwrap_or(tag).split('.');
    let mut version = [0; 3];
    for component in &mut version {
        let part = parts.next().ok_or_else(invalid)?;
        if part.is_empty()
            || !part.bytes().all(|byte| byte.is_ascii_digit())
            || (part.len() > 1 && part.starts_with('0'))
        {
            return Err(invalid());
        }
        *component = part.parse().map_err(|_| invalid())?;
    }
    if parts.next().is_some() {
        return Err(invalid());
    }
    Ok(version)
}

pub fn check_for_updates(current_version: &str) -> Result<UpdateCheck, AppError> {
    stable_version(current_version)?;
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .connect_timeout(Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| AppError::configuration(format!("Initialize update check: {error}")))?;
    fetch_update(&client, current_version, LATEST_RELEASE_API)
}

fn fetch_update(
    client: &Client,
    current_version: &str,
    url: &str,
) -> Result<UpdateCheck, AppError> {
    let current = stable_version(current_version)?;
    let response = client
        .get(url)
        .header(
            USER_AGENT,
            format!("opencode-mom/{current_version} (+{REPOSITORY_URL})"),
        )
        .header(ACCEPT, "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .map_err(|error| AppError::configuration(format!("Check for updates: {error}")))?;
    let mut result = UpdateCheck {
        current_version: current_version.to_owned(),
        latest_version: None,
        update_available: false,
    };
    match response.status() {
        StatusCode::NOT_FOUND => return Ok(result),
        StatusCode::FORBIDDEN | StatusCode::TOO_MANY_REQUESTS => {
            return Err(AppError::configuration(
                "GitHub update check is unavailable or rate limited; try again later",
            ));
        }
        status if !status.is_success() => {
            return Err(AppError::configuration(format!(
                "Check for updates: GitHub returned HTTP {status}"
            )));
        }
        _ => {}
    }
    if response
        .content_length()
        .is_some_and(|length| length > MAX_RESPONSE_BYTES)
    {
        return Err(AppError::configuration(
            "GitHub release response is too large",
        ));
    }
    let mut body = Vec::new();
    response
        .take(MAX_RESPONSE_BYTES + 1)
        .read_to_end(&mut body)
        .map_err(|error| AppError::configuration(format!("Read GitHub release: {error}")))?;
    if body.len() as u64 > MAX_RESPONSE_BYTES {
        return Err(AppError::configuration(
            "GitHub release response is too large",
        ));
    }
    let release: Release = serde_json::from_slice(&body)
        .map_err(|_| AppError::configuration("GitHub returned an invalid release response"))?;
    if release.draft || release.prerelease {
        return Err(AppError::configuration(
            "GitHub did not return a published stable release",
        ));
    }
    let latest = stable_version(&release.tag_name)?;
    result.latest_version = Some(format!("{}.{}.{}", latest[0], latest[1], latest[2]));
    result.update_available = latest > current;
    Ok(result)
}

fn project_url(page: &str) -> Result<&'static str, AppError> {
    match page {
        "repository" => Ok(REPOSITORY_URL),
        "releases" => Ok(RELEASES_URL),
        _ => Err(AppError::validation("Unknown project page")),
    }
}

fn project_command(page: &str) -> Result<Command, AppError> {
    let url = project_url(page)?;
    #[cfg(target_os = "macos")]
    let mut command = Command::new("/usr/bin/open");
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("rundll32.exe");
        command.arg("url.dll,FileProtocolHandler");
        command
    };
    #[cfg(target_os = "linux")]
    let mut command = Command::new("xdg-open");
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        let _ = url;
        return Err(AppError::configuration(
            "Opening project pages is not supported on this platform",
        ));
    }
    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    {
        command.arg(url);
        Ok(command)
    }
}

pub fn open_project_page(page: &str) -> Result<(), AppError> {
    let status = project_command(page)?
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|error| AppError::configuration(format!("Open project page: {error}")))?;
    if !status.success() {
        return Err(AppError::configuration(format!(
            "Open project page failed: {status}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
