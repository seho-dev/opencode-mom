#![allow(deprecated)]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use rmcp::model::*;
use rmcp::transport::streamable_http_server::SessionManager;
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio::sync::mpsc;

use crate::http_server::{auth, HttpServer, MomHttpSettings};
use crate::mcp_proxy::config::{
    save_config, GatewaySettings, ProxyConfig, ProxyProtocol, ProxyServiceConfig, ServiceTimeouts,
};
use crate::mcp_proxy::process::OwnerBudget;
use crate::mcp_proxy::session::{LegacySessionManager, Observation};
use crate::mcp_proxy::ProxyService;

fn test_peer_binary() -> String {
    let mut path = std::env::current_exe().expect("current test exe");
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    path.push("examples");
    path.push(format!("mcp-test-peer{}", std::env::consts::EXE_SUFFIX));
    path.to_string_lossy().to_string()
}

const DEADLINE: Duration = Duration::from_secs(8);
const MODERN: &str = "2026-07-28";
const LEGACY: &str = "2025-11-25";

type SseReader = std::pin::Pin<
    Box<dyn futures::Stream<Item = Result<sse_stream::Sse, sse_stream::Error>> + Send>,
>;

struct TestServer {
    pub url: String,
    pub raw_token: String,
    pub budget: OwnerBudget,
    pub legacy_sessions: LegacySessionManager,
    pub events: mpsc::UnboundedReceiver<Observation>,
    http: HttpServer,
    proxy: Arc<ProxyService>,
    _temp: tempfile::TempDir,
}

impl TestServer {
    async fn start(max_units: usize) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("mcp-proxy.json");

        let port = unused_port().await;

        let peer_bin = test_peer_binary();

        let services = vec![
            ProxyServiceConfig {
                id: "legacy-svc".into(),
                name: "Legacy Service".into(),
                enabled: true,
                command: vec![peer_bin.clone(), "legacy".into(), "leg-tag".into()],
                cwd: temp.path().to_string_lossy().to_string(),
                environment: HashMap::new(),
                protocol: ProxyProtocol::Legacy,
                timeouts: ServiceTimeouts::default(),
            },
            ProxyServiceConfig {
                id: "modern-svc".into(),
                name: "Modern Service".into(),
                enabled: true,
                command: vec![peer_bin.clone(), "modern".into(), "mod-tag".into()],
                cwd: temp.path().to_string_lossy().to_string(),
                environment: HashMap::new(),
                protocol: ProxyProtocol::Modern,
                timeouts: ServiceTimeouts::default(),
            },
            ProxyServiceConfig {
                id: "disabled-svc".into(),
                name: "Disabled Service".into(),
                enabled: false,
                command: vec![peer_bin, "modern".into(), "dis-tag".into()],
                cwd: temp.path().to_string_lossy().to_string(),
                environment: HashMap::new(),
                protocol: ProxyProtocol::Modern,
                timeouts: ServiceTimeouts::default(),
            },
        ];

        let config = ProxyConfig {
            schema_version: 1,
            revision: 1,
            gateway: GatewaySettings {
                enabled: true,
                idle_timeout_secs: 300,
                max_units,
            },
            services,
        };

        save_config(&config_path, &config).unwrap();

        let (tx, rx) = mpsc::unbounded_channel();
        let mut proxy = ProxyService::new_with_path(config_path).unwrap();
        proxy.events = Some(tx);
        let proxy = Arc::new(proxy);
        let budget = proxy.budget().clone();
        let legacy_sessions = proxy.legacy_sessions().clone();
        let http =
            HttpServer::new_with_path(temp.path().join("mom-http.json"), proxy.router()).unwrap();
        let raw_token = http.rotate_key(async { Ok(()) }).await.unwrap();
        http.set_settings(MomHttpSettings {
            port,
            allow_lan: false,
        })
        .await
        .unwrap();

        Self {
            url: format!("http://127.0.0.1:{port}/mcp-proxy"),
            raw_token,
            budget,
            legacy_sessions,
            events: rx,
            http,
            proxy,
            _temp: temp,
        }
    }

    async fn event(&mut self) -> Observation {
        tokio::time::timeout(DEADLINE, self.events.recv())
            .await
            .expect("event timeout")
            .expect("event sender closed")
    }

    async fn legacy_session(&self) -> Arc<crate::mcp_proxy::session::LegacySession> {
        let response = http_client()
            .post(format!("{}?server=legacy-svc", self.url))
            .bearer_auth(&self.raw_token)
            .json(&initialize(LEGACY))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let id = response.headers()["Mcp-Session-Id"].to_str().unwrap();
        self.legacy_sessions.get_session(id).await.unwrap()
    }

    async fn close(self) {
        let (http, proxy) =
            tokio::join!(self.http.shutdown(DEADLINE), self.proxy.shutdown(DEADLINE));
        http.unwrap();
        proxy.unwrap();
    }
}

async fn unused_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

#[tokio::test]
async fn mom_http_settings_are_independent_persisted_and_failed_updates_restore_listener() {
    let temp = tempfile::tempdir().unwrap();
    let proxy = Arc::new(ProxyService::new_with_path(temp.path().join("mcp-proxy.json")).unwrap());
    let path = temp.path().join("mom-http.json");
    let router = proxy
        .router()
        .merge(axum::Router::new().route("/health", axum::routing::get(|| async { "ok" })));
    let http = HttpServer::new_with_path(path.clone(), router).unwrap();
    assert_eq!(http.settings().await, MomHttpSettings::default());
    let first = MomHttpSettings {
        port: unused_port().await,
        allow_lan: false,
    };
    http.set_settings(first).await.unwrap();
    let addr = http.bound_addr().await.unwrap();
    assert_eq!(
        http_client()
            .get(format!("http://{addr}/health"))
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(http.start().await.unwrap(), addr);
    let url = format!("http://{addr}/mcp-proxy?server=missing");
    assert_eq!(
        http_client().post(&url).send().await.unwrap().status(),
        reqwest::StatusCode::SERVICE_UNAVAILABLE
    );

    let mut config = proxy.config().await;
    config.gateway.enabled = true;
    proxy
        .save_gateway_settings(config.gateway, config.revision)
        .await
        .unwrap();
    assert_eq!(http.bound_addr().await, Some(addr));
    assert_eq!(
        http_client().post(&url).send().await.unwrap().status(),
        reqwest::StatusCode::SERVICE_UNAVAILABLE
    );
    let token = http.rotate_key(async { Ok(()) }).await.unwrap();
    assert_eq!(
        http_client()
            .get(format!("http://{addr}/health"))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap(),
        "ok"
    );
    assert_eq!(
        http_client().post(&url).send().await.unwrap().status(),
        reqwest::StatusCode::UNAUTHORIZED
    );

    let reserved = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let occupied = MomHttpSettings {
        port: reserved.local_addr().unwrap().port(),
        allow_lan: false,
    };
    assert!(http.set_settings(occupied).await.is_err());
    assert_eq!(http.settings().await, first);
    assert_eq!(http.bound_addr().await, Some(addr));
    assert!(http
        .set_settings(MomHttpSettings {
            port: 0,
            allow_lan: true
        })
        .await
        .is_err());
    assert_eq!(http.bound_addr().await, Some(addr));

    let next = MomHttpSettings {
        port: unused_port().await,
        allow_lan: true,
    };
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    assert!(http.set_settings(next).await.is_err());
    assert_eq!(http.settings().await, first);
    assert_eq!(http.bound_addr().await, Some(addr));
    std::fs::remove_dir(&path).unwrap();
    http.set_settings(next).await.unwrap();
    assert!(http.bound_addr().await.unwrap().ip().is_unspecified());
    let reloaded = HttpServer::new_with_path(path.clone(), proxy.router()).unwrap();
    assert_eq!(reloaded.settings().await, next);
    assert!(reloaded.auth_ready());
    assert!(
        !serde_json::to_value(proxy.config().await).unwrap()["gateway"]
            .as_object()
            .unwrap()
            .contains_key("port")
    );
    let (http_result, proxy_result) =
        tokio::join!(http.shutdown(DEADLINE), proxy.shutdown(DEADLINE));
    http_result.unwrap();
    proxy_result.unwrap();
    assert!(http.start().await.is_err());
    crate::private_file::publish(&path, b"invalid").unwrap();
    assert!(HttpServer::new_with_path(path, proxy.router()).is_err());
}

#[tokio::test]
async fn disabling_proxy_does_not_stop_shared_http_listener() {
    let server = TestServer::start(4).await;
    let client = http_client();
    let url = format!("{}?server=legacy-svc", server.url);
    let response = client
        .post(&url)
        .bearer_auth(&server.raw_token)
        .json(&initialize(LEGACY))
        .send()
        .await
        .unwrap();
    let id = response.headers()["Mcp-Session-Id"].to_str().unwrap();
    let stream = client
        .get(&url)
        .bearer_auth(&server.raw_token)
        .header("Mcp-Session-Id", id)
        .send()
        .await
        .unwrap();
    let bound = server.http.bound_addr().await;
    let mut config = server.proxy.config().await;
    config.gateway.enabled = false;
    server
        .proxy
        .save_gateway_settings(config.gateway, config.revision)
        .await
        .unwrap();
    assert_eq!(server.http.bound_addr().await, bound);
    assert_eq!(server.legacy_sessions.active_count().await, 0);
    assert_eq!(server.budget.available_permits(), 4);
    tokio::time::timeout(DEADLINE, stream.bytes())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        client
            .post(&url)
            .bearer_auth(&server.raw_token)
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::SERVICE_UNAVAILABLE
    );
    let mut config = server.proxy.config().await;
    config.gateway.enabled = true;
    server
        .proxy
        .save_gateway_settings(config.gateway, config.revision)
        .await
        .unwrap();
    assert_eq!(
        client.post(&url).send().await.unwrap().status(),
        reqwest::StatusCode::UNAUTHORIZED
    );
    server.close().await;
}

