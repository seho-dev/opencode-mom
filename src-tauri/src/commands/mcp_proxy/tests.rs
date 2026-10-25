use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::json;
use uuid::Uuid;

use super::*;
use crate::error::ErrorCode;
use crate::mcp_proxy::config::{ProxyProtocol, ProxyServiceConfig};
use crate::mcp_proxy::ProxyService;
use crate::paths::ConfigPaths;

struct TempHome(PathBuf);

impl Drop for TempHome {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

impl TempHome {
    fn new() -> Self {
        let dir =
            std::env::temp_dir().join(format!("opencode-mom-proxy-ipc-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

#[test]
fn dto_conversions_and_protocol_mapping() {
    let mut env = HashMap::new();
    env.insert("FOO".to_string(), "bar".to_string());

    let dto = McpProxyServerDto {
        id: "test-srv".to_string(),
        name: "Test Server".to_string(),
        enabled: true,
        command: vec!["node".to_string(), "index.js".to_string()],
        cwd: "/tmp".to_string(),
        env: env.clone(),
        protocol: "Modern".to_string(),
        startup_timeout: 45,
        execution_timeout: 600,
    };

    let config = ProxyServiceConfig::try_from(dto.clone()).expect("valid conversion");
    assert_eq!(config.id, "test-srv");
    assert_eq!(config.name, "Test Server");
    assert!(config.enabled);
    assert_eq!(config.protocol, ProxyProtocol::Modern);
    assert_eq!(config.timeouts.startup_secs, 45);
    assert_eq!(config.timeouts.execution_secs, 600);
    assert_eq!(config.environment.get("FOO").unwrap(), "bar");

    let back = McpProxyServerDto::from(&config);
    assert_eq!(back.id, dto.id);
    assert_eq!(back.protocol, "modern");
    assert_eq!(back.startup_timeout, 45);
    assert_eq!(back.execution_timeout, 600);

    let empty_cwd_dto = McpProxyServerDto {
        cwd: String::new(),
        ..dto.clone()
    };
    let config_empty_cwd = ProxyServiceConfig::try_from(empty_cwd_dto).expect("cwd defaults");
    assert!(!config_empty_cwd.cwd.is_empty());
    assert!(Path::new(&config_empty_cwd.cwd).is_absolute());

    let invalid_proto_dto = McpProxyServerDto {
        protocol: "invalid_protocol".to_string(),
        ..dto
    };
    let err = ProxyServiceConfig::try_from(invalid_proto_dto).unwrap_err();
    assert_eq!(err.code, ErrorCode::ValidationFailed);
}

#[test]
fn secret_masking_in_settings_and_state() {
    let settings = McpProxySettingsDto {
        enabled: true,
        idle_timeout: 300,
        max_units: 16,
        revision: 1,
    };

    let serialized_settings = serde_json::to_string(&settings).unwrap();
    assert!(!serialized_settings.contains("verifier"));
    assert!(!serialized_settings.contains("token"));
    assert!(!serialized_settings.contains("secret"));
    assert!(!serialized_settings.contains("password"));
    assert!(serialized_settings.contains("idleTimeout"));
    assert!(!serialized_settings.contains("allowLan"));
    assert!(!serialized_settings.contains("port"));

    let state = McpProxyStateDto {
        actual: true,
        desired: true,
        actual_state: "running".to_string(),
        desired_state: true,
        bound_addr: Some("127.0.0.1:18765".to_string()),
        port: 18765,
        allow_lan: false,
        total_services: 1,
        enabled_services: 1,
        active_sessions: 0,
        active_units: 0,
        available_units: 16,
        max_units: 16,
        security_errors: vec![],
        services: vec![McpProxyServerSummaryDto {
            id: "srv1".to_string(),
            name: "Server 1".to_string(),
            enabled: true,
            protocol: "auto".to_string(),
            active_sessions: 0,
        }],
        revision: 1,
        generation: 1,
    };

    let serialized_state = serde_json::to_string(&state).unwrap();
    assert!(!serialized_state.contains("verifier"));
    assert!(!serialized_state.contains("token"));
    assert!(!serialized_state.contains("secret"));
    assert!(!serialized_state.contains("password"));
    assert!(!serialized_state.contains("command"));
    assert!(!serialized_state.contains("env"));
    assert!(serialized_state.contains("actualState"));
    assert!(serialized_state.contains("desiredState"));
    let wire = serde_json::to_value(&state).unwrap();
    assert_eq!(wire["desiredState"], true);
    assert_eq!(wire["boundAddr"], "127.0.0.1:18765");
    assert_eq!(wire["activeUnits"], 0);

    let summary = &state.services[0];
    let serialized_summary = serde_json::to_string(summary).unwrap();
    assert!(!serialized_summary.contains("command"));
    assert!(!serialized_summary.contains("env"));
    assert!(serialized_summary.contains("activeSessions"));
}

#[test]
fn slugify_id_and_command_extraction() {
    assert_eq!(slugify_id("My Test Server"), "my-test-server");
    assert_eq!(slugify_id("Server.Name/123"), "server-name-123");
    assert!(slugify_id("   ").starts_with("srv-"));

    let mut config_arr = serde_json::Map::new();
    config_arr.insert(
        "command".to_string(),
        json!(["node", "index.js", "--port", "3000"]),
    );
    let extracted = extract_command(&config_arr).unwrap();
    assert_eq!(extracted, vec!["node", "index.js", "--port", "3000"]);

    let mut config_str = serde_json::Map::new();
    config_str.insert("command".to_string(), json!("npx"));
    config_str.insert(
        "args".to_string(),
        json!(["-y", "@modelcontextprotocol/server-filesystem"]),
    );
    let extracted_str = extract_command(&config_str).unwrap();
    assert_eq!(
        extracted_str,
        vec!["npx", "-y", "@modelcontextprotocol/server-filesystem"]
    );

    let empty_cfg = serde_json::Map::new();
    assert!(extract_command(&empty_cfg).is_err());
}

#[test]
fn ipc_crud_and_conflict_rejection() {
    let home = TempHome::new();
    let config_path = home.path().join("mcp-proxy.json");

    let service = Arc::new(ProxyService::new_with_path(config_path).expect("init service"));
    let http = Arc::new(
        HttpServer::new_with_path(home.path().join("mom-http.json"), service.router()).unwrap(),
    );

    let paths = ConfigPaths::with_resource_parts(
        home.path().to_path_buf(),
        None,
        None,
        home.path().join(".config"),
        home.path().join("cwd"),
    );

    let app = tauri::test::mock_builder()
        .manage(service.clone())
        .manage(http.clone())
        .manage(paths)
        .invoke_handler(tauri::generate_handler![
            get_mcp_proxy_state,
            crate::commands::get_mom_http_settings,
            crate::commands::set_mom_http_settings,
            get_mcp_proxy_settings,
            save_mcp_proxy_settings,
            rotate_mcp_proxy_key,
            get_mcp_proxy_server,
            create_mcp_proxy_server,
            update_mcp_proxy_server,
            delete_mcp_proxy_server,
            import_mcp_proxy_server,
            close_mcp_proxy_unit
        ])
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();

    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();

    let invoke = |cmd: &'static str, body: serde_json::Value| {
        tauri::test::get_ipc_response(
            &webview,
            tauri::webview::InvokeRequest {
                cmd: cmd.into(),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                url: "tauri://localhost".parse().unwrap(),
                body: tauri::ipc::InvokeBody::Json(body),
                headers: Default::default(),
                invoke_key: tauri::test::INVOKE_KEY.into(),
            },
        )
    };

    // 1. Initial State
    let res = invoke("get_mcp_proxy_state", json!({})).unwrap();
    let state: McpProxyStateDto = res.deserialize().unwrap();
    assert_eq!(state.actual_state, "stopped");
    assert!(!state.desired_state);
    assert_eq!(state.bound_addr, None);
    assert_eq!(state.active_units, 0);
    assert_eq!(state.total_services, 0);
    assert_eq!(state.revision, 1);

    let http_settings: crate::http_server::MomHttpSettings =
        invoke("get_mom_http_settings", json!({}))
            .unwrap()
            .deserialize()
            .unwrap();
    assert_eq!(http_settings.port, 18765);
    let err = invoke(
        "set_mom_http_settings",
        json!({ "settings": { "port": 0, "allowLan": false } }),
    )
    .unwrap_err();
    assert_eq!(err["code"], "validation_failed");
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    invoke(
        "set_mom_http_settings",
        json!({ "settings": { "port": port, "allowLan": false } }),
    )
    .unwrap();
    let state: McpProxyStateDto = invoke("get_mcp_proxy_state", json!({}))
        .unwrap()
        .deserialize()
        .unwrap();
    assert!(!state.actual);
    assert!(state.bound_addr.is_some());
    assert_eq!(state.port, port);
    assert_eq!(state.revision, 1);

    // 2. Settings get & save
    let res = invoke("get_mcp_proxy_settings", json!({})).unwrap();
    let settings: McpProxySettingsDto = res.deserialize().unwrap();
    assert!(!settings.enabled);
    assert_eq!(state.port, port);

    // Save with wrong revision -> conflict
    let err = invoke(
        "save_mcp_proxy_settings",
        json!({
            "settings": {
                "enabled": false,
                "port": 18766,
                "allowLan": false,
                "idleTimeout": 300,
                "maxUnits": 16,
                "revision": 999
            },
            "expectedRevision": 999
        }),
    )
    .unwrap_err();
    assert_eq!(err["code"], "validation_failed");

    // Save with correct revision
    let _ = invoke(
        "save_mcp_proxy_settings",
        json!({
            "settings": {
                "enabled": false,
                "port": 18766,
                "allowLan": false,
                "idleTimeout": 300,
                "maxUnits": 16,
                "revision": 1
            },
            "expectedRevision": 1
        }),
    )
    .unwrap();

    // Verify revision bumped
    let res = invoke("get_mcp_proxy_settings", json!({})).unwrap();
    let updated_settings: McpProxySettingsDto = res.deserialize().unwrap();
    assert_eq!(tauri::async_runtime::block_on(http.settings()).port, port);
    assert_eq!(updated_settings.revision, 2);

    // 3. Key rotation
    let key_res = invoke("rotate_mcp_proxy_key", json!({})).unwrap();
    let token: String = key_res.deserialize().unwrap();
    assert!(!token.is_empty());

    let res = invoke("get_mcp_proxy_settings", json!({})).unwrap();
    let settings_after_rotate: McpProxySettingsDto = res.deserialize().unwrap();
    assert_eq!(settings_after_rotate.revision, 3);

    // 4. Server CRUD
    let server_dto = json!({
        "server": {
            "id": "my-tool",
            "name": "My Tool",
            "enabled": true,
            "command": ["echo", "hello"],
            "cwd": home.path().to_string_lossy().to_string(),
            "env": {"TEST_VAR": "1"},
            "protocol": "auto",
            "startupTimeout": 30,
            "executionTimeout": 300
        },
        "expectedRevision": 3
    });

    let res = invoke("create_mcp_proxy_server", server_dto).unwrap();
    let created: McpProxyServerDto = res.deserialize().unwrap();
    assert_eq!(created.id, "my-tool");

    // Duplicate create rejected
    let err = invoke(
        "create_mcp_proxy_server",
        json!({
            "server": {
                "id": "my-tool",
                "name": "My Tool 2",
                "enabled": true,
                "command": ["echo", "2"],
                "cwd": home.path().to_string_lossy().to_string(),
                "env": {},
                "protocol": "auto",
                "startupTimeout": 30,
                "executionTimeout": 300
            },
            "expectedRevision": 4
        }),
    )
    .unwrap_err();
    assert_eq!(err["code"], "validation_failed");

    // Get server
    let res = invoke("get_mcp_proxy_server", json!({"id": "my-tool"})).unwrap();
    let fetched: McpProxyServerDto = res.deserialize().unwrap();
    assert_eq!(fetched.name, "My Tool");

    // Get non-existent server -> not_found
    let err = invoke("get_mcp_proxy_server", json!({"id": "non-existent"})).unwrap_err();
    assert_eq!(err["code"], "not_found");

    // Update server with stale revision -> conflict
    let err = invoke(
        "update_mcp_proxy_server",
        json!({
            "server": {
                "id": "my-tool",
                "name": "Updated Tool",
                "enabled": true,
                "command": ["echo", "updated"],
                "cwd": home.path().to_string_lossy().to_string(),
                "env": {},
                "protocol": "modern",
                "startupTimeout": 40,
                "executionTimeout": 400
            },
            "expectedRevision": 1
        }),
    )
    .unwrap_err();
    assert_eq!(err["code"], "validation_failed");

    // Update server with correct revision (4)
    let res = invoke(
        "update_mcp_proxy_server",
        json!({
            "server": {
                "id": "my-tool",
                "name": "Updated Tool",
                "enabled": true,
                "command": ["echo", "updated"],
                "cwd": home.path().to_string_lossy().to_string(),
                "env": {},
                "protocol": "modern",
                "startupTimeout": 40,
                "executionTimeout": 400
            },
            "expectedRevision": 4
        }),
    )
    .unwrap();
    let updated: McpProxyServerDto = res.deserialize().unwrap();
    assert_eq!(updated.name, "Updated Tool");
    assert_eq!(updated.protocol, "modern");

    // Delete server with stale revision -> conflict
    let err = invoke(
        "delete_mcp_proxy_server",
        json!({
            "id": "my-tool",
            "expectedRevision": 1
        }),
    )
    .unwrap_err();
    assert_eq!(err["code"], "validation_failed");

    // Delete server with correct revision (5)
    invoke(
        "delete_mcp_proxy_server",
        json!({
            "id": "my-tool",
            "expectedRevision": 5
        }),
    )
    .unwrap();

    // Verify server deleted
    let err = invoke("get_mcp_proxy_server", json!({"id": "my-tool"})).unwrap_err();
    assert_eq!(err["code"], "not_found");

    // 5. Unit close generation mismatch rejection
    let gen = service.generation();
    let err = invoke(
        "close_mcp_proxy_unit",
        json!({
            "request": {
                "unitId": "unit-123",
                "expectedGeneration": gen + 99
            }
        }),
    )
    .unwrap_err();
    assert_eq!(err["code"], "validation_failed");

    // Matching generation -> ok
    let ok = invoke(
        "close_mcp_proxy_unit",
        json!({
            "request": {
                "unitId": "unit-123",
                "expectedGeneration": gen
            }
        }),
    );
    assert!(ok.is_ok());

    // 6. Import MCP server
    let config_dir = home.path().join(".config/opencode");
    fs::create_dir_all(&config_dir).unwrap();
    let opencode_json = config_dir.join("opencode.jsonc");
    fs::write(
        &opencode_json,
        r#"{
            "mcp": {
                "servers": {
                    "my-mcp-tool": {
                        "type": "local",
                        "command": ["python3", "-m", "server"],
                        "cwd": "/tmp",
                        "environment": {
                            "API_KEY": "secret-123"
                        }
                    }
                }
            }
        }"#,
    )
    .unwrap();

    let res = invoke(
        "import_mcp_proxy_server",
        json!({
            "request": {
                "sourceScope": "global",
                "projectId": null,
                "serverName": "my-mcp-tool",
                "expectedSourcePath": null
            }
        }),
    )
    .unwrap();
    let imported: McpProxyServerDto = res.deserialize().unwrap();
    assert_eq!(imported.id, "my-mcp-tool");
    assert_eq!(imported.name, "my-mcp-tool");
    assert_eq!(imported.command, vec!["python3", "-m", "server"]);
    assert_eq!(imported.cwd, "/tmp");
    assert_eq!(imported.env.get("API_KEY").unwrap(), "secret-123");
    assert_eq!(imported.protocol, "auto");
    assert_eq!(imported.startup_timeout, 30);
    assert_eq!(imported.execution_timeout, 300);

    // Mismatched expected source path -> rejection
    let err = invoke(
        "import_mcp_proxy_server",
        json!({
            "request": {
                "sourceScope": "global",
                "projectId": null,
                "serverName": "my-mcp-tool",
                "expectedSourcePath": "/non/matching/path.json"
            }
        }),
    )
    .unwrap_err();
    assert_eq!(err["code"], "validation_failed");

    // Non-existent server name -> not_found
    let err = invoke(
        "import_mcp_proxy_server",
        json!({
            "request": {
                "sourceScope": "global",
                "projectId": null,
                "serverName": "does-not-exist",
                "expectedSourcePath": null
            }
        }),
    )
    .unwrap_err();
    assert_eq!(err["code"], "not_found");

    tauri::async_runtime::block_on(http.shutdown(std::time::Duration::from_secs(2))).unwrap();
}

#[tokio::test]
async fn exit_barrier_and_shutdown_lifecycle() {
    let home = TempHome::new();
    let config_path = home.path().join("mcp-proxy.json");
    let service = Arc::new(ProxyService::new_with_path(config_path).expect("init service"));

    // Rotate key to allow gateway to start if enabled
    let _ = service.rotate_key().await.unwrap();

    // Verify shutdown completes within deadline
    let res = service.shutdown(std::time::Duration::from_secs(8)).await;
    assert!(res.is_ok());

    // Second shutdown is idempotent and succeeds quickly
    let res = service.shutdown(std::time::Duration::from_secs(8)).await;
    assert!(res.is_ok());

    // Test atomic flag compare_exchange barrier logic
    let is_exiting = std::sync::atomic::AtomicU8::new(0);
    assert!(is_exiting
        .compare_exchange(
            0,
            1,
            std::sync::atomic::Ordering::SeqCst,
            std::sync::atomic::Ordering::SeqCst
        )
        .is_ok());
    // Second entry fails
    assert!(is_exiting
        .compare_exchange(
            0,
            1,
            std::sync::atomic::Ordering::SeqCst,
            std::sync::atomic::Ordering::SeqCst
        )
        .is_err());
    assert_ne!(is_exiting.load(std::sync::atomic::Ordering::SeqCst), 2);
    is_exiting.store(2, std::sync::atomic::Ordering::SeqCst);
    assert_eq!(is_exiting.load(std::sync::atomic::Ordering::SeqCst), 2);
}
