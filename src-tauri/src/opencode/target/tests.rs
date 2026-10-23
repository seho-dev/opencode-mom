use std::fs;
use std::time::{Duration, Instant};

use serde_json::json;

use super::*;
use crate::opencode::transport;

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("mom-target-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn runtime(&self) -> Arc<TargetRuntime> {
        TargetRuntime::new(
            self.0.join("target.json"),
            self.0.join("registration.json"),
            self.0.clone(),
        )
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn only_exact_local_http_endpoints_with_positive_ports_are_accepted() {
    for (url, expected) in [
        ("http://localhost:80/", "http://127.0.0.1:80"),
        ("http://127.0.0.1:01234", "http://127.0.0.1:1234"),
        ("http://[::1]:65535", "http://[::1]:65535"),
    ] {
        assert_eq!(normalize_url(url).unwrap(), expected);
    }
    for url in [
        "http://127.1:1234",
        "http://2130706433:1234",
        "http://0x7f000001:1234",
        "http://127.0.0.2:1234",
        "http://localhost.:1234",
        "http://LOCALHOST:1234",
        "https://127.0.0.1:1234",
        "http://example.com:1234",
        "http://127.0.0.1",
        "http://localhost:0",
        "http://localhost:65536",
        "http://localhost:+80",
        "http://PRIVATE@localhost:1234",
        "http://localhost:1234/api",
        "http://localhost:1234/?PRIVATE",
        "http://localhost:1234/#PRIVATE",
        " http://localhost:1234",
        "http://localhost:1234\n",
        "http://[::ffff:127.0.0.1]:1234",
    ] {
        let error = normalize_url(url).unwrap_err();
        assert!(!error.message.contains("PRIVATE"));
        assert!(error.detail.is_none());
    }
}

#[test]
fn private_save_retains_only_same_normalized_endpoint_and_publishes_after_persistence() {
    let fixture = Fixture::new();
    let runtime = fixture.runtime();
    let original = runtime.snapshot().unwrap();
    let saved = runtime
        .set(
            Some("http://localhost:1234".into()),
            Some("PRIVATE_PASSWORD".into()),
            || Ok(()),
        )
        .unwrap();
    assert_eq!(
        saved.selection.url.as_deref(),
        Some("http://127.0.0.1:1234")
    );
    assert!(original.ensure_current().is_err());
    let json = serde_json::to_value(saved.selection.public()).unwrap();
    assert_eq!(
        json,
        json!({"mode":"explicit", "url":"http://127.0.0.1:1234", "hasPassword":true})
    );
    assert!(!json.to_string().contains("PRIVATE_PASSWORD"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&runtime.path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    let retained = runtime
        .set(Some("http://localhost:1234/".into()), None, || Ok(()))
        .unwrap();
    assert_eq!(
        retained.selection.password.as_deref(),
        Some("PRIVATE_PASSWORD")
    );
    let cleared = runtime
        .set(
            Some("http://localhost:1234".into()),
            Some(String::new()),
            || Ok(()),
        )
        .unwrap();
    assert!(cleared.selection.password.is_none());
    runtime
        .set(
            Some("http://localhost:1234".into()),
            Some("private".into()),
            || Ok(()),
        )
        .unwrap();
    let changed = runtime
        .set(Some("http://localhost:1235".into()), None, || Ok(()))
        .unwrap();
    assert!(changed.selection.password.is_none());
    let shared = runtime
        .set(None, Some("PRIVATE_IGNORED".into()), || Ok(()))
        .unwrap();
    assert!(shared.selection.password.is_none());
    assert_eq!(
        serde_json::to_value(shared.selection.public()).unwrap(),
        json!({"mode":"shared", "url":null, "hasPassword":false})
    );
    #[cfg(not(windows))]
    assert_eq!(
        serde_json::to_value(load(&runtime.path).unwrap()).unwrap(),
        json!({"url":null,"password":null})
    );
    #[cfg(windows)]
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&fs::read(&runtime.path).unwrap()).unwrap(),
        json!({"url":null,"encryptedPassword":null})
    );
    fs::remove_file(&runtime.path).unwrap();
    fs::create_dir(&runtime.path).unwrap();
    assert!(runtime
        .set(Some("http://localhost:1234".into()), None, || Ok(()))
        .is_err());
    shared.ensure_current().unwrap();
    assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 1);
}

