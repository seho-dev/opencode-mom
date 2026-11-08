use std::fs;
use std::sync::mpsc;

use super::*;

fn setup() -> (tempfile::TempDir, ConfigPaths, Arc<LoggingService>) {
    let directory = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(directory.path());
    let service = LoggingService::new(&paths);
    (directory, paths, service)
}

fn op(service: &Arc<LoggingService>, event: &'static str) -> Operation {
    service.operation(LogType::Config, event)
}

fn record(sequence: u64) -> LogRecord {
    LogRecord {
        schema_version: 1,
        timestamp: "2026-11-01T00:00:00Z".to_owned(),
        session_id: Uuid::nil(),
        sequence,
        operation_id: Uuid::nil(),
        kind: LogType::Config,
        status: LogStatus::Success,
        event: "preferences_save".to_owned(),
        message: "Operation completed".to_owned(),
        duration_ms: Some(1),
        error_code: None,
        context: None,
    }
}

fn query(service: &LoggingService) -> LogPage {
    service.query(LogQuery::default()).unwrap()
}

#[test]
fn defaults_off_and_enable_persists_without_changing_other_settings() {
    let (_directory, paths, service) = setup();
    assert!(!service.state().enabled);
    assert!(!service.state().recording);
    op(&service, "project_remove").finish(&Ok::<_, AppError>(()));
    assert!(query(&service).records.is_empty());
    assert!(!paths.logs_dir().exists());
    let mut config = models::AppConfig::default();
    config.state.island_enabled = true;
    config.state.preferences.locale = models::LocalePreference::Zh;
    models::save_config(&paths.config_file(), &config).unwrap();
    assert!(service.set_enabled(true).unwrap().recording);
    config.state.logging_enabled = true;
    assert_eq!(models::load_config(&paths.config_file()).unwrap(), config);
    service.shutdown(Duration::from_secs(1));
    let restarted = LoggingService::new(&paths);
    assert!(restarted.state().enabled);
    assert!(restarted.state().recording);
    restarted.shutdown(Duration::from_secs(1));
}

#[test]
fn disable_is_a_barrier_and_old_operations_cannot_cross_enable_generations() {
    let (_directory, paths, service) = setup();
    let before = op(&service, "project_add");
    service.set_enabled(true).unwrap();
    before.finish(&Ok::<_, AppError>(()));
    let old = Operation::new(&service, LogType::Group, "group_copy");
    for _ in 0..20 {
        op(&service, "preferences_save").finish(&Ok::<_, AppError>(()));
    }

    let disabled = service.set_enabled(false).unwrap();
    assert!(!disabled.enabled);
    assert!(!disabled.recording);
    assert!(
        !models::load_config(&paths.config_file())
            .unwrap()
            .state
            .logging_enabled
    );
    assert_eq!(query(&service).records.len(), 20);
    op(&service, "project_remove").finish(&Ok::<_, AppError>(()));
    service.set_enabled(true).unwrap();
    old.checkpoint("group_switch_projection_complete");
    old.finish(&Ok::<_, AppError>(()));
    let new = Operation::new(&service, LogType::Group, "group_switch");
    new.checkpoint("group_switch_validated");
    new.finish(&Ok::<_, AppError>(()));
    new.finish(&Ok::<_, AppError>(()));
    let page = query(&service);
    assert_eq!(page.records.len(), 22);
    assert!(page.records.iter().all(|record| matches!(
        record.event.as_str(),
        "preferences_save" | "group_switch" | "group_switch_validated"
    )));
    assert_eq!(page.records[0].event, "group_switch");
    assert_eq!(page.records[1].event, "group_switch_validated");
    assert_eq!(page.records[0].operation_id, page.records[1].operation_id);
    service.shutdown(Duration::from_secs(1));
}