#[tokio::test]
async fn rotation_cancels_old_http_epoch_and_rejects_old_key() {
    let server = TestServer::start(2).await;
    let client = http_client();
    let url = format!("{}?server=legacy-svc", server.url);
    let initialized = client
        .post(&url)
        .bearer_auth(&server.raw_token)
        .json(&initialize(LEGACY))
        .send()
        .await
        .unwrap();
    assert_eq!(initialized.status(), reqwest::StatusCode::OK);
    let id = initialized.headers()["Mcp-Session-Id"].to_str().unwrap();
    let stream = client
        .get(&url)
        .bearer_auth(&server.raw_token)
        .header("Mcp-Session-Id", id)
        .send()
        .await
        .unwrap();
    let epoch = server.proxy.request_token().await;
    let bound = server.http.bound_addr().await;
    let config = server.proxy.config().await;
    let new_key =
        crate::commands::app::rotate_http_key(server.http.clone(), server.proxy.as_ref().clone())
            .await
            .unwrap();
    assert!(epoch.is_cancelled());
    assert!(!server.proxy.request_token().await.is_cancelled());
    assert_eq!(server.http.bound_addr().await, bound);
    assert_eq!(server.proxy.config().await, config);
    assert_eq!(server.legacy_sessions.active_count().await, 0);
    assert_eq!(server.budget.available_permits(), 2);
    tokio::time::timeout(DEADLINE, stream.bytes())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        client
            .get(&url)
            .bearer_auth(&server.raw_token)
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        client
            .get(&url)
            .bearer_auth(&new_key)
            .header("Mcp-Session-Id", id)
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::NOT_FOUND
    );
    server.close().await;
}

#[tokio::test]
async fn unchanged_http_settings_preserve_proxy_epoch_and_live_session() {
    let server = TestServer::start(1).await;
    let session = server.legacy_session().await;
    let settings = server.http.settings().await;
    let addr = server.http.bound_addr().await;
    let epoch = server.proxy.request_token().await;
    let generation = server.proxy.generation();
    crate::commands::app::update_http_settings(
        server.http.clone(),
        server.proxy.as_ref().clone(),
        settings,
    )
    .await
    .unwrap();
    assert!(!epoch.is_cancelled());
    assert!(!session.owner.is_closed());
    assert_eq!(server.proxy.generation(), generation);
    assert_eq!(server.http.bound_addr().await, addr);
    assert_eq!(server.legacy_sessions.active_count().await, 1);
    assert_eq!(server.budget.available_permits(), 0);
    server.close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn rotation_cancels_legacy_initialize_waiting_for_admission_without_leaking_session() {
    let server = TestServer::start(1).await;
    let admission = server.proxy.admission.write().await;
    let request = http_client()
        .post(format!("{}?server=legacy-svc", server.url))
        .bearer_auth(&server.raw_token)
        .json(&initialize(LEGACY))
        .timeout(DEADLINE);
    let initialize = tokio::spawn(async move { request.send().await });
    tokio::time::timeout(DEADLINE, async {
        while server.budget.available_permits() != 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("legacy initialize must reach the owned creation await");
    let rotation = tokio::spawn(crate::commands::app::rotate_http_key(
        server.http.clone(),
        server.proxy.as_ref().clone(),
    ));
    let response = tokio::time::timeout(DEADLINE, initialize)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE);
    drop(admission);
    let key = tokio::time::timeout(DEADLINE, rotation)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(key.len(), 64);
    assert_eq!(server.legacy_sessions.active_count().await, 0);
    assert_eq!(server.budget.available_permits(), 1);
    assert!(server.proxy.admission.try_write().is_ok());
    assert!(!server.proxy.cleanup_failed());
    server.close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn revocation_drains_queued_legacy_creation_before_publishing_request_epoch() {
    for shutdown in [false, true] {
        let server = TestServer::start(1).await;
        let service = server.proxy.config().await.services[0].clone();
        let epoch = server.proxy.request_token().await;
        let admission = server.proxy.admission.write().await;
        let mut creation = Box::pin(server.legacy_sessions.create_session(
            &service,
            server.budget.try_acquire().unwrap(),
            300,
            server.proxy.admission.clone(),
            tokio_util::sync::CancellationToken::new(),
            epoch.clone(),
        ));
        assert!(futures::poll!(&mut creation).is_pending());
        tokio::task::yield_now().await;
        let mut revocation = Box::pin(server.proxy.revoke_http_requests());
        assert!(futures::poll!(&mut revocation).is_pending());
        assert!(epoch.is_cancelled());
        let mut next_epoch = Box::pin(server.proxy.request_token());
        assert!(futures::poll!(&mut next_epoch).is_pending());
        if shutdown {
            let mut stop = Box::pin(server.proxy.shutdown(DEADLINE));
            assert!(futures::poll!(&mut stop).is_pending());
            drop(admission);
            let (created, revoked, stopped) = tokio::join!(creation, revocation, stop);
            assert!(created.is_err());
            revoked.unwrap();
            stopped.unwrap();
        } else {
            drop(admission);
            let (created, revoked) = tokio::join!(creation, revocation);
            assert!(created.is_err());
            revoked.unwrap();
        }
        assert_eq!(next_epoch.await.is_cancelled(), shutdown);
        assert_eq!(server.legacy_sessions.active_count().await, 0);
        assert_eq!(server.budget.available_permits(), 1);
        assert!(server.proxy.admission.try_write().is_ok());
        assert!(!server.proxy.cleanup_failed());
        server.close().await;
    }
}

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .pool_max_idle_per_host(0)
        .build()
        .unwrap()
}

fn metadata(client: &str) -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": MODERN,
        "io.modelcontextprotocol/clientCapabilities": {"roots": {"listChanged": true}},
        "io.modelcontextprotocol/clientInfo": {"name": client, "version": "0.0.0"},
        "progressToken": "same-token",
    })
}

fn call(id: u64, name: &str, meta: Option<Value>) -> Value {
    let mut value = json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": {
            "name": name,
            "arguments": {"test": true}
        }
    });
    if let Some(meta) = meta {
        value["params"]["_meta"] = meta;
    }
    value
}

fn initialize(version: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": version,
            "capabilities": {},
            "clientInfo": {"name": "test-client", "version": "0.0.0"}
        }
    })
}

async fn next_sse(response: &mut SseReader) -> Value {
    loop {
        let event = tokio::time::timeout(DEADLINE, response.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        if let Some(data) = event.data.filter(|data| !data.is_empty()) {
            let message: ServerJsonRpcMessage =
                serde_json::from_str(&data).expect("official rmcp server model");

            return serde_json::to_value(message).unwrap();
        }
    }
}

async fn until_result(response: &mut SseReader) -> Value {
    loop {
        let wire = next_sse(response).await;
        if wire.get("result").is_some() || wire.get("error").is_some() {
            return wire;
        }
    }
}

// -----------------------------------------------------------------------------
// Test 1: Inbound Pipeline Security Gates (Zero Process Spawns)
// -----------------------------------------------------------------------------

#[tokio::test]
async fn external_proxy_path_requires_auth_and_old_mcp_path_is_not_found() {
    let mut server = TestServer::start(4).await;
    let client = http_client();
    let addr = server.http.bound_addr().await.unwrap();

    for (path, expected) in [
        ("/mcp-proxy", reqwest::StatusCode::UNAUTHORIZED),
        (
            "/mcp-proxy?server=legacy-svc",
            reqwest::StatusCode::UNAUTHORIZED,
        ),
        ("/mcp", reqwest::StatusCode::NOT_FOUND),
        ("/mcp?server=legacy-svc", reqwest::StatusCode::NOT_FOUND),
    ] {
        for method in [
            reqwest::Method::POST,
            reqwest::Method::GET,
            reqwest::Method::DELETE,
        ] {
            let response = client
                .request(method.clone(), format!("http://{addr}{path}"))
                .send()
                .await
                .unwrap();
            assert_eq!(response.status(), expected, "{method} {path}");
        }
    }

    for path in ["/mcp", "/mcp?server=legacy-svc"] {
        for method in [
            reqwest::Method::GET,
            reqwest::Method::POST,
            reqwest::Method::DELETE,
        ] {
            let response = client
                .request(method.clone(), format!("http://{addr}{path}"))
                .bearer_auth(&server.raw_token)
                .send()
                .await
                .unwrap();
            assert_eq!(
                response.status(),
                reqwest::StatusCode::NOT_FOUND,
                "{method} {path}"
            );
        }
    }
    assert_eq!(server.budget.available_permits(), 4);
    assert!(
        tokio::time::timeout(Duration::from_millis(50), server.events.recv())
            .await
            .is_err()
    );

    server.close().await;
}

#[tokio::test]
async fn unauthenticated_request_rejected_401_zero_spawns() {
    let mut server = TestServer::start(4).await;
    let client = http_client();

    let res = client
        .post(format!("{}?server=modern-svc", server.url))
        .json(&initialize(LEGACY))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), reqwest::StatusCode::UNAUTHORIZED);
    assert_eq!(server.budget.available_permits(), 4);

    let no_event = tokio::time::timeout(Duration::from_millis(50), server.events.recv())
        .await
        .is_err();
    assert!(no_event);

    server.close().await;
}

