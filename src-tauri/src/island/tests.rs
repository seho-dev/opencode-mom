use super::*;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;

struct FakeApi {
    address: SocketAddr,
    requests: Arc<AtomicUsize>,
    stopping: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl FakeApi {
    fn new(label: &'static str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(AtomicUsize::new(0));
        let count = requests.clone();
        let stopping = Arc::new(AtomicBool::new(false));
        let stop = stopping.clone();
        let thread = thread::spawn(move || {
            for stream in listener.incoming() {
                let mut stream = stream.unwrap();
                if stop.load(Ordering::SeqCst) {
                    break;
                }
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut bytes = Vec::new();
                while !bytes.ends_with(b"\r\n\r\n") {
                    let mut byte = [0];
                    stream.read_exact(&mut byte).unwrap();
                    bytes.push(byte[0]);
                    assert!(bytes.len() <= 8192);
                }
                let request = String::from_utf8(bytes).unwrap();
                assert!(request
                    .to_ascii_lowercase()
                    .contains("authorization: basic b3blbmnvzgu6chjpdmf0zq=="));
                count.fetch_add(1, Ordering::SeqCst);
                let target = request.split_whitespace().nth(1).unwrap();
                let body = if target == "/api/info" {
                    serde_json::json!({"version": "2.0.22", "pid": 42})
                } else if target.starts_with("/api/session/ses_a/message") {
                    serde_json::json!({"data": [{"id": "msg_a", "type": "user", "time": {"created": 1}, "text": label}]})
                } else {
                    serde_json::json!({"data": []})
                }.to_string();
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 Fixture\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",
                    body.len()
                );
            }
        });
        Self {
            address,
            requests,
            stopping,
            thread: Some(thread),
        }
    }

    fn api(&self) -> IslandApi {
        api::test_api(&format!("http://{}", self.address))
    }

    fn count(&self) -> usize {
        self.requests.load(Ordering::SeqCst)
    }
}

impl Drop for FakeApi {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.address);
        self.thread.take().unwrap().join().unwrap();
    }
}

fn transition_window(pointer: usize) -> transition::WindowId {
    transition::WindowId {
        pointer,
        number: pointer as isize,
    }
}

#[test]
fn native_transition_keeps_one_active_and_only_latest_pending() {
    let mut slot = transition::Slot::default();
    let window = transition_window(1);
    let (active, start, _) = slot.enqueue(7, window, IslandView::Expanded);
    assert!(start);
    let (pending, start, displaced) = slot.enqueue(7, window, IslandView::Peek);
    assert!(!start);
    assert!(displaced.is_none());
    assert!(pending.serial > active.serial);
    let (latest, start, displaced) = slot.enqueue(7, window, IslandView::Compact);
    assert!(!start);
    assert!(latest.serial > pending.serial);
    assert_eq!(displaced.unwrap().token, pending);
    assert_eq!(slot.active.as_ref().unwrap().token, active);
    assert!(slot.next().is_none());
    assert!(!slot.current(active, true, 7, window));
    assert_eq!(slot.release(active).unwrap().token, active);
    assert_eq!(slot.next(), Some(latest));
    assert!(slot.current(latest, true, 7, window));
}

#[test]
fn authoritative_collapse_excludes_old_focus_rollback_and_success() {
    let mut cache = Cache {
        enabled: true,
        ..Default::default()
    };
    let mut slot = transition::Slot::default();
    let window = transition_window(1);
    let (grow, _, _) = slot.enqueue(cache.generation, window, IslandView::Expanded);
    assert_eq!(cache.view, IslandView::Compact);
    cache.selected_session = Some("ses_a".into());
    let (collapse, _, _) = slot.enqueue(cache.generation, window, IslandView::Compact);
    cache.collapse();
    let mut effects = Vec::new();
    if slot.current(grow, cache.enabled, cache.generation, window) {
        effects.extend(["success", "focus", "rollback"]);
    }
    assert!(effects.is_empty());
    assert_eq!(cache.view, IslandView::Compact);
    assert!(cache.selected_session.is_none());
    slot.release(grow).unwrap();
    assert_eq!(slot.next(), Some(collapse));
}

#[test]
fn native_invalidation_resolves_receivers_and_old_completion_cannot_clear_replacement() {
    let mut slot = transition::Slot::default();
    let old_window = transition_window(1);
    let new_window = transition_window(2);
    assert!(!slot.current(
        transition::Token {
            generation: 7,
            serial: 0,
            window: old_window
        },
        false,
        7,
        old_window,
    ));
    let (sender, mut receiver) = tokio::sync::oneshot::channel::<Result<(), AppError>>();
    let (old, _, _) = slot.enqueue(7, old_window, sender);
    let (sender, mut pending) = tokio::sync::oneshot::channel::<Result<(), AppError>>();
    slot.enqueue(7, old_window, sender);
    let (active, queued) = slot.invalidate(true);
    for entry in active.into_iter().chain(queued) {
        let _ = entry.value.send(Err(disabled()));
    }
    assert!(receiver.try_recv().unwrap().is_err());
    assert!(pending.try_recv().unwrap().is_err());
    let (sender, _) = tokio::sync::oneshot::channel();
    let (new, _, _) = slot.enqueue(8, new_window, sender);
    assert!(!slot.current(old, true, 8, old_window));
    assert!(!slot.current(old, true, 7, new_window));
    assert!(!slot.current(new, false, 8, new_window));
    assert!(!slot.current(new, true, 8, old_window));
    assert!(slot.release(old).is_none());
    assert_eq!(slot.active.as_ref().unwrap().token, new);
    assert!(slot.current(new, true, 8, new_window));
}

#[test]
fn reconnect_invalidates_results_but_waits_for_old_animator_before_reconciliation() {
    let mut slot = transition::Slot::default();
    let window = transition_window(1);
    let (old, _, _) = slot.enqueue(7, window, IslandView::Expanded);
    let (_, pending) = slot.invalidate(false);
    assert!(pending.is_none());
    assert!(!slot.current(old, true, 8, window));
    let (new, start, _) = slot.enqueue(8, window, IslandView::Peek);
    assert!(!start);
    assert!(slot.dirty);
    slot.release(old).unwrap();
    assert_eq!(slot.next(), Some(new));
    assert!(slot.current(new, true, 8, window));
}

