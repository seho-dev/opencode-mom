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
use std::sync::{mpsc, Arc, Mutex, MutexGuard};
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
    control_generation: u64,
    enabled: bool,
    authorize: bool,
    shutdown: bool,
    preference_error: Option<String>,
}

struct Shared {
    desired: Desired,
    generation: u64,
    state: LidState,
    preparing: bool,
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
    effects: Arc<Mutex<()>>,
    wake: mpsc::Sender<()>,
    done: Mutex<mpsc::Receiver<()>>,
}

pub struct TargetRefresh<'a> {
    shared: MutexGuard<'a, Shared>,
    _effects: MutexGuard<'a, ()>,
}

impl TargetRefresh<'_> {
    pub fn invalidate(mut self) {
        self.shared.desired.generation += 1;
    }
}

struct Preparation(Arc<Mutex<Shared>>);

impl Drop for Preparation {
    fn drop(&mut self) {
        if let Ok(mut shared) = self.0.lock() {
            shared.preparing = false;
        }
    }
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
                control_generation: 1,
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
            preparing: false,
        }));
        let effects = Arc::new(Mutex::new(()));
        let (wake, events) = mpsc::channel();
        let (finished, done) = mpsc::channel();
        let worker_shared = Arc::clone(&shared);
        let worker_effects = Arc::clone(&effects);
        let journal_dir = directory.clone();
        thread::Builder::new()
            .name("lid-controller".into())
            .spawn(move || {
                let mut engine = Engine::new(
                    NativeController::new(journal_dir),
                    worker_shared,
                    worker_effects,
                );
                run_worker(&mut engine, events);
                finish_worker(&mut engine, finished);
            })
            .map_err(|error| AppError::configuration(format!("Start power monitor: {error}")))?;
        Ok(Self {
            directory,
            shared,
            effects,
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

    pub fn prepare_target_refresh(&self) -> Result<TargetRefresh<'_>, AppError> {
        let effects = self
            .effects
            .lock()
            .map_err(|_| AppError::configuration("Power effects lock failed"))?;
        let shared = self
            .shared
            .lock()
            .map_err(|_| AppError::configuration("Power monitor lock failed"))?;
        if shared.preparing {
            return Err(AppError::new(
                crate::error::ErrorCode::Busy,
                "Lid protection authorization is in progress; retry when it finishes",
            ));
        }
        Ok(TargetRefresh {
            _effects: effects,
            shared,
        })
    }

    pub fn finish_target_refresh(&self) {
        if self.wake.send(()).is_err() {
            eprintln!("Power monitor stopped after target refresh; native recovery/watchdog remains required");
        }
    }

    pub fn set(&self, enabled: bool) -> Result<LidState, AppError> {
        let _transition = crate::opencode::target::transition()?;
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
        shared.desired.control_generation += 1;
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
            shared.desired.control_generation += 1;
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
    stopped: bool,
    shared: Arc<Mutex<Shared>>,
    effects: Arc<Mutex<()>>,
    generation: u64,
    control_generation: u64,
    prepared: bool,
    preparation_attempted: bool,
    preparation_error: Option<String>,
    cleanup_failed: bool,
    protection: bool,
    last_busy: Option<Instant>,
    renew_at: Instant,
    query_at: Instant,
    inflight: Option<u64>,
}

impl<C: PowerController> Drop for Engine<C> {
    fn drop(&mut self) {
        if !thread::panicking() || self.stopped {
            return;
        }
        // The worker's guards have unwound; do not recover or reacquire poisoned locks.
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.controller.stop()));
        let message = match result {
            Ok(Ok(())) => return,
            Ok(Err(_)) => "Lid controller unwind cleanup failed; native recovery remains required",
            Err(payload) => {
                // A custom panic payload may itself panic on drop during this unwind.
                std::mem::forget(payload);
                "Lid controller unwind cleanup panicked; native recovery remains required"
            }
        };
        use std::io::Write;
        let _ = writeln!(std::io::stderr(), "{message}");
    }
}

impl<C: PowerController> Engine<C> {
    fn new(controller: C, shared: Arc<Mutex<Shared>>, effects: Arc<Mutex<()>>) -> Self {
        Self {
            controller,
            stopped: false,
            shared,
            effects,
            generation: 0,
            control_generation: 0,
            prepared: false,
            preparation_attempted: false,
            preparation_error: None,
            cleanup_failed: false,
            protection: false,
            last_busy: None,
            renew_at: Instant::now(),
            query_at: Instant::now(),
            inflight: None,
        }
    }