#[test]
fn failures_keep_only_error_code_and_fixed_safe_messages() {
    let (_directory, paths, service) = setup();
    service.set_enabled(true).unwrap();
    let error = AppError::references(
        "Bearer secret-api-key https://secret/path config contents prompt",
        "private session chat message",
    );
    op(&service, "preferences_save").finish::<()>(&Err(error));
    let page = query(&service);
    assert_eq!(page.records.len(), 1);
    assert_eq!(page.records[0].status, LogStatus::Error);
    assert_eq!(page.records[0].message, "Operation failed");
    assert_eq!(
        page.records[0].error_code,
        Some(ErrorCode::ReferencesBlocked)
    );
    let path = fs::read_dir(paths.logs_dir())
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let bytes = fs::read_to_string(path).unwrap();
    for forbidden in [
        "Bearer", "secret", "private", "prompt", "https://", "chat", "contents", "detail",
    ] {
        assert!(!bytes.contains(forbidden), "{forbidden}");
    }
    assert!(page.records[0].timestamp.ends_with('Z'));
    service.shutdown(Duration::from_secs(1));
}

#[test]
fn config_save_failure_does_not_toggle_runtime_or_overwrite_config() {
    let (_directory, paths, service) = setup();
    service.set_enabled(true).unwrap();
    fs::write(paths.config_file(), "{malformed").unwrap();
    assert!(service.set_enabled(false).is_err());
    assert!(service.state().enabled);
    assert!(service.state().recording);
    assert_eq!(
        service.state().error.as_deref(),
        Some("Logging preference could not be saved")
    );
    assert_eq!(
        fs::read_to_string(paths.config_file()).unwrap(),
        "{malformed"
    );
    service.shutdown(Duration::from_secs(1));
}

#[test]
fn bounded_nonblocking_queue_counts_drops_only_while_recording() {
    let (_directory, paths, real_service) = setup();
    real_service.shutdown(Duration::from_secs(1));
    let (sender, _receiver) = mpsc::sync_channel(1);
    let shared = Arc::new(Shared {
        gate: Mutex::new(Gate {
            generation: 1,
            active: true,
            sequence: 0,
        }),
        state: Mutex::new(LoggingState {
            enabled: true,
            recording: true,
            directory: String::new(),
            error: None,
            dropped_records: 0,
        }),
        recording: AtomicBool::new(true),
        dropped: AtomicU64::new(0),
    });
    let service = Arc::new(LoggingService {
        config_file: paths.config_file(),
        session_id: Uuid::new_v4(),
        shared,
        sender,
        controls: Mutex::new(()),
    });
    op(&service, "preferences_save").finish(&Ok::<_, AppError>(()));
    op(&service, "project_add").finish(&Ok::<_, AppError>(()));
    assert_eq!(service.state().dropped_records, 1);
    service.pause();
    op(&service, "project_remove").finish(&Ok::<_, AppError>(()));
    assert_eq!(service.state().dropped_records, 1);
}

#[test]
fn storage_failure_stops_recording_without_changing_business_result() {
    let (_directory, paths, service) = setup();
    service.set_enabled(true).unwrap();
    let path = fs::read_dir(paths.logs_dir())
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();
    let result: Result<_, AppError> = std::hint::black_box(Ok(42));
    op(&service, "preferences_save").finish(&result);
    assert_eq!(result.unwrap(), 42);
    assert!(service.query(LogQuery::default()).is_err());
    let state = service.state();
    assert!(state.enabled);
    assert!(!state.recording);
    assert!(state.error.is_some());
    assert_eq!(state.dropped_records, 1);
    service.shutdown(Duration::from_secs(1));
}

#[test]
fn shutdown_is_bounded_even_if_the_worker_does_not_acknowledge() {
    let (_directory, paths, real_service) = setup();
    real_service.shutdown(Duration::from_secs(1));
    let (sender, _receiver) = mpsc::sync_channel(1);
    let service = LoggingService {
        config_file: paths.config_file(),
        session_id: Uuid::new_v4(),
        shared: real_service.shared.clone(),
        sender,
        controls: Mutex::new(()),
    };
    let start = Instant::now();
    service.shutdown(Duration::from_millis(20));
    assert!(start.elapsed() < Duration::from_secs(1));
    assert!(!service.state().recording);
    assert!(service.state().error.is_some());
}

