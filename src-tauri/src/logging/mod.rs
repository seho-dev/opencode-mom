mod storage;
mod types;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex, MutexGuard, Weak};
use std::time::{Duration, Instant};

use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use uuid::Uuid;

use crate::error::{AppError, ErrorCode};
use crate::models;
use crate::paths::ConfigPaths;
use storage::Storage;
pub(crate) use types::{
    LogContext, LogPage, LogQuery, LogRecord, LogStatus, LogType, LoggingState,
};

const QUEUE_CAPACITY: usize = 512;
const CONTROL_TIMEOUT: Duration = Duration::from_secs(5);
static INSTALLED: Mutex<Weak<LoggingService>> = Mutex::new(Weak::new());

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|error| error.into_inner())
}

struct Gate {
    generation: u64,
    active: bool,
    sequence: u64,
}

struct Shared {
    gate: Mutex<Gate>,
    state: Mutex<LoggingState>,
    recording: AtomicBool,
    dropped: AtomicU64,
}

impl Shared {
    fn fail(&self, message: &'static str) {
        let mut gate = lock(&self.gate);
        gate.active = false;
        gate.generation = gate.generation.wrapping_add(1);
        self.recording.store(false, Ordering::Release);
        let mut state = lock(&self.state);
        state.recording = false;
        state.error = Some(message.to_owned());
    }

