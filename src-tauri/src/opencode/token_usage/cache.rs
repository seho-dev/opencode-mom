use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::{failure, scan, TokenUsageRecord, SCAN_TIMEOUT};
use crate::error::AppError;
use crate::opencode::{target, transport};

#[cfg(test)]
mod tests;

const TTL: Duration = Duration::from_secs(5 * 60);
static CACHE: Mutex<Cache> = Mutex::new(Cache::new());
static DIRECTORY: OnceLock<PathBuf> = OnceLock::new();

pub fn initialize_usage_cache(path: PathBuf) {
    let _ = DIRECTORY.set(path.join("token-usage"));
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DiskCache {
    version: u32,
    key: String,
    updated_at: u64,
    records: Vec<TokenUsageRecord>,
}

fn cache_path() -> Option<PathBuf> {
    let directory = DIRECTORY.get()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, MetadataExt};
        let mut builder = std::fs::DirBuilder::new();
        builder.mode(0o700);
        if !directory.exists() {
            builder.create(directory).ok()?;
        }
        let metadata = std::fs::symlink_metadata(directory).ok()?;
        // SAFETY: geteuid takes no arguments and has no memory-safety preconditions.
        let uid = unsafe { libc::geteuid() };
        if !metadata.is_dir() || metadata.uid() != uid || metadata.mode() & 0o777 != 0o700 {
            return None;
        }
        Some(directory.join("token-usage.json"))
    }
    // No private-directory ACL contract is available on this platform yet.
    #[cfg(not(unix))]
    {
        let _ = directory;
        None
    }
}

fn read_disk(key: &str) -> Option<DiskCache> {
    let bytes = crate::private_file::read(&cache_path()?, 32 * 1024 * 1024).ok()??;
    let disk: DiskCache = serde_json::from_slice(&bytes).ok()?;
    if disk.version != 1 || disk.key != key || disk.updated_at > super::JS_MAX_DATE {
        return None;
    }
    let mut input = 0;
    let mut output = 0;
    for record in &disk.records {
        if record.time > super::JS_MAX_DATE {
            return None;
        }
        input = super::safe_sum(input, record.input).ok()?;
        output = super::safe_sum(output, record.output).ok()?;
        super::safe_sum(input, output).ok()?;
    }
    Some(disk)
}

