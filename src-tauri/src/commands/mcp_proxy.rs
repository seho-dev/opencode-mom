use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;
use crate::http_server::HttpServer;
use crate::mcp_proxy::config::{
    GatewaySettings, ProxyProtocol, ProxyServiceConfig, ServiceTimeouts,
};
use crate::mcp_proxy::ProxyService;
use crate::paths::ConfigPaths;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct McpProxyServerSummaryDto {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub protocol: String,
    pub active_sessions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct McpProxyStateDto {
    pub actual: bool,
    pub desired: bool,
    pub actual_state: String,
    pub desired_state: bool,
    pub bound_addr: Option<String>,
    pub port: u16,
    pub allow_lan: bool,
    pub total_services: usize,
    pub enabled_services: usize,
    pub active_sessions: usize,
    pub active_units: usize,
    pub available_units: usize,
    pub max_units: usize,
    pub security_errors: Vec<String>,
    pub services: Vec<McpProxyServerSummaryDto>,
    pub revision: u64,
    pub generation: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct McpProxySettingsDto {
    pub enabled: bool,
    #[serde(alias = "idleTimeoutSecs")]
    pub idle_timeout: u64,
    pub max_units: usize,
    #[serde(default)]
    pub revision: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct McpProxyServerDto {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub command: Vec<String>,
    #[serde(default)]
    pub cwd: String,
    #[serde(alias = "environment", default)]
    pub env: HashMap<String, String>,
    pub protocol: String,
    #[serde(alias = "startupTimeoutSecs", default = "default_startup_secs")]
    pub startup_timeout: u64,
    #[serde(alias = "executionTimeoutSecs", default = "default_execution_secs")]
    pub execution_timeout: u64,
}

fn default_startup_secs() -> u64 {
    30
}

fn default_execution_secs() -> u64 {
    300
}

impl TryFrom<McpProxyServerDto> for ProxyServiceConfig {
    type Error = AppError;

    fn try_from(dto: McpProxyServerDto) -> Result<Self, Self::Error> {
        let protocol = match dto.protocol.to_ascii_lowercase().as_str() {
            "legacy" => ProxyProtocol::Legacy,
            "modern" => ProxyProtocol::Modern,
            "auto" => ProxyProtocol::Auto,
            other => {
                return Err(AppError::validation(format!(
                    "unsupported proxy protocol '{other}': expected 'legacy', 'modern', or 'auto'"
                )));
            }
        };

        let cwd = if dto.cwd.is_empty() {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("/"))
                .to_string_lossy()
                .to_string()
        } else {
            dto.cwd
        };

        Ok(ProxyServiceConfig {
            id: dto.id,
            name: dto.name,
            enabled: dto.enabled,
            command: dto.command,
            cwd,
            environment: dto.env,
            protocol,
            timeouts: ServiceTimeouts {
                startup_secs: dto.startup_timeout,
                execution_secs: dto.execution_timeout,
            },
        })
    }
}

impl From<&ProxyServiceConfig> for McpProxyServerDto {
    fn from(cfg: &ProxyServiceConfig) -> Self {
        Self {
            id: cfg.id.clone(),
            name: cfg.name.clone(),
            enabled: cfg.enabled,
            command: cfg.command.clone(),
            cwd: cfg.cwd.clone(),
            env: cfg.environment.clone(),
            protocol: match cfg.protocol {
                ProxyProtocol::Legacy => "legacy".to_string(),
                ProxyProtocol::Modern => "modern".to_string(),
                ProxyProtocol::Auto => "auto".to_string(),
            },
            startup_timeout: cfg.timeouts.startup_secs,
            execution_timeout: cfg.timeouts.execution_secs,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportMcpProxyServerRequest {
    pub source_scope: String,
    pub project_id: Option<Uuid>,
    pub server_name: String,
    pub expected_source_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloseMcpProxyUnitRequest {
    pub unit_id: String,
    pub expected_generation: u64,
}

#[tauri::command]
pub async fn get_mcp_proxy_state(
    proxy: tauri::State<'_, Arc<ProxyService>>,
    http: tauri::State<'_, Arc<HttpServer>>,
) -> Result<McpProxyStateDto, AppError> {
    let config = proxy.config().await;
    let bound = http.bound_addr().await;
    let settings = http.settings().await;
    let is_running =
        bound.is_some() && config.gateway.enabled && config.gateway.token_verifier.is_some();
    let actual_state = if proxy.cleanup_failed() {
        "cleanup_failed".to_string()
    } else if is_running {
        "running".to_string()
    } else {
        "stopped".to_string()
    };

    let mut security_errors = Vec::new();
    if proxy.cleanup_failed() {
        security_errors
            .push("MCP runtime cleanup failed; process outcome may be uncertain".to_string());
    }
    if config.gateway.token_verifier.is_none() {
        security_errors.push("Missing authentication token verifier".to_string());
    }
    if settings.allow_lan {
        security_errors.push("LAN access is enabled (0.0.0.0)".to_string());
    }

    let session_counts = proxy.legacy_sessions().session_counts_by_service().await;
    let services = config
        .services
        .iter()
        .map(|s| McpProxyServerSummaryDto {
            id: s.id.clone(),
            name: s.name.clone(),
            enabled: s.enabled,
            protocol: match s.protocol {
                ProxyProtocol::Legacy => "legacy".to_string(),
                ProxyProtocol::Modern => "modern".to_string(),
                ProxyProtocol::Auto => "auto".to_string(),
            },
            active_sessions: session_counts.get(&s.id).copied().unwrap_or(0),
        })
        .collect();

    Ok(McpProxyStateDto {
        actual: is_running,
        desired: config.gateway.enabled,
        actual_state,
        desired_state: config.gateway.enabled,
        bound_addr: bound.map(|addr| addr.to_string()),
        port: settings.port,
        allow_lan: settings.allow_lan,
        total_services: config.services.len(),
        enabled_services: config.services.iter().filter(|s| s.enabled).count(),
        active_sessions: proxy.legacy_sessions().active_count().await,
        active_units: proxy.budget().used_permits(),
        available_units: proxy.budget().available_permits(),
        max_units: proxy.budget().total_permits(),
        security_errors,
        services,
        revision: config.revision,
        generation: proxy.generation(),
    })
}

#[tauri::command]
pub async fn get_mcp_proxy_settings(
    proxy: tauri::State<'_, Arc<ProxyService>>,
) -> Result<McpProxySettingsDto, AppError> {
    let config = proxy.config().await;
    Ok(McpProxySettingsDto {
        enabled: config.gateway.enabled,
        idle_timeout: config.gateway.idle_timeout_secs,
        max_units: config.gateway.max_units,
        revision: config.revision,
    })
}

#[tauri::command]
pub async fn save_mcp_proxy_settings(
    proxy: tauri::State<'_, Arc<ProxyService>>,
    settings: McpProxySettingsDto,
    expected_revision: u64,
) -> Result<(), AppError> {
    let current_config = proxy.config().await;
    let gateway = GatewaySettings {
        enabled: settings.enabled,
        idle_timeout_secs: settings.idle_timeout,
        max_units: settings.max_units,
        token_verifier: current_config.gateway.token_verifier,
    };

    proxy
        .save_gateway_settings(gateway, expected_revision)
        .await
}

#[tauri::command]
pub async fn rotate_mcp_proxy_key(
    proxy: tauri::State<'_, Arc<ProxyService>>,
) -> Result<String, AppError> {
    proxy.rotate_key().await
}

#[tauri::command]
pub async fn get_mcp_proxy_server(
    proxy: tauri::State<'_, Arc<ProxyService>>,
    id: String,
) -> Result<McpProxyServerDto, AppError> {
    let config = proxy.config().await;
    let service = config
        .services
        .iter()
        .find(|s| s.id == id)
        .ok_or_else(|| AppError::not_found(format!("proxy server '{id}' not found")))?;
    Ok(McpProxyServerDto::from(service))
}

#[tauri::command]
pub async fn create_mcp_proxy_server(
    proxy: tauri::State<'_, Arc<ProxyService>>,
    server: McpProxyServerDto,
    expected_revision: u64,
) -> Result<McpProxyServerDto, AppError> {
    let mut config = proxy.config().await;
    if config.services.iter().any(|s| s.id == server.id) {
        return Err(AppError::validation(format!(
            "proxy server '{}' already exists",
            server.id
        )));
    }

    let service = ProxyServiceConfig::try_from(server)?;
    config.services.push(service.clone());
    proxy.save_config(config, expected_revision).await?;

    Ok(McpProxyServerDto::from(&service))
}

#[tauri::command]
pub async fn update_mcp_proxy_server(
    proxy: tauri::State<'_, Arc<ProxyService>>,
    server: McpProxyServerDto,
    expected_revision: u64,
) -> Result<McpProxyServerDto, AppError> {
    let mut config = proxy.config().await;
    let index = config
        .services
        .iter()
        .position(|s| s.id == server.id)
        .ok_or_else(|| AppError::not_found(format!("proxy server '{}' not found", server.id)))?;

    let service = ProxyServiceConfig::try_from(server)?;
    config.services[index] = service.clone();
    proxy.save_config(config, expected_revision).await?;

    Ok(McpProxyServerDto::from(&service))
}

#[tauri::command]
pub async fn delete_mcp_proxy_server(
    proxy: tauri::State<'_, Arc<ProxyService>>,
    id: String,
    expected_revision: u64,
) -> Result<(), AppError> {
    let mut config = proxy.config().await;
    let index = config
        .services
        .iter()
        .position(|s| s.id == id)
        .ok_or_else(|| AppError::not_found(format!("proxy server '{id}' not found")))?;

    config.services.remove(index);
    proxy.save_config(config, expected_revision).await?;

    Ok(())
}

fn extract_command(
    config: &serde_json::Map<String, serde_json::Value>,
) -> Result<Vec<String>, AppError> {
    if let Some(cmd_val) = config.get("command") {
        if let Some(arr) = cmd_val.as_array() {
            let cmd: Vec<String> = arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
            if !cmd.is_empty() {
                return Ok(cmd);
            }
        } else if let Some(cmd_str) = cmd_val.as_str() {
            let mut cmd = vec![cmd_str.to_string()];
            if let Some(args_arr) = config.get("args").and_then(serde_json::Value::as_array) {
                for arg in args_arr {
                    if let Some(arg_str) = arg.as_str() {
                        cmd.push(arg_str.to_string());
                    }
                }
            }
            return Ok(cmd);
        }
    }
    Err(AppError::validation(
        "MCP server configuration missing valid command",
    ))
}

fn extract_cwd(
    config: &serde_json::Map<String, serde_json::Value>,
    target_paths: &ConfigPaths,
) -> String {
    if let Some(cwd_str) = config.get("cwd").and_then(serde_json::Value::as_str) {
        if !cwd_str.is_empty() && Path::new(cwd_str).is_absolute() {
            return cwd_str.to_string();
        }
    }
    std::env::current_dir()
        .unwrap_or_else(|_| target_paths.app_config_dir())
        .to_string_lossy()
        .to_string()
}

fn extract_env(config: &serde_json::Map<String, serde_json::Value>) -> HashMap<String, String> {
    let mut env = HashMap::new();
    let env_val = config.get("environment").or_else(|| config.get("env"));
    if let Some(obj) = env_val.and_then(serde_json::Value::as_object) {
        for (k, v) in obj {
            if let Some(s) = v.as_str() {
                env.insert(k.clone(), s.to_string());
            }
        }
    }
    env
}

fn slugify_id(name: &str) -> String {
    let mut slug = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
            slug.push(c.to_ascii_lowercase());
        } else if (c == ' ' || c == '.' || c == '/') && !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    let slug = slug.trim_matches('-').trim_matches('_');
    if slug.is_empty() || slug.len() > 64 {
        format!("srv-{}", Uuid::new_v4().simple())
    } else {
        slug.to_string()
    }
}

#[tauri::command]
pub async fn import_mcp_proxy_server(
    _proxy: tauri::State<'_, Arc<ProxyService>>,
    paths: tauri::State<'_, ConfigPaths>,
    request: ImportMcpProxyServerRequest,
) -> Result<McpProxyServerDto, AppError> {
    let paths = paths.inner().clone();
    super::spawn_blocking_task("MCP proxy import", move || import_server(&paths, request)).await
}

fn import_server(
    paths: &ConfigPaths,
    request: ImportMcpProxyServerRequest,
) -> Result<McpProxyServerDto, AppError> {
    let target_paths = paths.for_project(request.project_id)?;
    let mcp_list = crate::mcp::list(&target_paths)?;
    let server = mcp_list
        .data
        .into_iter()
        .find(|s| s.name == request.server_name)
        .ok_or_else(|| {
            AppError::not_found(format!("MCP server '{}' not found", request.server_name))
        })?;

    if let Some(ref expected_path) = request.expected_source_path {
        if !expected_path.is_empty() && server.source_path != *expected_path {
            return Err(AppError::validation(format!(
                "source path mismatch for '{}': expected '{}', found '{}'",
                request.server_name, expected_path, server.source_path
            )));
        }
    }

    let command = extract_command(&server.config)?;
    let cwd = extract_cwd(&server.config, &target_paths);
    let env = extract_env(&server.config);
    let id = slugify_id(&request.server_name);

    Ok(McpProxyServerDto {
        id,
        name: request.server_name,
        enabled: !server.disabled,
        command,
        cwd,
        env,
        protocol: "auto".to_string(),
        startup_timeout: 30,
        execution_timeout: 300,
    })
}

#[tauri::command]
pub async fn close_mcp_proxy_unit(
    proxy: tauri::State<'_, Arc<ProxyService>>,
    request: CloseMcpProxyUnitRequest,
) -> Result<(), AppError> {
    if request.expected_generation != proxy.generation() {
        return Err(AppError::validation("proxy runtime generation conflict"));
    }
    proxy
        .legacy_sessions()
        .close_session(&request.unit_id)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests;