#[test]
fn malformed_oversized_nonprivate_and_nonregular_configuration_fail_closed() {
    let fixture = Fixture::new();
    let path = fixture.0.join("target.json");
    for bytes in [
        b"PRIVATE_INVALID_JSON".to_vec(),
        vec![b'x'; 8193],
        br#"{"url":null,"password":"PRIVATE"}"#.to_vec(),
        br#"{"url":"http://PRIVATE:1234","password":null}"#.to_vec(),
        br#"[null,null]"#.to_vec(),
        br#"{}"#.to_vec(),
        br#"{"url":"http://localhost:1234","url":null,"password":null}"#.to_vec(),
    ] {
        private_file::publish(&path, &bytes).unwrap();
        let error = load(&path).err().unwrap();
        assert!(!error.message.contains("PRIVATE"));
        assert!(fixture.runtime().snapshot().is_err());
        fs::remove_file(&path).unwrap();
    }
    #[cfg(unix)]
    {
        use std::ffi::CString;
        use std::os::unix::{ffi::OsStrExt, fs::PermissionsExt};
        fs::write(&path, b"{}").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(load(&path).is_err());
        fs::remove_file(&path).unwrap();
        let name = CString::new(path.as_os_str().as_bytes()).unwrap();
        // SAFETY: name owns a NUL-terminated path for this call; mkfifo retains no pointer.
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        let now = Instant::now();
        assert!(load(&path).is_err());
        assert!(now.elapsed() < Duration::from_millis(200));
    }
    for password in ["PRIVATE\nPASSWORD".into(), "x".repeat(4097)] {
        assert!(updated(None, Some("http://localhost:1234".into()), Some(password)).is_err());
    }
}

#[cfg(not(windows))]
#[test]
fn temporary_target_bytes_are_wiped_after_load_and_failed_persist() {
    let fixture = Fixture::new();
    let path = fixture.0.join("target.json");
    let wiped = || crate::protected_data::WIPED_BYTES.with(std::cell::Cell::get);
    let selection = Selection {
        url: Some("http://127.0.0.1:1234".into()),
        password: Some("private-test-password".into()),
    };
    let before = wiped();
    persist(&path, &selection).unwrap();
    assert!(wiped() > before, "serialized target buffer must be wiped");
    let length = fs::metadata(&path).unwrap().len() as usize;
    let before = wiped();
    let loaded = load(&path).unwrap();
    assert_eq!(loaded.password, selection.password);
    assert_eq!(wiped() - before, length);
    private_file::publish(&path, b"invalid private target").unwrap();
    let before = wiped();
    assert!(load(&path).is_err());
    assert_eq!(wiped() - before, b"invalid private target".len());
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();
    let before = wiped();
    assert!(persist(&path, &selection).is_err());
    assert!(
        wiped() > before,
        "failed publication must wipe its serialized buffer"
    );
}

#[test]
fn temporary_decrypted_target_password_is_wiped_on_success_and_validation_failure() {
    let bytes = br#"{"url":"http://localhost:1234","encryptedPassword":[1]}"#;
    let wiped = || crate::protected_data::WIPED_BYTES.with(std::cell::Cell::get);
    for plaintext in [
        b"private-test-password".to_vec(),
        vec![255],
        vec![b'x'; 4097],
    ] {
        let length = plaintext.len();
        let before = wiped();
        let result = windows::decode(bytes, |_| Ok(plaintext));
        assert_eq!(result.is_ok(), length == b"private-test-password".len());
        assert_eq!(wiped() - before, length);
    }
}

#[test]
fn missing_shared_status_does_not_bootstrap_and_always_serializes_nulls() {
    let fixture = Fixture::new();
    let runtime = fixture.runtime();
    let status = transport::service_status(
        runtime.snapshot().unwrap(),
        Instant::now() + Duration::from_secs(1),
    )
    .unwrap();
    let json = serde_json::to_value(status).unwrap();
    assert_eq!(json["connected"], false);
    for key in ["url", "pid", "version"] {
        assert!(json.get(key).unwrap().is_null());
    }
    assert!(json["target"]["url"].is_null());
    assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 0);
}