#[tokio::test]
async fn wrong_key_rejected_401_zero_spawns() {
    let mut server = TestServer::start(4).await;
    let client = http_client();

    let res = client
        .post(format!("{}?server=modern-svc", server.url))
        .header("Authorization", "Bearer invalid-wrong-token")
        .json(&initialize(LEGACY))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), reqwest::StatusCode::UNAUTHORIZED);
    assert_eq!(server.budget.available_permits(), 4);

    let no_event = tokio::time::timeout(Duration::from_millis(50), server.events.recv())
        .await
        .is_err();
    assert!(no_event);

    server.close().await;
}

#[tokio::test]
async fn bad_origin_rejected_403_zero_spawns() {
    let mut server = TestServer::start(4).await;
    let client = http_client();

    for bad_origin in ["null", "https://malicious.com", "http://evil.org:18765"] {
        let res = client
            .post(format!("{}?server=modern-svc", server.url))
            .header("Authorization", format!("Bearer {}", server.raw_token))
            .header("Origin", bad_origin)
            .json(&initialize(LEGACY))
            .send()
            .await
            .unwrap();

        assert_eq!(res.status(), reqwest::StatusCode::FORBIDDEN);
        assert_eq!(server.budget.available_permits(), 4);
    }

    let no_event = tokio::time::timeout(Duration::from_millis(50), server.events.recv())
        .await
        .is_err();
    assert!(no_event);

    server.close().await;
}

#[tokio::test]
async fn bad_host_rejected_400_zero_spawns() {
    let mut server = TestServer::start(4).await;
    let client = http_client();

    for bad_host in ["evil.com", "*.attacker.org", "internal.corp"] {
        let res = client
            .post(format!("{}?server=modern-svc", server.url))
            .header("Authorization", format!("Bearer {}", server.raw_token))
            .header("Host", bad_host)
            .json(&initialize(LEGACY))
            .send()
            .await
            .unwrap();

        assert_eq!(res.status(), reqwest::StatusCode::BAD_REQUEST);
        assert_eq!(server.budget.available_permits(), 4);
    }

    let no_event = tokio::time::timeout(Duration::from_millis(50), server.events.recv())
        .await
        .is_err();
    assert!(no_event);

    server.close().await;
}

#[tokio::test]
async fn unknown_and_disabled_services_rejected_404_zero_spawns() {
    let mut server = TestServer::start(4).await;
    let client = http_client();

    for target in ["non-existent-svc", "disabled-svc"] {
        let res = client
            .post(format!("{}?server={}", server.url, target))
            .header("Authorization", format!("Bearer {}", server.raw_token))
            .json(&initialize(LEGACY))
            .send()
            .await
            .unwrap();

        assert_eq!(res.status(), reqwest::StatusCode::NOT_FOUND);
        assert_eq!(server.budget.available_permits(), 4);
    }

    let no_event = tokio::time::timeout(Duration::from_millis(50), server.events.recv())
        .await
        .is_err();
    assert!(no_event);

    server.close().await;
}

#[tokio::test]
async fn duplicate_or_missing_server_query_rejected_400_zero_spawns() {
    let mut server = TestServer::start(4).await;
    let client = http_client();

    let res_missing = client
        .post(&server.url)
        .header("Authorization", format!("Bearer {}", server.raw_token))
        .json(&initialize(LEGACY))
        .send()
        .await
        .unwrap();
    assert_eq!(res_missing.status(), reqwest::StatusCode::BAD_REQUEST);

    let res_dup = client
        .post(format!(
            "{}?server=legacy-svc&server=modern-svc",
            server.url
        ))
        .header("Authorization", format!("Bearer {}", server.raw_token))
        .json(&initialize(LEGACY))
        .send()
        .await
        .unwrap();
    assert_eq!(res_dup.status(), reqwest::StatusCode::BAD_REQUEST);

    assert_eq!(server.budget.available_permits(), 4);

    let no_event = tokio::time::timeout(Duration::from_millis(50), server.events.recv())
        .await
        .is_err();
    assert!(no_event);

    server.close().await;
}

// -----------------------------------------------------------------------------
// Test 2: Legacy Protocol Loop (Initialize, Sessions, ID Isolation, DELETE)
// -----------------------------------------------------------------------------

