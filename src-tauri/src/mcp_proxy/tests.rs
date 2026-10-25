#![allow(deprecated)]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use rmcp::model::*;
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio::sync::mpsc;

use crate::http_server::{HttpServer, MomHttpSettings};
use crate::mcp_proxy::auth;
use crate::mcp_proxy::config::{
    save_config, GatewaySettings, ProxyConfig, ProxyProtocol, ProxyServiceConfig, ServiceTimeouts,
};
use crate::mcp_proxy::process::OwnerBudget;
use crate::mcp_proxy::session::{LegacySessionManager, Observation};
use crate::mcp_proxy::ProxyService;

fn test_peer_binary() -> String {
    if let Ok(bin) = std::env::var("CARGO_BIN_EXE_mcp-test-peer") {
        return bin;
    }
    let mut path = std::env::current_exe().expect("current test exe");
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
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

        let key = auth::generate_key();

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
                token_verifier: Some(key.verifier.clone()),
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
        http.set_settings(MomHttpSettings {
            port,
            allow_lan: false,
        })
        .await
        .unwrap();

        Self {
            url: format!("http://127.0.0.1:{port}/mcp"),
            raw_token: key.raw_token,
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
            .text()
            .await
            .unwrap(),
        "ok"
    );
    assert_eq!(http.start().await.unwrap(), addr);
    let url = format!("http://{addr}/mcp?server=missing");
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
    proxy.rotate_key().await.unwrap();
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
        client.post(&url).send().await.unwrap().status(),
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
    let new_key = server.proxy.rotate_key().await.unwrap();
    assert!(epoch.is_cancelled());
    assert!(!server.proxy.request_token().await.is_cancelled());
    assert_eq!(server.http.bound_addr().await, bound);
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

    let key = auth::generate_key();
    let config = ProxyConfig {
        schema_version: 1,
        revision: 1,
        gateway: GatewaySettings {
            enabled: true,
            idle_timeout_secs: 180,
            max_units: 8,
            token_verifier: Some(key.verifier),
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
async fn proxy_service_rotate_key_invalidates_sessions_and_shutdown() {
    let temp = tempfile::tempdir().unwrap();
    let config_path = temp.path().join("mcp-proxy.json");

    let initial_key = auth::generate_key();
    let config = ProxyConfig {
        schema_version: 1,
        revision: 1,
        gateway: GatewaySettings {
            enabled: true,
            idle_timeout_secs: 300,
            max_units: 4,
            token_verifier: Some(initial_key.verifier),
        },
        services: vec![],
    };
    save_config(&config_path, &config).unwrap();

    let service = ProxyService::new_with_path(config_path.clone()).unwrap();
    assert_eq!(service.generation(), 1);

    // Rotate key
    let new_token = service.rotate_key().await.unwrap();
    assert_eq!(new_token.len(), 64);
    assert_eq!(service.generation(), 2);

    let updated_config = service.config().await;
    assert_eq!(updated_config.revision, 2);
    let verifier = updated_config.gateway.token_verifier.unwrap();
    assert!(auth::verify_token(&new_token, &verifier));

    // Verify on-disk persistence of rotated verifier
    let on_disk = crate::mcp_proxy::config::load_config(&config_path).unwrap();
    assert_eq!(on_disk.revision, 2);
    assert_eq!(on_disk.gateway.token_verifier.unwrap(), verifier);

    // Shutdown completes within bounded deadline
    service.shutdown(Duration::from_secs(2)).await.unwrap();
}

#[tokio::test]
async fn failed_rotation_preserves_config_and_revision() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("mcp-proxy.json");
    let service = ProxyService::new_with_path(path.clone()).unwrap();
    service.rotate_key().await.unwrap();
    let before = service.config().await;
    let generation = service.generation();
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();

    assert!(service.rotate_key().await.is_err());
    assert_eq!(service.config().await, before);
    assert_eq!(service.generation(), generation);
}

#[tokio::test(flavor = "current_thread")]
async fn cancelled_publication_waiter_still_commits_disk_and_memory() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("mcp-proxy.json");
    let service = ProxyService::new_with_path(path.clone()).unwrap();
    let guard = service.config.write().await;
    let publisher = service.clone();
    let waiter = tokio::spawn(async move { publisher.rotate_key().await });
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
    service.rotate_key().await.unwrap();
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
    for rotation in [false, true] {
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
            if rotation {
                publisher.rotate_key().await.map(|_| ())
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
        .create_session(&service, budget.try_acquire().unwrap(), 300)
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
            token_verifier: Some(auth::generate_key().verifier),
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
    let token = service.rotate_key().await.unwrap();
    let http =
        HttpServer::new_with_path(temp.path().join("mom-http.json"), service.router()).unwrap();
    http.set_settings(MomHttpSettings {
        port: unused_port().await,
        allow_lan: false,
    })
    .await
    .unwrap();
    let addr = http.bound_addr().await.unwrap();
    let url = format!("http://{addr}/mcp?server=legacy");
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
        .post(format!("http://{addr}/mcp?server=modern"))
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