#[test]
fn midpoint_and_current_readback_failure_never_succeed() {
    let target = geometry::Rect {
        x: 655.5,
        y: 942.0,
        width: 400.0,
        height: 170.0,
    };
    let midpoint = geometry::Rect {
        x: 675.0,
        y: 1008.0,
        width: 360.0,
        height: 104.0,
    };
    let mut slot = transition::Slot::default();
    let window = transition_window(1);
    let (token, _, _) = slot.enqueue(7, window, IslandView::Peek);
    assert!(!slot.serial_matches(transition::Token {
        serial: token.serial + 1,
        ..token
    }));
    assert!(slot.current(token, true, 7, window));
    assert!(transition::verified_frame(midpoint, target).is_err());
    assert!(slot.active.is_some());
    let actual = geometry::Rect {
        height: target.height + 1.0,
        ..target
    };
    let result = transition::verified_frame(actual, target);
    assert!(result.is_err());
    assert!(transition::verified_frame(target, target).is_ok());
    assert!(slot.release(token).is_some());
}

#[test]
fn notifications_and_moved_do_not_start_an_animator_and_display_change_is_one_dirty_bit() {
    let mut slot = transition::Slot::default();
    let window = transition_window(1);
    let (token, _, _) = slot.enqueue(7, window, IslandView::Peek);
    for _ in 0..100 {
        assert!(!slot.geometry_event(false));
        assert_eq!(slot.active.as_ref().unwrap().token, token);
        assert!(slot.pending.is_none());
        assert!(!slot.dirty);
    }
    assert!(!slot.geometry_event(true));
    assert!(!slot.geometry_event(true));
    assert!(std::mem::take(&mut slot.dirty));
    assert!(!std::mem::take(&mut slot.dirty));
    slot.release(token).unwrap();
    assert!(slot.geometry_event(false));
}

#[test]
fn native_motion_is_grow_shrink_or_instant_for_reduce_motion_unchanged_and_placement() {
    let compact = geometry::Rect {
        x: 695.0,
        y: 1074.0,
        width: 321.0,
        height: 38.0,
    };
    let peek = geometry::Rect {
        x: 655.5,
        y: 942.0,
        width: 400.0,
        height: 170.0,
    };
    assert_eq!(transition::duration(compact, peek, false), Some(0.22));
    assert_eq!(transition::duration(peek, compact, false), Some(0.19));
    assert_eq!(transition::duration(compact, peek, true), None);
    assert_eq!(transition::duration(peek, compact, true), None);
    assert_eq!(transition::duration(peek, peek, false), None);
    assert_eq!(
        transition::duration(
            geometry::Rect {
                x: peek.x - 0.5,
                ..peek
            },
            peek,
            false
        ),
        None
    );
    assert_eq!(
        transition::duration(
            geometry::Rect {
                y: peek.y + 100.0,
                ..peek
            },
            peek,
            false
        ),
        None
    );
}

fn permission_snapshot() -> IslandSnapshot {
    IslandSnapshot {
        permissions: vec![api::IslandPermission {
            id: "per_pending".into(),
            session_id: "ses_a".into(),
            action: "read".into(),
            resources: vec!["/private/project".into()],
            save: Vec::new(),
            message: None,
        }],
        ..Default::default()
    }
}

#[test]
fn compact_is_default_and_pending_connection_updates_never_change_view() {
    let mut cache = Cache::default();
    assert_eq!(cache.view, IslandView::Compact);
    cache.enabled = true;
    let generation = cache.generation;
    assert!(cache.publish(generation, 1, permission_snapshot()));
    assert_eq!(cache.view, IslandView::Compact);
    assert!(cache.fail(generation));
    assert_eq!(cache.view, IslandView::Compact);
    let generation = cache.reconnect().unwrap();
    assert_eq!(cache.view, IslandView::Compact);
    assert!(cache.publish(generation, 2, permission_snapshot()));
    assert_eq!(cache.view, IslandView::Compact);
    for (revision, view) in [(3, IslandView::Peek), (4, IslandView::Expanded)] {
        cache.set_view(generation, view).unwrap();
        assert!(cache.publish(generation, revision, permission_snapshot()));
        assert_eq!(cache.view, view);
    }
}

#[test]
fn explicit_views_clear_hidden_selection_but_never_reply_to_pending() {
    let mut cache = Cache {
        enabled: true,
        connection: Some("connected"),
        snapshot: permission_snapshot(),
        ..Default::default()
    };
    let generation = cache.generation;
    assert_eq!(
        cache.set_view(generation, IslandView::Peek).unwrap().0,
        IslandView::Compact
    );
    assert_eq!(cache.view, IslandView::Peek);
    assert!(cache.selected_session.is_none());
    assert!(!cache.visible_message("ses_a"));
    assert!(cache.publish(generation, 1, permission_snapshot()));
    assert_eq!(cache.view, IslandView::Peek);
    for view in [IslandView::Peek, IslandView::Compact] {
        cache.set_view(generation, IslandView::Expanded).unwrap();
        cache.selected_session = Some("ses_a".into());
        assert!(cache.visible_message("ses_a"));
        cache.set_view(generation, view).unwrap();
        assert_eq!(cache.view, view);
        assert!(cache.selected_session.is_none());
        assert!(!cache.visible_message("ses_a"));
        assert_eq!(cache.snapshot.permissions.len(), 1);
        assert_eq!(cache.snapshot.permissions[0].id, "per_pending");
    }
}

