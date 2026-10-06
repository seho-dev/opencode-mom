use super::*;

#[derive(Default)]
struct Mock {
    calls: Vec<String>,
    leases: Vec<Duration>,
    fail_busy: bool,
    fail_idle: bool,
    fail_stop: bool,
    cancel_prepare: Option<Arc<Mutex<Shared>>>,
    cancel_busy: Option<Arc<Mutex<Shared>>>,
}

impl PowerController for Mock {
    fn prepare(&mut self) -> Result<(), AppError> {
        self.calls.push("prepare".into());
        if let Some(shared) = self.cancel_prepare.take() {
            let mut shared = shared.lock().unwrap();
            shared.desired.generation += 1;
            shared.desired.enabled = false;
        }
        Ok(())
    }
    fn busy(&mut self, ttl: Duration) -> Result<(), AppError> {
        self.calls.push("busy".into());
        self.leases.push(ttl);
        if let Some(shared) = self.cancel_busy.take() {
            let mut shared = shared.lock().unwrap();
            shared.desired.generation += 1;
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
        if self.fail_stop {
            Err(AppError::configuration("stop restore failed"))
        } else {
            Ok(())
        }
    }
}

fn engine(enabled: bool) -> Engine<Mock> {
    let shared = Arc::new(Mutex::new(Shared {
        desired: Desired {
            generation: 1,
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
    }));
    Engine::new(Mock::default(), shared)
}

fn state(engine: &Engine<Mock>) -> LidState {
    engine.shared.lock().unwrap().snapshot()
}

fn observe(engine: &mut Engine<Mock>, result: Result<usize, AppError>, now: Instant) {
    let generation = engine.start_query(now).unwrap();
    engine.observation(generation, result, now);
}

fn set(engine: &Engine<Mock>, enabled: bool) {
    let mut shared = engine.shared.lock().unwrap();
    shared.desired.enabled = enabled;
    shared.desired.generation += 1;
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
