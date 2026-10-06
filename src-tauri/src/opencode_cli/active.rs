use std::collections::HashSet;
use std::fmt;
use std::marker::PhantomData;
use std::path::Path;
use std::process::{Child, Command, Output, Stdio};
use std::time::Duration;

use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};

use crate::error::AppError;

use super::binary::opencode_candidates;
use super::command::collect_output;

const ACTIVE_SESSIONS_TIMEOUT: Duration = Duration::from_secs(5);
const ACTIVE_SESSIONS_MAX_BYTES: u64 = 1024 * 1024;

#[derive(Deserialize)]
struct ActiveSessionsEnvelope {
    #[serde(deserialize_with = "count_running_sessions")]
    data: usize,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum ActiveSession {
    #[serde(rename = "running")]
    Running,
}

struct JsonObject<T>(T);

impl<'de, T: Deserialize<'de>> Deserialize<'de> for JsonObject<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ObjectVisitor<T>(PhantomData<T>);

        impl<'de, T: Deserialize<'de>> Visitor<'de> for ObjectVisitor<T> {
            type Value = JsonObject<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a JSON object")
            }

            fn visit_map<M: MapAccess<'de>>(self, map: M) -> Result<Self::Value, M::Error> {
                T::deserialize(de::value::MapAccessDeserializer::new(map)).map(JsonObject)
            }
        }

        deserializer.deserialize_map(ObjectVisitor(PhantomData))
    }
}

fn count_running_sessions<'de, D>(deserializer: D) -> Result<usize, D::Error>
where
    D: Deserializer<'de>,
{
    struct RunningSessions;

    impl<'de> Visitor<'de> for RunningSessions {
        type Value = usize;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a map of running session IDs")
        }

        fn visit_map<M>(self, mut map: M) -> Result<usize, M::Error>
        where
            M: MapAccess<'de>,
        {
            let mut ids = HashSet::new();
            while let Some(id) = map.next_key::<String>()? {
                let valid = id.strip_prefix("ses_").is_some_and(|suffix| {
                    !suffix.is_empty()
                        && suffix.bytes().all(|byte| {
                            byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-'
                        })
                });
                if !valid || !ids.insert(id) {
                    return Err(de::Error::custom("invalid or duplicate session ID"));
                }
                map.next_value::<JsonObject<ActiveSession>>()?;
            }
            Ok(ids.len())
        }
    }

    deserializer.deserialize_map(RunningSessions)
}

fn active_sessions_command(binary: &Path) -> Command {
    let mut command = Command::new(binary);
    command
        .args(["api", "session.active"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    command
}

/// Queries only the server resolved by the CLI, which may auto-start its local service.
/// This is not a machine-wide inventory of OpenCode processes or servers.
pub fn active_sessions_via_cli() -> Result<usize, AppError> {
    let candidates = opencode_candidates();
    let mut last_error = None;
    for (index, binary) in candidates.iter().enumerate() {
        if !binary.exists() && index + 1 < candidates.len() {
            continue;
        }
        match active_sessions_command(binary).spawn() {
            Ok(child) => return collect_active_sessions(child, ACTIVE_SESSIONS_TIMEOUT),
            Err(error) => last_error = Some(error.kind()),
        }
    }
    Err(AppError::configuration(format!(
        "failed to run opencode CLI for session.active ({last_error:?}). Install OpenCode V2 and ensure it is on PATH, or set OPENCODE_BIN."
    )))
}

fn collect_active_sessions(child: Child, timeout: Duration) -> Result<usize, AppError> {
    active_sessions_result(collect_output(
        child,
        timeout,
        ACTIVE_SESSIONS_MAX_BYTES,
        "opencode session.active",
        "1 MiB",
    )?)
}

fn active_sessions_result(output: Output) -> Result<usize, AppError> {
    if !output.status.success() {
        return Err(AppError::configuration(format!(
            "opencode session.active failed ({}). This requires OpenCode V2 with the session.active API; update the CLI if this endpoint is unsupported.",
            output.status
        )));
    }
    let envelope: JsonObject<ActiveSessionsEnvelope> =
        serde_json::from_slice(&output.stdout).map_err(|error| {
            // Do not expose identifiers, response contents, or serde's untrusted error text.
            AppError::configuration(format!(
                "invalid opencode session.active response: expected the V2 data map of running sessions ({:?} at line {}, column {}).",
                error.classify(), error.line(), error.column()
            ))
        })?;
    Ok(envelope.0.data)
}

#[cfg(all(test, unix))]
mod tests;