#[test]
fn blur_collapse_and_disable_keep_generation_and_privacy_guards() {
    let (cancel, receiver) = watch::channel(false);
    let mut cache = Cache {
        enabled: true,
        connection: Some("connected"),
        snapshot: permission_snapshot(),
        cancel: Some(cancel),
        ..Default::default()
    };
    let generation = cache.generation;
    for view in [IslandView::Peek, IslandView::Expanded] {
        cache.set_view(generation, view).unwrap();
        cache.set_view(generation, IslandView::Compact).unwrap();
        assert_eq!(cache.view, IslandView::Compact);
        assert_eq!(cache.snapshot.permissions.len(), 1);
        assert!(!*receiver.borrow());
    }
    cache.set_view(generation, IslandView::Expanded).unwrap();
    cache.selected_session = Some("ses_a".into());
    cache.stop();
    assert_eq!(cache.view, IslandView::Compact);
    assert!(cache.selected_session.is_none());
    assert!(cache.snapshot.permissions.is_empty());
    assert!(*receiver.borrow());
    assert!(cache.set_view(generation, IslandView::Peek).is_err());
    cache.enabled = true;
    assert!(cache.set_view(generation, IslandView::Expanded).is_err());
    assert_eq!(cache.view, IslandView::Compact);
    assert!(cache.set_view(cache.generation, IslandView::Peek).is_ok());
}

#[test]
fn pending_resolution_publishes_before_notify_without_changing_explicit_view() {
    let now = tokio::time::Instant::now();
    let mut cache = Cache {
        enabled: true,
        generation: 7,
        connection: Some("connected"),
        ..Default::default()
    };
    let mut pending = Coalescer::default();
    pending.record(IslandChange::Snapshot, &cache, now);
    pending.record(
        IslandChange::Snapshot,
        &cache,
        now + Duration::from_millis(240),
    );
    assert_eq!(pending.deadline, Some(now + Duration::from_millis(250)));
    assert_eq!(cache.view, IslandView::Compact);
    assert!(cache.publish(7, 1, permission_snapshot()));
    assert!(pending.notify(&cache));
    assert_eq!(cache.view, IslandView::Compact);
    cache.set_view(7, IslandView::Peek).unwrap();

    let mut resolved = Coalescer::default();
    resolved.record(IslandChange::Snapshot, &cache, now);
    assert!(cache.publish(7, 3, IslandSnapshot::default()));
    assert!(resolved.notify(&cache));
    assert_eq!(cache.view, IslandView::Peek);
    assert!(!cache.publish(7, 2, permission_snapshot()));
    assert_eq!(cache.view, IslandView::Peek);

    assert!(cache.publish(7, 4, permission_snapshot()));
    assert!(cache.fail(7));
    assert_eq!(cache.view, IslandView::Peek);
    let generation = cache.reconnect().unwrap();
    assert!(!cache.publish(7, 5, permission_snapshot()));
    assert_eq!(cache.view, IslandView::Peek);
    assert!(cache.publish(generation, 6, IslandSnapshot::default()));
    assert_eq!(cache.view, IslandView::Peek);
    assert!(cache.publish(generation, 7, permission_snapshot()));
    assert_eq!(cache.view, IslandView::Peek);
    cache.stop();
    cache.enabled = true;
    cache.connection = Some("connecting");
    assert!(!cache.publish(generation, 8, permission_snapshot()));
    assert_eq!(cache.view, IslandView::Compact);
}

#[test]
fn view_ipc_accepts_only_the_three_explicit_lowercase_values() {
    for (value, view) in [
        ("compact", IslandView::Compact),
        ("peek", IslandView::Peek),
        ("expanded", IslandView::Expanded),
    ] {
        assert_eq!(
            serde_json::from_value::<IslandView>(serde_json::json!(value)).unwrap(),
            view
        );
    }
    for invalid in [
        serde_json::json!("attention"),
        serde_json::json!("Compact"),
        serde_json::json!(""),
        serde_json::json!(true),
        serde_json::json!(null),
    ] {
        assert!(serde_json::from_value::<IslandView>(invalid).is_err());
    }
}

