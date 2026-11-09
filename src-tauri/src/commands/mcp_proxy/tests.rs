use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use serde_json::json;
use tauri::Manager;

use super::*;
use crate::error::ErrorCode;
use crate::http_server::HttpServer;
use crate::mcp_proxy::config::{ProxyProtocol, ProxyServiceConfig};
use crate::mcp_proxy::ProxyService;
use crate::paths::ConfigPaths;

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
    let home = tempfile::tempdir().unwrap();
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
        .manage(Ok::<_, AppError>(http.clone()))
        .manage(paths)
        .invoke_handler(tauri::generate_handler![
            get_mcp_proxy_state,
            crate::commands::get_mom_http_settings,
            crate::commands::set_mom_http_settings,
            get_mcp_proxy_settings,
            save_mcp_proxy_settings,
            crate::commands::rotate_mom_http_key,
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
                url: (if cfg!(windows) {
                    "http://tauri.localhost"
                } else {
                    "tauri://localhost"
                })
                .parse()
                .unwrap(),
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

    // 3. Shared HTTP key rotation does not change proxy configuration.
    let config_before_rotate = tauri::async_runtime::block_on(service.config());
    let generation_before_rotate = service.generation();
    let key_res = invoke("rotate_mom_http_key", json!({})).unwrap();
    let token: String = key_res.deserialize().unwrap();
    assert_eq!(token.len(), 64);
    assert!(http.auth_ready());
    assert_eq!(
        tauri::async_runtime::block_on(service.config()),
        config_before_rotate
    );
    assert_eq!(service.generation(), generation_before_rotate + 1);
    assert!(invoke("rotate_mcp_proxy_key", json!({})).is_err());
    let persisted: serde_json::Value =
        serde_json::from_slice(&fs::read(home.path().join("mom-http.json")).unwrap()).unwrap();
    assert!(crate::http_server::auth::verify_token(
        &token,
        persisted["auth"]["tokenVerifier"].as_str().unwrap(),
    ));
    assert!(!persisted.to_string().contains(&token));
    let verifier = persisted["auth"]["tokenVerifier"].as_str().unwrap();
    for command in [
        "get_mom_http_settings",
        "get_mcp_proxy_settings",
        "get_mcp_proxy_state",
    ] {
        let wire: serde_json::Value = invoke(command, json!({})).unwrap().deserialize().unwrap();
        let serialized = wire.to_string();
        assert!(!serialized.contains(&token), "{command}");
        assert!(!serialized.contains(verifier), "{command}");
        assert!(!serialized.contains("tokenVerifier"), "{command}");
    }

    let res = invoke("get_mcp_proxy_settings", json!({})).unwrap();
    let settings_after_rotate: McpProxySettingsDto = res.deserialize().unwrap();
    assert_eq!(settings_after_rotate.revision, 2);

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
        "expectedRevision": 2
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
            "expectedRevision": 3
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

    // Update server with correct revision (3)
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
            "expectedRevision": 3
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

    // Delete server with correct revision (4)
    invoke(
        "delete_mcp_proxy_server",
        json!({
            "id": "my-tool",
            "expectedRevision": 4
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

fn http_test_app(paths: ConfigPaths) -> tauri::App<tauri::test::MockRuntime> {
    let mut app = tauri::test::mock_builder()
        .setup(move |app| {
            let proxy = Arc::new(ProxyService::new(&paths)?);
            app.manage(proxy.clone());
            app.manage(paths.clone());
            crate::setup_http(app, &paths, &proxy);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            crate::commands::get_mom_http_settings,
            crate::commands::set_mom_http_settings,
            crate::commands::rotate_mom_http_key,
            get_mcp_proxy_state,
            get_mcp_proxy_settings,
            save_mcp_proxy_settings,
        ])
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    #[allow(deprecated)]
    app.run_iteration(|_, _| {});
    app
}

fn invoke_http_test(
    webview: &tauri::WebviewWindow<tauri::test::MockRuntime>,
    cmd: &str,
    body: serde_json::Value,
) -> Result<tauri::ipc::InvokeResponseBody, serde_json::Value> {
    tauri::test::get_ipc_response(
        webview,
        tauri::webview::InvokeRequest {
            cmd: cmd.into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: (if cfg!(windows) {
                "http://tauri.localhost"
            } else {
                "tauri://localhost"
            })
            .parse()
            .unwrap(),
            body: tauri::ipc::InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.into(),
        },
    )
}

#[test]
fn http_initialization_failure_preserves_setup_proxy_ipc_and_files() {
    for (config, invalid_legacy_auth) in [
        (Some(b"invalid HTTP json".as_slice()), false),
        (Some(br#"{"port":23456,"auth":null}"#.as_slice()), false),
        (
            Some(br#"{"port":23456,"auth":{"tokenVerifier":"private-invalid"}}"#.as_slice()),
            false,
        ),
        (Some(br#"{"port":23456}"#.as_slice()), true),
        (None, false),
    ] {
        let home = tempfile::tempdir().unwrap();
        let paths = ConfigPaths::with_resource_parts(
            home.path().to_path_buf(),
            None,
            None,
            home.path().join(".config"),
            home.path().join("cwd"),
        );
        let directory = paths.app_config_dir();
        fs::create_dir_all(&directory).unwrap();
        let http_path = directory.join("mom-http.json");
        if let Some(bytes) = config {
            crate::private_file::publish(&http_path, bytes).unwrap();
        } else {
            fs::create_dir(&http_path).unwrap();
            fs::write(http_path.join("untouched"), b"private-invalid").unwrap();
        }

        let proxy_path = directory.join("mcp-proxy.json");
        crate::mcp_proxy::config::save_config(&proxy_path, &Default::default()).unwrap();
        if invalid_legacy_auth {
            let bytes = serde_json::to_vec(&json!({
                "gateway": {"tokenVerifier": "private-invalid"},
            }))
            .unwrap();
            #[cfg(windows)]
            let bytes = serde_json::to_vec(&json!({
                "version": 1,
                "encryptedPayload": crate::protected_data::protect(&bytes).unwrap(),
            }))
            .unwrap();
            crate::private_file::publish(&proxy_path, &bytes).unwrap();
        }

        let proxy_before = fs::read(&proxy_path).unwrap();
        let app = http_test_app(paths);
        let http = app.state::<HttpServerState>();
        let error = http.inner().as_ref().err().unwrap();
        assert_eq!(error.code, ErrorCode::ConfigurationFailed);
        let expected = serde_json::to_value(error).unwrap();
        assert!(!expected.to_string().contains("private-invalid"));
        assert!(app.try_state::<Arc<HttpServer>>().is_none());
        let proxy = app.state::<Arc<ProxyService>>();
        let config_before = tauri::async_runtime::block_on(proxy.config());
        let generation_before = proxy.generation();
        let request_token = tauri::async_runtime::block_on(proxy.request_token());
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();

        for (cmd, body) in [
            ("get_mom_http_settings", json!({})),
            (
                "set_mom_http_settings",
                json!({"settings": {"port": 23457, "allowLan": false}}),
            ),
            ("rotate_mom_http_key", json!({})),
            ("get_mcp_proxy_state", json!({})),
        ] {
            assert_eq!(invoke_http_test(&webview, cmd, body).unwrap_err(), expected);
        }

        let settings: McpProxySettingsDto =
            invoke_http_test(&webview, "get_mcp_proxy_settings", json!({}))
                .unwrap()
                .deserialize()
                .unwrap();
        assert_eq!(settings.revision, config_before.revision);
        assert_eq!(settings.max_units, config_before.gateway.max_units);
        assert_eq!(
            tauri::async_runtime::block_on(proxy.config()),
            config_before
        );
        assert_eq!(proxy.generation(), generation_before);
        assert!(!request_token.is_cancelled());
        assert_eq!(fs::read(&proxy_path).unwrap(), proxy_before);
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 2);
        if let Some(bytes) = config {
            assert_eq!(fs::read(&http_path).unwrap(), bytes);
        } else {
            assert!(http_path.is_dir());
            assert_eq!(fs::read_dir(&http_path).unwrap().count(), 1);
            assert_eq!(
                fs::read(http_path.join("untouched")).unwrap(),
                b"private-invalid"
            );
        }
        invoke_http_test(
            &webview,
            "save_mcp_proxy_settings",
            json!({
                "settings": {"enabled": false, "idleTimeout": 301, "maxUnits": 8},
                "expectedRevision": settings.revision,
            }),
        )
        .unwrap();
        let updated: McpProxySettingsDto =
            invoke_http_test(&webview, "get_mcp_proxy_settings", json!({}))
                .unwrap()
                .deserialize()
                .unwrap();
        assert_eq!(updated.revision, settings.revision + 1);
        assert_eq!(updated.idle_timeout, 301);
        assert_eq!(updated.max_units, 8);
        let generation = proxy.generation();
        tauri::async_runtime::block_on(crate::shutdown_http(
            app.handle(),
            &proxy,
            std::time::Duration::from_secs(2),
        ));
        assert!(request_token.is_cancelled());
        assert_eq!(proxy.generation(), generation + 1);
    }
}

#[test]
fn occupied_http_port_does_not_block_setup_or_http_reconfiguration() {
    let home = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::with_resource_parts(
        home.path().to_path_buf(),
        None,
        None,
        home.path().join(".config"),
        home.path().join("cwd"),
    );
    fs::create_dir_all(paths.app_config_dir()).unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let http_path = paths.app_config_dir().join("mom-http.json");
    let bytes = serde_json::to_vec(&json!({
        "port": port,
        "allowLan": false,
        "auth": {"tokenVerifier": null},
    }))
    .unwrap();
    crate::private_file::publish(&http_path, &bytes).unwrap();
    let app = http_test_app(paths);
    let http = app.state::<HttpServerState>();
    let http = http.inner().as_ref().unwrap();
    let error = tauri::async_runtime::block_on(http.start()).unwrap_err();
    assert_eq!(error.code, ErrorCode::ConfigurationFailed);
    assert_eq!(fs::read(&http_path).unwrap(), bytes);
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    let settings: crate::http_server::MomHttpSettings =
        invoke_http_test(&webview, "get_mom_http_settings", json!({}))
            .unwrap()
            .deserialize()
            .unwrap();
    assert_eq!(settings.port, port);
    let state: McpProxyStateDto = invoke_http_test(&webview, "get_mcp_proxy_state", json!({}))
        .unwrap()
        .deserialize()
        .unwrap();
    assert_eq!(state.actual_state, "stopped");
    assert_eq!(state.bound_addr, None);
    let token: String = invoke_http_test(&webview, "rotate_mom_http_key", json!({}))
        .unwrap()
        .deserialize()
        .unwrap();
    assert_eq!(token.len(), 64);
    let available = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = available.local_addr().unwrap().port();
    drop(available);
    invoke_http_test(
        &webview,
        "set_mom_http_settings",
        json!({"settings": {"port": port, "allowLan": false}}),
    )
    .unwrap();
    let state: McpProxyStateDto = invoke_http_test(&webview, "get_mcp_proxy_state", json!({}))
        .unwrap()
        .deserialize()
        .unwrap();
    assert_eq!(state.port, port);
    assert_eq!(state.bound_addr, Some(format!("127.0.0.1:{port}")));
    let proxy = app.state::<Arc<ProxyService>>();
    tauri::async_runtime::block_on(crate::shutdown_http(
        app.handle(),
        &proxy,
        std::time::Duration::from_secs(2),
    ));
    assert_eq!(tauri::async_runtime::block_on(http.bound_addr()), None);
}

#[tokio::test]
async fn exit_barrier_and_shutdown_lifecycle() {
    let home = tempfile::tempdir().unwrap();
    let config_path = home.path().join("mcp-proxy.json");
    let service = Arc::new(ProxyService::new_with_path(config_path).expect("init service"));

    let http =
        HttpServer::new_with_path(home.path().join("mom-http.json"), service.router()).unwrap();
    crate::commands::app::rotate_http_key(http.clone(), service.as_ref().clone())
        .await
        .unwrap();

    // Verify shutdown completes within deadline
    let res = service.shutdown(std::time::Duration::from_secs(8)).await;
    assert!(res.is_ok());

    http.shutdown(std::time::Duration::from_secs(2))
        .await
        .unwrap();

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