    fn dropped(&self) {
        if self.recording.load(Ordering::Acquire) {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
}

enum Control {
    Set { enabled: bool, generation: u64 },
    Clear { enabled: bool, generation: u64 },
    Query(LogQuery),
    Directory,
    Shutdown,
}

enum Reply {
    State,
    Page(LogPage),
    Directory(PathBuf),
}

enum Message {
    Record {
        generation: u64,
        record: Box<LogRecord>,
    },
    Control(Control, mpsc::Sender<Result<Reply, AppError>>),
}

pub(crate) struct LoggingService {
    config_file: PathBuf,
    session_id: Uuid,
    shared: Arc<Shared>,
    sender: mpsc::SyncSender<Message>,
    controls: Mutex<()>,
}

impl LoggingService {
    pub(crate) fn new(paths: &ConfigPaths) -> Arc<Self> {
        let (enabled, error) = match models::load_config(&paths.config_file()) {
            Ok(config) => (config.state.logging_enabled, None),
            Err(_) => (false, Some("Logging preference unavailable".to_owned())),
        };
        let shared = Arc::new(Shared {
            gate: Mutex::new(Gate {
                generation: 0,
                active: false,
                sequence: 0,
            }),
            state: Mutex::new(LoggingState {
                enabled,
                recording: false,
                directory: paths.logs_dir().to_string_lossy().into_owned(),
                error,
                dropped_records: 0,
            }),
            recording: AtomicBool::new(false),
            dropped: AtomicU64::new(0),
        });
        let (sender, receiver) = mpsc::sync_channel(QUEUE_CAPACITY);
        let service = Arc::new(Self {
            config_file: paths.config_file(),
            session_id: Uuid::new_v4(),
            shared: shared.clone(),
            sender,
            controls: Mutex::new(()),
        });
        let directory = paths.logs_dir();
        match std::thread::Builder::new()
            .name("mom-local-logs".to_owned())
            .spawn(move || writer(shared, receiver, Storage::new(directory)))
        {
            Ok(handle) => drop(handle),
            Err(_) => service.shared.fail("Logging worker unavailable"),
        }
        if enabled {
            let generation = service.pause();
            let _ = service.request(
                Control::Set {
                    enabled,
                    generation,
                },
                CONTROL_TIMEOUT,
            );
        }
        service
    }

    pub(crate) fn install(self: &Arc<Self>) {
        *lock(&INSTALLED) = Arc::downgrade(self);
    }

    pub(crate) fn operation(self: &Arc<Self>, kind: LogType, event: &'static str) -> Operation {
        Operation::new(self, kind, event)
    }

    pub(crate) fn record(self: &Arc<Self>, kind: LogType, status: LogStatus, event: &'static str) {
        self.operation(kind, event).emit(
            event,
            status,
            match status {
                LogStatus::Success => "Event completed",
                LogStatus::Error => "Event failed",
            },
            None,
            false,
        );
    }

    pub(crate) fn state(&self) -> LoggingState {
        let mut state = lock(&self.shared.state).clone();
        state.recording = self.shared.recording.load(Ordering::Acquire);
        state.dropped_records = self.shared.dropped.load(Ordering::Relaxed);
        state
    }

    fn pause(&self) -> u64 {
        let mut gate = lock(&self.shared.gate);
        gate.active = false;
        gate.generation = gate.generation.wrapping_add(1);
        self.shared.recording.store(false, Ordering::Release);
        lock(&self.shared.state).recording = false;
        gate.generation
    }

    fn request(&self, control: Control, timeout: Duration) -> Result<Reply, AppError> {
        let deadline = Instant::now() + timeout;
        let (reply, receiver) = mpsc::channel();
        let mut message = Message::Control(control, reply);
        loop {
            match self.sender.try_send(message) {
                Ok(()) => break,
                Err(mpsc::TrySendError::Full(returned)) if Instant::now() < deadline => {
                    message = returned;
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(_) => {
                    self.shared.fail("Logging worker unavailable");
                    return Err(AppError::configuration("Logging worker unavailable"));
                }
            }
        }
        match receiver.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok(result) => result,
            Err(_) => {
                self.shared.fail("Logging worker timed out");
                Err(AppError::configuration("Logging worker timed out"))
            }
        }
    }

    pub(crate) fn set_enabled(&self, enabled: bool) -> Result<LoggingState, AppError> {
        let _control = lock(&self.controls);
        {
            let _write = models::config_write_lock();
            let result = (|| {
                let mut config = models::load_config(&self.config_file)?;
                config.state.logging_enabled = enabled;
                models::save_config(&self.config_file, &config)
            })();
            if result.is_err() {
                lock(&self.shared.state).error =
                    Some("Logging preference could not be saved".to_owned());
                return Err(AppError::configuration(
                    "Logging preference could not be saved",
                ));
            }
        }
        lock(&self.shared.state).enabled = enabled;
        if !enabled || !self.shared.recording.load(Ordering::Acquire) {
            let generation = self.pause();
            let _ = self.request(
                Control::Set {
                    enabled,
                    generation,
                },
                CONTROL_TIMEOUT,
            );
        } else {
            let gate = lock(&self.shared.gate);
            if gate.active {
                lock(&self.shared.state).error = None;
            }
        }
        Ok(self.state())
    }

    pub(crate) fn clear(&self) -> Result<LoggingState, AppError> {
        let _control = lock(&self.controls);
        let enabled = self.state().enabled;
        let generation = self.pause();
        self.request(
            Control::Clear {
                enabled,
                generation,
            },
            CONTROL_TIMEOUT,
        )?;
        self.shared.dropped.store(0, Ordering::Relaxed);
        Ok(self.state())
    }

    pub(crate) fn query(&self, query: LogQuery) -> Result<LogPage, AppError> {
        let _control = lock(&self.controls);
        match self.request(Control::Query(query), CONTROL_TIMEOUT)? {
            Reply::Page(page) => Ok(page),
            _ => Err(AppError::configuration("Logging worker unavailable")),
        }
    }

    pub(crate) fn open_directory(&self) -> Result<(), AppError> {
        let control = lock(&self.controls);
        let Reply::Directory(directory) = self.request(Control::Directory, CONTROL_TIMEOUT)? else {
            return Err(AppError::configuration("Logging worker unavailable"));
        };
        drop(control);
        #[cfg(target_os = "macos")]
        let mut command = std::process::Command::new("open");
        #[cfg(target_os = "windows")]
        let mut command = std::process::Command::new("explorer.exe");
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        let mut command = std::process::Command::new("xdg-open");
        let status = command
            .arg(directory)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|_| AppError::configuration("Could not open local log directory"))?;
        if !status.success() {
            return Err(AppError::configuration(
                "Could not open local log directory",
            ));
        }
        Ok(())
    }

    pub(crate) fn shutdown(&self, timeout: Duration) {
        // Exit never waits for a blocked filesystem thread or joins it without a deadline.
        let Ok(_control) = self.controls.try_lock() else {
            self.shared.fail("Logging shutdown incomplete");
            return;
        };
        self.pause();
        let _ = self.request(Control::Shutdown, timeout);
    }

    fn emit(&self, generation: u64, mut record: LogRecord) {
        let Ok(mut gate) = self.shared.gate.try_lock() else {
            self.shared.dropped();
            return;
        };
        if !gate.active || gate.generation != generation {
            return;
        }
        if !record.is_safe() {
            self.shared.dropped();
            return;
        }
        gate.sequence = gate.sequence.saturating_add(1);
        record.sequence = gate.sequence;
        match self.sender.try_send(Message::Record {
            generation,
            record: Box::new(record),
        }) {
            Ok(()) => {}
            Err(mpsc::TrySendError::Full(_)) => self.shared.dropped(),
            Err(mpsc::TrySendError::Disconnected(_)) => {
                self.shared.dropped();
                gate.active = false;
                gate.generation = gate.generation.wrapping_add(1);
                self.shared.recording.store(false, Ordering::Release);
                drop(gate);
                if let Ok(mut state) = self.shared.state.try_lock() {
                    state.recording = false;
                    state.error = Some("Logging worker unavailable".to_owned());
                }
            }
        }
    }
}

impl Drop for LoggingService {
    fn drop(&mut self) {
        self.pause();
        let (sender, _) = mpsc::channel();
        let _ = self
            .sender
            .try_send(Message::Control(Control::Shutdown, sender));
    }
}

fn activate(shared: &Shared, generation: u64, enabled: bool) -> bool {
    let mut gate = lock(&shared.gate);
    if gate.generation != generation {
        return false;
    }
    gate.active = enabled;
    shared.recording.store(enabled, Ordering::Release);
    let mut state = lock(&shared.state);
    state.recording = enabled;
    state.error = None;
    true
}

fn writer(shared: Arc<Shared>, receiver: mpsc::Receiver<Message>, mut storage: Storage) {
    let mut generation = None;
    while let Ok(message) = receiver.recv() {
        match message {
            Message::Record {
                generation: record_generation,
                record,
            } => {
                if generation != Some(record_generation) {
                    shared.dropped.fetch_add(1, Ordering::Relaxed);
                    continue;
                }
                if storage.write(&record).is_err() {
                    shared.dropped.fetch_add(1, Ordering::Relaxed);
                    shared.fail("Local log write failed");
                    generation = None;
                    let _ = storage.stop();
                }
            }
            Message::Control(control, reply) => {
                let shutdown = matches!(control, Control::Shutdown);
                let lifecycle = matches!(
                    control,
                    Control::Set { .. } | Control::Clear { .. } | Control::Shutdown
                );
                let result = match control {
                    Control::Set {
                        enabled,
                        generation: next,
                    } => {
                        generation = None;
                        let result = if enabled {
                            storage.start()
                        } else {
                            storage.stop()
                        };
                        result.map(|()| {
                            if activate(&shared, next, enabled) && enabled {
                                generation = Some(next);
                            }
                            Reply::State
                        })
                    }
                    Control::Clear {
                        enabled,
                        generation: next,
                    } => {
                        generation = None;
                        storage.clear(enabled).map(|()| {
                            if activate(&shared, next, enabled) && enabled {
                                generation = Some(next);
                            }
                            Reply::State
                        })
                    }
                    Control::Query(query) => storage.query(query).map(Reply::Page),
                    Control::Directory => storage.directory().map(Reply::Directory),
                    Control::Shutdown => storage.stop().map(|()| Reply::State),
                };
                if result
                    .as_ref()
                    .is_err_and(|error| lifecycle || error.code == ErrorCode::ConfigurationFailed)
                {
                    generation = None;
                    shared.fail("Local log storage unavailable");
                    let _ = storage.stop();
                }
                let _ = reply.send(result);
                if shutdown {
                    return;
                }
            }
        }
    }
    if storage.stop().is_err() {
        shared.fail("Local log flush failed");
    }
}

pub(crate) struct Operation {
    inner: Option<OperationInner>,
    finished: AtomicBool,
}

struct OperationInner {
    logger: Weak<LoggingService>,
    generation: u64,
    kind: LogType,
    event: &'static str,
    id: Uuid,
    started: Instant,
}

impl Operation {
    fn new(logger: &Arc<LoggingService>, kind: LogType, event: &'static str) -> Self {
        let generation = match logger.shared.gate.try_lock() {
            Ok(gate) => gate.active.then_some(gate.generation),
            Err(_) => {
                logger.shared.dropped();
                None
            }
        };
        Self {
            inner: generation
                .filter(|_| types::safe_event(kind, event))
                .map(|generation| OperationInner {
                    logger: Arc::downgrade(logger),
                    generation,
                    kind,
                    event,
                    id: Uuid::new_v4(),
                    started: Instant::now(),
                }),
            finished: AtomicBool::new(false),
        }
    }