#[test]
fn disable_clears_private_snapshot_and_rejects_stale_generation() {
    let (cancel, receiver) = watch::channel(false);
    let mut cache = Cache {
        enabled: true,
        view: IslandView::Expanded,
        selected_session: Some("ses_private".into()),
        generation: 7,
        connection: Some("connected"),
        cancel: Some(cancel),
        snapshot: IslandSnapshot {
            sessions: vec![api::IslandSession {
                id: "ses_private".into(),
                title: "private".into(),
                directory: "/private/project".into(),
                updated_at: 1,
                parent_id: None,
                status: "running".into(),
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    cache.stop();
    assert!(*receiver.borrow());
    assert!(!cache.enabled);
    assert_eq!(cache.view, IslandView::Compact);
    assert!(cache.selected_session.is_none());
    assert!(cache.snapshot.sessions.is_empty());
    assert!(cache.snapshot.permissions.is_empty());
    assert!(cache.snapshot.questions.is_empty());
    assert!(cache.api.is_none());
    assert!(!cache.publish(7, 1, IslandSnapshot::default()));
    cache.enabled = true;
    assert!(!cache.publish(7, 2, IslandSnapshot::default()));
    assert!(cache.publish(8, 3, IslandSnapshot::default()));
    assert!(!cache.publish(8, 2, IslandSnapshot::default()));
}

#[test]
fn window_failure_is_disabled_retryable_and_generation_guarded() {
    let mut cache = Cache {
        enabled: true,
        connection: Some("connecting"),
        ..Default::default()
    };
    let generation = cache.generation;
    assert!(cache.window_failed(generation));
    assert!(!cache.enabled);
    assert!(cache.cancel.is_none());
    assert!(cache.api.is_none());
    assert_eq!(cache.connection, Some("error"));
    assert!(cache.reconnect().is_err());
    cache.enabled = true;
    cache.connection = Some("connecting");
    assert!(!cache.window_failed(generation));
    assert!(cache.enabled);
    let retry_generation = cache.reconnect().unwrap();
    assert_eq!(cache.connection, Some("connecting"));
    assert!(cache.error.is_none());
    assert!(!cache.publish(generation, 1, IslandSnapshot::default()));
    assert!(cache.publish(retry_generation, 2, IslandSnapshot::default()));
}

#[test]
fn coalescer_ignores_hidden_messages_and_keeps_first_deadline() {
    let now = tokio::time::Instant::now();
    let mut cache = Cache {
        enabled: true,
        ..Default::default()
    };
    let mut pending = Coalescer::default();
    let messages = |id: &str| IslandChange::Messages {
        session_id: id.into(),
    };
    pending.record(messages("ses_a"), &cache, now);
    assert!(pending.deadline.is_none());
    cache
        .set_view(cache.generation, IslandView::Expanded)
        .unwrap();
    cache.selected_session = Some("ses_a".into());
    pending.record(messages("ses_b"), &cache, now);
    assert!(pending.deadline.is_none());
    pending.record(messages("ses_a"), &cache, now);
    let deadline = pending.deadline;
    pending.record(messages("ses_a"), &cache, now + Duration::from_millis(240));
    assert_eq!(pending.deadline, deadline);
    assert_eq!(deadline, Some(now + Duration::from_millis(250)));
    assert!(!pending.snapshot);
    assert!(pending.notify(&cache));
    cache.collapse();
    assert!(!pending.notify(&cache));
    pending.record(
        IslandChange::Snapshot,
        &cache,
        now + Duration::from_millis(249),
    );
    assert!(pending.snapshot);
    assert!(pending.notify(&cache));
    assert_eq!(pending.deadline, deadline);
}

#[test]
fn message_invalidations_do_not_fetch_snapshots_or_change_bounds() {
    let mut cache = Cache {
        enabled: true,
        view: IslandView::Expanded,
        selected_session: Some("ses_a".into()),
        connection: Some("connected"),
        snapshot: permission_snapshot(),
        ..Default::default()
    };
    let screen = geometry::Screen {
        frame: geometry::Rect {
            x: -1512.0,
            y: 0.0,
            width: 1512.0,
            height: 982.0,
        },
        visible: geometry::Rect::default(),
        top_inset: 32.0,
        notch_left: -846.0,
        notch_right: -666.0,
    };
    let frame = geometry::placement(screen, cache.view);
    let mut pending = Coalescer::default();
    pending.record(
        IslandChange::Messages {
            session_id: "ses_a".into(),
        },
        &cache,
        tokio::time::Instant::now(),
    );
    assert!(pending.notify(&cache));
    assert!(!pending.snapshot);
    assert_eq!(geometry::placement(screen, cache.view), frame);
    cache.collapse();
    assert_eq!(cache.view, IslandView::Compact);
    assert!(!pending.notify(&cache));
    let mut hidden = Coalescer::default();
    hidden.record(
        IslandChange::Messages {
            session_id: "ses_a".into(),
        },
        &cache,
        tokio::time::Instant::now(),
    );
    assert!(hidden.deadline.is_none());
    assert!(!hidden.snapshot);
}

#[test]
fn state_preferences_are_cached_without_config_reads() {
    let service = IslandService::default();
    let preferences = AppPreferences {
        theme: models::ThemePreference::Light,
        locale: models::LocalePreference::Zh,
    };
    service.update_preferences(preferences);
    assert_eq!(service.state().preferences, preferences);
    service.shutdown();
    assert_eq!(service.state().preferences, preferences);
    assert!(!service.state().enabled);
}

#[test]
fn disabled_message_reads_never_start_a_task() {
    tauri::async_runtime::block_on(async {
        let service = IslandService::default();
        assert!(service.messages("ses_a".into(), 0).await.is_err());
        assert!(service.context(None).is_err());
        assert!(lock(&service.task).is_none());
        assert!(lock(&service.cache).cancel.is_none());
        assert!(lock(&service.cache).selected_session.is_none());
    });
}

#[test]
fn old_connection_failure_cannot_overwrite_disabled_or_reenabled_state() {
    let service = IslandService::default();
    assert!(lock(&service.task).is_none());
    assert!(lock(&service.cache).cancel.is_none());
    assert!(service.context(None).is_err());
    let mut cache = lock(&service.cache);
    cache.enabled = true;
    let old_generation = cache.generation;
    cache.stop();
    assert!(!cache.fail(old_generation));
    assert_eq!(cache.connection, None);
    assert!(cache.error.is_none());
    cache.enabled = true;
    cache.connection = Some("connecting");
    assert!(!cache.fail(old_generation));
    assert_eq!(cache.connection, Some("connecting"));
    assert!(cache.error.is_none());
    let current_generation = cache.generation;
    assert!(cache.fail(current_generation));
    assert_eq!(cache.connection, Some("error"));
}

#[test]
fn shutdown_aborts_owned_task_without_touching_sessions() {
    tauri::async_runtime::block_on(async {
        let service = IslandService::default();
        let (started, ready) = tokio::sync::oneshot::channel();
        let (dropped, done) = tokio::sync::oneshot::channel();
        struct OnDrop(Option<tokio::sync::oneshot::Sender<()>>);
        impl Drop for OnDrop {
            fn drop(&mut self) {
                let _ = self.0.take().unwrap().send(());
            }
        }
        *lock(&service.task) = Some(tauri::async_runtime::spawn(async move {
            let _drop = OnDrop(Some(dropped));
            started.send(()).unwrap();
            std::future::pending::<()>().await;
        }));
        ready.await.unwrap();
        service.shutdown();
        tokio::time::timeout(Duration::from_secs(1), done)
            .await
            .unwrap()
            .unwrap();
        assert!(lock(&service.task).is_none());
        assert!(service.context(None).is_err());
    });
}

#[test]
fn public_state_is_camel_case_and_disabled_by_default() {
    let state = IslandState {
        supported: cfg!(target_os = "macos"),
        enabled: false,
        connection: "disabled",
        connection_generation: 0,
        error: None,
        sessions: Vec::new(),
        permissions: Vec::new(),
        questions: Vec::new(),
        preferences: AppPreferences::default(),
        notch_width: 0.0,
        top_inset: 0.0,
    };
    let value = serde_json::to_value(state).unwrap();
    assert_eq!(value["connection"], "disabled");
    assert_eq!(value["enabled"], false);
    assert_eq!(value["connectionGeneration"], 0);
    assert_eq!(value["notchWidth"], 0.0);
    assert_eq!(value["topInset"], 0.0);
    assert!(value.get("error").is_none());
    assert!(value.get("notch_width").is_none());
}

#[test]
fn disable_cancels_in_flight_future_and_drops_its_private_data() {
    tauri::async_runtime::block_on(async {
        let (sender, mut receiver) = watch::channel(false);
        let (started, ready) = tokio::sync::oneshot::channel();
        let (dropped, done) = tokio::sync::oneshot::channel();
        struct OnDrop(Option<tokio::sync::oneshot::Sender<()>>);
        impl Drop for OnDrop {
            fn drop(&mut self) {
                let _ = self.0.take().unwrap().send(());
            }
        }
        let task = tauri::async_runtime::spawn(async move {
            cancellable(&mut receiver, async move {
                let _private = OnDrop(Some(dropped));
                started.send(()).unwrap();
                std::future::pending::<()>().await;
            })
            .await
        });
        ready.await.unwrap();
        sender.send(true).unwrap();
        assert!(tokio::time::timeout(Duration::from_secs(1), task)
            .await
            .unwrap()
            .unwrap()
            .is_err());
        done.await.unwrap();
    });
}

#[test]
fn api_failure_wakes_and_drops_fresh_read_before_post_without_stopping_monitor() {
    tauri::async_runtime::block_on(async {
        let (monitor, monitor_receiver) = watch::channel(false);
        let cache = Arc::new(Mutex::new(Cache {
            enabled: true,
            cancel: Some(monitor),
            ..Default::default()
        }));
        let (generation, api_generation, mut cancel) = {
            let mut cache = lock(&cache);
            cache.start_actions();
            (
                cache.generation,
                cache.api_generation,
                cache.action_cancel.as_ref().unwrap().subscribe(),
            )
        };
        let (started, ready) = tokio::sync::oneshot::channel();
        let (get_response, get) = tokio::sync::oneshot::channel::<()>();
        let (post, posted) = tokio::sync::oneshot::channel();
        let (dropped, done) = tokio::sync::oneshot::channel();
        struct OnDrop(Option<tokio::sync::oneshot::Sender<()>>);
        impl Drop for OnDrop {
            fn drop(&mut self) {
                let _ = self.0.take().unwrap().send(());
            }
        }
        let task = tauri::async_runtime::spawn(async move {
            cancellable(&mut cancel, async move {
                let _private = OnDrop(Some(dropped));
                started.send(()).unwrap();
                get.await.unwrap();
                post.send(()).unwrap();
            })
            .await
        });
        ready.await.unwrap();
        {
            let mut cache = lock(&cache);
            assert!(cache.fail(generation));
            assert_ne!(cache.api_generation, api_generation);
            assert!(cache.api.is_none());
            assert!(cache.action_cancel.is_none());
        }
        assert!(tokio::time::timeout(Duration::from_secs(1), task)
            .await
            .unwrap()
            .unwrap()
            .is_err());
        done.await.unwrap();
        assert!(get_response.send(()).is_err());
        assert!(posted.await.is_err());
        assert!(!*monitor_receiver.borrow());
        assert!(!monitor_receiver.has_changed().unwrap());
    });
}

#[test]
fn failed_api_cancels_queued_mutation_before_it_can_start() {
    tauri::async_runtime::block_on(async {
        let service = IslandService::default();
        let (generation, mut cancel) = {
            let mut cache = lock(&service.cache);
            cache.enabled = true;
            cache.start_actions();
            (
                cache.generation,
                cache.action_cancel.as_ref().unwrap().subscribe(),
            )
        };
        let _held = service.mutations.lock().await;
        let (started, start) = tokio::sync::oneshot::channel();
        let mut queued = std::pin::pin!(async {
            let _mutation = cancellable(&mut cancel, service.mutations.lock()).await?;
            started.send(()).unwrap();
            Ok::<_, AppError>(())
        });
        std::future::poll_fn(|context| {
            assert!(queued.as_mut().poll(context).is_pending());
            std::task::Poll::Ready(())
        })
        .await;
        assert!(lock(&service.cache).fail(generation));
        assert!(tokio::time::timeout(Duration::from_secs(1), queued)
            .await
            .unwrap()
            .is_err());
        assert!(start.await.is_err());
    });
}

#[test]
fn api_replacement_cancels_old_ready_read_but_not_monitor_or_new_actions() {
    tauri::async_runtime::block_on(async {
        let (monitor, mut monitor_receiver) = watch::channel(false);
        let mut cache = Cache {
            enabled: true,
            cancel: Some(monitor),
            ..Default::default()
        };
        cache.start_actions();
        let generation = cache.generation;
        let api_generation = cache.api_generation;
        let mut old_cancel = cache.action_cancel.as_ref().unwrap().subscribe();
        let (response, get) = tokio::sync::oneshot::channel();
        let (post, posted) = tokio::sync::oneshot::channel();
        let mut old_action = std::pin::pin!(cancellable(&mut old_cancel, async move {
            get.await.unwrap();
            post.send(()).unwrap();
        }));
        std::future::poll_fn(|context| {
            assert!(old_action.as_mut().poll(context).is_pending());
            std::task::Poll::Ready(())
        })
        .await;
        response.send(()).unwrap();
        assert!(cache.publish(generation, 2, IslandSnapshot::default()));
        cache.start_actions();
        assert_eq!(cache.generation, generation);
        assert_ne!(cache.api_generation, api_generation);
        assert!(old_action.await.is_err());
        assert!(posted.await.is_err());
        let mut new_cancel = cache.action_cancel.as_ref().unwrap().subscribe();
        assert!(!cache.publish(generation, 1, permission_snapshot()));
        assert!(cache.snapshot.permissions.is_empty());
        assert_eq!(
            cancellable(&mut new_cancel, async { 42 }).await.unwrap(),
            42
        );
        assert!(cancellable(&mut monitor_receiver, async {}).await.is_ok());
        assert!(!monitor_receiver.has_changed().unwrap());
    });
}

#[test]
fn reconnect_and_disable_cancel_both_lifetimes_and_reject_stale_publication() {
    let (monitor, monitor_receiver) = watch::channel(false);
    let mut cache = Cache {
        enabled: true,
        cancel: Some(monitor),
        ..Default::default()
    };
    cache.start_actions();
    let old_generation = cache.generation;
    let old_api_generation = cache.api_generation;
    let old_action = cache.action_cancel.as_ref().unwrap().subscribe();
    let generation = cache.reconnect().unwrap();
    assert!(*monitor_receiver.borrow());
    assert!(*old_action.borrow());
    assert!(cache.cancel.is_none());
    assert!(cache.action_cancel.is_none());
    assert!(cache.api.is_none());
    assert_ne!(cache.api_generation, old_api_generation);
    assert!(!cache.publish(old_generation, 1, permission_snapshot()));
    let (monitor, monitor_receiver) = watch::channel(false);
    cache.cancel = Some(monitor);
    assert!(cache.publish(generation, 2, permission_snapshot()));
    cache.start_actions();
    let action = cache.action_cancel.as_ref().unwrap().subscribe();
    assert!(!cache.fail(old_generation));
    assert!(!*monitor_receiver.borrow());
    assert!(!*action.borrow());
    assert_eq!(cache.snapshot.permissions.len(), 1);
    cache.stop();
    assert!(*monitor_receiver.borrow());
    assert!(*action.borrow());
    assert!(cache.cancel.is_none());
    assert!(cache.action_cancel.is_none());
    assert!(cache.api.is_none());
    assert!(cache.snapshot.permissions.is_empty());
    assert!(!cache.publish(generation, 3, permission_snapshot()));
    assert!(cache.reconnect().is_err());
}

#[test]
fn target_refresh_guard_drop_leaves_connection_and_private_state_unchanged() {
    let service = IslandService::default();
    let (monitor, receiver) = watch::channel(false);
    let action = {
        let mut cache = lock(&service.cache);
        cache.enabled = true;
        cache.generation = 7;
        cache.view = IslandView::Expanded;
        cache.selected_session = Some("ses_a".into());
        cache.connection = Some("connected");
        cache.snapshot = permission_snapshot();
        cache.cancel = Some(monitor);
        cache.start_actions();
        cache.action_cancel.as_ref().unwrap().subscribe()
    };
    let before = serde_json::to_value(service.state()).unwrap();
    let api_generation = lock(&service.cache).api_generation;
    let guard = service.prepare_target_refresh();
    assert!(service.cache.try_lock().is_err());
    drop(guard);
    assert_eq!(serde_json::to_value(service.state()).unwrap(), before);
    let cache = lock(&service.cache);
    assert_eq!(cache.selected_session.as_deref(), Some("ses_a"));
    assert_eq!(cache.api_generation, api_generation);
    assert!(!*receiver.borrow());
    assert!(!*action.borrow());
    assert!(lock(&service.task).is_none());
}

#[test]
fn disabled_target_refresh_is_a_noop_without_a_monitor_or_preference_change() {
    let service = IslandService::default();
    service.update_preferences(AppPreferences {
        theme: models::ThemePreference::Light,
        locale: models::LocalePreference::Zh,
    });
    let before = serde_json::to_value(service.state()).unwrap();
    assert_eq!(service.prepare_target_refresh().invalidate(), None);
    assert_eq!(serde_json::to_value(service.state()).unwrap(), before);
    assert!(lock(&service.task).is_none());
    assert!(lock(&service.cache).cancel.is_none());
    assert!(lock(&service.cache).action_cancel.is_none());
}

#[test]
fn target_refresh_preserves_enabled_view_preferences_and_geometry_but_clears_private_data() {
    let service = IslandService::default();
    for view in [IslandView::Compact, IslandView::Peek, IslandView::Expanded] {
        let (monitor, receiver) = watch::channel(false);
        let (old_generation, api_generation, action, preferences) = {
            let mut cache = lock(&service.cache);
            cache.enabled = true;
            cache.view = view;
            cache.selected_session = Some("ses_a".into());
            cache.connection = Some("connected");
            cache.error = Some("old error".into());
            cache.snapshot = permission_snapshot();
            cache.snapshot.sessions.push(api::IslandSession {
                id: "ses_a".into(),
                title: "private".into(),
                directory: "/private/project".into(),
                updated_at: 1,
                parent_id: None,
                status: "running".into(),
            });
            cache.snapshot.questions.push(api::IslandQuestion {
                id: "form_a".into(),
                session_id: "ses_a".into(),
                title: "private".into(),
                answerable: false,
                fields: Vec::new(),
            });
            cache.cancel = Some(monitor);
            cache.notch_width = 209.0;
            cache.top_inset = 38.0;
            cache.preferences = AppPreferences {
                theme: models::ThemePreference::Light,
                locale: models::LocalePreference::Zh,
            };
            cache.start_actions();
            (
                cache.generation,
                cache.api_generation,
                cache.action_cancel.as_ref().unwrap().subscribe(),
                cache.preferences,
            )
        };
        let generation = service.prepare_target_refresh().invalidate().unwrap();
        let state = service.state();
        assert!(state.enabled);
        assert_eq!(state.connection, "connecting");
        assert_eq!(state.connection_generation, generation);
        assert_ne!(generation, old_generation);
        assert_eq!(state.preferences, preferences);
        assert_eq!((state.notch_width, state.top_inset), (209.0, 38.0));
        assert!(state.error.is_none());
        assert!(state.sessions.is_empty());
        assert!(state.permissions.is_empty());
        assert!(state.questions.is_empty());
        let mut cache = lock(&service.cache);
        assert_eq!(cache.view, view);
        assert!(cache.selected_session.is_none());
        assert!(cache.api.is_none());
        assert_ne!(cache.api_generation, api_generation);
        assert!(cache.cancel.is_none());
        assert!(cache.action_cancel.is_none());
        assert!(*receiver.borrow());
        assert!(*action.borrow());
        assert!(!cache.publish(old_generation, u64::MAX, permission_snapshot()));
        assert!(!cache.fail(old_generation));
        assert!(cache.publish(generation, 1, permission_snapshot()));
        assert!(!cache.visible_message("ses_a"));
        let mut pending = Coalescer::default();
        pending.record(
            IslandChange::Messages {
                session_id: "ses_a".into(),
            },
            &cache,
            tokio::time::Instant::now(),
        );
        assert!(pending.deadline.is_none());
        assert!(!pending.notify(&cache));
        assert!(lock(&service.task).is_none());
    }
}

#[test]
fn target_refresh_cancels_in_flight_reads_and_actions_even_with_reused_session_ids() {
    tauri::async_runtime::block_on(async {
        let service = IslandService::default();
        let (mut read_cancel, mut action_cancel) = {
            let mut cache = lock(&service.cache);
            cache.enabled = true;
            cache.view = IslandView::Expanded;
            cache.selected_session = Some("ses_a".into());
            cache.start_actions();
            let cancel = cache.action_cancel.as_ref().unwrap();
            (cancel.subscribe(), cancel.subscribe())
        };
        let (read_response, read) = tokio::sync::oneshot::channel::<&str>();
        let (action_response, action) = tokio::sync::oneshot::channel::<()>();
        let (post, posted) = tokio::sync::oneshot::channel();
        let mut reading = std::pin::pin!(cancellable(&mut read_cancel, read));
        let mut writing = std::pin::pin!(cancellable(&mut action_cancel, async move {
            action.await.unwrap();
            post.send("ses_a").unwrap();
        }));
        std::future::poll_fn(|context| {
            assert!(reading.as_mut().poll(context).is_pending());
            assert!(writing.as_mut().poll(context).is_pending());
            std::task::Poll::Ready(())
        })
        .await;
        let generation = service.prepare_target_refresh().invalidate().unwrap();
        read_response.send("ses_a").unwrap();
        action_response.send(()).unwrap();
        {
            let mut cache = lock(&service.cache);
            assert!(cache.publish(generation, 1, permission_snapshot()));
            cache.start_actions();
            assert!(cache.selected_session.is_none());
        }
        assert!(reading.await.is_err());
        assert!(writing.await.is_err());
        assert!(posted.await.is_err());
        let mut new_action = lock(&service.cache)
            .action_cancel
            .as_ref()
            .unwrap()
            .subscribe();
        assert_eq!(
            cancellable(&mut new_action, async { "ses_a" })
                .await
                .unwrap(),
            "ses_a"
        );
    });
}

#[test]
fn stale_target_finalization_cannot_replace_or_abort_the_latest_monitor() {
    tauri::async_runtime::block_on(async {
        let service = IslandService::default();
        lock(&service.cache).enabled = true;
        let old = service.prepare_target_refresh().invalidate().unwrap();
        let latest = service.prepare_target_refresh().invalidate().unwrap();
        let (started, ready) = tokio::sync::oneshot::channel();
        let (inspect, inspected) = tokio::sync::oneshot::channel();
        let (observed, observation) = tokio::sync::oneshot::channel();
        assert!(service.replace_monitor(latest, move |cancel| async move {
            started.send(()).unwrap();
            inspected.await.unwrap();
            observed.send(*cancel.borrow()).unwrap();
            std::future::pending::<()>().await;
        }));
        ready.await.unwrap();
        assert!(!service.replace_monitor(old, |_| async { panic!("stale monitor started") }));
        inspect.send(()).unwrap();
        assert!(!tokio::time::timeout(Duration::from_secs(1), observation)
            .await
            .unwrap()
            .unwrap());
        assert!(lock(&service.cache).current(latest));
        assert!(lock(&service.task).is_some());
        service.shutdown();
        assert!(!service.replace_monitor(latest, |_| async { panic!("disabled monitor started") }));
        assert!(lock(&service.task).is_none());
        assert!(!service.is_enabled());
    });
}

#[test]
fn monitor_install_rechecks_generation_after_spawn_without_holding_cache() {
    tauri::async_runtime::block_on(async {
        let service = IslandService::default();
        lock(&service.cache).enabled = true;
        let generation = service.prepare_target_refresh().invalidate().unwrap();
        assert!(!service.replace_monitor(generation, |receiver| {
            assert!(service.cache.try_lock().is_ok());
            assert_ne!(
                service.prepare_target_refresh().invalidate(),
                Some(generation)
            );
            assert!(*receiver.borrow());
            async {}
        }));
        assert!(lock(&service.task).is_none());
        assert!(lock(&service.cache).cancel.is_none());
    });
}

#[test]
fn stale_request_boundary_rejects_before_api_lookup_action_or_selection_change() {
    tauri::async_runtime::block_on(async {
        let service = IslandService::default();
        {
            let mut cache = lock(&service.cache);
            cache.enabled = true;
            cache.generation = 8;
            cache.view = IslandView::Expanded;
            cache.selected_session = Some("ses_current".into());
        }
        let called = std::sync::atomic::AtomicUsize::new(0);
        let result = service
            .request(
                7,
                |_| {
                    called.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    async { Ok(()) }
                },
                true,
            )
            .await;
        assert_eq!(result.unwrap_err(), disabled());
        assert_eq!(called.load(std::sync::atomic::Ordering::SeqCst), 0);
        assert_eq!(
            service.messages("ses_a".into(), 7).await.unwrap_err(),
            disabled()
        );
        assert_eq!(
            lock(&service.cache).selected_session.as_deref(),
            Some("ses_current")
        );
    });
}

#[test]
fn all_island_data_ipcs_require_an_unsigned_expected_connection_generation() {
    let server_a = FakeApi::new("A");
    let server_b = FakeApi::new("B");
    let api_a = server_a.api();
    let api_b = server_b.api();
    let service = IslandService::default();
    let snapshot = {
        let mut cache = lock(&service.cache);
        cache.enabled = true;
        cache.generation = 7;
        cache.view = IslandView::Expanded;
        cache.snapshot = permission_snapshot();
        cache.snapshot.sessions.push(api::IslandSession {
            id: "ses_a".into(),
            title: "shared".into(),
            directory: "/synthetic".into(),
            updated_at: 1,
            parent_id: None,
            status: "running".into(),
        });
        cache.snapshot.questions.push(api::IslandQuestion {
            id: "frm_a".into(),
            session_id: "ses_a".into(),
            title: "shared".into(),
            answerable: true,
            fields: Vec::new(),
        });
        cache.connection = Some("connected");
        cache.start_actions();
        cache.api = Some(api_a);
        cache.snapshot.clone()
    };
    assert_eq!(
        tauri::async_runtime::block_on(service.messages("ses_a".into(), 7)).unwrap()[0].text,
        "A"
    );
    assert_eq!(service.prepare_target_refresh().invalidate(), Some(8));
    {
        let mut cache = lock(&service.cache);
        assert!(cache.publish(8, 1, snapshot));
        cache.start_actions();
        cache.api = Some(api_b);
        cache.selected_session = Some("ses_current".into());
    }
    let initial_a = server_a.count();
    let initial_b = server_b.count();
    assert_eq!(initial_b, 1);
    let app = tauri::test::mock_builder()
        .manage(service)
        .invoke_handler(tauri::generate_handler![
            crate::commands::island::island_messages,
            crate::commands::island::island_send,
            crate::commands::island::island_reply_permission,
            crate::commands::island::island_reply_question,
            crate::commands::island::island_cancel_question,
            crate::commands::island::island_interrupt,
        ])
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    let payload = serde_json::json!({
        "sessionId": "ses_a", "text": "hello", "messageId": "msg_a",
        "requestId": "per_pending", "decision": "once", "formId": "frm_a", "answer": {}
    });
    for command in [
        "island_messages",
        "island_send",
        "island_reply_permission",
        "island_reply_question",
        "island_cancel_question",
        "island_interrupt",
    ] {
        for invalid in [
            None,
            Some(serde_json::json!(-1)),
            Some(serde_json::Value::Null),
        ] {
            let mut body = payload.clone();
            if let Some(invalid) = invalid {
                body["expectedConnectionGeneration"] = invalid;
            }
            let error = tauri::test::get_ipc_response(
                &webview,
                tauri::webview::InvokeRequest {
                    cmd: command.into(),
                    callback: tauri::ipc::CallbackFn(0),
                    error: tauri::ipc::CallbackFn(1),
                    url: "tauri://localhost".parse().unwrap(),
                    body: tauri::ipc::InvokeBody::Json(body),
                    headers: Default::default(),
                    invoke_key: tauri::test::INVOKE_KEY.into(),
                },
            )
            .unwrap_err();
            assert!(
                error
                    .as_str()
                    .unwrap()
                    .contains("expectedConnectionGeneration"),
                "{command}: {error}"
            );
        }
        let mut body = payload.clone();
        body["expectedConnectionGeneration"] = serde_json::json!(7);
        let error = tauri::test::get_ipc_response(
            &webview,
            tauri::webview::InvokeRequest {
                cmd: command.into(),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                url: "tauri://localhost".parse().unwrap(),
                body: tauri::ipc::InvokeBody::Json(body),
                headers: Default::default(),
                invoke_key: tauri::test::INVOKE_KEY.into(),
            },
        )
        .unwrap_err();
        assert_eq!(
            error,
            serde_json::to_value(disabled()).unwrap(),
            "{command}"
        );
        let service = app.state::<IslandService>();
        assert_eq!(
            lock(&service.cache).selected_session.as_deref(),
            Some("ses_current")
        );
        assert_eq!(server_a.count(), initial_a, "{command}");
        assert_eq!(server_b.count(), initial_b, "{command}");
    }
    let mut body = payload;
    body["expectedConnectionGeneration"] = serde_json::json!(8);
    let messages = tauri::test::get_ipc_response(
        &webview,
        tauri::webview::InvokeRequest {
            cmd: "island_messages".into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: "tauri://localhost".parse().unwrap(),
            body: tauri::ipc::InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.into(),
        },
    )
    .unwrap()
    .deserialize::<serde_json::Value>()
    .unwrap();
    assert_eq!(messages[0]["text"], "B");
    assert_eq!(server_b.count(), initial_b + 1);
    let service = app.state::<IslandService>();
    let called = AtomicUsize::new(0);
    let messages = tauri::async_runtime::block_on(service.request(
        8,
        |api| {
            called.fetch_add(1, Ordering::SeqCst);
            async move { api.messages("ses_a").await }
        },
        true,
    ))
    .unwrap();
    assert_eq!(messages[0].text, "B");
    assert_eq!(called.load(Ordering::SeqCst), 1);
    assert_eq!(server_a.count(), initial_a);
    assert_eq!(server_b.count(), initial_b + 2);
}

#[test]
fn actual_request_waiting_for_mutation_lock_cannot_run_after_target_refresh() {
    let server_a = FakeApi::new("A");
    let server_b = FakeApi::new("B");
    let api_a = server_a.api();
    let api_b = server_b.api();
    let service = IslandService::default();
    {
        let mut cache = lock(&service.cache);
        cache.enabled = true;
        cache.generation = 7;
        cache.connection = Some("connected");
        cache.start_actions();
        cache.api = Some(api_a);
    }
    let initial_a = server_a.count();
    let initial_b = server_b.count();
    tauri::async_runtime::block_on(async {
        let held = service.mutations.lock().await;
        let called = AtomicUsize::new(0);
        let mut queued = std::pin::pin!(service.request(
            7,
            |api| {
                called.fetch_add(1, Ordering::SeqCst);
                async move { api.interrupt("ses_a").await }
            },
            true
        ));
        std::future::poll_fn(|context| {
            assert!(queued.as_mut().poll(context).is_pending());
            std::task::Poll::Ready(())
        })
        .await;
        let generation = service.prepare_target_refresh().invalidate().unwrap();
        {
            let mut cache = lock(&service.cache);
            assert!(cache.publish(generation, 1, permission_snapshot()));
            cache.start_actions();
            cache.api = Some(api_b);
        }
        drop(held);
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), queued)
                .await
                .unwrap()
                .unwrap_err(),
            disabled()
        );
        assert_eq!(called.load(Ordering::SeqCst), 0);
        assert!(service.context(Some(generation)).is_ok());
    });
    assert_eq!(server_a.count(), initial_a);
    assert_eq!(server_b.count(), initial_b);
}
