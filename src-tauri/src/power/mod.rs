#[cfg(target_os = "macos")]
mod macos;
mod settings;
#[cfg(any(target_os = "windows", test))]
mod windows;

#[cfg(target_os = "macos")]
use macos::NativeController;
#[cfg(target_os = "windows")]
use windows::NativeController;

use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::error::AppError;

pub trait PowerController: Send {
    fn prepare(&mut self) -> Result<(), AppError>;
    fn busy(&mut self, ttl: Duration) -> Result<(), AppError>;
    fn idle(&mut self) -> Result<(), AppError>;
    fn stop(&mut self) -> Result<(), AppError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LidPhase {
    Disabled,
    Idle,
    Checking,
    Protected,
    Unknown,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LidState {
    pub enabled: bool,
    pub phase: LidPhase,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_sessions: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

const POLL: Duration = Duration::from_secs(2);
const LEASE: Duration = Duration::from_secs(10);
const GRACE: Duration = Duration::from_secs(30);
const SHUTDOWN_WAIT: Duration = Duration::from_secs(3);

struct Desired {
    generation: u64,
    enabled: bool,
    authorize: bool,
    shutdown: bool,
    preference_error: Option<String>,
}

struct Shared {
    desired: Desired,
    generation: u64,
    state: LidState,
}

impl Shared {
    fn snapshot(&self) -> LidState {
        if self.generation == self.desired.generation {
            self.state.clone()
        } else {
            LidState {
                enabled: self.desired.enabled,
                phase: LidPhase::Checking,
                active_sessions: None,
                error: None,
            }
        }
    }
}

pub struct PowerService {
    directory: PathBuf,
    shared: Arc<Mutex<Shared>>,
    wake: mpsc::Sender<()>,
    done: Mutex<mpsc::Receiver<()>>,
}

impl PowerService {
    pub fn new(directory: PathBuf) -> Result<Self, AppError> {
        let preference = settings::load(&directory);
        let (enabled, error) = match preference {
            Ok(settings) => (settings.enabled, None),
            Err(error) => (false, Some(error.to_string())),
        };
        let shared = Arc::new(Mutex::new(Shared {
            desired: Desired {
                generation: 1,
                enabled,
                authorize: !cfg!(target_os = "macos"),
                shutdown: false,
                preference_error: error.clone(),
            },
            generation: 0,
            state: LidState {
                enabled,
                phase: if error.is_some() {
                    LidPhase::Error
                } else {
                    LidPhase::Checking
                },
                active_sessions: None,
                error,
            },
        }));
        let (wake, events) = mpsc::channel();
        let (finished, done) = mpsc::channel();
        let worker_shared = Arc::clone(&shared);
        let journal_dir = directory.clone();
        thread::Builder::new()
            .name("lid-controller".into())
            .spawn(move || {
                let mut engine = Engine::new(NativeController::new(journal_dir), worker_shared);
                run_worker(&mut engine, events);
                let _ = finished.send(());
            })
            .map_err(|error| AppError::configuration(format!("Start power monitor: {error}")))?;
        Ok(Self {
            directory,
            shared,
            wake,
            done: Mutex::new(done),
        })
    }

    pub fn get(&self) -> Result<LidState, AppError> {
        self.shared
            .lock()
            .map(|shared| shared.snapshot())
            .map_err(|_| AppError::configuration("Power monitor lock failed"))
    }

    pub fn set(&self, enabled: bool) -> Result<LidState, AppError> {
        let mut shared = self
            .shared
            .lock()
            .map_err(|_| AppError::configuration("Power monitor lock failed"))?;
        if shared.desired.shutdown {
            return Err(AppError::configuration("Power monitor is shutting down"));
        }
        let saved = settings::save(&self.directory, enabled);
        if enabled {
            saved.as_ref().map_err(Clone::clone)?;
        }
        // Failed preference writes must never prevent an in-memory opt-out/restore.
        shared.desired.generation += 1;
        shared.desired.enabled = enabled;
        shared.desired.authorize = true;
        shared.desired.preference_error = saved.as_ref().err().map(ToString::to_string);
        let state = shared.snapshot();
        drop(shared);
        self.wake
            .send(())
            .map_err(|_| AppError::configuration("Power monitor stopped"))?;
        saved?;
        Ok(state)
    }

    pub fn shutdown(&self) {
        if let Ok(mut shared) = self.shared.lock() {
            if shared.desired.shutdown {
                return;
            }
            shared.desired.generation += 1;
            shared.desired.enabled = false;
            shared.desired.shutdown = true;
        }
        let _ = self.wake.send(());
        // Authorization cannot be interrupted safely; late results stop before publication.
        if self
            .done
            .lock()
            .map_or(true, |done| done.recv_timeout(SHUTDOWN_WAIT).is_err())
        {
            eprintln!(
                "Lid protection shutdown timed out; native recovery/watchdog remains required"
            );
        }
    }
}

struct Engine<C: PowerController> {
    controller: C,
    shared: Arc<Mutex<Shared>>,
    generation: u64,
    prepared: bool,
    preparation_attempted: bool,
    cleanup_failed: bool,
    protection: bool,
    last_busy: Option<Instant>,
    renew_at: Instant,
    query_at: Instant,
    inflight: Option<u64>,
}

impl<C: PowerController> Engine<C> {
    fn new(controller: C, shared: Arc<Mutex<Shared>>) -> Self {
        Self {
            controller,
            shared,
            generation: 0,
            prepared: false,
            preparation_attempted: false,
            cleanup_failed: false,
            protection: false,
            last_busy: None,
            renew_at: Instant::now(),
            query_at: Instant::now(),
            inflight: None,
        }
    }

    fn current(&self) -> bool {
        self.shared
            .lock()
            .is_ok_and(|shared| shared.desired.generation == self.generation)
    }

    fn publish(&self, phase: LidPhase, count: Option<usize>, error: Option<String>) {
        if let Ok(mut shared) = self.shared.lock() {
            if shared.desired.generation == self.generation {
                shared.generation = self.generation;
                shared.state = LidState {
                    enabled: shared.desired.enabled,
                    phase,
                    active_sessions: count,
                    error,
                };
            }
        }
    }

    fn error(&self, error: AppError) {
        self.publish(LidPhase::Error, None, Some(error.to_string()));
    }

    fn restore_failed(&mut self, error: AppError) {
        self.cleanup_failed = true;
        self.prepared = false;
        self.preparation_attempted = true;
        self.protection = false;
        self.last_busy = None;
        self.error(error);
    }

    fn stale_stop(&mut self) -> bool {
        if self.current() {
            return false;
        }
        self.prepared = false;
        self.protection = false;
        self.last_busy = None;
        if let Err(error) = self.controller.stop() {
            self.cleanup_failed = true;
            eprintln!("Restore after canceled lid operation: {error}");
        }
        true
    }

    fn tick(&mut self, now: Instant) -> bool {
        let (generation, enabled, authorize, shutdown, preference_error) = {
            let shared = self.shared.lock().expect("power monitor lock poisoned");
            (
                shared.desired.generation,
                shared.desired.enabled,
                shared.desired.authorize,
                shared.desired.shutdown,
                shared.desired.preference_error.clone(),
            )
        };
        if generation != self.generation {
            self.generation = generation;
            self.prepared = false;
            self.preparation_attempted = false;
            self.protection = false;
            self.last_busy = None;
            self.query_at = now;
            // An ordinary opt-out restores policy but keeps this run's authorized helper reusable.
            let restored = if shutdown {
                self.controller.stop()
            } else {
                self.controller.idle()
            };
            if self.stale_stop() {
                return false;
            }
            self.cleanup_failed = restored.is_err();
            match restored {
                Err(error) => self.error(error),
                Ok(()) if !enabled => self.publish(
                    if preference_error.is_some() {
                        LidPhase::Error
                    } else {
                        LidPhase::Disabled
                    },
                    None,
                    preference_error.clone(),
                ),
                Ok(()) => self.publish(LidPhase::Checking, None, None),
            }
            self.renew_at = now + POLL;
        }
        if shutdown {
            if self.cleanup_failed {
                if let Err(error) = self.controller.stop() {
                    eprintln!("Restore lid protection on exit: {error}");
                }
            }
            return true;
        }
        if self.cleanup_failed {
            if now >= self.renew_at {
                let result = self.controller.idle();
                if self.stale_stop() {
                    return false;
                }
                self.cleanup_failed = result.is_err();
                if let Err(error) = result {
                    self.error(error);
                } else {
                    self.publish(
                        if preference_error.is_some() || (enabled && self.preparation_attempted) {
                            LidPhase::Error
                        } else if enabled {
                            LidPhase::Checking
                        } else {
                            LidPhase::Disabled
                        },
                        None,
                        preference_error.clone().or_else(|| {
                            (enabled && self.preparation_attempted).then(|| {
                                "Restoration completed; enable lid protection again to retry".into()
                            })
                        }),
                    );
                }
                self.renew_at = now + POLL;
            }
            return false;
        }
        if !enabled {
            return false;
        }
        if !authorize {
            self.publish(
                LidPhase::Error,
                None,
                Some("Enable lid protection again to authorize this app run".into()),
            );
            return false;
        }
        if !self.prepared && !self.preparation_attempted && now >= self.query_at {
            self.preparation_attempted = true;
            let result = self.controller.prepare();
            if self.stale_stop() {
                return false;
            }
            match result {
                Ok(()) => self.prepared = true,
                Err(error) => {
                    self.error(error);
                    return false;
                }
            }
        }
        if self.protection
            && (now >= self.renew_at || self.last_busy.is_some_and(|busy| now >= busy + GRACE))
        {
            let remaining = self
                .last_busy
                .and_then(|busy| (busy + GRACE).checked_duration_since(now));
            if let Some(remaining) = remaining.filter(|duration| !duration.is_zero()) {
                let result = self.controller.busy(LEASE.min(remaining));
                if self.stale_stop() {
                    return false;
                }
                if let Err(error) = result {
                    let restored = self.controller.idle();
                    if self.stale_stop() {
                        return false;
                    }
                    self.protection = false;
                    self.prepared = false;
                    match restored {
                        Ok(()) => self.error(error),
                        Err(restore) => self.restore_failed(AppError::configuration(format!(
                            "{error}; restore: {restore}"
                        ))),
                    }
                }
            } else {
                let result = self.controller.idle();
                if self.stale_stop() {
                    return false;
                }
                match result {
                    Ok(()) => {
                        self.protection = false;
                        self.publish(
                            LidPhase::Unknown,
                            None,
                            Some("Session status unavailable; protection lease expired".into()),
                        );
                    }
                    Err(error) => self.restore_failed(error),
                }
            }
            self.renew_at = now + POLL;
        }
        false
    }

    fn start_query(&mut self, now: Instant) -> Option<u64> {
        let enabled = self
            .shared
            .lock()
            .is_ok_and(|shared| shared.desired.enabled);
        if enabled
            && self.current()
            && self.prepared
            && !self.cleanup_failed
            && self.inflight.is_none()
            && now >= self.query_at
        {
            self.inflight = Some(self.generation);
            self.query_at = now + POLL;
            Some(self.generation)
        } else {
            None
        }
    }

    fn observation(&mut self, generation: u64, result: Result<usize, AppError>, now: Instant) {
        if self.inflight != Some(generation) {
            return;
        }
        self.inflight = None;
        self.query_at = now + POLL;
        if generation != self.generation || !self.current() || self.cleanup_failed || !self.prepared
        {
            return;
        }
        match result {
            Err(error) => self.publish(LidPhase::Unknown, None, Some(error.to_string())),
            Ok(0) => {
                self.last_busy = None;
                let result = self.controller.idle();
                if self.stale_stop() {
                    return;
                }
                match result {
                    Ok(()) => {
                        self.protection = false;
                        self.publish(LidPhase::Idle, Some(0), None);
                    }
                    Err(error) => self.restore_failed(error),
                }
                self.renew_at = now + POLL;
            }
            Ok(count) => {
                let result = self.controller.busy(LEASE);
                if self.stale_stop() {
                    return;
                }
                match result {
                    Ok(()) => {
                        self.last_busy = Some(now);
                        self.protection = true;
                        self.renew_at = now + POLL;
                        self.publish(LidPhase::Protected, Some(count), None);
                    }
                    Err(error) => {
                        // A failed apply may have changed one field: restore, never claim protection.
                        let restored = self.controller.idle();
                        if self.stale_stop() {
                            return;
                        }
                        self.protection = false;
                        self.last_busy = None;
                        self.prepared = false;
                        self.renew_at = now + POLL;
                        match restored {
                            Ok(()) => self.error(error),
                            Err(restore) => self.restore_failed(AppError::configuration(format!(
                                "{error}; restore: {restore}"
                            ))),
                        }
                    }
                }
            }
        }
    }
}

fn run_worker<C: PowerController>(engine: &mut Engine<C>, wake: mpsc::Receiver<()>) {
    let (requests, queries) = mpsc::channel();
    let (results, observations) = mpsc::channel();
    let query_worker = thread::Builder::new()
        .name("lid-session-query".into())
        .spawn(move || {
            while let Ok(generation) = queries.recv() {
                let result = crate::opencode_cli::active_sessions_via_cli();
                if results.send((generation, result)).is_err() {
                    break;
                }
            }
        });
    if let Err(error) = query_worker {
        engine.generation = engine
            .shared
            .lock()
            .expect("power monitor lock poisoned")
            .desired
            .generation;
        engine.error(AppError::configuration(format!(
            "Start session query worker: {error}"
        )));
        return;
    }
    loop {
        if engine.tick(Instant::now()) {
            break;
        }
        while let Ok((generation, result)) = observations.try_recv() {
            engine.observation(generation, result, Instant::now());
        }
        if let Some(generation) = engine.start_query(Instant::now()) {
            if requests.send(generation).is_err() {
                engine.observation(
                    generation,
                    Err(AppError::configuration("Session query worker stopped")),
                    Instant::now(),
                );
            }
        }
        if matches!(
            wake.recv_timeout(Duration::from_millis(100)),
            Err(mpsc::RecvTimeoutError::Disconnected)
        ) {
            let _ = engine.controller.stop();
            break;
        }
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
struct NativeController;

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
impl NativeController {
    fn new(_: PathBuf) -> Self {
        Self
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
impl PowerController for NativeController {
    fn prepare(&mut self) -> Result<(), AppError> {
        Err(AppError::configuration(
            "Lid protection is unsupported on this platform",
        ))
    }
    fn busy(&mut self, _: Duration) -> Result<(), AppError> {
        self.prepare()
    }
    fn idle(&mut self) -> Result<(), AppError> {
        Ok(())
    }
    fn stop(&mut self) -> Result<(), AppError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests;
