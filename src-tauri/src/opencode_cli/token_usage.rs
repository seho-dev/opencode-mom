use std::collections::{HashMap, HashSet};
use std::mem::size_of;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use serde::{
    de::{self, IgnoredAny, SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};

use crate::error::AppError;

mod transport;

use transport::ManagedConnection;

const SCAN_TIMEOUT: Duration = Duration::from_secs(60);
const PAGE_MAX_BYTES: u64 = 32 * 1024 * 1024;
const MAX_RECORDS: usize = 100_000;
const MAX_METADATA_ITEMS: usize = 300_000;
const MAX_METADATA_BYTES: usize = 64 * 1024 * 1024;
const JS_MAX_INTEGER: u64 = 9_007_199_254_740_991;
const JS_MAX_DATE: u64 = 8_640_000_000_000_000;
const WORKERS: usize = 4;

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct TokenUsageRecord {
    pub time: u64,
    pub input: u64,
    pub output: u64,
}

#[derive(Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
struct Page<T> {
    #[serde(deserialize_with = "page_items")]
    data: Vec<T>,
    cursor: Cursor,
}

fn page_items<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Items<T>(std::marker::PhantomData<T>);
    impl<'de, T: Deserialize<'de>> Visitor<'de> for Items<T> {
        type Value = Vec<T>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("at most 1000 page items")
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Vec<T>, A::Error> {
            let mut items = Vec::new();
            while let Some(item) = sequence.next_element()? {
                if items.len() == 1000 {
                    return Err(de::Error::custom("page item limit exceeded"));
                }
                items.push(item);
            }
            Ok(items)
        }
    }
    deserializer.deserialize_seq(Items(std::marker::PhantomData))
}

#[derive(Deserialize)]
struct Cursor {
    next: Option<String>,
}

#[derive(Deserialize)]
struct Session {
    id: String,
    fork: Option<Fork>,
}

#[derive(Deserialize)]
struct Fork {
    #[serde(rename = "sessionID")]
    session_id: String,
    boundary: Boundary,
}

#[derive(Deserialize)]
struct Boundary {
    #[serde(rename = "type")]
    kind: BoundaryKind,
    #[serde(rename = "messageID")]
    message_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum BoundaryKind {
    Before,
    Through,
}

// No content, titles, model details, or error payloads enter the retained metadata.
#[derive(Deserialize)]
struct Message {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    time: MessageTime,
    tokens: Option<Tokens>,
    finish: Option<String>,
    status: Option<String>,
    #[serde(default, deserialize_with = "error_present")]
    error: bool,
}

#[derive(Deserialize, PartialEq, Eq)]
struct MessageTime {
    created: u64,
    completed: Option<u64>,
    streamed: Option<u64>,
}

#[derive(Deserialize, PartialEq, Eq)]
struct Tokens {
    input: u64,
    output: u64,
}

fn error_present<'de, D: Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
    struct ErrorValue;
    impl<'de> Visitor<'de> for ErrorValue {
        type Value = bool;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("an error payload or boolean")
        }

        fn visit_bool<E: de::Error>(self, value: bool) -> Result<bool, E> {
            Ok(value)
        }

        fn visit_none<E: de::Error>(self) -> Result<bool, E> {
            Ok(false)
        }

        fn visit_unit<E: de::Error>(self) -> Result<bool, E> {
            Ok(false)
        }

        fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<bool, D::Error> {
            deserializer.deserialize_any(ErrorValue)
        }

        fn visit_map<A: de::MapAccess<'de>>(self, mut access: A) -> Result<bool, A::Error> {
            while access.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
            Ok(true)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<bool, A::Error> {
            while access.next_element::<IgnoredAny>()?.is_some() {}
            Ok(true)
        }

        fn visit_str<E: de::Error>(self, value: &str) -> Result<bool, E> {
            Ok(!value.is_empty())
        }

        fn visit_string<E: de::Error>(self, value: String) -> Result<bool, E> {
            self.visit_str(&value)
        }
    }
    deserializer.deserialize_option(ErrorValue)
}

#[derive(Default)]
struct Budget {
    items: AtomicUsize,
    bytes: AtomicUsize,
}

impl Budget {
    fn reserve(&self, items: usize, bytes: usize) -> Result<(), AppError> {
        for (counter, amount, max) in [
            (&self.items, items, MAX_METADATA_ITEMS),
            (&self.bytes, bytes, MAX_METADATA_BYTES),
        ] {
            counter
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                    value.checked_add(amount).filter(|value| *value <= max)
                })
                .map_err(|_| failure("metadata limit exceeded"))?;
        }
        Ok(())
    }
}

fn failure(reason: &str) -> AppError {
    AppError::configuration(format!("opencode token usage: {reason}."))
}

fn remaining(deadline: Instant) -> Result<Duration, AppError> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|remaining| !remaining.is_zero())
        .ok_or_else(|| failure("scan timed out"))
}

fn valid_text(value: &str, max: usize) -> bool {
    !value.is_empty() && value.len() <= max && !value.chars().any(char::is_control)
}