#[test]
fn fixed_offline_save_succeeds_and_current_directory_is_captured_in_catalog_query() {
    use crate::opencode::models;
    use crate::opencode::token_usage::tests::{healthy_response, respond, FakeApi};
    let fixture = Fixture::new();
    let runtime = fixture.runtime();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let saved = runtime
        .set(Some(url), Some("private".into()), || Ok(()))
        .unwrap();
    let status = transport::service_status(saved, Instant::now() + Duration::from_secs(1)).unwrap();
    assert!(!status.connected && status.target.has_password);
    assert!(load(&runtime.path).unwrap().url.is_some());
    let directory = runtime.directory.to_str().unwrap().to_owned();
    let api = FakeApi::new(move |target, stream| {
        if target == "/api/info" {
            healthy_response(stream);
        } else {
            let url = reqwest::Url::parse(&format!("http://127.0.0.1:1234{target}")).unwrap();
            assert!(matches!(url.path(), "/api/model" | "/api/location/reload"));
            assert_eq!(
                url.query_pairs().collect::<Vec<_>>(),
                [("location[directory]".into(), directory.clone().into())]
            );
            if url.path() == "/api/model" {
                respond(stream, 200, "", br#"{"location":{},"data":[]}"#);
            } else {
                respond(stream, 204, "", b"");
            }
        }
    });
    let snapshot = runtime
        .set(Some(api.url()), Some("private".into()), || Ok(()))
        .unwrap();
    let connection = transport::connect_target(
        snapshot,
        &runtime.registration,
        Instant::now() + Duration::from_secs(1),
        |_| panic!("explicit must not bootstrap"),
    )
    .unwrap();
    assert!(
        models::model_refs(&connection, Instant::now() + Duration::from_secs(1))
            .unwrap()
            .is_empty()
    );
    connection
        .reload(Instant::now() + Duration::from_secs(1))
        .unwrap();
}

#[test]
fn stale_long_read_and_old_async_connection_cannot_return_or_write_after_target_change() {
    use crate::opencode::active;
    use crate::opencode::token_usage::tests::{healthy_response, respond, FakeApi};
    let fixture = Fixture::new();
    let runtime = fixture.runtime();
    let changed = runtime.clone();
    let api = FakeApi::new(move |target, stream| {
        if target == "/api/info" {
            healthy_response(stream);
        } else {
            changed.set(None, None, || Ok(())).unwrap();
            respond(stream, 200, "", br#"{"data":{}}"#);
        }
    });
    let snapshot = runtime
        .set(Some(api.url()), Some("private".into()), || Ok(()))
        .unwrap();
    let connection = transport::connect_target(
        snapshot,
        &runtime.registration,
        Instant::now() + Duration::from_secs(1),
        |_| panic!("no bootstrap"),
    )
    .unwrap();
    let async_connection = transport::connect_target(
        runtime.snapshot().unwrap(),
        &runtime.registration,
        Instant::now() + Duration::from_secs(1),
        |_| panic!("no bootstrap"),
    )
    .unwrap()
    .into_async()
    .unwrap();
    let error =
        active::active_sessions(&connection, Instant::now() + Duration::from_secs(1)).unwrap_err();
    assert!(error.message.contains("target changed"));
    assert!(connection
        .reload(Instant::now() + Duration::from_secs(1))
        .is_err());
    assert!(connection.into_async().is_err());
    assert!(async_connection
        .request(reqwest::Method::POST, &["location", "reload"])
        .is_err());
}

#[test]
fn dispatch_lock_wait_is_bounded_by_operation_deadline() {
    let guard = transition().unwrap();
    let now = Instant::now();
    let worker = std::thread::spawn(|| {
        transition_until(Instant::now() + Duration::from_millis(30))
            .err()
            .unwrap()
    });
    assert!(worker.join().unwrap().message.contains("timed out"));
    assert!(now.elapsed() < Duration::from_millis(200));
    drop(guard);
}

#[test]
fn async_reservation_connect_yields_and_cancelled_worker_retains_guard() {
    use crate::opencode::token_usage::tests::{healthy_response, FakeApi};

    let fixture = Fixture::new();
    let target = fixture.runtime();
    let (entered_tx, mut entered_rx) = tokio::sync::mpsc::unbounded_channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let release_rx = std::sync::Mutex::new(release_rx);
    let api = FakeApi::new(move |path, stream| {
        assert_eq!(path, "/api/info");
        entered_tx.send(()).unwrap();
        release_rx
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        healthy_response(stream);
    });
    let snapshot = target
        .set(Some(api.url()), Some("private".into()), || Ok(()))
        .unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let guard = transition().unwrap();
    let heartbeat = runtime.block_on(async {
        let mut connecting = Box::pin(crate::opencode::island::connect_snapshot(
            snapshot,
            Instant::now() + Duration::from_secs(1),
        ));
        let pending = futures::poll!(&mut connecting).is_pending();
        tokio::task::yield_now().await;
        drop(connecting);
        drop(guard);
        if !pending {
            return false;
        }
        tokio::time::timeout(Duration::from_secs(2), entered_rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(target.in_flight.load(Ordering::SeqCst), 1);
        assert_eq!(
            target.set(None, None, || Ok(())).err().unwrap().code,
            ErrorCode::Busy
        );
        release_tx.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(2), async {
            while target.in_flight.load(Ordering::SeqCst) != 0 || Arc::strong_count(&target) != 1 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        true
    });
    assert!(
        heartbeat,
        "connect reservation blocked the current-thread heartbeat"
    );
    target.set(None, None, || Ok(())).unwrap();
}

#[test]
fn async_reservation_pending_write_yields_and_drops_unclaimed_guard() {
    use crate::opencode::token_usage::tests::{healthy_response, FakeApi};

    let fixture = Fixture::new();
    let target = fixture.runtime();
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let api = FakeApi::new(move |path, stream| {
        assert_eq!(path, "/api/info");
        count.fetch_add(1, Ordering::SeqCst);
        healthy_response(stream);
    });
    let snapshot = target
        .set(Some(api.url()), Some("private".into()), || Ok(()))
        .unwrap();
    let connection = transport::connect_target(
        snapshot,
        &target.registration,
        Instant::now() + Duration::from_secs(2),
        |_| panic!("fake fixed target must not bootstrap"),
    )
    .unwrap()
    .into_async()
    .unwrap();
    let owners = Arc::strong_count(&target);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let guard = transition().unwrap();
    let heartbeat = runtime.block_on(async {
        let mut prepare =
            Box::pin(connection.prepare_write(Instant::now() + Duration::from_secs(1)));
        let pending = futures::poll!(&mut prepare).is_pending();
        tokio::task::yield_now().await;
        drop(prepare);
        let owned = Arc::strong_count(&target) > owners;
        drop(guard);
        if !pending {
            return false;
        }
        assert!(
            owned,
            "cancelled waiter must leave the blocking snapshot owned"
        );
        tokio::time::timeout(Duration::from_secs(2), async {
            while target.in_flight.load(Ordering::SeqCst) != 0
                || Arc::strong_count(&target) != owners
            {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        true
    });
    assert!(
        heartbeat,
        "write reservation blocked the current-thread heartbeat"
    );
    assert_eq!(
        calls.load(Ordering::SeqCst),
        1,
        "cancelled acquisition must not dispatch health"
    );
    target.set(None, None, || Ok(())).unwrap();
}

#[test]
fn async_reservation_write_covers_health_and_request_until_completion_or_cancel() {
    use crate::opencode::token_usage::tests::{healthy_response, respond, FakeApi};
    use std::io::Read;

    for cancel in [false, true] {
        let fixture = Fixture::new();
        let target = fixture.runtime();
        let health = Arc::new(AtomicUsize::new(0));
        let count = health.clone();
        let (entered_tx, mut entered_rx) = tokio::sync::mpsc::unbounded_channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let release_rx = std::sync::Mutex::new(release_rx);
        let api = FakeApi::new(move |path, stream| {
            if path == "/api/info" {
                if count.fetch_add(1, Ordering::SeqCst) > 0 {
                    entered_tx.send("health").unwrap();
                    release_rx
                        .lock()
                        .unwrap()
                        .recv_timeout(Duration::from_secs(5))
                        .unwrap();
                }
                healthy_response(stream);
            } else {
                assert_eq!(path, "/api/session/ses_fake/prompt");
                let mut body = vec![
                    0;
                    serde_json::to_vec(&json!({
                        "id": "msg_fake", "text": "fake text", "delivery": "queue",
                    }))
                    .unwrap()
                    .len()
                ];
                stream.read_exact(&mut body).unwrap();
                entered_tx.send("request").unwrap();
                release_rx
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(5))
                    .unwrap();
                respond(stream, 200, "", b"null");
            }
        });
        let snapshot = target
            .set(Some(api.url()), Some("private".into()), || Ok(()))
            .unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let api = crate::opencode::island::connect_snapshot(
                snapshot,
                Instant::now() + Duration::from_secs(2),
            )
            .await
            .unwrap();
            assert_eq!(target.in_flight.load(Ordering::SeqCst), 0);
            let waiter =
                tokio::spawn(async move { api.send("ses_fake", "fake text", "msg_fake").await });
            for phase in ["health", "request"] {
                assert_eq!(
                    tokio::time::timeout(Duration::from_secs(2), entered_rx.recv())
                        .await
                        .unwrap()
                        .unwrap(),
                    phase
                );
                assert_eq!(target.in_flight.load(Ordering::SeqCst), 1);
                assert_eq!(
                    target.set(None, None, || Ok(())).err().unwrap().code,
                    ErrorCode::Busy
                );
                if phase == "health" {
                    release_tx.send(()).unwrap();
                }
            }
            if cancel {
                waiter.abort();
                assert!(waiter.await.unwrap_err().is_cancelled());
                assert_eq!(target.in_flight.load(Ordering::SeqCst), 0);
                release_tx.send(()).unwrap();
            } else {
                release_tx.send(()).unwrap();
                waiter.await.unwrap().unwrap();
                assert_eq!(target.in_flight.load(Ordering::SeqCst), 0);
            }
        });
        target.set(None, None, || Ok(())).unwrap();
    }
}

#[test]
fn reservation_blocks_save_without_holding_transition_and_rejects_stale_dispatch() {
    let fixture = Fixture::new();
    let runtime = fixture.runtime();
    let snapshot = runtime.snapshot().unwrap();
    let reservation = snapshot
        .reserve_side_effect(Instant::now() + Duration::from_secs(1))
        .unwrap();
    drop(transition().unwrap());
    let started = Instant::now();
    let error = runtime
        .set_with_refresh(
            None,
            None,
            || -> Result<(), AppError> { panic!("busy save must not prepare") },
            |()| panic!("busy save must not commit"),
        )
        .err()
        .unwrap();
    assert_eq!(error.code, ErrorCode::Busy);
    assert!(started.elapsed() < Duration::from_millis(200));
    assert!(!runtime.path.exists());
    drop(reservation);
    runtime.set(None, None, || Ok(())).unwrap();
    assert!(snapshot
        .reserve_side_effect(Instant::now() + Duration::from_secs(1))
        .is_err());
}

#[test]
fn in_flight_reload_post_rejects_target_save_before_prepare() {
    use crate::opencode::token_usage::tests::{healthy_response, respond, FakeApi};

    let fixture = Fixture::new();
    let runtime = fixture.runtime();
    let changed = runtime.clone();
    let posts = Arc::new(AtomicUsize::new(0));
    let calls = posts.clone();
    let api = FakeApi::new(move |target, stream| {
        if target == "/api/info" {
            healthy_response(stream);
        } else {
            assert!(target.starts_with("/api/location/reload?"));
            calls.fetch_add(1, Ordering::SeqCst);
            let error = changed
                .set_with_refresh(
                    None,
                    None,
                    || -> Result<(), AppError> { panic!("in-flight POST must not prepare") },
                    |()| panic!("in-flight POST must not commit"),
                )
                .err()
                .unwrap();
            assert_eq!(error.code, ErrorCode::Busy);
            respond(stream, 204, "", b"");
        }
    });
    let snapshot = runtime
        .set(Some(api.url()), Some("private".into()), || Ok(()))
        .unwrap();
    let connection = transport::connect_target(
        snapshot.clone(),
        &runtime.registration,
        Instant::now() + Duration::from_secs(1),
        |_| panic!("explicit must not bootstrap"),
    )
    .unwrap();
    connection
        .reload(Instant::now() + Duration::from_secs(1))
        .unwrap();
    snapshot.ensure_current().unwrap();
    assert_eq!(posts.load(Ordering::SeqCst), 1);
}

#[test]
fn cancelled_blocking_bootstrap_still_reserves_target_until_worker_finishes() {
    let fixture = Fixture::new();
    let runtime = fixture.runtime();
    let snapshot = runtime.snapshot().unwrap();
    let registration = runtime.registration.clone();
    let deadline = Instant::now() + Duration::from_secs(5);
    let reservation = snapshot.reserve_side_effect(deadline).unwrap();
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let (finished_tx, finished_rx) = std::sync::mpsc::channel();
    let worker = tauri::async_runtime::spawn_blocking(move || {
        let result = {
            let _reservation = reservation;
            transport::connect_target(snapshot, &registration, deadline, |_| {
                entered_tx.send(()).unwrap();
                release_rx.recv_timeout(Duration::from_secs(5)).unwrap();
                Err(AppError::configuration("synthetic bootstrap complete"))
            })
        };
        finished_tx.send(()).unwrap();
        result.map(|_| ())
    });
    entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!(runtime.in_flight.load(Ordering::SeqCst), 2);
    worker.abort();
    drop(worker);
    let error = runtime
        .set_with_refresh(
            None,
            None,
            || -> Result<(), AppError> { panic!("in-flight bootstrap must not prepare") },
            |()| panic!("in-flight bootstrap must not commit"),
        )
        .err()
        .unwrap();
    assert_eq!(error.code, ErrorCode::Busy);
    drop(transition().unwrap());
    release_tx.send(()).unwrap();
    finished_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    runtime.set(None, None, || Ok(())).unwrap();
}

#[test]
fn registration_uses_captured_home_and_absolute_state_override_not_config_parents() {
    let fixture = Fixture::new();
    let home = fixture.0.join("home");
    let paths = ConfigPaths::with_resource_parts(
        home.clone(),
        Some(fixture.0.join("custom/config.json")),
        Some(fixture.0.join("elsewhere")),
        fixture.0.join("moved/config"),
        fixture.0.clone(),
    );
    assert_eq!(
        registration_path(&paths, None),
        home.join(".local/state/opencode/service.json")
    );
    assert_eq!(
        registration_path(&paths, Some("relative".into())),
        home.join(".local/state/opencode/service.json")
    );
    let state = fixture.0.join("state");
    assert_eq!(
        registration_path(&paths, Some(state.clone())),
        state.join("opencode/service.json")
    );
}

#[test]
fn windows_disk_contract_keeps_only_ciphertext_and_preserves_password_selection_rules() {
    let selection = updated(
        None,
        Some("http://localhost:1234".into()),
        Some("local-secret".into()),
    )
    .unwrap();
    let bytes = windows::encode(&selection, |password| {
        assert_eq!(password, b"local-secret");
        Ok(vec![7, 8, 9])
    })
    .unwrap();
    let disk: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        disk,
        json!({"url":"http://127.0.0.1:1234", "encryptedPassword":[7,8,9]})
    );
    assert!(!String::from_utf8(bytes.clone())
        .unwrap()
        .contains("local-secret"));
    assert!(disk.get("password").is_none());
    let restored = windows::decode(&bytes, |cipher| {
        assert_eq!(cipher, [7, 8, 9]);
        Ok(b"local-secret".to_vec())
    })
    .unwrap();
    assert_eq!(restored.password.as_deref(), Some("local-secret"));
    let retained = updated(Some(&restored), Some("http://localhost:1234/".into()), None).unwrap();
    assert_eq!(retained.password, restored.password);
    for cleared in [
        updated(
            Some(&restored),
            Some("http://localhost:1234".into()),
            Some(String::new()),
        )
        .unwrap(),
        updated(Some(&restored), Some("http://localhost:1235".into()), None).unwrap(),
        updated(Some(&restored), None, None).unwrap(),
    ] {
        assert!(cleared.password.is_none());
        let bytes = windows::encode(&cleared, |_| panic!("no crypto for absent password")).unwrap();
        assert!(
            windows::decode(&bytes, |_| panic!("no crypto for absent password"))
                .unwrap()
                .password
                .is_none()
        );
    }
    assert_eq!(
        serde_json::to_value(restored.public()).unwrap(),
        json!({"mode":"explicit", "url":"http://127.0.0.1:1234", "hasPassword":true})
    );
}

#[test]
fn windows_encrypted_config_rejects_legacy_malformed_and_oversized_blobs_before_crypto() {
    for value in [
        json!({"url":"http://localhost:1234", "password":"local-secret"}),
        json!({"url":"http://localhost:1234", "encryptedPassword":[]}),
        json!({"url":"http://localhost:1234", "encryptedPassword":[256]}),
        json!({"url":"http://localhost:1234", "encryptedPassword":[-1]}),
        json!({"url":"http://localhost:1234", "encryptedPassword":"local-secret"}),
        json!({"url":"http://localhost:1234", "encryptedPassword":vec![0; 8193]}),
        json!({"url":null, "encryptedPassword":[1]}),
        json!({"url":"http://remote:1234", "encryptedPassword":[1]}),
        json!({"url":"http://localhost:1234"}),
    ] {
        let bytes = serde_json::to_vec(&value).unwrap();
        let error = windows::decode(&bytes, |_| panic!("invalid config must not decrypt"))
            .err()
            .unwrap();
        assert!(!error.message.contains("local-secret"));
        assert!(error.detail.is_none());
    }
    let bytes = vec![b' '; windows::MAX_FILE_BYTES + 1];
    assert!(windows::decode(&bytes, |_| panic!("oversized file must not decrypt")).is_err());
    assert!(windows::decode(
        br#"{"url":"http://localhost:1234","encryptedPassword":[1],"encryptedPassword":null}"#,
        |_| panic!("duplicate must not decrypt")
    )
    .is_err());
    let max = serde_json::to_vec(
        &json!({"url":"http://localhost:1234", "encryptedPassword":vec![0; 8192]}),
    )
    .unwrap();
    assert!(windows::decode(&max, |cipher| {
        assert_eq!(cipher.len(), 8192);
        Ok(b"valid".to_vec())
    })
    .is_ok());
}

#[test]
fn windows_crypto_failure_and_invalid_plaintext_never_fallback_or_leak_diagnostics() {
    let selection = Selection {
        url: Some("http://localhost:1234".into()),
        password: Some("local-secret".into()),
    };
    let error = windows::encode(&selection, |_| {
        Err(AppError::configuration("PRIVATE_PROTECT_ERROR"))
    })
    .unwrap_err();
    assert!(!error.message.contains("PRIVATE"));
    for ciphertext in [Vec::new(), vec![0; 8193]] {
        assert!(windows::encode(&selection, |_| Ok(ciphertext)).is_err());
    }
    let bytes = br#"{"url":"http://localhost:1234","encryptedPassword":[1,2]}"#;
    let error = windows::decode(bytes, |_| {
        Err(AppError::configuration("PRIVATE_WRONG_USER"))
    })
    .err()
    .unwrap();
    assert!(error.message.contains("decryption failed"));
    assert!(!error.message.contains("PRIVATE"));
    for plaintext in [
        Vec::new(),
        vec![b'x'; 4097],
        vec![255],
        b"PRIVATE\nSECRET".to_vec(),
    ] {
        let error = windows::decode(bytes, |_| Ok(plaintext)).err().unwrap();
        assert!(!error.message.contains("PRIVATE"));
        assert!(error.detail.is_none());
    }
    assert!(windows::decode(bytes, |_| Ok(vec![b'x'; 4096])).is_ok());
}

#[cfg(windows)]
#[test]
fn native_dpapi_roundtrip_corruption_and_legacy_repair_are_fail_closed() {
    let fixture = Fixture::new();
    let runtime = fixture.runtime();
    runtime
        .set(
            Some("http://localhost:1234".into()),
            Some("local-secret".into()),
            || Ok(()),
        )
        .unwrap();
    let bytes = fs::read(&runtime.path).unwrap();
    assert!(!String::from_utf8(bytes.clone())
        .unwrap()
        .contains("local-secret"));
    assert_eq!(
        load(&runtime.path).unwrap().password.as_deref(),
        Some("local-secret")
    );
    let mut disk: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    disk["encryptedPassword"] = json!([1, 2, 3, 4]);
    fs::write(&runtime.path, serde_json::to_vec(&disk).unwrap()).unwrap();
    assert!(load(&runtime.path).is_err());
    assert!(crate::protected_data::unprotect(&[]).is_err());
    assert!(crate::protected_data::unprotect(&[1, 2, 3]).is_err());
    fs::write(
        &runtime.path,
        br#"{"url":"http://localhost:1234","password":"local-secret"}"#,
    )
    .unwrap();
    let broken = fixture.runtime();
    assert!(broken.snapshot().is_err());
    let cleared = broken
        .set(
            Some("http://localhost:1234".into()),
            Some(String::new()),
            || Ok(()),
        )
        .unwrap();
    assert!(!cleared.selection.public().has_password);
    assert!(load(&runtime.path).unwrap().password.is_none());
}

#[test]
fn private_publication_failure_preserves_original_and_cleans_owned_temporary() {
    let fixture = Fixture::new();
    let path = fixture.0.join("target.json");
    fs::write(&path, b"original bytes").unwrap();
    let result = private_file::publish_with(
        &path,
        b"replacement bytes",
        |_, _| Err(std::io::Error::other("injected")),
        || Ok(()),
    );
    assert_eq!(result, Err(private_file::PublishError::Publication));
    assert_eq!(fs::read(&path).unwrap(), b"original bytes");
    assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 1);
}

#[cfg(unix)]
#[test]
fn private_publication_reports_directory_sync_failure_after_commit() {
    let fixture = Fixture::new();
    let path = fixture.0.join("target.json");
    let outcome = private_file::publish_with(
        &path,
        b"committed bytes",
        |source, destination| fs::rename(source, destination),
        || Err(std::io::Error::other("injected")),
    )
    .unwrap();
    assert_eq!(outcome, private_file::PublishOutcome::CommittedWithWarning);
    assert_eq!(fs::read(&path).unwrap(), b"committed bytes");
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let metadata = fs::metadata(&path).unwrap();
    assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
    // SAFETY: geteuid has no pointer arguments or preconditions and only reads process identity.
    assert_eq!(metadata.uid(), unsafe { libc::geteuid() });
}

#[cfg(unix)]
#[test]
fn failed_target_save_keeps_existing_private_file_bytes_and_runtime_selection() {
    // SAFETY: geteuid has no pointer arguments or preconditions and only reads process identity.
    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    let runtime = fixture.runtime();
    runtime
        .set(
            Some("http://localhost:1234".into()),
            Some("private".into()),
            || Ok(()),
        )
        .unwrap();
    let original = fs::read(&runtime.path).unwrap();
    let directory_mode = fs::metadata(&fixture.0).unwrap().permissions().mode() & 0o777;
    fs::set_permissions(&fixture.0, fs::Permissions::from_mode(0o500)).unwrap();
    let result = runtime.set(
        Some("http://localhost:1235".into()),
        Some("replacement".into()),
        || Ok(()),
    );
    fs::set_permissions(&fixture.0, fs::Permissions::from_mode(directory_mode)).unwrap();
    assert!(result.is_err());
    assert_eq!(fs::read(&runtime.path).unwrap(), original);
    assert_eq!(
        runtime.snapshot().unwrap().selection.url.as_deref(),
        Some("http://127.0.0.1:1234")
    );
    assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 1);
}

#[cfg(not(windows))]
#[test]
fn non_windows_data_protection_stub_fails_closed_without_claiming_dpapi() {
    let error = crate::protected_data::protect(b"bytes").unwrap_err();
    assert!(error.message.contains("unavailable"));
    assert!(crate::protected_data::unprotect(b"bytes").is_err());
}

#[test]
fn invalid_selection_prepare_failure_and_failed_persist_never_invalidate_monitors() {
    let fixture = Fixture::new();
    let runtime = fixture.runtime();
    let old = runtime.snapshot().unwrap();
    let monitor = Mutex::new(17);
    assert!(runtime
        .set_with_refresh(
            Some("http://PRIVATE:1234".into()),
            None,
            || -> Result<(), AppError> { panic!("invalid selection must not prepare") },
            |()| panic!("invalid selection must not commit"),
        )
        .is_err());
    assert!(runtime
        .set_with_refresh(
            None,
            None,
            || {
                let _guard = monitor.lock().unwrap();
                Err::<(), _>(AppError::configuration("synthetic prepare failure"))
            },
            |()| panic!("failed preparation must not commit"),
        )
        .is_err());
    assert!(!runtime.path.exists());
    fs::create_dir(&runtime.path).unwrap();
    assert!(runtime
        .set_with_refresh(
            Some("http://localhost:1234".into()),
            Some("PRIVATE_PASSWORD".into()),
            || Ok(monitor.lock().unwrap()),
            |_| panic!("failed persistence must not commit"),
        )
        .is_err());
    assert_eq!(*monitor.try_lock().unwrap(), 17);
    old.ensure_current().unwrap();
    assert!(runtime.snapshot().unwrap().selection.url.is_none());
    assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 1);
}

