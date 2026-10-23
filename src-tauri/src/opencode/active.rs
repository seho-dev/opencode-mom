use std::collections::HashSet;
use std::fmt;
use std::marker::PhantomData;
use std::time::{Duration, Instant};

use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};

use crate::error::AppError;

use super::transport::{self, ManagedConnection};

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

pub(super) struct JsonObject<T>(pub(super) T);

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
    running_sessions(deserializer).map(|ids| ids.len())
}

pub(super) fn running_sessions<'de, D>(deserializer: D) -> Result<HashSet<String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct RunningSessions;

    impl<'de> Visitor<'de> for RunningSessions {
        type Value = HashSet<String>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a map of running session IDs")
        }

        fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
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
            Ok(ids)
        }
    }

    deserializer.deserialize_map(RunningSessions)
}

/// Queries only the selected local service; Shared mode may bootstrap through the CLI.
/// This is not a machine-wide inventory of OpenCode processes or servers.
pub fn active_sessions_via_service() -> Result<usize, AppError> {
    let deadline = Instant::now() + ACTIVE_SESSIONS_TIMEOUT;
    active_sessions(&transport::connect(deadline)?, deadline)
}

pub(super) fn active_sessions(
    connection: &ManagedConnection,
    deadline: Instant,
) -> Result<usize, AppError> {
    let envelope: JsonObject<ActiveSessionsEnvelope> = connection.get_json(
        &["session", "active"],
        &[],
        deadline,
        ACTIVE_SESSIONS_MAX_BYTES,
        "opencode session.active",
        "1 MiB",
    )?;
    Ok(envelope.0.data)
}

#[cfg(test)]
mod tests;