fn inspect_session(session: &Session) -> Result<(&str, usize), AppError> {
    if !valid_text(&session.id, 4096) {
        return Err(failure("invalid session identifier"));
    }
    let mut bytes = size_of::<Session>() + 2 * session.id.len() + size_of::<String>();
    if let Some(fork) = &session.fork {
        if !valid_text(&fork.session_id, 4096) || !valid_text(&fork.boundary.message_id, 4096) {
            return Err(failure("invalid fork provenance"));
        }
        bytes += fork.session_id.len() + fork.boundary.message_id.len();
    }
    Ok((&session.id, bytes))
}

impl Message {
    fn usage(&self) -> Result<Option<&Tokens>, AppError> {
        if !matches!(self.kind.as_str(), "assistant" | "compaction") {
            return Ok(None);
        }
        let pending = match self.kind.as_str() {
            "compaction" => {
                self.status.as_deref() == Some("running")
                    && self.finish.is_none()
                    && !self.error
                    && self.time.completed.is_none()
            }
            "assistant" => {
                self.finish.is_none()
                    && !self.error
                    && self.time.completed.is_none()
                    && matches!(self.status.as_deref(), None | Some("running"))
            }
            _ => false,
        };
        if pending {
            Ok(None)
        } else {
            // Absent recorded usage does not establish zero provider billing.
            Ok(self.tokens.as_ref())
        }
    }

    fn copied_by_fork(&self) -> bool {
        match self.kind.as_str() {
            "assistant" => self.time.completed.is_some(),
            "shell" | "compaction" => self.status.as_deref() != Some("running"),
            _ => true,
        }
    }

    fn same_projection(&self, other: &Self) -> bool {
        self.kind == other.kind
            && self.time == other.time
            && self.tokens == other.tokens
            && self.finish == other.finish
            && self.status == other.status
            && self.error == other.error
    }
}

fn inspect_message(message: &Message) -> Result<(&str, usize), AppError> {
    if !valid_text(&message.id, 4096)
        || !valid_text(&message.kind, 256)
        || [&message.finish, &message.status]
            .into_iter()
            .flatten()
            .any(|value| !valid_text(value, 256))
        || [
            Some(message.time.created),
            message.time.completed,
            message.time.streamed,
        ]
        .into_iter()
        .flatten()
        .any(|time| time > JS_MAX_DATE)
    {
        return Err(failure("invalid message metadata"));
    }
    if let Some(tokens) = &message.tokens {
        safe_sum(tokens.input, tokens.output)?;
    }
    message.usage()?;
    Ok((
        &message.id,
        size_of::<Message>()
            + size_of::<String>()
            + 2 * message.id.len()
            + message.kind.len()
            + message.finish.as_ref().map_or(0, String::len)
            + message.status.as_ref().map_or(0, String::len),
    ))
}

fn safe_sum(left: u64, right: u64) -> Result<u64, AppError> {
    left.checked_add(right)
        .filter(|sum| *sum <= JS_MAX_INTEGER)
        .ok_or_else(|| failure("token counts exceed safe integer bounds"))
}

fn read_pages<T>(
    mut fetch: impl FnMut(Option<&str>) -> Result<Page<T>, AppError>,
    inspect: impl Fn(&T) -> Result<(&str, usize), AppError>,
    budget: &Budget,
    deadline: Instant,
) -> Result<Vec<T>, AppError> {
    let mut items = Vec::new();
    let mut ids = HashSet::new();
    let mut cursors = HashSet::new();
    let mut cursor: Option<String> = None;
    loop {
        remaining(deadline)?;
        let page = fetch(cursor.as_deref())?;
        remaining(deadline)?;
        let empty = page.data.is_empty();
        for item in page.data {
            let (id, bytes) = inspect(&item)?;
            budget.reserve(1, bytes)?;
            if !ids.insert(id.to_owned()) {
                return Err(failure("duplicate identifier across pages"));
            }
            items.push(item);
        }
        remaining(deadline)?;
        if empty {
            break;
        }
        let Some(next) = page.cursor.next else {
            break;
        };
        if !valid_text(&next, 4096) {
            return Err(failure("invalid page cursor"));
        }
        budget.reserve(0, 2 * next.len() + size_of::<String>())?;
        if !cursors.insert(next.clone()) {
            return Err(failure("page cursor loop"));
        }
        cursor = Some(next);
    }
    Ok(items)
}

