use super::*;

#[derive(Default)]
struct Mock {
    calls: Vec<String>,
    leases: Vec<Duration>,
    fail_busy: bool,
    fail_prepare: bool,
    fail_idle: bool,
    fail_stop: bool,
    prepare_gate: Option<Gate>,
    busy_gate: Option<Gate>,
    stop_gate: Option<Gate>,
    cancel_prepare: Option<Arc<Mutex<Shared>>>,
    cancel_busy: Option<Arc<Mutex<Shared>>>,
}

impl PowerController for Mock {
    fn prepare(&mut self) -> Result<(), AppError> {
        self.calls.push("prepare".into());
        if let Some(gate) = self.prepare_gate.take() {
            gate.wait();
        }
        if let Some(shared) = self.cancel_prepare.take() {
            let mut shared = shared.lock().unwrap();
            shared.desired.generation += 1;
            shared.desired.control_generation += 1;
            shared.desired.enabled = false;
        }
        if self.fail_prepare {
            Err(AppError::configuration("authorization failed"))
        } else {
            Ok(())
        }
    }
    fn busy(&mut self, ttl: Duration) -> Result<(), AppError> {
        self.calls.push("busy".into());
        self.leases.push(ttl);
        if let Some(gate) = self.busy_gate.take() {
            gate.wait();
        }
        if let Some(shared) = self.cancel_busy.take() {
            let mut shared = shared.lock().unwrap();
            shared.desired.generation += 1;
            shared.desired.control_generation += 1;
            shared.desired.enabled = false;
        }
        if self.fail_busy {
            Err(AppError::configuration("apply failed"))
        } else {
            Ok(())
        }
    }
    fn idle(&mut self) -> Result<(), AppError> {
        self.calls.push("idle".into());
        if self.fail_idle {
            Err(AppError::configuration("restore failed"))
        } else {
            Ok(())
        }
    }
    fn stop(&mut self) -> Result<(), AppError> {
        self.calls.push("stop".into());
        if let Some(gate) = self.stop_gate.take() {
            gate.wait();
        }
        if self.fail_stop {
            Err(AppError::configuration("stop restore failed"))
        } else {
            Ok(())
        }
    }
}

struct Gate {
    entered: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
}

impl Gate {
    fn wait(self) {
        self.entered.send(()).unwrap();
        self.release.recv_timeout(Duration::from_secs(5)).unwrap();
    }
}

fn gate() -> (Gate, mpsc::Receiver<()>, mpsc::Sender<()>) {
    let (entered, arrival) = mpsc::channel();
    let (release, departure) = mpsc::channel();
    (
        Gate {
            entered,
            release: departure,
        },
        arrival,
        release,
    )
}

fn engine(enabled: bool) -> Engine<Mock> {
    let shared = Arc::new(Mutex::new(Shared {
        desired: Desired {
            generation: 1,
            control_generation: 1,
            enabled,
            authorize: true,
            shutdown: false,
            preference_error: None,
        },
        generation: 0,
        state: LidState {
            enabled,
            phase: LidPhase::Checking,
            active_sessions: None,
            error: None,
        },
        preparing: false,
    }));
    Engine::new(Mock::default(), shared, Arc::new(Mutex::new(())))
}

fn service(engine: &Engine<Mock>) -> PowerService {
    let (wake, _events) = mpsc::channel();
    let (_finished, done) = mpsc::channel();
    PowerService {
        directory: std::env::temp_dir().join(format!("mom-power-{}", uuid::Uuid::new_v4())),
        shared: Arc::clone(&engine.shared),
        effects: Arc::clone(&engine.effects),
        wake,
        done: Mutex::new(done),
    }
}

fn state(engine: &Engine<Mock>) -> LidState {
    engine.shared.lock().unwrap().snapshot()
}

fn observe(engine: &mut Engine<Mock>, result: Result<usize, AppError>, now: Instant) {
    let generation = engine.start_query(now).unwrap();
    engine.observation(generation, result, now);
}