#[tokio::test]
async fn closing_legacy_process_id_cleans_session_bridge_sdk_and_budget() {
    let server = TestServer::start(2).await;
    let session = server.legacy_session().await;
    let unrelated = server.legacy_session().await;
    let (generation, processes) = server.proxy.process_snapshot().await;
    let process_id = session.owner.summary().0;
    assert_eq!(generation, server.proxy.generation());
    assert_eq!(processes.len(), 2);
    assert!(processes
        .iter()
        .any(|owner| owner.summary().0 == process_id));

    server
        .proxy
        .close_unit(&process_id, generation)
        .await
        .unwrap();

    assert!(session.owner.is_closed());
    assert!(session.task.lock().await.is_none());
    assert!(server
        .legacy_sessions
        .get_session(&session.id)
        .await
        .is_none());
    assert!(session
        .manager
        .create_standalone_stream(&session.id)
        .await
        .is_err());
    assert_eq!(server.legacy_sessions.active_count().await, 1);
    assert!(!unrelated.owner.is_closed());
    assert_eq!(server.budget.available_permits(), 1);
    let (next_generation, remaining) = server.proxy.process_snapshot().await;
    assert_eq!(next_generation, generation);
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].summary().0, unrelated.owner.summary().0);
    server.close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn queued_unit_close_rechecks_generation_before_selecting_target() {
    let server = TestServer::start(1).await;
    let session = server.legacy_session().await;
    let generation = server.proxy.generation();
    let process_id = session.owner.summary().0;
    let update = server.proxy.updates.lock().await;
    let mut closing = Box::pin(server.proxy.close_unit(&process_id, generation));
    assert!(futures::poll!(&mut closing).is_pending());
    tokio::task::yield_now().await;
    server
        .proxy
        .generation
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let mut snapshot = Box::pin(server.proxy.process_snapshot());
    assert!(futures::poll!(&mut snapshot).is_pending());
    drop(update);

    assert_eq!(
        closing.await.unwrap_err().code,
        crate::error::ErrorCode::ValidationFailed
    );
    let (next_generation, processes) = snapshot.await;
    assert_eq!(next_generation, generation + 1);
    assert_eq!(processes.len(), 1);
    assert_eq!(processes[0].summary().0, process_id);
    assert!(!session.owner.is_closed());
    assert_eq!(server.legacy_sessions.active_count().await, 1);
    assert_eq!(server.budget.available_permits(), 0);
    server.close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn cancelled_process_close_waiter_still_cleans_the_legacy_session() {
    let server = TestServer::start(1).await;
    let session = server.legacy_session().await;
    let process_id = session.owner.summary().0;
    let update = server.proxy.updates.lock().await;
    let mut closing = Box::pin(
        server
            .proxy
            .close_unit(&process_id, server.proxy.generation()),
    );
    assert!(futures::poll!(&mut closing).is_pending());
    drop(closing);
    drop(update);
    tokio::time::timeout(DEADLINE, async {
        while server.legacy_sessions.active_count().await != 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("unit teardown must outlive its caller");
    assert!(session.owner.is_closed());
    assert!(session.task.lock().await.is_none());
    assert!(session
        .manager
        .create_standalone_stream(&session.id)
        .await
        .is_err());
    assert_eq!(server.budget.available_permits(), 1);
    server.close().await;
}

#[tokio::test]
async fn auto_protocol_forwards_standard_handshake_without_draft_metadata() {
    let server = TestServer::start(4).await;
    let mut configuration = server.proxy.config().await;
    configuration
        .services
        .iter_mut()
        .find(|service| service.id == "legacy-svc")
        .unwrap()
        .protocol = ProxyProtocol::Auto;
    let revision = configuration.revision;
    server
        .proxy
        .save_config(configuration, revision)
        .await
        .unwrap();
    let client = http_client();
    let response = client
        .post(format!("{}?server=legacy-svc", server.url))
        .bearer_auth(&server.raw_token)
        .json(&initialize(LEGACY))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let session = response
        .headers()
        .get("Mcp-Session-Id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let response = client
        .post(format!("{}?server=legacy-svc", server.url))
        .bearer_auth(&server.raw_token)
        .header("Mcp-Session-Id", &session)
        .json(&call(7, "echo", None))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let mut stream: SseReader = Box::pin(sse_stream::SseStream::from_bytes_stream(
        response.bytes_stream(),
    ));
    let result = until_result(&mut stream).await;
    assert_eq!(result["result"]["structuredContent"]["tag"], "leg-tag");
    drop(stream);
    server.close().await;
}

#[tokio::test]
async fn legacy_lifecycle_initialize_sessions_no_crosstalk_and_delete_closes() {
    let server = TestServer::start(4).await;
    let client = http_client();

    // Session A
    let res_a = client
        .post(format!("{}?server=legacy-svc", server.url))
        .header("Authorization", format!("Bearer {}", server.raw_token))
        .json(&initialize(LEGACY))
        .send()
        .await
        .unwrap();

    assert_eq!(res_a.status(), reqwest::StatusCode::OK);
    let session_a_id = res_a
        .headers()
        .get("Mcp-Session-Id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    // Session B
    let res_b = client
        .post(format!("{}?server=legacy-svc", server.url))
        .header("Authorization", format!("Bearer {}", server.raw_token))
        .json(&initialize(LEGACY))
        .send()
        .await
        .unwrap();

    assert_eq!(res_b.status(), reqwest::StatusCode::OK);
    let session_b_id = res_b
        .headers()
        .get("Mcp-Session-Id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    assert_ne!(session_a_id, session_b_id);
    assert_eq!(server.budget.available_permits(), 2);

    // Call tool on session A with ID 7
    let stream_a_res = client
        .post(format!("{}?server=legacy-svc", server.url))
        .header("Authorization", format!("Bearer {}", server.raw_token))
        .header("Mcp-Session-Id", &session_a_id)
        .json(&call(7, "echo", None))
        .send()
        .await
        .unwrap();
    assert_eq!(stream_a_res.status(), reqwest::StatusCode::OK);

    let mut stream_a: SseReader = Box::pin(sse_stream::SseStream::from_bytes_stream(
        stream_a_res.bytes_stream(),
    ));
    let result_a = until_result(&mut stream_a).await;
    assert_eq!(result_a["id"], 7);
    assert_eq!(result_a["result"]["structuredContent"]["tag"], "leg-tag");

    // Call tool on session B with same ID 7 - verifying no crosstalk
    let stream_b_res = client
        .post(format!("{}?server=legacy-svc", server.url))
        .header("Authorization", format!("Bearer {}", server.raw_token))
        .header("Mcp-Session-Id", &session_b_id)
        .json(&call(7, "echo", None))
        .send()
        .await
        .unwrap();
    assert_eq!(stream_b_res.status(), reqwest::StatusCode::OK);

    let mut stream_b: SseReader = Box::pin(sse_stream::SseStream::from_bytes_stream(
        stream_b_res.bytes_stream(),
    ));
    let result_b = until_result(&mut stream_b).await;
    assert_eq!(result_b["id"], 7);
    assert_eq!(result_b["result"]["structuredContent"]["tag"], "leg-tag");

    // Dropping stream A does NOT close session A
    drop(stream_a);
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(server
        .legacy_sessions
        .get_session(&session_a_id)
        .await
        .is_some());

    // DELETE session A explicitly closes and releases permit
    let del_a = client
        .delete(format!("{}?server=legacy-svc", server.url))
        .header("Authorization", format!("Bearer {}", server.raw_token))
        .header("Mcp-Session-Id", &session_a_id)
        .send()
        .await
        .unwrap();
    assert_eq!(del_a.status(), reqwest::StatusCode::NO_CONTENT);

    assert_eq!(server.budget.available_permits(), 3);
    assert!(server
        .legacy_sessions
        .get_session(&session_a_id)
        .await
        .is_none());

    // Subsequent call to deleted session A returns 404
    let post_del = client
        .post(format!("{}?server=legacy-svc", server.url))
        .header("Authorization", format!("Bearer {}", server.raw_token))
        .header("Mcp-Session-Id", &session_a_id)
        .json(&call(8, "echo", None))
        .send()
        .await
        .unwrap();
    assert_eq!(post_del.status(), reqwest::StatusCode::NOT_FOUND);

    // Session B is still alive
    assert!(server
        .legacy_sessions
        .get_session(&session_b_id)
        .await
        .is_some());

    server.close().await;
}

// -----------------------------------------------------------------------------
// Test 3: Modern Protocol (Per-Request Process, Disconnect Cancel & Reap, MRTR)
// -----------------------------------------------------------------------------

#[tokio::test]
async fn modern_fresh_process_per_request_and_cancellation_reaps_child() {
    let mut server = TestServer::start(4).await;
    let client = http_client();

    let wire = call(7, "echo", Some(metadata("client-1")));
    let res = client
        .post(format!("{}?server=modern-svc", server.url))
        .header("Authorization", format!("Bearer {}", server.raw_token))
        .header("Accept", "application/json, text/event-stream")
        .header("MCP-Protocol-Version", MODERN)
        .header("Mcp-Method", "tools/call")
        .json(&wire)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), reqwest::StatusCode::OK);
    assert!(res.headers().get("Mcp-Session-Id").is_none());

    let mut reader: SseReader =
        Box::pin(sse_stream::SseStream::from_bytes_stream(res.bytes_stream()));
    let progress = next_sse(&mut reader).await;
    assert_eq!(progress["method"], "notifications/progress");

    let result = until_result(&mut reader).await;
    assert_eq!(result["id"], 7);
    assert_eq!(result["result"]["structuredContent"]["tag"], "mod-tag");

    let first_pid = match server.event().await {
        Observation::Started(pid) => pid,
        other => panic!("expected Started, got {other:?}"),
    };
    assert!(matches!(server.event().await, Observation::Closed(pid) if pid == first_pid));
    assert_eq!(server.budget.available_permits(), 4);

    // Now test client disconnect on a hanging tool ("wait")
    let wait_wire = call(8, "wait", Some(metadata("client-1")));
    let wait_res = client
        .post(format!("{}?server=modern-svc", server.url))
        .header("Authorization", format!("Bearer {}", server.raw_token))
        .header("Accept", "application/json, text/event-stream")
        .header("MCP-Protocol-Version", MODERN)
        .header("Mcp-Method", "tools/call")
        .json(&wait_wire)
        .send()
        .await
        .unwrap();

    let mut wait_reader: SseReader = Box::pin(sse_stream::SseStream::from_bytes_stream(
        wait_res.bytes_stream(),
    ));
    let _wait_prog = next_sse(&mut wait_reader).await;

    let wait_pid = match server.event().await {
        Observation::Started(pid) => pid,
        other => panic!("expected Started, got {other:?}"),
    };

    // Client drops the response stream!
    drop(wait_reader);

    // Server must cancel stdio child and reap it
    assert!(matches!(
        server.event().await,
        Observation::Cancelled(RequestId::Number(8))
    ));
    assert!(matches!(server.event().await, Observation::Closed(pid) if pid == wait_pid));
    assert_eq!(server.budget.available_permits(), 4);

    server.close().await;
}

#[tokio::test]
async fn modern_unread_stdin_cancellation_and_timeout_reap_owner_and_release_admission() {
    for action in [
        "disconnect",
        "rotate",
        "disable",
        "service-disable",
        "shutdown",
        "timeout",
    ] {
        let mut server = TestServer::start(1).await;
        let mut config = server.proxy.config().await;
        let revision = config.revision;
        let service = config
            .services
            .iter_mut()
            .find(|service| service.id == "modern-svc")
            .unwrap();
        service.command[1] = "unread".into();
        service.timeouts.execution_secs = if action == "timeout" { 1 } else { 30 };
        server.proxy.save_config(config, revision).await.unwrap();

        let mut wire = call(17, "wait", Some(metadata("unread-client")));
        wire["params"]["arguments"] = json!({"payload": "x".repeat(4 * 1024 * 1024)});
        let response = http_client()
            .post(format!("{}?server=modern-svc", server.url))
            .bearer_auth(&server.raw_token)
            .header("Accept", "application/json, text/event-stream")
            .header("MCP-Protocol-Version", MODERN)
            .header("Mcp-Method", "tools/call")
            .json(&wire)
            .timeout(DEADLINE)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let pid = match server.event().await {
            Observation::Started(pid) => pid,
            other => panic!("expected Started, got {other:?}"),
        };
        assert_eq!(server.budget.available_permits(), 0);
        assert!(server.proxy.admission.try_write().is_err());

        match action {
            "disconnect" => drop(response),
            "rotate" => {
                let key = tokio::time::timeout(
                    Duration::from_secs(4),
                    crate::commands::app::rotate_http_key(
                        server.http.clone(),
                        server.proxy.as_ref().clone(),
                    ),
                )
                .await
                .expect("rotation must not wait for a blocked stdin write")
                .unwrap();
                assert_eq!(key.len(), 64);
                drop(response);
            }
            "shutdown" => {
                let (http, proxy) = tokio::join!(
                    server.http.shutdown(Duration::from_secs(4)),
                    server.proxy.shutdown(Duration::from_secs(4)),
                );
                http.unwrap();
                proxy.unwrap();
                drop(response);
            }
            "disable" | "service-disable" => {
                let mut config = server.proxy.config().await;
                let revision = config.revision;
                if action == "disable" {
                    config.gateway.enabled = false;
                } else {
                    config
                        .services
                        .iter_mut()
                        .find(|service| service.id == "modern-svc")
                        .unwrap()
                        .enabled = false;
                }
                tokio::time::timeout(
                    Duration::from_secs(4),
                    server.proxy.save_config(config, revision),
                )
                .await
                .expect("disable must not wait for a blocked stdin write")
                .unwrap();
                drop(response);
            }
            "timeout" => {
                let mut reader: SseReader = Box::pin(sse_stream::SseStream::from_bytes_stream(
                    response.bytes_stream(),
                ));
                let result = until_result(&mut reader).await;
                assert!(result["error"]["message"]
                    .as_str()
                    .unwrap()
                    .contains("timed out sending request"));
            }
            _ => unreachable!(),
        }
        assert!(matches!(server.event().await, Observation::Closed(closed) if closed == pid));
        assert_eq!(server.budget.available_permits(), 1, "{action}");
        assert!(server.proxy.admission.try_write().is_ok(), "{action}");
        assert!(!server.proxy.cleanup_failed(), "{action}");
        #[cfg(unix)]
        {
            // SAFETY: signal zero only checks the test child's PID after owner cleanup.
            assert_eq!(unsafe { libc::kill(pid as libc::pid_t, 0) }, -1);
            assert_eq!(
                std::io::Error::last_os_error().raw_os_error(),
                Some(libc::ESRCH)
            );
        }
        server.close().await;
    }
}

#[tokio::test(flavor = "current_thread")]
async fn committed_disable_releases_config_lock_and_bounds_stalled_cleanup() {
    let server = TestServer::start(1).await;
    let session = server.legacy_session().await;
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = tokio::sync::oneshot::channel();
    let stalled = tokio::spawn(async move {
        entered_tx.send(()).unwrap();
        release_rx.await.unwrap();
        Ok(())
    });
    entered_rx.await.unwrap();
    let previous = session.task.lock().await.replace(stalled).unwrap();
    previous.abort();
    let _ = previous.await;

    let mut config = server.proxy.config().await;
    let revision = config.revision;
    let epoch = server.proxy.request_token().await;
    config.gateway.enabled = false;
    let proxy = server.proxy.clone();
    let mut save = tokio::spawn(async move { proxy.save_config(config, revision).await });
    tokio::time::timeout(Duration::from_secs(2), epoch.cancelled())
        .await
        .unwrap();
    let committed = tokio::time::timeout(Duration::from_secs(1), server.proxy.config())
        .await
        .expect("committed config must be readable during stalled cleanup");
    assert!(!committed.gateway.enabled);
    assert_eq!(committed.revision, revision + 1);
    assert_eq!(
        crate::mcp_proxy::config::load_config(&server.proxy.config_path).unwrap(),
        committed
    );
    assert!(futures::poll!(&mut save).is_pending());
    let closing = session.clone();
    let close = tokio::spawn(async move { closing.close().await });
    assert_eq!(
        http_client()
            .post(format!("{}?server=legacy-svc", server.url))
            .bearer_auth(&server.raw_token)
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::SERVICE_UNAVAILABLE
    );
    tokio::time::timeout(super::CLEANUP_TIMEOUT + Duration::from_secs(2), save)
        .await
        .expect("post-commit cleanup must be bounded")
        .unwrap()
        .expect("cleanup failure must not report committed config as a failed save");
    assert!(server.proxy.cleanup_failed());
    assert!(tokio::time::timeout(Duration::from_secs(2), close)
        .await
        .expect("closing an active unit must also be bounded")
        .unwrap()
        .is_err());
    release_tx.send(()).unwrap();
    session.close().await.unwrap();
    server.http.shutdown(DEADLINE).await.unwrap();
    assert!(server.proxy.shutdown(DEADLINE).await.is_err());
}

#[tokio::test(flavor = "current_thread")]
async fn changed_service_cleanup_preserves_sessions_from_the_committed_revision() {
    let server = TestServer::start(2).await;
    let session = server.legacy_session().await;
    let token = server.proxy.service_token("legacy-svc").await;
    let (release_tx, release_rx) = tokio::sync::oneshot::channel();
    let stalled = tokio::spawn(async move {
        release_rx.await.unwrap();
        Ok(())
    });
    let previous = session.task.lock().await.replace(stalled).unwrap();
    previous.abort();
    let _ = previous.await;
    let mut config = server.proxy.config().await;
    let revision = config.revision;
    config.services[0].name = "Updated service".into();
    let proxy = server.proxy.clone();
    let save = tokio::spawn(async move { proxy.save_config(config, revision).await });
    tokio::time::timeout(DEADLINE, token.cancelled())
        .await
        .unwrap();
    assert_eq!(server.proxy.config().await.revision, revision + 1);
    let fresh = server.legacy_session().await;
    release_tx.send(()).unwrap();
    save.await.unwrap().unwrap();
    assert!(session.owner.is_closed());
    assert!(!fresh.owner.is_closed());
    assert_eq!(server.legacy_sessions.active_count().await, 1);
    assert!(!server.proxy.cleanup_failed());
    server.close().await;
}

#[tokio::test]
async fn modern_service_change_cancels_only_its_inflight_requests() {
    let mut server = TestServer::start(2).await;
    let legacy = server.legacy_session().await;
    let epoch = server.proxy.request_token().await;
    let response = http_client()
        .post(format!("{}?server=modern-svc", server.url))
        .bearer_auth(&server.raw_token)
        .header("MCP-Protocol-Version", MODERN)
        .json(&call(23, "wait", Some(metadata("service-change"))))
        .send()
        .await
        .unwrap();
    let mut reader: SseReader = Box::pin(sse_stream::SseStream::from_bytes_stream(
        response.bytes_stream(),
    ));
    next_sse(&mut reader).await;
    let pid = match server.event().await {
        Observation::Started(pid) => pid,
        other => panic!("expected Started, got {other:?}"),
    };
    let mut config = server.proxy.config().await;
    let revision = config.revision;
    config
        .services
        .iter_mut()
        .find(|service| service.id == "modern-svc")
        .unwrap()
        .enabled = false;
    server.proxy.save_config(config, revision).await.unwrap();
    assert!(matches!(
        server.event().await,
        Observation::Cancelled(RequestId::Number(23))
    ));
    assert!(matches!(server.event().await, Observation::Closed(closed) if closed == pid));
    assert!(!epoch.is_cancelled());
    assert!(!legacy.owner.is_closed());
    assert_eq!(server.legacy_sessions.active_count().await, 1);
    assert_eq!(server.budget.available_permits(), 1);
    server.close().await;
}

#[tokio::test]
async fn modern_mrtr_stateless_fresh_processes() {
    let mut server = TestServer::start(4).await;
    let client = http_client();

    // Round 1: invoke mrtr tool
    let wire1 = call(7, "mrtr", Some(metadata("client-1")));
    let res1 = client
        .post(format!("{}?server=modern-svc", server.url))
        .header("Authorization", format!("Bearer {}", server.raw_token))
        .header("Accept", "application/json, text/event-stream")
        .header("MCP-Protocol-Version", MODERN)
        .header("Mcp-Method", "tools/call")
        .json(&wire1)
        .send()
        .await
        .unwrap();

    let mut reader1: SseReader = Box::pin(sse_stream::SseStream::from_bytes_stream(
        res1.bytes_stream(),
    ));
    let input = until_result(&mut reader1).await;
    assert_eq!(input["result"]["resultType"], "input_required");
    let state_str = input["result"]["requestState"].as_str().unwrap();

    let pid1 = match server.event().await {
        Observation::Started(pid) => pid,
        other => panic!("expected Started, got {other:?}"),
    };
    assert!(matches!(server.event().await, Observation::Closed(pid) if pid == pid1));

    // Round 2: invoke mrtr with requestState and response
    let mut wire2 = call(7, "mrtr", Some(metadata("client-1")));
    wire2["params"]["requestState"] = json!(state_str);
    wire2["params"]["inputResponses"] = json!({"roots": {"roots": []}});

    let res2 = client
        .post(format!("{}?server=modern-svc", server.url))
        .header("Authorization", format!("Bearer {}", server.raw_token))
        .header("Accept", "application/json, text/event-stream")
        .header("MCP-Protocol-Version", MODERN)
        .header("Mcp-Method", "tools/call")
        .json(&wire2)
        .send()
        .await
        .unwrap();

    let mut reader2: SseReader = Box::pin(sse_stream::SseStream::from_bytes_stream(
        res2.bytes_stream(),
    ));
    let final_res = until_result(&mut reader2).await;
    assert_eq!(final_res["result"]["structuredContent"]["continued"], true);

    let pid2 = match server.event().await {
        Observation::Started(pid) => pid,
        other => panic!("expected Started, got {other:?}"),
    };
    assert_ne!(pid1, pid2);
    assert!(matches!(server.event().await, Observation::Closed(pid) if pid == pid2));

    server.close().await;
}

// -----------------------------------------------------------------------------
// Test 4: Capacity and Budget Limits
// -----------------------------------------------------------------------------

#[tokio::test]
async fn owner_budget_exhaustion_rejects_with_429() {
    let server = TestServer::start(1).await;
    let client = http_client();

    // Start a legacy session which holds the 1 available unit
    let res = client
        .post(format!("{}?server=legacy-svc", server.url))
        .header("Authorization", format!("Bearer {}", server.raw_token))
        .json(&initialize(LEGACY))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), reqwest::StatusCode::OK);
    assert_eq!(server.budget.available_permits(), 0);

    // Second request cannot spawn a process -> 429
    let res_blocked = client
        .post(format!("{}?server=modern-svc", server.url))
        .header("Authorization", format!("Bearer {}", server.raw_token))
        .header("Accept", "application/json, text/event-stream")
        .header("MCP-Protocol-Version", MODERN)
        .header("Mcp-Method", "tools/call")
        .json(&call(7, "echo", Some(metadata("client-1"))))
        .send()
        .await
        .unwrap();

    assert_eq!(res_blocked.status(), reqwest::StatusCode::TOO_MANY_REQUESTS);

    server.close().await;
}

// -----------------------------------------------------------------------------
// Test 5: Config Persistence, Validation & Unix 0600
// -----------------------------------------------------------------------------

#[test]
fn config_persistence_validation_and_permissions() {
    let temp = tempfile::tempdir().unwrap();
    let config_path = temp.path().join("mcp-proxy.json");

    let config = ProxyConfig {
        schema_version: 1,
        revision: 1,
        gateway: GatewaySettings {
            enabled: true,
            idle_timeout_secs: 180,
            max_units: 8,
        },
        services: vec![ProxyServiceConfig {
            id: "test-svc".into(),
            name: "Test Svc".into(),
            enabled: true,
            command: vec!["/bin/echo".into(), "hello".into()],
            cwd: temp.path().to_string_lossy().to_string(),
            environment: HashMap::new(),
            protocol: ProxyProtocol::Modern,
            timeouts: ServiceTimeouts::default(),
        }],
    };

    save_config(&config_path, &config).unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let meta = std::fs::metadata(&config_path).unwrap();
        assert_eq!(meta.mode() & 0o777, 0o600, "Unix config must be 0600 mode");
    }

    let loaded = crate::mcp_proxy::config::load_config(&config_path).unwrap();
    assert_eq!(loaded.schema_version, 1);
    assert_eq!(loaded.revision, 1);
    assert_eq!(loaded.gateway.max_units, 8);
    assert_eq!(loaded.services.len(), 1);
    assert_eq!(loaded.services[0].id, "test-svc");

    // Corrupt config fails closed
    std::fs::write(&config_path, b"corrupted { invalid json").unwrap();
    assert!(crate::mcp_proxy::config::load_config(&config_path).is_err());

    // Validation checks
    let mut bad_id = config.clone();
    bad_id.services.push(ProxyServiceConfig {
        id: "test-svc".into(), // Duplicate!
        name: "Dup".into(),
        enabled: true,
        command: vec!["/bin/echo".into()],
        cwd: temp.path().to_string_lossy().to_string(),
        environment: HashMap::new(),
        protocol: ProxyProtocol::Auto,
        timeouts: ServiceTimeouts::default(),
    });
    assert!(crate::mcp_proxy::config::validate_config(&bad_id).is_err());

    let mut rel_cwd = config;
    rel_cwd.services[0].cwd = "relative/path".into();
    assert!(crate::mcp_proxy::config::validate_config(&rel_cwd).is_err());
}

#[cfg(not(windows))]
#[test]
fn config_temporary_plaintext_is_wiped_on_parse_and_publication_failures() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("mcp-proxy.json");
    let wiped = || crate::protected_data::WIPED_BYTES.with(std::cell::Cell::get);
    let before = wiped();
    save_config(&path, &ProxyConfig::default()).unwrap();
    assert!(wiped() > before);
    crate::private_file::publish(&path, b"invalid private config").unwrap();
    let before = wiped();
    assert!(crate::mcp_proxy::config::load_config(&path).is_err());
    assert_eq!(wiped() - before, b"invalid private config".len());
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    let before = wiped();
    assert!(save_config(&path, &ProxyConfig::default()).is_err());
    assert!(wiped() > before);
}