#[test]
fn commit_publishes_then_invalidates_under_guards_and_finish_precedes_probe_without_transition() {
    use crate::opencode::token_usage::tests::{healthy_response, FakeApi};
    use std::sync::atomic::AtomicBool;

    let fixture = Fixture::new();
    let runtime = fixture.runtime();
    let old = runtime.snapshot().unwrap();
    let monitor = Mutex::new(Some(old.revision));
    let commits = AtomicUsize::new(0);
    let finished = Arc::new(AtomicBool::new(false));
    let probed = Arc::new(AtomicBool::new(false));
    let finished_copy = finished.clone();
    let probed_copy = probed.clone();
    let api = FakeApi::new(move |target, stream| {
        assert_eq!(target, "/api/info");
        assert!(finished_copy.load(Ordering::SeqCst));
        probed_copy.store(true, Ordering::SeqCst);
        healthy_response(stream);
    });
    let (saved, refresh) = runtime
        .set_with_refresh(
            Some(api.url()),
            Some("private".into()),
            || {
                old.ensure_current().unwrap();
                assert!(!runtime.path.exists());
                Ok(monitor.lock().unwrap())
            },
            |mut guard| {
                assert_eq!(commits.fetch_add(1, Ordering::SeqCst), 0);
                assert!(TRANSITION.try_lock().is_err());
                assert!(monitor.try_lock().is_err());
                let current = runtime.snapshot().unwrap();
                assert_eq!(current.revision, old.revision + 1);
                assert_eq!(load(&runtime.path).unwrap().url, current.selection.url);
                assert_eq!(current.selection.password.as_deref(), Some("private"));
                assert!(old.ensure_current().is_err());
                assert_eq!(*guard, Some(old.revision));
                *guard = None;
                assert!(!finished.load(Ordering::SeqCst));
                assert!(!probed.load(Ordering::SeqCst));
                current.revision
            },
        )
        .unwrap();
    let status = finish_target(saved, refresh, |revision| {
        let _transition = transition_until(Instant::now() + Duration::from_secs(1)).unwrap();
        assert_eq!(runtime.snapshot().unwrap().revision, revision);
        assert!(monitor.try_lock().unwrap().is_none());
        assert!(!probed.load(Ordering::SeqCst));
        finished.store(true, Ordering::SeqCst);
    });
    assert_eq!(commits.load(Ordering::SeqCst), 1);
    assert!(status.connected);
    assert!(probed.load(Ordering::SeqCst));
}