#[test]
fn poisoned_worker_locks_unwind_to_cleanup_without_recovering_policy_state() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct StopMock(Arc<AtomicUsize>);
    impl PowerController for StopMock {
        fn prepare(&mut self) -> Result<(), AppError> {
            panic!("poisoned worker must not prepare");
        }
        fn busy(&mut self, _: Duration) -> Result<(), AppError> {
            panic!("poisoned worker must not protect");
        }
        fn idle(&mut self) -> Result<(), AppError> {
            panic!("poisoned worker must not resume policy state");
        }
        fn stop(&mut self) -> Result<(), AppError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    for poison_effects in [true, false] {
        let fixture = engine(true);
        let shared = Arc::clone(&fixture.shared);
        let effects = Arc::clone(&fixture.effects);
        assert!(std::panic::catch_unwind(|| {
            if poison_effects {
                let _guard = effects.lock().unwrap();
                panic!("injected effects poison");
            } else {
                let _guard = shared.lock().unwrap();
                panic!("injected shared poison");
            }
        })
        .is_err());
        let stops = Arc::new(AtomicUsize::new(0));
        let controller = StopMock(Arc::clone(&stops));
        assert!(std::panic::catch_unwind(move || {
            let mut worker = Engine::new(controller, shared, effects);
            worker.tick(Instant::now());
        })
        .is_err());
        assert_eq!(stops.load(Ordering::SeqCst), 1);
        assert!(fixture.effects.is_poisoned());
        assert_eq!(fixture.shared.is_poisoned(), !poison_effects);
    }
}

#[test]
fn worker_unwind_attempts_cleanup_without_panicking_or_recovering_poison() {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    struct UnwindMock {
        stops: Arc<AtomicUsize>,
        unlocked: Arc<AtomicBool>,
        effects: Arc<Mutex<()>>,
        stop_failure: u8,
    }

    impl PowerController for UnwindMock {
        fn prepare(&mut self) -> Result<(), AppError> {
            Ok(())
        }
        fn busy(&mut self, _: Duration) -> Result<(), AppError> {
            panic!("injected controller unwind");
        }
        fn idle(&mut self) -> Result<(), AppError> {
            Ok(())
        }
        fn stop(&mut self) -> Result<(), AppError> {
            self.stops.fetch_add(1, Ordering::SeqCst);
            self.unlocked.store(
                !matches!(
                    self.effects.try_lock(),
                    Err(std::sync::TryLockError::WouldBlock)
                ),
                Ordering::SeqCst,
            );
            match self.stop_failure {
                0 => Ok(()),
                1 => Err(AppError::configuration("injected cleanup failure")),
                _ => panic!("injected cleanup unwind"),
            }
        }
    }

    for stop_failure in 0..3 {
        let fixture = engine(true);
        let stops = Arc::new(AtomicUsize::new(0));
        let unlocked = Arc::new(AtomicBool::new(false));
        let controller = UnwindMock {
            stops: Arc::clone(&stops),
            unlocked: Arc::clone(&unlocked),
            effects: Arc::clone(&fixture.effects),
            stop_failure,
        };
        let shared = Arc::clone(&fixture.shared);
        let effects = Arc::clone(&fixture.effects);
        let unwind = std::panic::catch_unwind(move || {
            let mut worker = Engine::new(controller, shared, effects);
            let now = Instant::now();
            worker.tick(now);
            let generation = worker.start_query(now).unwrap();
            worker.observation(generation, Ok(1), now);
        });
        assert!(unwind.is_err());
        assert_eq!(stops.load(Ordering::SeqCst), 1);
        assert!(unlocked.load(Ordering::SeqCst));
        assert!(fixture.effects.is_poisoned());
    }

    let fixture = engine(true);
    let stops = Arc::new(AtomicUsize::new(0));
    let unlocked = Arc::new(AtomicBool::new(false));
    let controller = UnwindMock {
        stops: Arc::clone(&stops),
        unlocked: Arc::clone(&unlocked),
        effects: Arc::clone(&fixture.effects),
        stop_failure: 0,
    };
    let shared = Arc::clone(&fixture.shared);
    let effects = Arc::clone(&fixture.effects);
    let unwind = std::panic::catch_unwind(move || {
        let mut worker = Engine::new(controller, shared, effects);
        assert!(worker.stop().is_ok());
        panic!("injected post-stop unwind");
    });
    assert!(unwind.is_err());
    assert_eq!(stops.load(Ordering::SeqCst), 1);
    assert!(unlocked.load(Ordering::SeqCst));
}