fn read_timelines(
    connection: &ManagedConnection,
    sessions: &[Session],
    budget: &Budget,
    deadline: Instant,
) -> Result<Vec<Vec<Message>>, AppError> {
    let index = AtomicUsize::new(0);
    let failed = AtomicBool::new(false);
    thread::scope(|scope| {
        let mut workers = Vec::new();
        for _ in 0..WORKERS.min(sessions.len()) {
            workers.push(scope.spawn(|| {
                let result = (|| {
                    let mut timelines = Vec::new();
                    loop {
                        remaining(deadline)?;
                        if failed.load(Ordering::Relaxed) {
                            return Err(failure("scan cancelled after a page failure"));
                        }
                        let index = index.fetch_add(1, Ordering::Relaxed);
                        let Some(session) = sessions.get(index) else {
                            break;
                        };
                        let messages = read_pages(
                            |cursor| {
                                if failed.load(Ordering::Relaxed) {
                                    return Err(failure("scan cancelled after a page failure"));
                                }
                                connection.fetch_page(Some(&session.id), cursor, deadline)
                            },
                            inspect_message,
                            budget,
                            deadline,
                        )?;
                        timelines.push((index, messages));
                    }
                    Ok(timelines)
                })();
                if result.is_err() {
                    failed.store(true, Ordering::Relaxed);
                }
                result
            }));
        }
        let mut timelines = Vec::new();
        let mut error = None;
        // Every in-flight request and body read shares the scan deadline.
        for worker in workers {
            let worker_error = match worker.join() {
                Ok(Ok(mut result)) => {
                    timelines.append(&mut result);
                    continue;
                }
                Ok(Err(error)) => error,
                Err(_) => {
                    failed.store(true, Ordering::Relaxed);
                    failure("timeline worker failed")
                }
            };
            if error.as_ref().map_or(true, |error: &AppError| {
                error.message == failure("scan cancelled after a page failure").message
            }) {
                error = Some(worker_error);
            }
        }
        if let Some(error) = error {
            return Err(error);
        }
        remaining(deadline)?;
        timelines.sort_unstable_by_key(|(index, _)| *index);
        Ok(timelines
            .into_iter()
            .map(|(_, messages)| messages)
            .collect())
    })
}

fn fork_offsets(
    sessions: &[Session],
    timelines: &[Vec<Message>],
    deadline: Instant,
) -> Result<Vec<usize>, AppError> {
    let lookup: HashMap<_, _> = sessions
        .iter()
        .enumerate()
        .map(|(index, session)| (session.id.as_str(), index))
        .collect();
    let mut states = vec![0; sessions.len()];
    let mut offsets = vec![0; sessions.len()];
    for start in 0..sessions.len() {
        remaining(deadline)?;
        let mut path = Vec::new();
        let mut current = start;
        loop {
            remaining(deadline)?;
            match states[current] {
                2 => break,
                1 => return Err(failure("fork provenance unavailable: origin cycle")),
                _ => {}
            }
            states[current] = 1;
            path.push(current);
            let Some(fork) = &sessions[current].fork else {
                break;
            };
            current = *lookup
                .get(fork.session_id.as_str())
                .ok_or_else(|| failure("fork provenance unavailable: missing origin"))?;
        }
        for index in path.into_iter().rev() {
            remaining(deadline)?;
            if let Some(fork) = &sessions[index].fork {
                let origin = &timelines[lookup[fork.session_id.as_str()]];
                let boundary = origin
                    .iter()
                    .position(|message| message.id == fork.boundary.message_id)
                    .ok_or_else(|| failure("fork provenance unavailable: missing boundary"))?;
                let prefix = boundary
                    + match fork.boundary.kind {
                        BoundaryKind::Before => 0,
                        BoundaryKind::Through => 1,
                    };
                let child = &timelines[index];
                // Native forks copy eligible projected prefixes with new IDs, not a timestamp cutoff.
                let expected = origin[..prefix]
                    .iter()
                    .filter(|message| message.copied_by_fork());
                let copied = expected.clone().count();
                if child.len() < copied
                    || !expected
                        .zip(&child[..copied])
                        .all(|(original, copied)| original.same_projection(copied))
                {
                    return Err(failure(
                        "fork provenance unavailable: copied prefix mismatch",
                    ));
                }
                offsets[index] = copied;
            }
            states[index] = 2;
        }
    }
    Ok(offsets)
}

fn records(
    sessions: &[Session],
    timelines: &[Vec<Message>],
    deadline: Instant,
) -> Result<Vec<TokenUsageRecord>, AppError> {
    let offsets = fork_offsets(sessions, timelines, deadline)?;
    let mut records = Vec::new();
    let mut input = 0;
    let mut output = 0;
    for (timeline, offset) in timelines.iter().zip(offsets) {
        for message in &timeline[offset..] {
            remaining(deadline)?;
            let Some(tokens) = message.usage()? else {
                continue;
            };
            if records.len() == MAX_RECORDS {
                return Err(failure("100,000 record limit exceeded"));
            }
            input = safe_sum(input, tokens.input)?;
            output = safe_sum(output, tokens.output)?;
            safe_sum(input, output)?;
            records.push(TokenUsageRecord {
                time: message.time.created,
                input: tokens.input,
                output: tokens.output,
            });
        }
    }
    remaining(deadline)?;
    Ok(records)
}

pub fn token_usage_records_via_cli() -> Result<Vec<TokenUsageRecord>, AppError> {
    let deadline = Instant::now() + SCAN_TIMEOUT;
    let budget = Budget::default();
    let connection = transport::connect(deadline)?;
    let sessions = read_pages(
        |cursor| connection.fetch_page(None, cursor, deadline),
        inspect_session,
        &budget,
        deadline,
    )?;
    let timelines = read_timelines(&connection, &sessions, &budget, deadline)?;
    records(&sessions, &timelines, deadline)
}

#[cfg(test)]
mod tests;