#[test]
fn post_commit_status_race_returns_saved_public_target_not_an_error_or_later_target() {
    use crate::opencode::token_usage::tests::{healthy_response, FakeApi};

    let fixture = Fixture::new();
    let runtime = fixture.runtime();
    let later_calls = Arc::new(AtomicUsize::new(0));
    let calls = later_calls.clone();
    let later = FakeApi::new(move |_, stream| {
        calls.fetch_add(1, Ordering::SeqCst);
        healthy_response(stream);
    });
    let later_url = later.url();
    let changed = runtime.clone();
    let saved_api = FakeApi::new(move |_, stream| {
        changed
            .set(Some(later_url.clone()), Some("private".into()), || Ok(()))
            .unwrap();
        healthy_response(stream);
    });
    let (saved, ()) = runtime
        .set_with_refresh(
            Some(saved_api.url()),
            Some("private".into()),
            || Ok(()),
            |()| (),
        )
        .unwrap();
    let old = saved.clone();
    let status = finish_target(saved, (), |()| ());
    assert!(!status.connected);
    assert_eq!(status.target.url.as_deref(), Some(saved_api.url().as_str()));
    assert!(status.target.has_password);
    assert!(status.url.is_none() && status.pid.is_none() && status.version.is_none());
    assert!(status.error.as_deref().unwrap().contains("target changed"));
    assert!(!serde_json::to_string(&status).unwrap().contains("private"));
    assert_eq!(runtime.snapshot().unwrap().selection.url, Some(later.url()));
    assert_eq!(later_calls.load(Ordering::SeqCst), 0);
    assert!(transport::service_status(old, Instant::now() + Duration::from_secs(1)).is_err());
}

#[test]
fn monitor_enable_and_disable_serialize_with_target_save_without_requiring_disabled() {
    let fixture = Fixture::new();
    let runtime = fixture.runtime();
    let enabled = Arc::new(std::sync::atomic::AtomicBool::new(false));
    for next in [true, false] {
        let lock = transition().unwrap();
        let enabled_copy = enabled.clone();
        let runtime_copy = runtime.clone();
        let worker = std::thread::spawn(move || {
            runtime_copy.set_with_refresh(
                Some("http://localhost:1234".into()),
                None,
                || Ok(enabled_copy.load(Ordering::SeqCst)),
                |captured| {
                    assert_eq!(captured, next);
                    assert_eq!(enabled_copy.load(Ordering::SeqCst), next);
                },
            )
        });
        enabled.store(next, Ordering::SeqCst);
        drop(lock);
        worker.join().unwrap().unwrap();
        assert_eq!(enabled.load(Ordering::SeqCst), next);
    }
    assert!(runtime.path.is_file());
}