#[test]
fn disconnected_worker_final_cleanup_retries_before_completion_and_deduplicates_success() {
    struct StopMock {
        fail_initial: bool,
        stops: usize,
        effects: Arc<Mutex<()>>,
        completion: mpsc::Receiver<()>,
    }

    impl PowerController for StopMock {
        fn prepare(&mut self) -> Result<(), AppError> {
            panic!("disabled worker must not prepare");
        }
        fn busy(&mut self, _: Duration) -> Result<(), AppError> {
            panic!("disabled worker must not protect");
        }
        fn idle(&mut self) -> Result<(), AppError> {
            Ok(())
        }
        fn stop(&mut self) -> Result<(), AppError> {
            assert!(matches!(
                self.effects.try_lock(),
                Err(std::sync::TryLockError::WouldBlock)
            ));
            assert!(matches!(
                self.completion.try_recv(),
                Err(mpsc::TryRecvError::Empty)
            ));
            self.stops += 1;
            if self.fail_initial && self.stops == 1 {
                Err(AppError::configuration("injected first stop failure"))
            } else {
                Ok(())
            }
        }
    }

    for fail_initial in [true, false] {
        let fixture = engine(false);
        let (finished, completion) = mpsc::channel();
        let controller = StopMock {
            fail_initial,
            stops: 0,
            effects: Arc::clone(&fixture.effects),
            completion,
        };
        let mut worker = Engine::new(
            controller,
            Arc::clone(&fixture.shared),
            Arc::clone(&fixture.effects),
        );
        let (wake, events) = mpsc::channel();
        drop(wake);
        run_worker(&mut worker, events);
        assert_eq!(worker.controller.stops, 1);
        finish_worker(&mut worker, finished);
        assert_eq!(worker.controller.stops, if fail_initial { 2 } else { 1 });
        assert!(worker.stopped);
        assert_eq!(worker.controller.completion.try_recv(), Ok(()));
        assert!(fixture.effects.try_lock().is_ok());
    }
}

fn set(engine: &Engine<Mock>, enabled: bool) {
    let mut shared = engine.shared.lock().unwrap();
    shared.desired.enabled = enabled;
    shared.desired.generation += 1;
    shared.desired.control_generation += 1;
}

#[test]
fn disabled_enable_multiple_busy_and_idle_ack() {
    let now = Instant::now();
    let mut engine = engine(false);
    engine.tick(now);
    assert_eq!(state(&engine).phase, LidPhase::Disabled);
    assert_eq!(engine.controller.calls, ["idle"]);
    assert!(engine.start_query(now).is_none());
    set(&engine, true);
    engine.tick(now);
    assert_eq!(engine.controller.calls, ["idle", "idle", "prepare"]);
    observe(&mut engine, Ok(3), now);
    assert_eq!(state(&engine).phase, LidPhase::Protected);
    assert_eq!(state(&engine).active_sessions, Some(3));
    observe(&mut engine, Ok(0), now + POLL);
    assert_eq!(state(&engine).phase, LidPhase::Idle);
    assert_eq!(engine.controller.calls.last().unwrap(), "idle");
}

#[test]
fn unknown_does_not_extend_grace_and_lease_is_capped() {
    let now = Instant::now();
    let mut engine = engine(true);
    engine.tick(now);
    observe(&mut engine, Ok(1), now);
    observe(
        &mut engine,
        Err(AppError::configuration("query failed")),
        now + POLL,
    );
    assert_eq!(state(&engine).phase, LidPhase::Unknown);
    engine.tick(now + Duration::from_secs(28));
    assert_eq!(
        engine.controller.leases.last(),
        Some(&Duration::from_secs(2))
    );
    assert_eq!(engine.last_busy, Some(now));
    engine.tick(now + Duration::from_secs(29));
    engine.tick(now + GRACE);
    assert!(!engine.protection);
    assert_eq!(state(&engine).phase, LidPhase::Unknown);
    assert_eq!(engine.controller.calls.last().unwrap(), "idle");
}