fn write_disk(key: String, updated_at: u64, records: &[TokenUsageRecord]) {
    let Some(path) = cache_path() else {
        return;
    };
    let Ok(bytes) = serde_json::to_vec(&DiskCache {
        version: 1,
        key,
        updated_at,
        records: records.to_vec(),
    }) else {
        return;
    };
    if bytes.len() <= 32 * 1024 * 1024 {
        let _ = crate::private_file::publish(&path, &bytes);
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsageSnapshot {
    pub records: Vec<TokenUsageRecord>,
    pub updated_at: Option<u64>,
    pub refreshing: bool,
    pub stale: bool,
    pub error: Option<AppError>,
    pub target_revision: u64,
    pub cache_scope: &'static str,
    pub result_revision: u64,
}

struct Cache {
    revision: u64,
    records: Vec<TokenUsageRecord>,
    updated_at: Option<u64>,
    refreshed_at: Option<Instant>,
    attempted_at: Option<Instant>,
    error: Option<AppError>,
    worker_running: bool,
    active_revision: Option<u64>,
    pending: Option<target::Snapshot>,
    result_revision: u64,
}

impl Cache {
    const fn new() -> Self {
        Self {
            revision: 0,
            records: Vec::new(),
            updated_at: None,
            refreshed_at: None,
            attempted_at: None,
            error: None,
            worker_running: false,
            active_revision: None,
            pending: None,
            result_revision: 0,
        }
    }

    fn select(&mut self, revision: u64) {
        if self.revision != revision {
            self.revision = revision;
            self.records.clear();
            self.updated_at = None;
            self.refreshed_at = None;
            self.attempted_at = None;
            self.error = None;
            self.pending = None;
            self.result_revision += 1;
        }
    }

    fn stale(&self, now: Instant) -> bool {
        self.refreshed_at
            .is_none_or(|updated| now.saturating_duration_since(updated) >= TTL)
    }

    fn request(&mut self, snapshot: target::Snapshot, refresh: bool, now: Instant) {
        self.select(snapshot.revision);
        if self.active_revision == Some(self.revision) || self.pending.is_some() {
            return;
        }
        // Failed attempts also cool down; explicit refresh bypasses the cooldown.
        if refresh
            || (self.stale(now)
                && self
                    .attempted_at
                    .is_none_or(|attempted| now.saturating_duration_since(attempted) >= TTL))
        {
            self.attempted_at = Some(now);
            self.pending = Some(snapshot);
        }
    }

    #[cfg(test)]
    fn snapshot(&self, now: Instant) -> TokenUsageSnapshot {
        self.snapshot_since(now, None)
    }

    fn snapshot_since(&self, now: Instant, known_revision: Option<u64>) -> TokenUsageSnapshot {
        TokenUsageSnapshot {
            records: if known_revision == Some(self.result_revision) {
                Vec::new()
            } else {
                self.records.clone()
            },
            updated_at: self.updated_at,
            refreshing: self.active_revision == Some(self.revision) || self.pending.is_some(),
            stale: self.stale(now),
            error: self.error.clone(),
            target_revision: self.revision,
            // PID/version do not prove service identity across application restarts.
            cache_scope: "process",
            result_revision: self.result_revision,
        }
    }

    fn complete(
        &mut self,
        revision: u64,
        result: Result<Vec<TokenUsageRecord>, AppError>,
        now: Instant,
        updated_at: u64,
    ) {
        if self.active_revision == Some(revision) {
            self.active_revision = None;
        }
        if self.revision != revision {
            return;
        }
        match result {
            Ok(records) => {
                self.records = records;
                self.updated_at = Some(updated_at);
                self.refreshed_at = Some(now);
                self.error = None;
                self.result_revision += 1;
            }
            Err(error) => self.error = Some(error),
        }
    }

    /// Starts the next queued scan and marks its revision active. `None` when idle.
    fn begin(&mut self) -> Option<target::Snapshot> {
        let snapshot = self.pending.take()?;
        self.active_revision = Some(snapshot.revision);
        Some(snapshot)
    }
}

pub fn token_usage_snapshot(
    refresh: bool,
    known_revision: Option<u64>,
) -> Result<TokenUsageSnapshot, AppError> {
    // Use the target publication lock only for local state, never network work.
    let _transition = target::transition()?;
    let snapshot = target::snapshot()?;
    let mut cache = CACHE.lock().map_err(|_| failure("cache unavailable"))?;
    cache.request(snapshot, refresh, Instant::now());
    if cache.pending.is_some() && !cache.worker_running {
        cache.worker_running = true;
        if std::thread::Builder::new()
            .name("opencode-token-usage".into())
            .spawn(refresh_worker)
            .is_err()
        {
            cache.worker_running = false;
            cache.pending = None;
            cache.error = Some(failure("refresh worker unavailable"));
        }
    }
    Ok(cache.snapshot_since(Instant::now(), known_revision))
}

fn refresh_worker() {
    loop {
        let snapshot = {
            let Ok(mut cache) = CACHE.lock() else {
                return;
            };
            let Some(snapshot) = cache.begin() else {
                cache.worker_running = false;
                cache.active_revision = None;
                return;
            };
            snapshot
        };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            snapshot.ensure_current()?;
            let deadline = Instant::now() + SCAN_TIMEOUT;
            let connection = transport::connect_snapshot(snapshot.clone(), deadline)?;
            if let Some(key) = connection.usage_cache_key() {
                // A protected read is required before exposing a persisted snapshot.
                let _: super::Page<super::Session> = connection.fetch_page(None, None, deadline)?;
                if let Some(disk) = read_disk(&key) {
                    let _transition = target::transition()?;
                    snapshot.ensure_current()?;
                    let mut cache = CACHE.lock().map_err(|_| failure("cache unavailable"))?;
                    if cache.revision == snapshot.revision && cache.updated_at.is_none() {
                        cache.records = disk.records;
                        cache.updated_at = Some(disk.updated_at);
                        cache.result_revision += 1;
                    }
                }
            }
            let records = scan(&connection, deadline)?;
            connection.verify_identity(deadline)?;
            if let Some(key) = connection.usage_cache_key() {
                snapshot.ensure_current()?;
                write_disk(key, epoch_millis(), &records);
            }
            Ok(records)
        }))
        .unwrap_or_else(|_| Err(failure("refresh worker failed")));

        let Ok(_transition) = target::transition() else {
            if let Ok(mut cache) = CACHE.lock() {
                cache.worker_running = false;
                cache.active_revision = None;
                cache.pending = None;
                if cache.revision == snapshot.revision {
                    cache.error = Some(failure("target state unavailable"));
                }
            }
            return;
        };
        let Ok(mut cache) = CACHE.lock() else {
            return;
        };
        if snapshot.ensure_current().is_ok() {
            let updated_at = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                .min(super::JS_MAX_DATE as u128) as u64;
            cache.complete(snapshot.revision, result, Instant::now(), updated_at);
        }
        cache.active_revision = None;
    }
}

fn epoch_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(super::JS_MAX_DATE as u128) as u64
}