#[test]
fn pages_are_latest_first_snapshot_safe_and_filterable() {
    let directory = tempfile::tempdir().unwrap();
    let mut storage = Storage::new(directory.path().join("logs"));
    storage.start().unwrap();
    let operation_id = Uuid::new_v4();
    for sequence in 1..=5 {
        let mut value = record(sequence);
        value.operation_id = operation_id;
        if sequence % 2 == 0 {
            value.status = LogStatus::Error;
            value.message = "Operation failed".to_owned();
            value.error_code = Some(ErrorCode::ValidationFailed);
        }
        storage.write(&value).unwrap();
    }

    let first = storage
        .query(LogQuery {
            limit: Some(2),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(
        first
            .records
            .iter()
            .map(|record| record.sequence)
            .collect::<Vec<_>>(),
        [5, 4]
    );
    storage.write(&record(6)).unwrap();
    let second = storage
        .query(LogQuery {
            limit: Some(2),
            cursor: first.next_cursor.clone(),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(
        second
            .records
            .iter()
            .map(|record| record.sequence)
            .collect::<Vec<_>>(),
        [3, 2]
    );
    let third = storage
        .query(LogQuery {
            limit: Some(2),
            cursor: second.next_cursor,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(third.records[0].sequence, 1);
    assert!(third.next_cursor.is_none());
    let filtered = storage
        .query(LogQuery {
            kind: Some(LogType::Config),
            status: Some(LogStatus::Error),
            from: Some("2026-11-01T00:00:00Z".to_owned()),
            to: Some("2026-11-01T01:00:00+01:00".to_owned()),
            operation_id: Some(operation_id),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(
        filtered
            .records
            .iter()
            .map(|record| record.sequence)
            .collect::<Vec<_>>(),
        [4, 2]
    );
    assert!(storage
        .query(LogQuery {
            kind: Some(LogType::Power),
            ..Default::default()
        })
        .unwrap()
        .records
        .is_empty());

    for invalid in [
        LogQuery {
            limit: Some(0),
            ..Default::default()
        },
        LogQuery {
            limit: Some(201),
            ..Default::default()
        },
        LogQuery {
            from: Some("invalid".to_owned()),
            ..Default::default()
        },
        LogQuery {
            from: Some("2026-11-02T00:00:00Z".to_owned()),
            to: Some("2026-11-01T00:00:00Z".to_owned()),
            ..Default::default()
        },
        LogQuery {
            cursor: Some("x".repeat(4096)),
            ..Default::default()
        },
    ] {
        assert!(storage.query(invalid).is_err());
    }
    assert!(storage
        .query(LogQuery {
            kind: Some(LogType::Power),
            cursor: first.next_cursor,
            ..Default::default()
        })
        .is_err());
}

#[test]
fn rotation_and_clear_bound_storage_and_invalidate_cursors() {
    let directory = tempfile::tempdir().unwrap();
    let logs = directory.path().join("logs");
    let mut storage = Storage::new(logs.clone());
    storage.max_bytes = 1500;
    storage.start().unwrap();
    storage.write(&record(1)).unwrap();
    storage.write(&record(2)).unwrap();
    let cursor = storage
        .query(LogQuery {
            limit: Some(1),
            ..Default::default()
        })
        .unwrap()
        .next_cursor
        .unwrap();

    for sequence in 3..100 {
        storage.write(&record(sequence)).unwrap();
    }
    assert!(storage
        .query(LogQuery {
            cursor: Some(cursor),
            ..Default::default()
        })
        .is_err());
    assert!(fs::read_dir(&logs).unwrap().count() <= 5);

    for entry in fs::read_dir(&logs).unwrap() {
        assert!(entry.unwrap().metadata().unwrap().len() <= 1500);
    }

    let first = storage
        .query(LogQuery {
            limit: Some(1),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(first.records[0].sequence, 99);
    fs::write(logs.join("unrelated.txt"), "keep").unwrap();
    storage.clear(false).unwrap();
    assert_eq!(
        fs::read_to_string(logs.join("unrelated.txt")).unwrap(),
        "keep"
    );
    assert!(storage
        .query(LogQuery {
            cursor: first.next_cursor,
            ..Default::default()
        })
        .is_err());
    assert!(storage
        .query(LogQuery::default())
        .unwrap()
        .records
        .is_empty());
}

#[test]
fn malformed_and_oversized_lines_and_incomplete_tail_are_skipped() {
    let directory = tempfile::tempdir().unwrap();
    let logs = directory.path().join("logs");
    let mut storage = Storage::new(logs.clone());
    storage.start().unwrap();
    storage.write(&record(1)).unwrap();
    storage.stop().unwrap();
    let path = fs::read_dir(&logs).unwrap().next().unwrap().unwrap().path();
    let mut file = fs::OpenOptions::new().append(true).open(&path).unwrap();
    use std::io::Write;
    file.write_all(b"invalid\n").unwrap();
    file.write_all(&vec![b'x'; 9000]).unwrap();
    file.write_all(b"\n{\"unfinished\":").unwrap();
    drop(file);
    let page = storage.query(LogQuery::default()).unwrap();
    assert_eq!(page.records.len(), 1);
    assert_eq!(page.invalid_lines, 3);
    let mut unsafe_record = record(2);
    unsafe_record.message = "Bearer secret".to_owned();
    assert!(!unsafe_record.is_safe());
}

#[cfg(unix)]
#[test]
fn private_modes_and_symlink_nonregular_oversized_managed_files_are_enforced() {
    use std::os::unix::fs::{symlink, OpenOptionsExt, PermissionsExt};
    let directory = tempfile::tempdir().unwrap();
    let logs = directory.path().join("logs");
    let mut storage = Storage::new(logs.clone());
    storage.start().unwrap();
    storage.write(&record(1)).unwrap();
    storage.stop().unwrap();
    assert_eq!(
        fs::metadata(&logs).unwrap().permissions().mode() & 0o777,
        0o700
    );
    let path = fs::read_dir(&logs).unwrap().next().unwrap().unwrap().path();
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(storage.query(LogQuery::default()).is_err());
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();
    assert!(storage.clear(false).is_err());
    fs::remove_dir(&path).unwrap();
    let target = directory.path().join("secret");
    fs::write(&target, "untouched").unwrap();
    symlink(&target, &path).unwrap();
    assert!(storage.query(LogQuery::default()).is_err());
    assert!(storage.clear(false).is_err());
    assert_eq!(fs::read_to_string(&target).unwrap(), "untouched");
    fs::remove_file(&path).unwrap();
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)
        .unwrap();
    file.set_len(storage::MAX_FILE_BYTES + 1).unwrap();
    drop(file);
    assert!(storage.query(LogQuery::default()).is_err());
    fs::remove_file(path).unwrap();
    fs::remove_dir(&logs).unwrap();
    let target_directory = directory.path().join("elsewhere");
    fs::create_dir(&target_directory).unwrap();
    symlink(&target_directory, &logs).unwrap();
    assert!(storage.start().is_err());
}

#[test]
fn unsafe_startup_is_nonfatal_and_reports_safe_failure_state() {
    let directory = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(directory.path());
    let mut config = models::AppConfig::default();
    config.state.logging_enabled = true;
    models::save_config(&paths.config_file(), &config).unwrap();
    fs::write(paths.logs_dir(), "not a directory; secret").unwrap();
    let service = LoggingService::new(&paths);
    assert!(service.state().enabled);
    assert!(!service.state().recording);
    assert_eq!(
        service.state().error.as_deref(),
        Some("Local log storage unavailable")
    );
    service.shutdown(Duration::from_secs(1));
}

#[test]
fn enable_storage_failure_preserves_enabled_intent_and_returns_safe_runtime_state() {
    let (_directory, paths, service) = setup();
    fs::create_dir_all(paths.app_config_dir()).unwrap();
    fs::write(paths.logs_dir(), "not a directory").unwrap();
    let enabled = service.set_enabled(true).unwrap();
    assert!(enabled.enabled);
    assert!(!enabled.recording);
    assert_eq!(
        enabled.error.as_deref(),
        Some("Local log storage unavailable")
    );
    assert!(
        models::load_config(&paths.config_file())
            .unwrap()
            .state
            .logging_enabled
    );
    service.shutdown(Duration::from_secs(1));
}

#[test]
fn failed_enable_persistence_never_starts_the_writer() {
    let (_directory, paths, service) = setup();
    fs::create_dir_all(paths.app_config_dir()).unwrap();
    fs::write(paths.config_file(), "{malformed").unwrap();
    assert!(service.set_enabled(true).is_err());
    assert!(!service.state().enabled);
    assert!(!service.state().recording);
    assert!(!paths.logs_dir().exists());
    op(&service, "preferences_save").finish(&Ok::<_, AppError>(()));
    assert!(query(&service).records.is_empty());
    service.shutdown(Duration::from_secs(1));
}

#[test]
fn failed_enable_persistence_preserves_every_retained_file() {
    let (_directory, paths, service) = setup();
    for _ in 0..5 {
        service.set_enabled(true).unwrap();
        op(&service, "preferences_save").finish(&Ok::<_, AppError>(()));
        service.set_enabled(false).unwrap();
    }

    let snapshots = || {
        fs::read_dir(paths.logs_dir())
            .unwrap()
            .map(|entry| {
                let path = entry.unwrap().path();
                let bytes = fs::read(&path).unwrap();
                (path, bytes)
            })
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    let before = snapshots();
    assert_eq!(before.len(), 5);
    fs::write(paths.config_file(), "{malformed").unwrap();
    assert!(service.set_enabled(true).is_err());
    assert!(!service.state().enabled);
    assert!(!service.state().recording);
    assert_eq!(snapshots(), before);
    assert_eq!(
        fs::read_to_string(paths.config_file()).unwrap(),
        "{malformed"
    );
    service.shutdown(Duration::from_secs(1));
}

#[test]
fn replacing_a_snapshot_file_invalidates_the_cursor() {
    let directory = tempfile::tempdir().unwrap();
    let logs = directory.path().join("logs");
    let mut storage = Storage::new(logs.clone());
    storage.start().unwrap();
    storage.write(&record(1)).unwrap();
    storage.write(&record(2)).unwrap();
    storage.stop().unwrap();
    let first = storage
        .query(LogQuery {
            limit: Some(1),
            ..Default::default()
        })
        .unwrap();
    let path = fs::read_dir(&logs).unwrap().next().unwrap().unwrap().path();
    let bytes = fs::read(&path).unwrap();
    let old = logs.join("unrelated.backup");
    fs::rename(&path, &old).unwrap();
    fs::write(&path, bytes).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    }
    assert!(storage
        .query(LogQuery {
            cursor: first.next_cursor,
            ..Default::default()
        })
        .is_err());
}

#[test]
fn clear_acknowledges_old_records_and_starts_a_fresh_generation() {
    let (_directory, _paths, service) = setup();
    service.set_enabled(true).unwrap();
    let old = op(&service, "global_instructions_save");
    op(&service, "preferences_save").finish(&Ok::<_, AppError>(()));
    let cleared = service.clear().unwrap();
    assert!(cleared.enabled);
    assert!(cleared.recording);
    assert_eq!(cleared.dropped_records, 0);
    old.finish(&Ok::<_, AppError>(()));
    op(&service, "project_select").finish(&Ok::<_, AppError>(()));
    let page = query(&service);
    assert_eq!(page.records.len(), 1);
    assert_eq!(page.records[0].event, "project_select");
    service.set_enabled(false).unwrap();
    let cleared = service.clear().unwrap();
    assert!(!cleared.enabled);
    assert!(!cleared.recording);
    assert!(query(&service).records.is_empty());
    service.shutdown(Duration::from_secs(1));
}

#[cfg(unix)]
#[test]
fn partial_write_failure_disables_recording_without_exposing_io_details() {
    const CHILD: &str = "MOM_LOG_WRITE_FAULT_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "logging::tests::partial_write_failure_disables_recording_without_exposing_io_details",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        return;
    }

    let (_directory, _paths, service) = setup();
    service.set_enabled(true).unwrap();
    let limit = libc::rlimit {
        rlim_cur: 64,
        rlim_max: 64,
    };
    assert_ne!(
        // SAFETY: only this disposable child installs SIG_IGN and a file-size limit.
        unsafe { libc::signal(libc::SIGXFSZ, libc::SIG_IGN) },
        libc::SIG_ERR
    );
    // SAFETY: limit is a live initialized rlimit; no parent process limit is changed.
    assert_eq!(unsafe { libc::setrlimit(libc::RLIMIT_FSIZE, &limit) }, 0);
    op(&service, "preferences_save").finish(&Ok::<_, AppError>(()));
    let page = query(&service);
    assert!(page.records.is_empty());
    assert_eq!(page.invalid_lines, 1);
    let state = service.state();
    assert!(!state.recording);
    assert_eq!(state.error.as_deref(), Some("Local log write failed"));
    assert_eq!(state.dropped_records, 1);
    service.shutdown(Duration::from_secs(1));
}

#[test]
fn enabling_recovers_an_enabled_but_not_recording_logger() {
    let directory = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(directory.path());
    let mut config = models::AppConfig::default();
    config.state.logging_enabled = true;
    models::save_config(&paths.config_file(), &config).unwrap();
    fs::write(paths.logs_dir(), "unsafe").unwrap();
    let service = LoggingService::new(&paths);
    assert!(service.state().enabled);
    assert!(!service.state().recording);
    fs::remove_file(paths.logs_dir()).unwrap();
    let retried = service.set_enabled(true).unwrap();
    assert!(retried.enabled);
    assert!(retried.recording);
    assert!(retried.error.is_none());
    op(&service, "preferences_save").finish(&Ok::<_, AppError>(()));
    assert_eq!(query(&service).records.len(), 1);
    service.shutdown(Duration::from_secs(1));
}

#[test]
fn query_never_returns_untrusted_unknown_or_sensitive_string_fields() {
    use serde_json::json;
    use std::io::Write;

    let directory = tempfile::tempdir().unwrap();
    let logs = directory.path().join("logs");
    let mut storage = Storage::new(logs.clone());
    storage.start().unwrap();
    storage.write(&record(1)).unwrap();
    storage.stop().unwrap();
    let path = fs::read_dir(logs).unwrap().next().unwrap().unwrap().path();
    let base = serde_json::to_value(record(2)).unwrap();
    let mut bad_lines = Vec::new();
    for (key, field) in [
        ("event", json!("secret_api_key")),
        ("event", json!("group_switch")),
        ("message", json!("Bearer secret")),
        ("type", json!("secret_provider")),
        ("status", json!("secret")),
        ("errorCode", json!("secret_error")),
        ("timestamp", json!("private_prompt")),
        ("sessionId", json!("private_session")),
        ("operationId", json!("private_operation")),
        ("schemaVersion", json!(2)),
        ("context", json!({"step": "secret_api_key"})),
        ("context", json!({"step": "preferences_save"})),
        ("context", json!({"content": "private_config"})),
        ("content", json!("private_prompt")),
    ] {
        let mut value = base.clone();
        value[key] = field;
        bad_lines.push(value);
    }

    let mut wrong_step = base;
    wrong_step["type"] = json!("group");
    wrong_step["event"] = json!("group_switch_validated");
    wrong_step["message"] = json!("Checkpoint reached");
    wrong_step["context"] = json!({"step": "secret_api_key", "count": 1});
    bad_lines.push(wrong_step.clone());
    wrong_step["context"] = json!({"step": "group_switch_validated", "prompt": "private"});
    bad_lines.push(wrong_step);
    let mut file = fs::OpenOptions::new().append(true).open(path).unwrap();
    for value in &bad_lines {
        serde_json::to_writer(&mut file, value).unwrap();
        file.write_all(b"\n").unwrap();
    }
    drop(file);
    let page = storage.query(LogQuery::default()).unwrap();
    assert_eq!(page.records.len(), 1);
    assert_eq!(page.records[0].sequence, 1);
    assert_eq!(page.invalid_lines, bad_lines.len() as u64);
    let encoded = serde_json::to_string(&page).unwrap();
    assert!(!encoded.contains("private"));
    assert!(!encoded.contains("secret"));
}

#[test]
fn instance_api_captures_flows_lifecycle_and_filters_without_global_install() {
    let (_directory, _paths, service) = setup();
    service.set_enabled(true).unwrap();

    for result in [
        Ok(()),
        Err(AppError::references(
            "private configuration",
            "secret contents",
        )),
    ] {
        let operation = service.operation(LogType::Group, "group_switch");
        operation.checkpoint("group_switch_validated");
        operation.checkpoint("group_switch_projection_complete");
        operation.finish(&result);
    }
    service.record(LogType::Http, LogStatus::Error, "http_listener_failed");
    let page = query(&service);
    assert_eq!(page.records.len(), 7);
    assert_eq!(page.records[0].kind, LogType::Http);
    assert_eq!(page.records[0].status, LogStatus::Error);
    assert_ne!(page.records[1].operation_id, page.records[4].operation_id);

    for (index, status, code) in [
        (1, LogStatus::Error, Some(ErrorCode::ReferencesBlocked)),
        (4, LogStatus::Success, None),
    ] {
        let id = page.records[index].operation_id;
        let flow = service
            .query(LogQuery {
                operation_id: Some(id),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(flow.records.len(), 3);
        let finished = &flow.records[0];
        assert_eq!(finished.event, "group_switch");
        assert_eq!(finished.status, status);
        assert_eq!(finished.error_code, code);
        assert!(finished.duration_ms.is_some());
        assert!(finished.context.is_none());

        for (checkpoint, event) in flow.records[1..]
            .iter()
            .zip(["group_switch_projection_complete", "group_switch_validated"])
        {
            assert_eq!(checkpoint.status, LogStatus::Success);
            assert_eq!(checkpoint.event, event);
            assert_eq!(
                checkpoint.context.as_ref().unwrap().step.as_deref(),
                Some(event)
            );
            assert_eq!(checkpoint.operation_id, finished.operation_id);
            assert_eq!(checkpoint.session_id, finished.session_id);
            assert!(checkpoint.sequence < finished.sequence);
            assert!(checkpoint.duration_ms.is_none());
            assert!(checkpoint.error_code.is_none());
        }
    }

    let encoded = serde_json::to_string(&page).unwrap();
    assert!(!encoded.contains("private"));
    assert!(!encoded.contains("secret"));
    service.record(LogType::Config, LogStatus::Success, "secret_api_key");
    assert_eq!(query(&service).records.len(), 7);
    service.shutdown(Duration::from_secs(1));
}

#[test]
fn queued_records_drain_but_inflight_old_work_never_enters_a_new_generation() {
    let (_directory, _paths, service) = setup();
    service.set_enabled(true).unwrap();
    let (started, ready) = mpsc::channel();
    let (release, wait) = mpsc::channel();
    let worker_service = service.clone();
    let worker = std::thread::spawn(move || {
        let success = worker_service.operation(LogType::Group, "group_switch");
        let failure = worker_service.operation(LogType::Group, "group_switch");
        success.checkpoint("group_switch_validated");
        failure.checkpoint("group_switch_validated");
        started.send(()).unwrap();
        wait.recv().unwrap();
        success.checkpoint("group_switch_projection_complete");
        success.finish(&Ok::<_, AppError>(()));
        failure.checkpoint("group_switch_projection_complete");
        failure.finish::<()>(&Err(AppError::configuration("private failure")));
    });
    ready.recv().unwrap();

    for _ in 0..20 {
        op(&service, "preferences_save").finish(&Ok::<_, AppError>(()));
    }
    service.set_enabled(false).unwrap();
    let drained = query(&service);
    assert_eq!(drained.records.len(), 22);
    assert_eq!(drained.records[20].event, "group_switch_validated");
    service.set_enabled(true).unwrap();
    release.send(()).unwrap();
    worker.join().unwrap();
    op(&service, "project_select").finish(&Ok::<_, AppError>(()));
    let page = query(&service);
    assert_eq!(page.records.len(), 23);
    assert_eq!(page.records[0].event, "project_select");
    assert_eq!(
        page.records
            .iter()
            .filter(|record| record.kind == LogType::Group)
            .count(),
        2
    );
    service.shutdown(Duration::from_secs(1));
}

#[test]
fn every_declared_type_roundtrips_through_the_instance_writer_and_query() {
    let (_directory, _paths, service) = setup();
    service.set_enabled(true).unwrap();
    let events = [
        (LogType::App, "app.started"),
        (LogType::Config, "preferences_save"),
        (LogType::Group, "group_save"),
        (LogType::Resource, "provider_create"),
        (LogType::Opencode, "opencode_reload"),
        (LogType::Http, "http_key_rotate_unavailable"),
        (LogType::McpProxy, "mcp_proxy_settings_save"),
        (LogType::Power, "lid_protection_set"),
        (LogType::Island, "island_enabled_set"),
        (LogType::Logging, "logging_cleared"),
    ];

    for (kind, event) in events {
        service.record(kind, LogStatus::Success, event);
    }

    let page = query(&service);
    assert_eq!(page.records.len(), events.len());

    for (value, (kind, event)) in page.records.iter().zip(events.into_iter().rev()) {
        assert_eq!(value.kind, kind);
        assert_eq!(value.event, event);
    }
    service.shutdown(Duration::from_secs(1));
}

#[test]
fn invalid_queries_do_not_stop_healthy_recording() {
    let (_directory, _paths, service) = setup();
    service.set_enabled(true).unwrap();

    for query in [
        LogQuery {
            cursor: Some("invalid".to_owned()),
            ..Default::default()
        },
        LogQuery {
            from: Some("invalid".to_owned()),
            ..Default::default()
        },
        LogQuery {
            limit: Some(201),
            ..Default::default()
        },
    ] {
        assert_eq!(
            service.query(query).unwrap_err().code,
            ErrorCode::ValidationFailed
        );
        assert!(service.state().recording);
        assert!(service.state().error.is_none());
    }
    service.record(
        LogType::Http,
        LogStatus::Error,
        "http_key_rotate_unavailable",
    );
    let page = query(&service);
    assert_eq!(page.records.len(), 1);
    assert_eq!(page.records[0].event, "http_key_rotate_unavailable");
    service.shutdown(Duration::from_secs(1));
}

#[test]
fn committed_disable_returns_authoritative_state_when_the_barrier_fails() {
    let (_directory, paths, service) = setup();
    service.set_enabled(true).unwrap();
    service.shutdown(Duration::from_secs(1));
    let state = service.set_enabled(false).unwrap();
    assert!(!state.enabled);
    assert!(!state.recording);
    assert!(state.error.is_some());
    assert!(
        !models::load_config(&paths.config_file())
            .unwrap()
            .state
            .logging_enabled
    );
}

#[test]
fn notifications_safe_event_allowlist_fixed_strings() {
    use super::types::safe_event;
    use crate::logging::LogType;
    for event in [
        "notifications_connected",
        "notifications_connection_failed",
        "notifications_session_unavailable",
        "notifications_show_failed",
        "notifications_dispatch_failed",
        "notifications_preferences_unavailable",
    ] {
        assert!(safe_event(LogType::App, event), "allowlist missing {event}");
    }
    assert!(!safe_event(LogType::App, "notifications_injected"));
    assert!(!safe_event(LogType::App, "user_private_data"));
}