#[test]
fn timer_renews_with_a_query_inflight_and_never_overlaps_queries() {
    let now = Instant::now();
    let mut engine = engine(true);
    engine.tick(now);
    observe(&mut engine, Ok(2), now);
    let generation = engine.start_query(now + POLL).unwrap();
    engine.tick(now + POLL);
    assert_eq!(engine.controller.leases.len(), 2);
    assert!(engine.start_query(now + POLL * 2).is_none());
    engine.observation(generation, Ok(1), now + POLL * 2);
    assert!(engine.start_query(now + POLL * 3).is_some());
}

#[test]
fn apply_and_restore_errors_never_claim_protected_or_idle() {
    let now = Instant::now();
    let mut engine = engine(true);
    engine.tick(now);
    engine.controller.fail_busy = true;
    observe(&mut engine, Ok(1), now);
    assert_eq!(state(&engine).phase, LidPhase::Error);
    assert!(engine.last_busy.is_none());
    engine.controller.fail_busy = false;
    set(&engine, true);
    engine.tick(now + POLL);
    observe(&mut engine, Ok(1), now + POLL);
    engine.controller.fail_idle = true;
    observe(&mut engine, Ok(0), now + POLL * 2);
    assert_eq!(state(&engine).phase, LidPhase::Error);
    assert!(engine.last_busy.is_none());
    engine.tick(now + POLL * 3);
    assert_eq!(state(&engine).phase, LidPhase::Error);
}

#[test]
fn disable_restore_failure_is_visible_and_retried() {
    let now = Instant::now();
    let mut engine = engine(true);
    engine.tick(now);
    observe(&mut engine, Ok(1), now);
    engine.controller.fail_idle = true;
    set(&engine, false);
    assert_eq!(state(&engine).phase, LidPhase::Checking);
    engine.tick(now + POLL);
    assert!(!state(&engine).enabled);
    assert_eq!(state(&engine).phase, LidPhase::Error);
    engine.controller.fail_idle = false;
    engine.tick(now + POLL * 2);
    assert_eq!(state(&engine).phase, LidPhase::Disabled);
}

#[test]
fn stale_query_and_late_authorization_stop_before_publication() {
    let now = Instant::now();
    let mut engine = engine(true);
    engine.tick(now);
    let generation = engine.start_query(now).unwrap();
    set(&engine, false);
    engine.tick(now + POLL);
    set(&engine, true);
    engine.tick(now + POLL * 2);
    assert!(engine.start_query(now + POLL * 2).is_none());
    engine.observation(generation, Ok(1), now + POLL * 2);
    assert!(!engine.controller.calls.iter().any(|call| call == "busy"));
    assert!(engine.start_query(now + POLL * 3).is_some());

    let mut late = super::tests::engine(true);
    late.controller.cancel_prepare = Some(Arc::clone(&late.shared));
    late.tick(now);
    assert_eq!(late.controller.calls, ["idle", "prepare", "stop"]);
    assert!(!late.prepared);
    assert_ne!(state(&late).phase, LidPhase::Protected);
    late.tick(now + POLL);
    assert_eq!(state(&late).phase, LidPhase::Disabled);
}

#[test]
fn startup_authorization_is_explicit_and_not_repeated() {
    let now = Instant::now();
    let mut engine = engine(true);
    engine.shared.lock().unwrap().desired.authorize = false;
    engine.tick(now);
    assert_eq!(state(&engine).phase, LidPhase::Error);
    assert_eq!(engine.controller.calls, ["idle"]);
    assert!(engine.start_query(now).is_none());
    let error = state(&engine).error;
    let service = service(&engine);
    service.prepare_target_refresh().unwrap().invalidate();
    engine.tick(now + POLL);
    assert_eq!(state(&engine).phase, LidPhase::Error);
    assert_eq!(state(&engine).error, error);
    assert!(!engine.shared.lock().unwrap().desired.authorize);
    assert_eq!(engine.controller.calls, ["idle", "idle"]);
    assert!(engine.start_query(now + POLL).is_none());
}