    fn controller(&mut self) -> &mut C {
        // A new effect can reacquire policy ownership even if it later fails or panics.
        self.stopped = false;
        &mut self.controller
    }

    fn stop(&mut self) -> Result<(), AppError> {
        if self.stopped {
            return Ok(());
        }
        let result = self.controller.stop();
        self.stopped = result.is_ok();
        result
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
        self.preparation_error = Some(error.to_string());
        self.protection = false;
        self.last_busy = None;
        self.error(error);
    }

    fn preparation_failed(&mut self, error: AppError) {
        self.prepared = false;
        self.preparation_error = Some(error.to_string());
        self.error(error);
    }

    fn publish_ready(&self, enabled: bool, preference_error: Option<String>) {
        let error =
            preference_error.or_else(|| enabled.then(|| self.preparation_error.clone()).flatten());
        self.publish(
            if error.is_some() {
                LidPhase::Error
            } else if !enabled {
                LidPhase::Disabled
            } else {
                LidPhase::Checking
            },
            None,
            error,
        );
    }

    fn stale_stop(&mut self) -> bool {
        if self
            .shared
            .lock()
            .is_ok_and(|shared| shared.desired.control_generation == self.control_generation)
        {
            return false;
        }
        self.prepared = false;
        self.protection = false;
        self.last_busy = None;
        if let Err(error) = self.stop() {
            self.cleanup_failed = true;
            eprintln!("Restore after canceled lid operation: {error}");
        }
        true
    }

    fn tick(&mut self, now: Instant) -> bool {
        let effects = Arc::clone(&self.effects);
        // Poison is fatal to this worker: unwind restores via Engine::drop, never
        // resume a potentially half-applied policy by recovering either mutex.
        let guard = effects.lock().expect("power effects lock poisoned");
        if self.tick_locked(now) {
            return true;
        }
        let preparation = {
            let mut shared = self.shared.lock().expect("power monitor lock poisoned");
            if shared.desired.generation == self.generation
                && shared.desired.control_generation == self.control_generation
                && shared.desired.enabled
                && shared.desired.authorize
                && !shared.desired.shutdown
                && !self.cleanup_failed
                && !self.prepared
                && !self.preparation_attempted
                && now >= self.query_at
            {
                shared.preparing = true;
                self.preparation_attempted = true;
                Some(Preparation(Arc::clone(&self.shared)))
            } else {
                None
            }
        };
        drop(guard);
        if let Some(preparation) = preparation {
            // Interactive authorization holds neither transition, effects nor shared locks.
            let result = self.controller().prepare();
            let _guard = effects.lock().expect("power effects lock poisoned");
            let mut shared = self.shared.lock().expect("power monitor lock poisoned");
            if shared.desired.control_generation != self.control_generation {
                drop(shared);
                self.stale_stop();
            } else {
                self.prepared = result.is_ok();
                self.preparation_error = result.as_ref().err().map(ToString::to_string);
                shared.preparing = false;
                drop(shared);
                if let Err(error) = result {
                    self.error(error);
                }
            }
            drop(preparation);
        }
        false
    }