// -----------------------------------------------------------------------------
// Test 6: Key Rotation and ProxyService Lifecycle
// -----------------------------------------------------------------------------

#[tokio::test]
async fn shared_http_key_rotation_persists_verifier_without_changing_proxy_revision() {
    let temp = tempfile::tempdir().unwrap();
    let config_path = temp.path().join("mcp-proxy.json");

    let config = ProxyConfig {
        schema_version: 1,
        revision: 1,
        gateway: GatewaySettings {
            enabled: true,
            idle_timeout_secs: 300,
            max_units: 4,
        },
        services: vec![],
    };
    save_config(&config_path, &config).unwrap();

    let service = ProxyService::new_with_path(config_path.clone()).unwrap();
    let http_path = temp.path().join("mom-http.json");
    let http = HttpServer::new_with_path(http_path.clone(), service.router()).unwrap();
    assert!(!http.auth_ready());
    assert_eq!(service.generation(), 1);

    let new_token = crate::commands::app::rotate_http_key(http.clone(), service.clone())
        .await
        .unwrap();
    assert_eq!(new_token.len(), 64);
    assert_eq!(service.generation(), 2);
    assert!(http.auth_ready());

    let updated_config = service.config().await;
    assert_eq!(updated_config, config);
    let persisted: Value = serde_json::from_slice(&std::fs::read(&http_path).unwrap()).unwrap();
    let verifier = persisted["auth"]["tokenVerifier"].as_str().unwrap();
    assert!(auth::verify_token(&new_token, verifier));
    assert!(!persisted.to_string().contains(&new_token));
    assert!(
        HttpServer::new_with_path(http_path.clone(), service.router())
            .unwrap()
            .auth_ready()
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        assert_eq!(std::fs::metadata(&http_path).unwrap().mode() & 0o777, 0o600);
    }

    let on_disk = crate::mcp_proxy::config::load_config(&config_path).unwrap();
    assert_eq!(on_disk, config);
    let proxy_wire = serde_json::to_value(on_disk).unwrap();
    assert!(!proxy_wire["gateway"]
        .as_object()
        .unwrap()
        .contains_key("tokenVerifier"));

    // Shutdown completes within bounded deadline
    service.shutdown(Duration::from_secs(2)).await.unwrap();
    http.shutdown(Duration::from_secs(2)).await.unwrap();
}