#[test]
fn shutdown_invalidates_inflight_query() {
    let now = Instant::now();
    let mut engine = engine(true);
    engine.tick(now);
    let generation = engine.start_query(now).unwrap();
    {
        let mut shared = engine.shared.lock().unwrap();
        shared.desired.generation += 1;
        shared.desired.control_generation += 1;
        shared.desired.enabled = false;
        shared.desired.shutdown = true;
    }
    assert!(engine.tick(now + POLL));
    engine.observation(generation, Ok(5), now + POLL);
    assert_eq!(state(&engine).phase, LidPhase::Disabled);
    assert!(!engine.controller.calls.iter().any(|call| call == "busy"));
}

#[test]
fn dto_contract_omits_absent_fields() {
    assert_eq!(
        serde_json::to_value(LidState {
            enabled: false,
            phase: LidPhase::Disabled,
            active_sessions: None,
            error: None,
        })
        .unwrap(),
        serde_json::json!({"enabled": false, "phase": "disabled"})
    );
}

#[test]
fn late_busy_ack_stops_and_normal_toggle_reuses_authorization() {
    let now = Instant::now();
    let mut engine = engine(true);
    engine.tick(now);
    engine.controller.cancel_busy = Some(Arc::clone(&engine.shared));
    observe(&mut engine, Ok(1), now);
    assert_eq!(engine.controller.calls.last().unwrap(), "stop");
    assert_ne!(state(&engine).phase, LidPhase::Protected);
    engine.tick(now + POLL);
    set(&engine, true);
    engine.tick(now + POLL * 2);
    observe(&mut engine, Ok(1), now + POLL * 2);
    let stops = engine
        .controller
        .calls
        .iter()
        .filter(|call| *call == "stop")
        .count();
    set(&engine, false);
    engine.tick(now + POLL * 3);
    set(&engine, true);
    engine.tick(now + POLL * 4);
    assert_eq!(
        engine
            .controller
            .calls
            .iter()
            .filter(|call| *call == "stop")
            .count(),
        stops
    );
}

