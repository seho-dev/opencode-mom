use super::*;
use axum::http::StatusCode;
use serde_json::{json, Value};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

fn publish(path: &Path, value: &Value) {
    crate::private_file::publish(path, &serde_json::to_vec(value).unwrap()).unwrap();
}

fn publish_legacy(path: &Path, value: &Value) {
    #[cfg(not(windows))]
    publish(path, value);
    #[cfg(windows)]
    publish(
        path,
        &json!({
            "version": 1,
            "encryptedPayload": crate::protected_data::protect(&serde_json::to_vec(value).unwrap()).unwrap(),
        }),
    );
}

async fn start(http: &HttpServer) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let settings = MomHttpSettings {
        port: listener.local_addr().unwrap().port(),
        allow_lan: false,
    };
    drop(listener);
    http.set_settings(settings).await.unwrap();
    http.bound_addr().await.unwrap()
}

#[tokio::test]
async fn migration_adopts_once_preserves_network_and_never_retains_proxy_credentials() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("mom-http.json");
    let legacy_path = temp.path().join("mcp-proxy.json");
    let key = auth::generate_key();
    publish(&path, &json!({"port": 23456, "allowLan": true}));
    publish_legacy(
        &legacy_path,
        &json!({"gateway": {"tokenVerifier": key.verifier}}),
    );
    let http = HttpServer::new_with_path(path.clone(), Router::new()).unwrap();
    assert_eq!(
        http.settings().await,
        MomHttpSettings {
            port: 23456,
            allow_lan: true
        }
    );
    let disk: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(disk["auth"]["tokenVerifier"], key.verifier);
    assert!(!disk.to_string().contains(&key.raw_token));
    let proxy = crate::mcp_proxy::config::load_config(&legacy_path).unwrap();
    assert!(!format!("{proxy:?}").contains(&key.verifier));
    crate::mcp_proxy::config::save_config(&legacy_path, &proxy).unwrap();
    assert!(
        !String::from_utf8_lossy(&std::fs::read(&legacy_path).unwrap()).contains("tokenVerifier")
    );
    let restarted = HttpServer::new_with_path(path.clone(), Router::new()).unwrap();
    assert!(auth::verify_token(
        &key.raw_token,
        restarted
            .auth
            .read()
            .unwrap()
            .config
            .token_verifier
            .as_deref()
            .unwrap()
    ));
    let new_key = restarted.rotate_key(async { Ok(()) }).await.unwrap();
    publish_legacy(
        &legacy_path,
        &json!({"gateway": {"tokenVerifier": key.verifier}}),
    );
    let restarted = HttpServer::new_with_path(path.clone(), Router::new()).unwrap();
    {
        let auth = restarted.auth.read().unwrap();
        let verifier = auth.config.token_verifier.as_deref().unwrap();
        assert!(auth::verify_token(&new_key, verifier));
        assert!(!auth::verify_token(&key.raw_token, verifier));
    }
    publish(
        &path,
        &json!({"port": 23456, "allowLan": true, "auth": {"tokenVerifier": null}}),
    );
    let explicit_none = HttpServer::new_with_path(path.clone(), Router::new()).unwrap();
    assert!(!explicit_none.auth_ready());
    assert_eq!(explicit_none.settings().await.port, 23456);
    crate::private_file::publish(&legacy_path, b"invalid legacy data").unwrap();
    assert!(HttpServer::new_with_path(path, Router::new()).is_ok());
}