#[tokio::test]
async fn cancelled_shared_rotation_waiter_still_commits_and_revokes_sessions() {
    let server = TestServer::start(1).await;
    let session = server.legacy_session().await;
    let before = server.proxy.config().await;
    let generation = server.proxy.generation();
    let epoch = server.proxy.request_token().await;
    let path = server._temp.path().join("mom-http.json");
    let guard = server.proxy.config.write().await;
    let http = server.http.clone();
    let proxy = server.proxy.as_ref().clone();
    let waiter = tokio::spawn(crate::commands::app::rotate_http_key(http, proxy));
    tokio::time::timeout(DEADLINE, async {
        loop {
            let persisted: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
            if !auth::verify_token(
                &server.raw_token,
                persisted["auth"]["tokenVerifier"].as_str().unwrap(),
            ) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("shared HTTP rotation must persist before proxy revocation");
    waiter.abort();
    assert!(waiter.await.unwrap_err().is_cancelled());
    drop(guard);
    tokio::time::timeout(DEADLINE, async {
        while !epoch.is_cancelled() || !session.owner.is_closed() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("owned key rotation must outlive its waiter");
    assert_eq!(server.proxy.config().await, before);
    assert_eq!(server.proxy.generation(), generation + 1);
    assert_eq!(server.legacy_sessions.active_count().await, 0);
    assert_eq!(server.budget.available_permits(), 1);
    assert_eq!(
        http_client()
            .get(format!("{}?server=legacy-svc", server.url))
            .bearer_auth(&server.raw_token)
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::UNAUTHORIZED
    );
    server.close().await;
}

#[tokio::test]
async fn failed_rotation_preserves_config_and_revision() {
    let server = TestServer::start(1).await;
    let session = server.legacy_session().await;
    let before = server.proxy.config().await;
    let generation = server.proxy.generation();
    let epoch = server.proxy.request_token().await;
    let path = server._temp.path().join("mom-http.json");
    let persisted = std::fs::read(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();

    assert!(crate::commands::app::rotate_http_key(
        server.http.clone(),
        server.proxy.as_ref().clone(),
    )
    .await
    .is_err());
    assert_eq!(server.proxy.config().await, before);
    assert_eq!(server.proxy.generation(), generation);
    assert!(!epoch.is_cancelled());
    assert!(!session.owner.is_closed());
    assert_eq!(server.legacy_sessions.active_count().await, 1);
    assert_eq!(server.budget.available_permits(), 0);
    assert_eq!(
        http_client()
            .get(format!("{}?server=missing", server.url))
            .bearer_auth(&server.raw_token)
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::NOT_FOUND
    );
    std::fs::remove_dir(&path).unwrap();
    crate::private_file::publish(&path, &persisted).unwrap();
    server.close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn cancelled_publication_waiter_still_commits_disk_and_memory() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("mcp-proxy.json");
    let service = ProxyService::new_with_path(path.clone()).unwrap();
    let mut config = service.config().await;
    let revision = config.revision;
    config.gateway.max_units = 3;
    let guard = service.config.write().await;
    let publisher = service.clone();
    let waiter = tokio::spawn(async move { publisher.save_config(config, revision).await });
    tokio::task::yield_now().await;
    waiter.abort();
    assert!(waiter.await.unwrap_err().is_cancelled());
    drop(guard);

    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if service.config().await.revision == 2 {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("owned publication must outlive its waiter");
    assert_eq!(
        crate::mcp_proxy::config::load_config(&path).unwrap(),
        service.config().await
    );
    assert_eq!(service.generation(), 2);
}

#[tokio::test(flavor = "current_thread")]
async fn cancelled_legacy_close_waiter_still_reaps_after_bridge_join() {
    let server = TestServer::start(1).await;
    let session = server.legacy_session().await;
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let blocking = tokio::task::spawn_blocking(move || {
        let _ = entered_tx.send(());
        release_rx.recv().unwrap();

        Ok(())
    });
    entered_rx.await.unwrap();
    let previous = session.task.lock().await.replace(blocking).unwrap();
    // Keep the original bridge from closing the owner naturally.
    previous.abort();
    let _ = previous.await;
    let mut close = Box::pin(session.close());
    assert!(futures::poll!(&mut close).is_pending());
    tokio::task::yield_now().await;
    drop(close);
    release_tx.send(()).unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        while !session.owner.is_closed() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("session cleanup must outlive its waiter");
    assert_eq!(server.budget.available_permits(), 1);
    server.close().await;
}

#[tokio::test]
async fn legacy_bridge_join_failure_is_visible_after_owner_cleanup() {
    let server = TestServer::start(2).await;
    let session = server.legacy_session().await;
    let failed = tokio::spawn(async { panic!("fake bridge panic") });
    let previous = session.task.lock().await.replace(failed).unwrap();
    previous.abort();
    let _ = previous.await;
    tokio::task::yield_now().await;
    let outcome = session.close().await;
    assert!(
        format!("{outcome:?}").contains("panic"),
        "cleanup error was lost: {outcome:?}"
    );
    assert!(session.owner.is_closed());
    assert_eq!(server.budget.available_permits(), 2);
    let second = server.legacy_session().await;
    assert!(server.proxy.shutdown(DEADLINE).await.is_err());
    assert!(second.owner.is_closed());
    assert_eq!(server.budget.available_permits(), 2);
    assert_eq!(server.legacy_sessions.active_count().await, 0);
    server.http.shutdown(DEADLINE).await.unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn cancelled_bulk_cleanup_still_finishes_every_session() {
    let server = TestServer::start(2).await;
    let mut sessions = Vec::new();
    for _ in 0..2 {
        sessions.push(server.legacy_session().await);
    }

    let mut closing = Box::pin(server.legacy_sessions.close_all());
    assert!(futures::poll!(&mut closing).is_pending());
    drop(closing);
    tokio::time::timeout(DEADLINE, async {
        loop {
            if server.legacy_sessions.active_count().await == 0
                && sessions.iter().all(|session| session.owner.is_closed())
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(server.budget.available_permits(), 2);
    server.close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn concurrent_bulk_cleanup_waits_for_the_same_drained_sessions() {
    let server = TestServer::start(1).await;
    let session = server.legacy_session().await;
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let blocking = tokio::task::spawn_blocking(move || {
        let _ = entered_tx.send(());
        release_rx.recv().unwrap();

        Ok(())
    });
    entered_rx.await.unwrap();
    let previous = session.task.lock().await.replace(blocking).unwrap();
    previous.abort();
    let _ = previous.await;
    let mut first = Box::pin(server.legacy_sessions.close_all());
    assert!(futures::poll!(&mut first).is_pending());
    while server.legacy_sessions.active_count().await != 0 {
        tokio::task::yield_now().await;
    }

    let mut second = Box::pin(server.legacy_sessions.close_all());
    assert!(futures::poll!(&mut second).is_pending());
    tokio::task::yield_now().await;
    let still_waiting = futures::poll!(&mut second).is_pending();
    release_tx.send(()).unwrap();
    first.await.unwrap();
    assert!(
        still_waiting,
        "concurrent cleanup returned before the drained bridge finished"
    );
    second.await.unwrap();
    assert_eq!(server.budget.available_permits(), 1);
    server.close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn cancelled_config_waiter_still_closes_changed_service_sessions() {
    let server = TestServer::start(1).await;
    let session = server.legacy_session().await;
    let epoch = server.proxy.request_token().await;
    let mut config = server.proxy.config().await;
    let revision = config.revision;
    config.services[0].enabled = false;
    let guard = server.proxy.config.write().await;
    let publisher = server.proxy.clone();
    let waiter = tokio::spawn(async move { publisher.save_config(config, revision).await });
    tokio::task::yield_now().await;
    waiter.abort();
    assert!(waiter.await.unwrap_err().is_cancelled());
    drop(guard);
    tokio::time::timeout(DEADLINE, async {
        loop {
            if server.proxy.config().await.revision == revision + 1 && session.owner.is_closed() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(!epoch.is_cancelled());
    assert_eq!(server.legacy_sessions.active_count().await, 0);
    assert_eq!(server.budget.available_permits(), 1);
    server.close().await;
}

#[tokio::test(flavor = "current_thread")]
async fn cancelled_failed_publication_preserves_memory_budget_and_epoch() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("mcp-proxy.json");
    let mut service = ProxyService::new_with_path(path.clone()).unwrap();
    let config = service.config().await;
    service
        .save_config(config.clone(), config.revision)
        .await
        .unwrap();
    let before = service.config().await;
    let generation = service.generation();
    let token = service.request_token().await;
    let (entered_tx, mut entered_rx) = mpsc::unbounded_channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let release_rx = std::sync::Mutex::new(release_rx);
    service.publish_hook = Some(Arc::new(move || {
        entered_tx.send(()).unwrap();
        release_rx.lock().unwrap().recv().unwrap();
    }));
    let publisher = service.clone();
    let mut updated = before.clone();
    updated.gateway.max_units = 2;
    let revision = before.revision;
    let waiter = tokio::spawn(async move { publisher.save_config(updated, revision).await });
    entered_rx.recv().await.unwrap();
    waiter.abort();
    assert!(waiter.await.unwrap_err().is_cancelled());
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    release_tx.send(()).unwrap();
    assert_eq!(service.config().await, before);
    assert_eq!(service.generation(), generation);
    assert_eq!(service.budget.total_permits(), before.gateway.max_units);
    assert!(!token.is_cancelled());
    assert!(path.is_dir());
}

#[tokio::test(flavor = "current_thread")]
async fn single_worker_publication_offloads_io_and_survives_cancel() {
    for settings_only in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("mcp-proxy.json");
        let mut service = ProxyService::new_with_path(path.clone()).unwrap();
        let (entered_tx, mut entered_rx) = mpsc::unbounded_channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let release_rx = std::sync::Mutex::new(release_rx);
        let runtime_thread = std::thread::current().id();
        service.publish_hook = Some(Arc::new(move || {
            assert_ne!(
                runtime_thread,
                std::thread::current().id(),
                "filesystem work must not run on the async worker"
            );
            entered_tx.send(()).unwrap();
            release_rx.lock().unwrap().recv().unwrap();
        }));
        let publisher = service.clone();
        let mut updated = service.config().await;
        let revision = updated.revision;
        updated.gateway.max_units = 3;
        let waiter = tokio::spawn(async move {
            if settings_only {
                publisher
                    .save_gateway_settings(updated.gateway, revision)
                    .await
            } else {
                publisher.save_config(updated, revision).await
            }
        });
        tokio::time::timeout(Duration::from_secs(1), entered_rx.recv())
            .await
            .unwrap()
            .unwrap();
        waiter.abort();
        assert!(waiter.await.unwrap_err().is_cancelled());
        let contender = service.clone();
        let stale = service.config.try_read().is_err();
        assert!(stale, "publisher must retain the config lock through I/O");
        let conflict = tokio::spawn(async move {
            contender
                .save_config(ProxyConfig::default(), revision)
                .await
        });
        release_tx.send(()).unwrap();
        assert_eq!(
            conflict.await.unwrap().unwrap_err().code,
            crate::error::ErrorCode::ValidationFailed
        );
        let memory = service.config().await;
        assert_eq!(memory.revision, revision + 1);
        assert_eq!(
            crate::mcp_proxy::config::load_config(&path).unwrap(),
            memory
        );
        assert_eq!(service.generation(), 2);
    }
}

#[tokio::test]
async fn disabled_fallback_preserves_corrupt_file() {
    let temp = tempfile::tempdir().unwrap();
    let paths = crate::paths::ConfigPaths::for_home(temp.path());
    let path = paths.app_config_dir().join("mcp-proxy.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    crate::private_file::publish(&path, b"invalid JSON").unwrap();
    assert!(ProxyService::new(&paths).is_err());
    let service = ProxyService::new_disabled(&paths);
    assert!(!service.config().await.gateway.enabled);
    assert_eq!(std::fs::read(&path).unwrap(), b"invalid JSON");
}

#[tokio::test]
async fn capacity_changes_apply_to_existing_budget_clones_and_held_permits() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("mcp-proxy.json");
    let service = ProxyService::new_with_path(path.clone()).unwrap();
    let budget = service.budget().clone();
    let mut config = service.config().await;
    config.gateway.max_units = 2;
    service
        .save_config(config.clone(), config.revision)
        .await
        .unwrap();
    let first = budget.try_acquire().unwrap();
    let second = budget.try_acquire().unwrap();

    let mut config = service.config().await;
    config.gateway.max_units = 1;
    service
        .save_gateway_settings(config.gateway, config.revision)
        .await
        .unwrap();
    assert_eq!(budget.total_permits(), 1);
    assert_eq!(budget.used_permits(), 2);
    drop(first);
    assert!(budget.try_acquire().is_err());
    drop(second);
    assert_eq!(budget.available_permits(), 1);

    let mut config = service.config().await;
    config.gateway.max_units = 3;
    service
        .save_gateway_settings(config.gateway, config.revision)
        .await
        .unwrap();
    assert_eq!(budget.available_permits(), 3);

    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    let mut config = service.config().await;
    config.gateway.max_units = 4;
    assert!(service
        .save_gateway_settings(config.gateway, config.revision)
        .await
        .is_err());
    assert_eq!(budget.total_permits(), 3);
    assert_eq!(budget.available_permits(), 3);
}

#[tokio::test]
async fn failed_legacy_spawn_removes_sdk_session_and_releases_budget() {
    let sessions = LegacySessionManager::new();
    let budget = OwnerBudget::new(1);
    let temp = tempfile::tempdir().unwrap();
    let service = ProxyServiceConfig {
        id: "missing".into(),
        name: "Missing".into(),
        enabled: true,
        command: vec![temp
            .path()
            .join("missing-executable")
            .to_string_lossy()
            .into()],
        cwd: temp.path().to_string_lossy().into(),
        environment: HashMap::new(),
        protocol: ProxyProtocol::Legacy,
        timeouts: ServiceTimeouts::default(),
    };
    assert!(sessions
        .create_session(
            &service,
            budget.try_acquire().unwrap(),
            300,
            Arc::new(tokio::sync::RwLock::new(())),
            tokio_util::sync::CancellationToken::new(),
            tokio_util::sync::CancellationToken::new(),
        )
        .await
        .is_err());
    assert_eq!(sessions.active_count().await, 0);
    assert_eq!(budget.available_permits(), 1);
}

#[tokio::test]
async fn shutdown_closes_live_streams_and_sessions_before_waiting_for_listener() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("mcp-proxy.json");
    let config = ProxyConfig {
        gateway: GatewaySettings {
            enabled: true,
            max_units: 5,
            ..GatewaySettings::default()
        },
        services: vec![ProxyServiceConfig {
            id: "legacy".into(),
            name: "Legacy".into(),
            enabled: true,
            command: vec![test_peer_binary(), "legacy".into(), "shutdown".into()],
            cwd: temp.path().to_string_lossy().into(),
            environment: HashMap::new(),
            protocol: ProxyProtocol::Auto,
            timeouts: ServiceTimeouts::default(),
        }],
        ..ProxyConfig::default()
    };
    save_config(&path, &config).unwrap();
    let service = Arc::new(ProxyService::new_with_path(path).unwrap());
    let http =
        HttpServer::new_with_path(temp.path().join("mom-http.json"), service.router()).unwrap();
    let token = http.rotate_key(async { Ok(()) }).await.unwrap();
    http.set_settings(MomHttpSettings {
        port: unused_port().await,
        allow_lan: false,
    })
    .await
    .unwrap();
    let addr = http.bound_addr().await.unwrap();
    let url = format!("http://{addr}/mcp-proxy?server=legacy");
    let client = http_client();
    let mut streams = Vec::new();
    for _ in 0..4 {
        let response = client
            .post(&url)
            .bearer_auth(&token)
            .json(&initialize(LEGACY))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let id = response.headers()["Mcp-Session-Id"].to_str().unwrap();
        let stream = client
            .get(&url)
            .bearer_auth(&token)
            .header("Mcp-Session-Id", id)
            .send()
            .await
            .unwrap();
        assert_eq!(stream.status(), reqwest::StatusCode::OK);
        streams.push(stream);
    }

    let mut config = service.config().await;
    let mut modern = config.services[0].clone();
    modern.id = "modern".into();
    modern.command[1] = "modern".into();
    config.services.push(modern);
    service
        .save_config(config.clone(), config.revision)
        .await
        .unwrap();
    let response = client
        .post(format!("http://{addr}/mcp-proxy?server=modern"))
        .bearer_auth(&token)
        .header("MCP-Protocol-Version", MODERN)
        .header("Mcp-Method", "tools/call")
        .json(&call(7, "wait", Some(metadata("shutdown"))))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let mut reader: SseReader = Box::pin(sse_stream::SseStream::from_bytes_stream(
        response.bytes_stream(),
    ));
    next_sse(&mut reader).await;
    assert_eq!(service.budget().available_permits(), 0);
    let (http_result, proxy_result) = tokio::join!(
        http.shutdown(Duration::from_secs(2)),
        service.shutdown(Duration::from_secs(2))
    );
    http_result.unwrap();
    proxy_result.unwrap();
    assert_eq!(service.legacy_sessions().active_count().await, 0);
    assert_eq!(service.budget().available_permits(), 5);
    assert_eq!(http.bound_addr().await, None);
    drop(streams);
}

// -----------------------------------------------------------------------------
// Test 7: Churn under sequential rapid requests (no permit leak)
// -----------------------------------------------------------------------------

#[tokio::test]
async fn modern_churn_rapid_sequential_requests_no_permit_leak() {
    let mut server = TestServer::start(4).await;
    let client = http_client();

    for i in 0..10 {
        let wire = call(i, "echo", Some(metadata(&format!("client-{i}"))));
        let res = client
            .post(format!("{}?server=modern-svc", server.url))
            .header("Authorization", format!("Bearer {}", server.raw_token))
            .header("Accept", "application/json, text/event-stream")
            .header("MCP-Protocol-Version", MODERN)
            .header("Mcp-Method", "tools/call")
            .json(&wire)
            .send()
            .await
            .unwrap();

        assert_eq!(res.status(), reqwest::StatusCode::OK);
        let mut reader: SseReader =
            Box::pin(sse_stream::SseStream::from_bytes_stream(res.bytes_stream()));
        let _prog = next_sse(&mut reader).await;
        let result = until_result(&mut reader).await;
        assert_eq!(result["id"], i);

        let pid = match server.event().await {
            Observation::Started(p) => p,
            other => panic!("expected Started, got {other:?}"),
        };
        assert!(matches!(server.event().await, Observation::Closed(p) if p == pid));
    }

    // All permits returned after churn
    assert_eq!(server.budget.available_permits(), 4);

    server.close().await;
}