    fn tick_locked(&mut self, now: Instant) -> bool {
        let (generation, control_generation, enabled, authorize, shutdown, preference_error) = {
            let shared = self.shared.lock().expect("power monitor lock poisoned");
            (
                shared.desired.generation,
                shared.desired.control_generation,
                shared.desired.enabled,
                shared.desired.authorize,
                shared.desired.shutdown,
                shared.desired.preference_error.clone(),
            )
        };
        if generation != self.generation {
            let control_changed = control_generation != self.control_generation;
            self.generation = generation;
            self.control_generation = control_generation;
            if control_changed {
                self.prepared = false;
                self.preparation_attempted = false;
                self.preparation_error = None;
            }
            self.protection = false;
            self.last_busy = None;
            self.query_at = now;
            // An ordinary opt-out restores policy but keeps this run's authorized helper reusable.
            let restored = if shutdown {
                self.stop()
            } else if !control_changed && !enabled && !self.cleanup_failed {
                Ok(())
            } else {
                self.controller().idle()
            };
            if self.stale_stop() {
                return false;
            }
            self.cleanup_failed = restored.is_err();
            match restored {
                Err(error) => self.error(error),
                Ok(()) => self.publish_ready(enabled, preference_error.clone()),
            }
            self.renew_at = now + POLL;
        }
        if shutdown {
            if self.cleanup_failed {
                if let Err(error) = self.stop() {
                    eprintln!("Restore lid protection on exit: {error}");
                }
            }
            return true;
        }
        if self.cleanup_failed {
            if now >= self.renew_at {
                let result = self.controller().idle();
                if self.stale_stop() {
                    return false;
                }
                self.cleanup_failed = result.is_err();
                if let Err(error) = result {
                    self.error(error);
                } else {
                    self.publish_ready(enabled, preference_error);
                }
                self.renew_at = now + POLL;
            }
            return false;
        }
        if !enabled {
            return false;
        }
        if !authorize {
            self.preparation_attempted = true;
            self.preparation_error =
                Some("Enable lid protection again to authorize this app run".into());
            self.publish_ready(enabled, preference_error);
            return false;
        }
        if self.protection
            && (now >= self.renew_at || self.last_busy.is_some_and(|busy| now >= busy + GRACE))
        {
            let remaining = self
                .last_busy
                .and_then(|busy| (busy + GRACE).checked_duration_since(now));
            if let Some(remaining) = remaining.filter(|duration| !duration.is_zero()) {
                let result = self.controller().busy(LEASE.min(remaining));
                if self.stale_stop() {
                    return false;
                }
                if let Err(error) = result {
                    let restored = self.controller().idle();
                    if self.stale_stop() {
                        return false;
                    }
                    self.protection = false;
                    match restored {
                        Ok(()) => self.preparation_failed(error),
                        Err(restore) => self.restore_failed(AppError::configuration(format!(
                            "{error}; restore: {restore}"
                        ))),
                    }
                }
            } else {
                let result = self.controller().idle();
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
        let effects = Arc::clone(&self.effects);
        let _guard = effects.lock().expect("power effects lock poisoned");
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
        let effects = Arc::clone(&self.effects);
        let _guard = effects.lock().expect("power effects lock poisoned");
        self.observation_locked(generation, result, now);
    }

    fn observation_locked(
        &mut self,
        generation: u64,
        result: Result<usize, AppError>,
        now: Instant,
    ) {
        if self.inflight != Some(generation) {
            return;
        }
        self.inflight = None;
        if generation != self.generation || !self.current() || self.cleanup_failed || !self.prepared
        {
            self.query_at = now;
            return;
        }
        self.query_at = now + POLL;
        match result {
            Err(error) => self.publish(LidPhase::Unknown, None, Some(error.to_string())),
            Ok(0) => {
                self.last_busy = None;
                let result = self.controller().idle();
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
                let result = self.controller().busy(LEASE);
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
                        let restored = self.controller().idle();
                        if self.stale_stop() {
                            return;
                        }
                        self.protection = false;
                        self.last_busy = None;
                        self.renew_at = now + POLL;
                        match restored {
                            Ok(()) => self.preparation_failed(error),
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

fn finish_worker<C: PowerController>(engine: &mut Engine<C>, finished: mpsc::Sender<()>) {
    let effects = Arc::clone(&engine.effects);
    match effects.lock() {
        Ok(_guard) => {
            if engine.stop().is_err() {
                eprintln!("Lid controller final cleanup failed; native recovery remains required");
            }
        }
        Err(_) => {
            eprintln!("Lid controller final cleanup lock failed; native recovery remains required");
        }
    }
    let _ = finished.send(());
}

fn run_worker<C: PowerController>(engine: &mut Engine<C>, wake: mpsc::Receiver<()>) {
    let (requests, queries) = mpsc::channel();
    let (results, observations) = mpsc::channel();
    let query_worker = thread::Builder::new()
        .name("lid-session-query".into())
        .spawn(move || {
            while let Ok(generation) = queries.recv() {
                let result = crate::opencode::active_sessions_via_service();
                if results.send((generation, result)).is_err() {
                    break;
                }
            }
        });
    if let Err(error) = query_worker {
        let effects = Arc::clone(&engine.effects);
        let _guard = effects.lock().expect("power effects lock poisoned");
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
            let effects = Arc::clone(&engine.effects);
            let _guard = effects.lock().expect("power effects lock poisoned");
            if engine.stop().is_err() {
                eprintln!(
                    "Lid controller disconnected cleanup failed; native recovery remains required"
                );
            }
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