#[test]
fn migration_marks_absence_and_rejects_invalid_relevant_auth_without_rewriting() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("mom-http.json");
    let legacy_path = temp.path().join("mcp-proxy.json");
    let http = HttpServer::new_with_path(path.clone(), Router::new()).unwrap();
    assert!(!http.auth_ready());
    let disk: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(disk["auth"], json!({"tokenVerifier": null}));
    publish_legacy(
        &legacy_path,
        &json!({"gateway": {"tokenVerifier": auth::generate_key().verifier}}),
    );
    assert!(!HttpServer::new_with_path(path.clone(), Router::new())
        .unwrap()
        .auth_ready());

    for legacy in [
        json!([]),
        json!({"gateway": null}),
        json!({"gateway": 1}),
        json!({"gateway": {"tokenVerifier": "private-invalid"}}),
        json!({"gateway": {"tokenVerifier": 7}}),
    ] {
        publish(&path, &json!({"port": 23456, "allowLan": true}));
        let before = std::fs::read(&path).unwrap();
        publish_legacy(&legacy_path, &legacy);
        let error = HttpServer::new_with_path(path.clone(), Router::new())
            .err()
            .unwrap();
        assert!(!error.to_string().contains("private-invalid"));
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }

    for auth in [
        json!(null),
        json!(false),
        json!({"tokenVerifier": 7}),
        json!({"tokenVerifier": "private-invalid"}),
        json!({"rawToken": "private-invalid"}),
    ] {
        publish(
            &path,
            &json!({"port": 23456, "allowLan": true, "auth": auth}),
        );
        let before = std::fs::read(&path).unwrap();
        assert!(HttpServer::new_with_path(path.clone(), Router::new()).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
}

#[cfg(unix)]
#[test]
fn failed_migration_publication_preserves_original_settings_and_legacy_verifier() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("mom-http.json");
    let legacy_path = temp.path().join("mcp-proxy.json");
    publish(&path, &json!({"port": 23456, "allowLan": true}));
    publish_legacy(
        &legacy_path,
        &json!({"gateway": {"tokenVerifier": auth::generate_key().verifier}}),
    );
    let before = std::fs::read(&path).unwrap();
    std::fs::set_permissions(temp.path(), std::fs::Permissions::from_mode(0o500)).unwrap();
    let result = HttpServer::new_with_path(path.clone(), Router::new());
    std::fs::set_permissions(temp.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    assert!(result.is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert!(crate::mcp_proxy::config::legacy_http_verifier(&legacy_path)
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn every_http_method_is_authenticated_before_handlers_and_settings_cannot_replace_key() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("mom-http.json");
    let calls = Arc::new(AtomicUsize::new(0));
    let counted = calls.clone();
    let router = Router::new().route(
        "/health",
        axum::routing::any(move || {
            let calls = counted.clone();
            async move {
                calls.fetch_add(1, Ordering::SeqCst);
                "ok"
            }
        }),
    );
    let http = HttpServer::new_with_path(path.clone(), router).unwrap();
    let addr = start(&http).await;
    let client = reqwest::Client::new();
    let url = format!("http://{addr}/health");
    assert_eq!(
        client.get(&url).send().await.unwrap().status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
    let token = http.rotate_key(async { Ok(()) }).await.unwrap();
    for method in [
        reqwest::Method::GET,
        reqwest::Method::POST,
        reqwest::Method::DELETE,
        reqwest::Method::PUT,
        reqwest::Method::PATCH,
        reqwest::Method::HEAD,
        reqwest::Method::OPTIONS,
        reqwest::Method::TRACE,
    ] {
        for key in [None, Some("wrong")] {
            let mut request = client.request(method.clone(), &url);
            if let Some(key) = key {
                request = request.bearer_auth(key);
            }
            assert_eq!(
                request.send().await.unwrap().status(),
                StatusCode::UNAUTHORIZED
            );
        }
        assert_eq!(
            client
                .request(method, &url)
                .bearer_auth(&token)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
    }
    assert_eq!(calls.load(Ordering::SeqCst), 8);
    let verifier = http.auth.read().unwrap().config.token_verifier.clone();
    let next = start(&http).await;
    assert_eq!(
        client
            .get(format!("http://{next}/health"))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    assert_eq!(http.auth.read().unwrap().config.token_verifier, verifier);
    assert_eq!(
        serde_json::to_value(http.settings().await)
            .unwrap()
            .as_object()
            .unwrap()
            .len(),
        2
    );
    assert!(!String::from_utf8_lossy(&std::fs::read(&path).unwrap()).contains(&token));
    http.shutdown(Duration::from_secs(2)).await.unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn cancelled_rotation_commits_before_revocation_and_serializes_settings_and_next_rotation() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("mom-http.json");
    let mut http = HttpServer::new_with_path(path.clone(), Router::new()).unwrap();
    let old_key = http.rotate_key(async { Ok(()) }).await.unwrap();
    let addr = start(&http).await;
    let old_epoch = http.auth.read().unwrap().epoch.clone();
    let (entered_tx, mut entered_rx) = tokio::sync::mpsc::unbounded_channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let release_rx = std::sync::Mutex::new(release_rx);
    let worker = std::thread::current().id();
    http.publish_hook = Some(Arc::new(move || {
        assert_ne!(worker, std::thread::current().id());
        entered_tx.send(()).unwrap();
        release_rx.lock().unwrap().recv().unwrap();
    }));
    let publisher = http.clone();
    let (clean_tx, clean_rx) = tokio::sync::oneshot::channel();
    let (resume_tx, resume_rx) = tokio::sync::oneshot::channel();
    let committed_path = path.clone();
    let waiter = tokio::spawn(async move {
        publisher
            .rotate_key(async move {
                let disk: Value =
                    serde_json::from_slice(&std::fs::read(committed_path).unwrap()).unwrap();
                assert!(!auth::verify_token(
                    &old_key,
                    disk["auth"]["tokenVerifier"].as_str().unwrap()
                ));
                clean_tx.send(()).unwrap();
                resume_rx.await.unwrap();

                Ok(())
            })
            .await
    });
    entered_rx.recv().await.unwrap();
    assert!(!old_epoch.is_cancelled());
    waiter.abort();
    assert!(waiter.await.unwrap_err().is_cancelled());
    release_tx.send(()).unwrap();
    clean_rx.await.unwrap();
    assert!(old_epoch.is_cancelled());
    assert!(http.runtime.try_lock().is_err());
    let mut contender = http.clone();
    contender.publish_hook = None;
    let next_key_server = contender.clone();
    let next = tokio::spawn(async move { next_key_server.rotate_key(async { Ok(()) }).await });
    let settings = MomHttpSettings {
        port: addr.port(),
        allow_lan: false,
    };
    let setter = tokio::spawn(async move { contender.set_settings(settings).await });
    resume_tx.send(()).unwrap();
    let key = next.await.unwrap().unwrap();
    setter.await.unwrap().unwrap();
    let disk: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert!(auth::verify_token(
        &key,
        disk["auth"]["tokenVerifier"].as_str().unwrap()
    ));
    assert_eq!(http.settings().await, settings);
    assert!(!http.auth.read().unwrap().epoch.is_cancelled());
    http.shutdown(Duration::from_secs(2)).await.unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn rotation_cleanup_failure_preserves_committed_key_and_keeps_admission_closed() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("mom-http.json");
    let http = HttpServer::new_with_path(path.clone(), Router::new()).unwrap();
    let old_key = http.rotate_key(async { Ok(()) }).await.unwrap();
    start(&http).await;
    let old_epoch = http.auth.read().unwrap().epoch.clone();
    let key = http
        .rotate_key(async { Err(AppError::configuration("mock cleanup failure")) })
        .await
        .expect("post-commit cleanup failure must not discard the rotated key");
    let disk: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let verifier = disk["auth"]["tokenVerifier"].as_str().unwrap();
    assert!(auth::verify_token(&key, verifier));
    assert!(!auth::verify_token(&old_key, verifier));
    assert!(!disk.to_string().contains(&key));
    assert!(old_epoch.is_cancelled());
    assert!(http.auth.read().unwrap().epoch.is_cancelled());
    assert!(!http.auth_ready());
    assert!(http.start().await.is_err());
    http.shutdown(Duration::from_secs(2)).await.unwrap();
    assert!(HttpServer::new_with_path(path, Router::new())
        .unwrap()
        .auth_ready());
}

#[tokio::test(flavor = "current_thread")]
async fn shutdown_during_rotation_publication_cannot_reopen_auth_or_listener() {
    let temp = tempfile::tempdir().unwrap();
    let mut http =
        HttpServer::new_with_path(temp.path().join("mom-http.json"), Router::new()).unwrap();
    start(&http).await;
    let old_epoch = http.auth.read().unwrap().epoch.clone();
    let listener_token = http.runtime.lock().await.shutdown_token.clone();
    let (entered_tx, mut entered_rx) = tokio::sync::mpsc::unbounded_channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let release_rx = std::sync::Mutex::new(release_rx);
    http.publish_hook = Some(Arc::new(move || {
        entered_tx.send(()).unwrap();
        release_rx.lock().unwrap().recv().unwrap();
    }));
    let publisher = http.clone();
    let rotation = tokio::spawn(async move { publisher.rotate_key(async { Ok(()) }).await });
    entered_rx.recv().await.unwrap();
    let stopper = http.clone();
    let shutdown = tokio::spawn(async move { stopper.shutdown(Duration::from_secs(2)).await });
    tokio::task::yield_now().await;
    assert!(http.shutdown_token.is_cancelled());
    assert!(old_epoch.is_cancelled());
    assert!(listener_token.is_cancelled());
    assert!(!http.auth_ready());
    release_tx.send(()).unwrap();
    assert_eq!(rotation.await.unwrap().unwrap().len(), 64);
    shutdown.await.unwrap().unwrap();
    assert!(!http.auth_ready());
    assert!(http.auth.read().unwrap().epoch.is_cancelled());
    assert!(http.bound_addr().await.is_none());
    assert!(http.rotate_key(async { Ok(()) }).await.is_err());
    assert!(http.start().await.is_err());
}

#[tokio::test(flavor = "current_thread")]
async fn shutdown_during_rotation_cleanup_cannot_reopen_admission_without_listener() {
    let temp = tempfile::tempdir().unwrap();
    let http = HttpServer::new_with_path(temp.path().join("mom-http.json"), Router::new()).unwrap();
    let old_epoch = http.auth.read().unwrap().epoch.clone();
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = tokio::sync::oneshot::channel();
    let publisher = http.clone();
    let rotation = tokio::spawn(async move {
        publisher
            .rotate_key(async move {
                entered_tx.send(()).unwrap();
                release_rx.await.unwrap();

                Ok(())
            })
            .await
    });
    entered_rx.await.unwrap();
    assert!(old_epoch.is_cancelled());
    let stopper = http.clone();
    let shutdown = tokio::spawn(async move { stopper.shutdown(Duration::from_secs(2)).await });
    tokio::task::yield_now().await;
    assert!(http.shutdown_token.is_cancelled());
    release_tx.send(()).unwrap();
    let key = rotation.await.unwrap().unwrap();
    assert_eq!(key.len(), 64);
    assert!(http.auth.read().unwrap().epoch.is_cancelled());
    shutdown.await.unwrap().unwrap();
    assert!(!http.auth_ready());
    assert!(http.bound_addr().await.is_none());
    let disk: Value = serde_json::from_slice(&std::fs::read(&http.config_path).unwrap()).unwrap();
    assert!(auth::verify_token(
        &key,
        disk["auth"]["tokenVerifier"].as_str().unwrap()
    ));
}