#[test]
fn preference_save_failure_does_not_block_disable_restore() {
    let now = Instant::now();
    let mut engine = engine(true);
    engine.tick(now);
    observe(&mut engine, Ok(1), now);
    let path = std::env::temp_dir().join(format!("mom-power-blocked-{}", uuid::Uuid::new_v4()));
    std::fs::write(&path, "not a directory").unwrap();
    let (wake, _events) = mpsc::channel();
    let (_finished, done) = mpsc::channel();
    let service = PowerService {
        directory: path.clone(),
        shared: Arc::clone(&engine.shared),
        effects: Arc::clone(&engine.effects),
        wake,
        done: Mutex::new(done),
    };
    assert!(service.set(false).is_err());
    assert!(!service.get().unwrap().enabled);
    engine.tick(now + POLL);
    assert_eq!(engine.controller.calls.last().unwrap(), "idle");
    assert_eq!(service.get().unwrap().phase, LidPhase::Error);
    assert!(!engine.protection);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn target_refresh_clears_observations_and_grace_without_reauthorizing_or_stopping() {
    let now = Instant::now();
    let mut engine = engine(true);
    engine.tick(now);
    observe(&mut engine, Ok(4), now);
    let service = service(&engine);
    let control = engine.control_generation;
    service.prepare_target_refresh().unwrap().invalidate();
    assert_eq!(service.get().unwrap().active_sessions, None);
    assert_eq!(service.get().unwrap().phase, LidPhase::Checking);
    engine.tick(now + POLL);
    assert!(engine.prepared);
    assert!(engine.preparation_attempted);
    assert_eq!(engine.control_generation, control);
    assert!(!engine.protection);
    assert_eq!(engine.last_busy, None);
    assert_eq!(engine.controller.calls, ["idle", "prepare", "busy", "idle"]);
    observe(
        &mut engine,
        Err(AppError::configuration("new target unavailable")),
        now + POLL,
    );
    assert_eq!(state(&engine).phase, LidPhase::Unknown);
    assert_eq!(state(&engine).active_sessions, None);
    assert_eq!(engine.last_busy, None);
    assert!(engine.prepared);
    assert_eq!(engine.controller.calls, ["idle", "prepare", "busy", "idle"]);
    observe(&mut engine, Ok(2), now + POLL * 2);
    assert_eq!(state(&engine).active_sessions, Some(2));
    engine.tick(now + POLL * 3);
    assert_eq!(
        engine
            .controller
            .calls
            .iter()
            .filter(|s| *s == "prepare")
            .count(),
        1
    );
    assert!(!engine.controller.calls.iter().any(|s| s == "stop"));
    assert!(!service.directory.exists());
}

#[test]
fn target_refresh_preserves_disabled_authorization_and_preference_errors() {
    let now = Instant::now();
    let mut disabled = engine(false);
    disabled.tick(now);
    let disabled_service = service(&disabled);
    disabled_service
        .prepare_target_refresh()
        .unwrap()
        .invalidate();
    disabled.tick(now + POLL);
    assert_eq!(state(&disabled).phase, LidPhase::Disabled);
    assert_eq!(disabled.controller.calls, ["idle"]);
    assert!(disabled.start_query(now + POLL).is_none());
    for enabled in [false, true] {
        let now = Instant::now();
        let mut engine = engine(enabled);
        {
            let mut shared = engine.shared.lock().unwrap();
            shared.desired.authorize = false;
            shared.desired.preference_error = Some("preference failed".into());
        }
        engine.tick(now);
        let service = service(&engine);
        let calls = engine.controller.calls.clone();
        let before = service.get().unwrap();
        let generation = engine.generation;
        drop(service.prepare_target_refresh().unwrap());
        assert_eq!(engine.shared.lock().unwrap().desired.generation, generation);
        assert_eq!(service.get().unwrap(), before);
        service.prepare_target_refresh().unwrap().invalidate();
        engine.tick(now + POLL);
        let shared = engine.shared.lock().unwrap();
        assert_eq!(shared.desired.enabled, enabled);
        assert!(!shared.desired.authorize);
        assert_eq!(
            shared.desired.preference_error.as_deref(),
            Some("preference failed")
        );
        assert_eq!(
            shared.snapshot().error.as_deref(),
            Some("preference failed")
        );
        assert_eq!(shared.snapshot().phase, LidPhase::Error);
        drop(shared);
        assert!(engine.start_query(now + POLL).is_none());
        assert!(!engine
            .controller
            .calls
            .iter()
            .any(|s| s == "prepare" || s == "stop"));
        if !enabled {
            assert_eq!(engine.controller.calls, calls);
        }
    }
}

#[test]
fn failed_preparation_stays_visible_across_target_refresh_and_restore_retry() {
    let now = Instant::now();
    let mut engine = engine(true);
    engine.controller.fail_prepare = true;
    engine.tick(now);
    assert!(!engine.shared.lock().unwrap().preparing);
    let error = state(&engine).error;
    assert_eq!(error.as_deref(), Some("authorization failed"));
    let service = service(&engine);
    engine.controller.fail_idle = true;
    service.prepare_target_refresh().unwrap().invalidate();
    engine.tick(now + POLL);
    assert_eq!(state(&engine).error.as_deref(), Some("restore failed"));
    assert!(engine.start_query(now + POLL).is_none());
    engine.controller.fail_idle = false;
    engine.tick(now + POLL * 2);
    assert_eq!(state(&engine).phase, LidPhase::Error);
    assert_eq!(state(&engine).error, error);
    engine.tick(now + POLL * 3);
    service.prepare_target_refresh().unwrap().invalidate();
    engine.tick(now + POLL * 4);
    assert_eq!(state(&engine).phase, LidPhase::Error);
    assert_eq!(state(&engine).error, error);
    assert!(engine.start_query(now + POLL * 4).is_none());
    assert_eq!(
        engine
            .controller
            .calls
            .iter()
            .filter(|s| *s == "prepare")
            .count(),
        1
    );
    assert!(!engine.controller.calls.iter().any(|s| s == "stop"));
}

#[test]
fn old_positive_query_keeps_its_slot_until_arrival_and_cannot_apply_after_commit() {
    for tick_before_result in [false, true] {
        let now = Instant::now();
        let mut engine = engine(true);
        engine.tick(now);
        let old = engine.start_query(now).unwrap();
        let service = service(&engine);
        service.prepare_target_refresh().unwrap().invalidate();
        if tick_before_result {
            engine.tick(now + POLL);
        }
        assert_eq!(engine.inflight, Some(old));
        assert!(engine.start_query(now + POLL).is_none());
        engine.observation(old, Ok(7), now + POLL);
        assert_eq!(engine.inflight, None);
        assert!(!engine.controller.calls.iter().any(|s| s == "busy"));
        assert_eq!(service.get().unwrap().active_sessions, None);
        engine.tick(now + POLL);
        let fresh = engine.start_query(now + POLL).unwrap();
        assert_ne!(fresh, old);
        engine.observation(fresh, Ok(1), now + POLL);
        assert_eq!(state(&engine).active_sessions, Some(1));
    }
}

#[test]
fn target_refresh_restore_failure_blocks_queries_and_busy_until_cleanup_recovers() {
    let now = Instant::now();
    let mut engine = engine(true);
    engine.tick(now);
    observe(&mut engine, Ok(1), now);
    let service = service(&engine);
    engine.controller.fail_idle = true;
    service.prepare_target_refresh().unwrap().invalidate();
    engine.tick(now + POLL);
    let calls = engine.controller.calls.len();
    assert!(engine.prepared);
    assert!(engine.cleanup_failed);
    assert_eq!(state(&engine).phase, LidPhase::Error);
    assert!(engine.start_query(now + POLL).is_none());
    engine.tick(now + POLL + POLL / 2);
    assert_eq!(engine.controller.calls.len(), calls);
    engine.tick(now + POLL * 2);
    assert_eq!(engine.controller.calls.last().unwrap(), "idle");
    assert_eq!(engine.controller.leases.len(), 1);
    engine.controller.fail_idle = false;
    engine.tick(now + POLL * 3);
    assert!(!engine.cleanup_failed);
    assert_eq!(state(&engine).phase, LidPhase::Checking);
    observe(&mut engine, Ok(2), now + POLL * 3);
    assert_eq!(state(&engine).phase, LidPhase::Protected);
    assert_eq!(
        engine
            .controller
            .calls
            .iter()
            .filter(|s| *s == "prepare")
            .count(),
        1
    );
    assert!(!engine.controller.calls.iter().any(|s| s == "stop"));
}

#[test]
fn authorization_releases_locks_but_target_refresh_stays_busy_through_late_stop() {
    let now = Instant::now();
    let mut engine = engine(true);
    let mut service = service(&engine);
    let (prepare, entered, release) = gate();
    let (stop, stopping, stopped) = gate();
    engine.controller.prepare_gate = Some(prepare);
    engine.controller.stop_gate = Some(stop);
    let worker = thread::spawn(move || {
        engine.tick(now);
        engine
    });
    entered.recv_timeout(Duration::from_secs(5)).unwrap();
    let error = service.prepare_target_refresh().err().unwrap();
    assert_eq!(error.code, crate::error::ErrorCode::Busy);
    assert!(service.effects.try_lock().is_ok());
    assert!(service.shared.try_lock().is_ok());
    let (wake, _events) = mpsc::channel();
    service.wake = wake;
    let blocked = std::env::temp_dir().join(format!("mom-power-optout-{}", uuid::Uuid::new_v4()));
    std::fs::write(&blocked, "not a directory").unwrap();
    service.directory = blocked.clone();
    assert!(service.set(false).is_err());
    assert!(!service.get().unwrap().enabled);
    assert_eq!(
        service.prepare_target_refresh().err().unwrap().code,
        crate::error::ErrorCode::Busy
    );
    release.send(()).unwrap();
    stopping.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(service.shared.lock().unwrap().preparing);
    assert!(service.effects.try_lock().is_err());
    stopped.send(()).unwrap();
    let mut engine = worker.join().unwrap();
    assert!(!service.shared.lock().unwrap().preparing);
    assert_eq!(engine.controller.calls, ["idle", "prepare", "stop"]);
    drop(service.prepare_target_refresh().unwrap());
    engine.tick(now + POLL);
    assert!(!engine.prepared);
    std::fs::remove_file(blocked).unwrap();
}

#[test]
fn target_commit_and_old_busy_or_renew_are_serialized_in_both_orders() {
    for renew in [false, true] {
        for commit_first in [false, true] {
            let now = Instant::now();
            let mut engine = engine(true);
            engine.tick(now);
            let generation = if renew {
                observe(&mut engine, Ok(1), now);
                engine.generation
            } else {
                engine.start_query(now).unwrap()
            };
            let service = service(&engine);
            let baseline = engine.controller.leases.len();
            let (busy, entered, release) = gate();
            engine.controller.busy_gate = Some(busy);
            if commit_first {
                let refresh = service.prepare_target_refresh().unwrap();
                let (attempting, attempt) = mpsc::channel();
                let worker = thread::spawn(move || {
                    attempting.send(()).unwrap();
                    if renew {
                        engine.tick(now + POLL);
                    } else {
                        engine.observation(generation, Ok(8), now + POLL);
                    }
                    engine
                });
                attempt.recv_timeout(Duration::from_secs(5)).unwrap();
                assert!(entered.try_recv().is_err());
                refresh.invalidate();
                engine = worker.join().unwrap();
                assert_eq!(engine.controller.leases.len(), baseline);
                engine.controller.busy_gate = None;
            } else {
                let worker = thread::spawn(move || {
                    if renew {
                        engine.tick(now + POLL);
                    } else {
                        engine.observation(generation, Ok(8), now + POLL);
                    }
                    engine
                });
                entered.recv_timeout(Duration::from_secs(5)).unwrap();
                assert!(service.effects.try_lock().is_err());
                let (attempting, attempt) = mpsc::channel();
                let (committed, commit) = mpsc::channel();
                thread::scope(|scope| {
                    scope.spawn(|| {
                        attempting.send(()).unwrap();
                        service.prepare_target_refresh().unwrap().invalidate();
                        committed.send(()).unwrap();
                    });
                    attempt.recv_timeout(Duration::from_secs(5)).unwrap();
                    assert!(commit.try_recv().is_err());
                    release.send(()).unwrap();
                    commit.recv_timeout(Duration::from_secs(5)).unwrap();
                });
                engine = worker.join().unwrap();
                assert_eq!(engine.controller.leases.len(), baseline + 1);
            }
            engine.tick(now + POLL * 2);
            assert!(!engine.protection);
            assert_eq!(engine.last_busy, None);
            assert_eq!(state(&engine).active_sessions, None);
            assert_eq!(
                engine
                    .controller
                    .calls
                    .iter()
                    .filter(|s| *s == "prepare")
                    .count(),
                1
            );
            assert!(!engine.controller.calls.iter().any(|s| s == "stop"));
        }
    }
}

#[test]
fn target_guard_prevents_authorization_until_commit_and_releases_on_abort() {
    let now = Instant::now();
    let mut engine = engine(true);
    let service = service(&engine);
    let (prepare, entered, release) = gate();
    engine.controller.prepare_gate = Some(prepare);
    let refresh = service.prepare_target_refresh().unwrap();
    let worker = thread::spawn(move || {
        engine.tick(now);
        engine
    });
    assert!(entered.try_recv().is_err());
    refresh.invalidate();
    entered.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(
        service.prepare_target_refresh().err().unwrap().code,
        crate::error::ErrorCode::Busy
    );
    release.send(()).unwrap();
    let engine = worker.join().unwrap();
    assert!(engine.prepared);
    assert!(!service.shared.lock().unwrap().preparing);
    let generation = engine.generation;
    drop(service.prepare_target_refresh().unwrap());
    assert_eq!(
        service.shared.lock().unwrap().desired.generation,
        generation
    );
    assert_eq!(engine.controller.calls, ["idle", "prepare"]);
}

#[test]
fn target_refresh_finish_only_queues_wake_and_does_not_fail_a_committed_save() {
    let engine = engine(false);
    let mut service = service(&engine);
    let (wake, events) = mpsc::channel();
    service.wake = wake;
    service.prepare_target_refresh().unwrap().invalidate();
    let generation = service.shared.lock().unwrap().desired.generation;
    service.finish_target_refresh();
    assert_eq!(events.try_recv(), Ok(()));
    drop(events);
    service.finish_target_refresh();
    assert_eq!(
        service.shared.lock().unwrap().desired.generation,
        generation
    );
    assert!(!service.get().unwrap().enabled);
    assert!(engine.controller.calls.is_empty());
}
