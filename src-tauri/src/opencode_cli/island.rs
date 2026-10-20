use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};

use reqwest::{Method, RequestBuilder, Response};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::AppError;

use super::token_usage::transport::{self, ManagedAsyncConnection};

mod events;
mod projection;

pub(crate) use events::IslandChange;
pub(crate) type IslandEvents = events::IslandEvents;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_BYTES: usize = 2 * 1024 * 1024;
const MAX_SESSIONS: usize = 100;
const MAX_PENDING: usize = 100;
const MAX_FIELDS: usize = 32;
const MAX_OPTIONS: usize = 100;
const MAX_TEXT: usize = 16 * 1024;
const JS_MAX_DATE: u64 = 8_640_000_000_000_000;

#[derive(Default, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IslandSnapshot {
    pub sessions: Vec<IslandSession>,
    pub permissions: Vec<IslandPermission>,
    pub questions: Vec<IslandQuestion>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IslandSession {
    pub id: String,
    pub title: String,
    pub directory: String,
    pub updated_at: u64,
    pub parent_id: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IslandPermission {
    pub id: String,
    pub session_id: String,
    pub action: String,
    pub resources: Vec<String>,
    pub save: Vec<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IslandQuestion {
    pub id: String,
    pub session_id: String,
    pub title: String,
    pub answerable: bool,
    pub fields: Vec<IslandQuestionField>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IslandQuestionField {
    pub key: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub options: Vec<IslandQuestionOption>,
    pub custom: bool,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IslandQuestionOption {
    pub value: String,
    pub label: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IslandMessage {
    pub id: String,
    pub role: String,
    pub text: String,
    pub time: u64,
    pub truncated: bool,
}

#[derive(Clone)]
pub(crate) struct IslandApi {
    connection: Arc<ManagedAsyncConnection>,
}

pub(crate) async fn connect() -> Result<IslandApi, AppError> {
    let snapshot = super::target::snapshot()?;
    let deadline = Instant::now() + Duration::from_secs(20);
    let reservation = snapshot.reserve_side_effect(deadline)?;
    let connection = tauri::async_runtime::spawn_blocking(move || {
        // A dropped async caller cannot cancel a blocking bootstrap worker.
        let _reservation = reservation;
        transport::connect_snapshot(snapshot, deadline)?.into_async()
    })
    .await
    .map_err(|_| failure("connection worker failed"))??;
    Ok(IslandApi {
        connection: Arc::new(connection),
    })
}

#[cfg(test)]
pub(crate) fn test_api(url: &str) -> IslandApi {
    IslandApi {
        connection: Arc::new(
            transport::test_connection(url, Instant::now() + REQUEST_TIMEOUT)
                .into_async()
                .unwrap(),
        ),
    }
}

#[derive(Deserialize)]
struct Data<T> {
    data: T,
}

fn failure(kind: &str) -> AppError {
    AppError::configuration(format!("opencode island: {kind}."))
}

fn invalid(kind: &str) -> AppError {
    AppError::validation(format!("opencode island: invalid {kind}."))
}

fn identifier(value: &str, prefix: &str) -> Result<(), AppError> {
    if value.len() <= 256
        && value.strip_prefix(prefix).is_some_and(|suffix| {
            !suffix.is_empty()
                && suffix
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        })
    {
        Ok(())
    } else {
        Err(invalid("identifier"))
    }
}

fn plain(value: &str, max: usize, multiline: bool) -> Result<(), AppError> {
    if value.len() > max
        || value
            .chars()
            .any(|c| c.is_control() && !(multiline && matches!(c, '\n' | '\r' | '\t')))
    {
        Err(invalid("plaintext"))
    } else {
        Ok(())
    }
}

fn safe_time(value: u64) -> Result<u64, AppError> {
    if value <= JS_MAX_DATE {
        Ok(value)
    } else {
        Err(invalid("time"))
    }
}

fn request_error(error: reqwest::Error) -> AppError {
    failure(if error.is_timeout() {
        "API request timed out"
    } else if error.is_connect() {
        "API connection failed"
    } else {
        "API request failed"
    })
}

async fn body(mut response: Response) -> Result<Vec<u8>, AppError> {
    if response
        .content_length()
        .is_some_and(|size| size > MAX_BYTES as u64)
    {
        return Err(failure("response byte limit exceeded"));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(request_error)? {
        if bytes.len() + chunk.len() > MAX_BYTES {
            return Err(failure("response byte limit exceeded"));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

impl IslandApi {
    fn request(&self, method: Method, path: &[&str]) -> Result<RequestBuilder, AppError> {
        Ok(self
            .connection
            .request(method, path)?
            .timeout(REQUEST_TIMEOUT))
    }

    async fn read<T: for<'de> Deserialize<'de>>(
        &self,
        path: &[&str],
        query: &[(&str, &str)],
    ) -> Result<T, AppError> {
        let response = self
            .request(Method::GET, path)?
            .query(query)
            .send()
            .await
            .map_err(request_error)?;
        if response.status() != reqwest::StatusCode::OK {
            return Err(failure(&format!("API HTTP {}", response.status().as_u16())));
        }
        let value = transport::decode_json::<super::active::JsonObject<T>>(
            &body(response).await?,
            "island response",
        )
        .map(|value| value.0)?;
        self.connection.ensure_current()?;
        Ok(value)
    }

    async fn write(&self, request: RequestBuilder) -> Result<(), AppError> {
        let deadline = Instant::now() + REQUEST_TIMEOUT;
        let _reservation = self.connection.prepare_write(deadline).await?;
        let timeout = deadline
            .checked_duration_since(Instant::now())
            .filter(|timeout| !timeout.is_zero())
            .ok_or_else(|| failure("write timed out before dispatch"))?;
        let request = request.timeout(timeout);
        let response = request
            .send()
            .await
            .map_err(|_| failure("write outcome unknown; may have been accepted; do not retry"))?;
        let status = response.status();
        if !status.is_success() {
            if status.is_client_error() && status != reqwest::StatusCode::REQUEST_TIMEOUT {
                return Err(failure(&format!(
                    "API HTTP {}; write rejected",
                    status.as_u16()
                )));
            }
            return Err(failure(&format!(
                "API HTTP {}; write may have been accepted; do not retry",
                status.as_u16()
            )));
        }
        body(response)
            .await
            .map_err(|_| failure("write outcome unknown; may have been accepted; do not retry"))?;
        self.connection.ensure_current()?;
        Ok(())
    }

    pub(crate) async fn send(
        &self,
        session_id: &str,
        text: &str,
        message_id: &str,
    ) -> Result<(), AppError> {
        identifier(session_id, "ses_")?;
        identifier(message_id, "msg_")?;
        plain(text, MAX_TEXT, true)?;
        if text.trim().is_empty() {
            return Err(invalid("message"));
        }
        self.write(
            self.request(Method::POST, &["session", session_id, "prompt"])?
                .json(&serde_json::json!({ "id": message_id, "text": text, "delivery": "queue" })),
        )
        .await
    }

    pub(crate) async fn reply_permission(
        &self,
        session_id: &str,
        request_id: &str,
        decision: &str,
    ) -> Result<(), AppError> {
        identifier(session_id, "ses_")?;
        identifier(request_id, "per")?;
        if !matches!(decision, "once" | "always" | "reject") {
            return Err(invalid("permission decision"));
        }
        if decision == "always"
            && !self
                .permissions(session_id)
                .await
                .map_err(|_| invalid("pending permission"))?
                .iter()
                .any(|permission| {
                    permission.session_id == session_id
                        && permission.id == request_id
                        && !permission.save.is_empty()
                })
        {
            return Err(invalid("stale or unsavable permission"));
        }
        self.write(
            self.request(
                Method::POST,
                &["session", session_id, "permission", request_id, "reply"],
            )?
            .json(&serde_json::json!({ "decision": decision })),
        )
        .await
    }

    pub(crate) async fn interrupt(&self, session_id: &str) -> Result<(), AppError> {
        identifier(session_id, "ses_")?;
        self.write(
            self.request(Method::POST, &["session", session_id, "interrupt"])?
                .query(&[("resume", "false")]),
        )
        .await
    }

    pub(crate) async fn cancel_question(
        &self,
        session_id: &str,
        form_id: &str,
    ) -> Result<(), AppError> {
        identifier(session_id, "ses_")?;
        identifier(form_id, "frm_")?;
        self.write(self.request(Method::DELETE, &["session", session_id, "form", form_id])?)
            .await
    }
}

#[cfg(test)]
mod tests;
