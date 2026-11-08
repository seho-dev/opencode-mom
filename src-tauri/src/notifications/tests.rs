use super::*;

#[test]
fn categories_require_master_and_only_their_own_flag() {
    for mask in 0..16 {
        let settings = NotificationSettings {
            enabled: mask & 1 != 0,
            agent_completed: mask & 2 != 0,
            approval_required: mask & 4 != 0,
            agent_error: mask & 8 != 0,
        };
        for (category, selected) in [
            (Category::Completed, settings.agent_completed),
            (Category::ApprovalRequired, settings.approval_required),
            (Category::Error, settings.agent_error),
        ] {
            assert_eq!(category.enabled(settings), settings.enabled && selected);
        }
    }
}

#[test]
fn category_text_is_localized_and_contains_no_runtime_payload() {
    for category in [
        Category::Completed,
        Category::ApprovalRequired,
        Category::Error,
    ] {
        let english = category.text(LocalePreference::En);
        let chinese = category.text(LocalePreference::Zh);
        assert_ne!(english, chinese);
        for (title, body) in [english, chinese] {
            assert!(!title.is_empty() && !body.is_empty());
            assert!(!format!("{title}{body}").contains("ses_"));
        }
    }
}

#[test]
fn target_invalidation_and_shutdown_cancel_the_current_generation() {
    let service = NotificationService {
        state: Arc::new(Mutex::new(State {
            settings: NotificationSettings {
                enabled: true,
                ..Default::default()
            },
            ..Default::default()
        })),
        app: Weak::new(),
    };
    let cancel = lock(&service.state).cancel.clone();
    assert!(lock(&service.state).current(0));
    service.prepare_target_refresh().invalidate();
    assert!(cancel.is_cancelled());
    assert!(!lock(&service.state).current(0));
    assert!(lock(&service.state).current(1));
    service.shutdown();
    assert!(!lock(&service.state).current(2));
    assert!(service.settings().enabled);
}

#[test]
fn notification_settings_commit_without_replacing_unrelated_state() {
    let home = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(home.path());
    let mut config = models::AppConfig::default();
    config.state.preferences.locale = LocalePreference::Zh;
    config.state.island_enabled = true;
    config.state.logging_enabled = true;
    config.state.selected_group_name = Some("retained group".into());
    models::save_config(&paths.config_file(), &config).unwrap();
    let service = NotificationService {
        state: Arc::new(Mutex::new(State {
            stopped: true,
            ..Default::default()
        })),
        app: Weak::new(),
    };
    let settings = NotificationSettings {
        enabled: true,
        agent_completed: false,
        approval_required: true,
        agent_error: false,
    };
    assert_eq!(service.set_settings(&paths, settings).unwrap(), settings);
    assert_eq!(service.settings(), settings);
    config.state.notifications = settings;
    assert_eq!(models::load_config(&paths.config_file()).unwrap(), config);
    assert_eq!(lock(&service.state).locale, LocalePreference::Zh);
    std::fs::write(paths.config_file(), "invalid configuration").unwrap();
    assert!(service
        .set_settings(&paths, NotificationSettings::default())
        .is_err());
    assert_eq!(service.settings(), settings);
    assert_eq!(
        std::fs::read_to_string(paths.config_file()).unwrap(),
        "invalid configuration"
    );
}

#[test]
fn notification_handler_deduplicates_and_bounds_real_event_history() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            stream.read_exact(&mut byte).unwrap();
            request.push(byte[0]);
        }
        let body = r#"{"pid":42,"version":"2.0.22"}"#;
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
    });
    let api = api::test_api(&format!("http://{address}"));
    server.join().unwrap();
    tauri::async_runtime::block_on(async {
        let state = Arc::new(Mutex::new(State::default()));
        let cancel = CancellationToken::new();
        let mut seen = VecDeque::new();
        for index in 0..=MAX_EVENT_IDS {
            for _ in 0..2 {
                handle_event(
                    &Weak::new(),
                    &state,
                    0,
                    &cancel,
                    &api,
                    &mut seen,
                    IslandChange::Completed {
                        event_id: format!("evt_{index}"),
                        session_id: "ses_root".into(),
                    },
                )
                .await;
            }
        }
        assert_eq!(seen.len(), MAX_EVENT_IDS);
        assert_eq!(seen.front().unwrap(), "evt_1");
        assert_eq!(seen.back().unwrap(), &format!("evt_{MAX_EVENT_IDS}"));
        handle_event(
            &Weak::new(),
            &state,
            0,
            &cancel,
            &api,
            &mut seen,
            IslandChange::Snapshot,
        )
        .await;
        assert_eq!(seen.len(), MAX_EVENT_IDS);
    });
}
