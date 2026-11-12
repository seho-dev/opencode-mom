use std::borrow::Borrow;
use std::collections::BTreeMap;

use super::*;

#[derive(Default)]
pub(super) struct Usage {
    records: BTreeMap<u64, TokenUsageRecord>,
    input: u64,
    output: u64,
}

impl Usage {
    pub(super) fn add(&mut self, message: &Message, budget: &Budget) -> Result<(), AppError> {
        let Some(tokens) = message.usage() else {
            return Ok(());
        };
        self.input = safe_sum(self.input, tokens.input)?;
        self.output = safe_sum(self.output, tokens.output)?;
        safe_sum(self.input, self.output)?;

        let record = match self.records.entry(message.time.created) {
            std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::btree_map::Entry::Vacant(entry) => {
                budget.reserve(
                    0,
                    2 * size_of::<TokenUsageRecord>() + 4 * size_of::<usize>(),
                )?;
                entry.insert(TokenUsageRecord {
                    time: message.time.created,
                    input: 0,
                    output: 0,
                })
            }
        };
        record.input = safe_sum(record.input, tokens.input)?;
        record.output = safe_sum(record.output, tokens.output)?;

        Ok(())
    }

    pub(super) fn records(self) -> Vec<TokenUsageRecord> {
        self.records.into_values().collect()
    }
}

struct Timeline<'a> {
    connection: &'a ManagedConnection,
    session: &'a Session,
    budget: &'a Budget,
    deadline: Instant,
    failed: &'a AtomicBool,
    ids: HashSet<String>,
    cursors: HashSet<String>,
    cursor: Option<String>,
    page: std::vec::IntoIter<Message>,
    finished: bool,
    bytes: usize,
}

impl<'a> Timeline<'a> {
    fn new(
        connection: &'a ManagedConnection,
        session: &'a Session,
        budget: &'a Budget,
        deadline: Instant,
        failed: &'a AtomicBool,
    ) -> Self {
        Self {
            connection,
            session,
            budget,
            deadline,
            failed,
            ids: HashSet::new(),
            cursors: HashSet::new(),
            cursor: None,
            page: Vec::new().into_iter(),
            finished: false,
            bytes: 0,
        }
    }

    fn reserve(&mut self, bytes: usize) -> Result<(), AppError> {
        self.budget.reserve(0, bytes)?;
        self.bytes += bytes;
        Ok(())
    }

    fn next(&mut self) -> Result<Option<Message>, AppError> {
        loop {
            remaining(self.deadline)?;
            if self.failed.load(Ordering::Relaxed) {
                return Err(failure("scan cancelled after a page failure"));
            }
            if let Some(message) = self.page.next() {
                inspect_message(&message)?;
                self.reserve(2 * (size_of::<String>() + message.id.len()))?;
                if !self.ids.insert(message.id.clone()) {
                    return Err(failure("duplicate identifier across pages"));
                }
                return Ok(Some(message));
            }
            if self.finished {
                return Ok(None);
            }

            let page: Page<Message> = self.connection.fetch_page(
                Some(&self.session.id),
                self.cursor.as_deref(),
                self.deadline,
            )?;
            self.finished = page.data.is_empty() || page.cursor.next.is_none();
            if !self.finished {
                let next = page.cursor.next.unwrap();
                if !valid_text(&next, 4096) {
                    return Err(failure("invalid page cursor"));
                }
                self.reserve(2 * (size_of::<String>() + next.len()))?;
                if !self.cursors.insert(next.clone()) {
                    return Err(failure("page cursor loop"));
                }
                self.cursor = Some(next);
            }
            self.page = page.data.into_iter();
        }
    }
}

impl Drop for Timeline<'_> {
    fn drop(&mut self) {
        self.budget.release_bytes(self.bytes);
    }
}

pub(super) fn fork_prefix<T: Borrow<Message>>(
    fork: &Fork,
    mut origin: impl FnMut() -> Result<Option<T>, AppError>,
    mut child: impl FnMut() -> Result<Option<T>, AppError>,
    deadline: Instant,
) -> Result<(), AppError> {
    loop {
        remaining(deadline)?;
        let original =
            origin()?.ok_or_else(|| failure("fork provenance unavailable: missing boundary"))?;
        let original = original.borrow();
        let boundary = original.id == fork.boundary.message_id;
        if boundary && matches!(fork.boundary.kind, BoundaryKind::Before) {
            break;
        }
        if original.copied_by_fork()
            && !child()?.is_some_and(|message| original.same_projection(message.borrow()))
        {
            return Err(failure(
                "fork provenance unavailable: copied prefix mismatch",
            ));
        }
        if boundary {
            break;
        }
    }

    Ok(())
}

pub(super) fn read_timeline(
    connection: &ManagedConnection,
    session: &Session,
    origin: Option<&Session>,
    budget: &Budget,
    usage: &Mutex<Usage>,
    deadline: Instant,
    failed: &AtomicBool,
) -> Result<(), AppError> {
    let mut timeline = Timeline::new(connection, session, budget, deadline, failed);
    if let (Some(fork), Some(origin)) = (&session.fork, origin) {
        // ponytail: reread fork prefixes to bound memory; cache on disk if fork-heavy scans need it.
        let mut origin = Timeline::new(connection, origin, budget, deadline, failed);
        fork_prefix(fork, || origin.next(), || timeline.next(), deadline)?;
    }
    while let Some(message) = timeline.next()? {
        // Skip usage-less messages before contending on the shared lock; `add` re-checks.
        if message.usage().is_none() {
            continue;
        }
        usage
            .lock()
            .map_err(|_| failure("timeline worker failed"))?
            .add(&message, budget)?;
    }
    remaining(deadline)?;

    Ok(())
}