    pub(crate) fn checkpoint(&self, event: &'static str) {
        if !self.finished.load(Ordering::Relaxed) {
            self.emit(event, LogStatus::Success, "Checkpoint reached", None, false);
        }
    }

    pub(crate) fn finish<T>(&self, result: &Result<T, AppError>) {
        if self.finished.swap(true, Ordering::Relaxed) {
            return;
        }
        if let Some(inner) = &self.inner {
            match result {
                Ok(_) => self.emit(
                    inner.event,
                    LogStatus::Success,
                    "Operation completed",
                    None,
                    true,
                ),
                Err(error) => self.emit(
                    inner.event,
                    LogStatus::Error,
                    "Operation failed",
                    Some(error.code),
                    true,
                ),
            }
        }
    }

    fn emit(
        &self,
        event: &'static str,
        status: LogStatus,
        message: &'static str,
        error_code: Option<ErrorCode>,
        finished: bool,
    ) {
        let Some(inner) = &self.inner else { return };
        let Some(logger) = inner.logger.upgrade() else {
            return;
        };
        let Ok(timestamp) = OffsetDateTime::now_utc().format(&Rfc3339) else {
            return;
        };
        logger.emit(
            inner.generation,
            LogRecord {
                schema_version: 1,
                timestamp,
                session_id: logger.session_id,
                sequence: 0,
                operation_id: inner.id,
                kind: inner.kind,
                status,
                event: event.to_owned(),
                message: message.to_owned(),
                duration_ms: finished
                    .then(|| inner.started.elapsed().as_millis().min(u64::MAX as u128) as u64),
                error_code,
                context: (!finished && message == "Checkpoint reached").then(|| LogContext {
                    step: Some(event.to_owned()),
                    count: None,
                    target_written: None,
                }),
            },
        );
    }
}

pub(crate) fn operation(kind: LogType, event: &'static str) -> Operation {
    let logger = INSTALLED
        .try_lock()
        .ok()
        .and_then(|logger| logger.upgrade());
    match logger {
        Some(logger) => logger.operation(kind, event),
        None => Operation {
            inner: None,
            finished: AtomicBool::new(false),
        },
    }
}

pub(crate) fn record(kind: LogType, status: LogStatus, event: &'static str) {
    if let Some(logger) = INSTALLED
        .try_lock()
        .ok()
        .and_then(|logger| logger.upgrade())
    {
        logger.record(kind, status, event);
    }
}

#[cfg(test)]
mod tests;
