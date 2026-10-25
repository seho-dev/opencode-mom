#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::protected_data::SecretBytes;

pub const MAX_CONFIG_PAYLOAD_BYTES: u64 = 1024 * 1024; // 1 MiB
pub const MAX_ENVELOPE_BYTES: u64 = 8 * 1024 * 1024; // 8 MiB
pub const MAX_SERVICES: usize = 64;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProxyProtocol {
    Legacy,
    Modern,
    #[default]
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ServiceTimeouts {
    #[serde(default = "default_startup_secs")]
    pub startup_secs: u64,
    #[serde(default = "default_execution_secs")]
    pub execution_secs: u64,
}

fn default_startup_secs() -> u64 {
    30
}

fn default_execution_secs() -> u64 {
    300
}

impl Default for ServiceTimeouts {
    fn default() -> Self {
        Self {
            startup_secs: default_startup_secs(),
            execution_secs: default_execution_secs(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProxyServiceConfig {
    pub id: String,
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub command: Vec<String>,
    pub cwd: String,
    #[serde(default)]
    pub environment: HashMap<String, String>,
    #[serde(default)]
    pub protocol: ProxyProtocol,
    #[serde(default)]
    pub timeouts: ServiceTimeouts,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GatewaySettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_idle_timeout_secs")]
    pub idle_timeout_secs: u64,
    #[serde(default = "default_max_units")]
    pub max_units: usize,
    #[serde(default)]
    pub token_verifier: Option<String>,
}

fn default_idle_timeout_secs() -> u64 {
    300
}

fn default_max_units() -> usize {
    16
}

impl Default for GatewaySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            idle_timeout_secs: default_idle_timeout_secs(),
            max_units: default_max_units(),
            token_verifier: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProxyConfig {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default = "default_revision")]
    pub revision: u64,
    #[serde(default)]
    pub gateway: GatewaySettings,
    #[serde(default)]
    pub services: Vec<ProxyServiceConfig>,
}

fn default_schema_version() -> u32 {
    1
}

fn default_revision() -> u64 {
    1
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            schema_version: 1,
            revision: 1,
            gateway: GatewaySettings::default(),
            services: Vec::new(),
        }
    }
}

pub fn validate_config(config: &ProxyConfig) -> Result<(), AppError> {
    if config.schema_version != 1 {
        return Err(AppError::validation(format!(
            "unsupported schema version: {}",
            config.schema_version
        )));
    }
    if config.gateway.idle_timeout_secs == 0 || config.gateway.idle_timeout_secs > 86400 {
        return Err(AppError::validation(
            "idle timeout must be between 1 and 86400 seconds",
        ));
    }
    if config.gateway.max_units == 0 || config.gateway.max_units > 128 {
        return Err(AppError::validation("max units must be between 1 and 128"));
    }
    if let Some(ref verifier) = config.gateway.token_verifier {
        if verifier.len() != 64 || !verifier.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(AppError::validation(
                "token verifier must be a 64-character hex string",
            ));
        }
    }
    if config.services.len() > MAX_SERVICES {
        return Err(AppError::validation(format!(
            "services count exceeds maximum limit of {}",
            MAX_SERVICES
        )));
    }

    let mut seen_ids = HashSet::new();
    for service in &config.services {
        if service.id.is_empty() || service.id.len() > 64 {
            return Err(AppError::validation(
                "service id must be between 1 and 64 characters",
            ));
        }
        if !service
            .id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(AppError::validation(format!(
                "service id '{}' contains invalid characters; must be alphanumeric, hyphen, or underscore",
                service.id
            )));
        }
        if !seen_ids.insert(&service.id) {
            return Err(AppError::validation(format!(
                "duplicate service id: '{}'",
                service.id
            )));
        }
        if service.name.is_empty() || service.name.len() > 128 {
            return Err(AppError::validation(
                "service name must be between 1 and 128 characters",
            ));
        }
        if service.command.is_empty() || service.command[0].is_empty() {
            return Err(AppError::validation(format!(
                "service '{}' command argv cannot be empty",
                service.id
            )));
        }
        if service.cwd.is_empty() || !Path::new(&service.cwd).is_absolute() {
            return Err(AppError::validation(format!(
                "service '{}' cwd must be a valid absolute path",
                service.id
            )));
        }
        if service.timeouts.startup_secs == 0 || service.timeouts.startup_secs > 3600 {
            return Err(AppError::validation(format!(
                "service '{}' startup timeout must be between 1 and 3600 seconds",
                service.id
            )));
        }
        if service.timeouts.execution_secs == 0 || service.timeouts.execution_secs > 86400 {
            return Err(AppError::validation(format!(
                "service '{}' execution timeout must be between 1 and 86400 seconds",
                service.id
            )));
        }
        for k in service.environment.keys() {
            if k.is_empty() || k.contains('=') || k.contains('\0') {
                return Err(AppError::validation(format!(
                    "service '{}' environment variable '{}' is invalid",
                    service.id, k
                )));
            }
        }
    }
    Ok(())
}

pub fn load_config(path: &Path) -> Result<ProxyConfig, AppError> {
    #[cfg(not(windows))]
    {
        let bytes = crate::private_file::read(path, MAX_CONFIG_PAYLOAD_BYTES).map_err(|e| {
            AppError::configuration(format!("failed to read proxy configuration: {:?}", e))
        })?;
        let Some(bytes) = bytes else {
            return Ok(ProxyConfig::default());
        };
        let bytes = SecretBytes(bytes);
        let config: ProxyConfig = serde_json::from_slice(&bytes.0)
            .map_err(|e| AppError::configuration(format!("invalid proxy configuration: {e}")))?;
        validate_config(&config)?;
        Ok(config)
    }

    #[cfg(windows)]
    {
        let bytes = crate::private_file::read(path, MAX_ENVELOPE_BYTES).map_err(|e| {
            AppError::configuration(format!("failed to read proxy configuration: {:?}", e))
        })?;
        let Some(bytes) = bytes else {
            return Ok(ProxyConfig::default());
        };
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct WindowsEnvelope {
            version: u32,
            encrypted_payload: Vec<u8>,
        }
        let envelope: WindowsEnvelope = serde_json::from_slice(&bytes)
            .map_err(|e| AppError::configuration(format!("invalid envelope: {e}")))?;
        if envelope.version != 1 {
            return Err(AppError::configuration("unsupported envelope version"));
        }
        let decrypted = SecretBytes(crate::protected_data::unprotect(
            &envelope.encrypted_payload,
        )?);
        if decrypted.0.len() as u64 > MAX_CONFIG_PAYLOAD_BYTES {
            return Err(AppError::configuration(
                "decrypted proxy configuration exceeds maximum payload limit",
            ));
        }
        let config: ProxyConfig = serde_json::from_slice(&decrypted.0)
            .map_err(|e| AppError::configuration(format!("invalid proxy configuration: {e}")))?;
        validate_config(&config)?;
        Ok(config)
    }
}

pub fn save_config(path: &Path, config: &ProxyConfig) -> Result<(), AppError> {
    validate_config(config)?;
    let serialized = SecretBytes(
        serde_json::to_vec_pretty(config)
            .map_err(|e| AppError::configuration(format!("failed to serialize config: {e}")))?,
    );
    if serialized.0.len() as u64 > MAX_CONFIG_PAYLOAD_BYTES {
        return Err(AppError::configuration(
            "configuration exceeds maximum size limit of 1 MiB",
        ));
    }

    #[cfg(not(windows))]
    {
        crate::private_file::publish(path, &serialized.0).map_err(|e| {
            AppError::configuration(format!("failed to save proxy configuration: {:?}", e))
        })?;
        Ok(())
    }

    #[cfg(windows)]
    {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct WindowsEnvelope {
            version: u32,
            encrypted_payload: Vec<u8>,
        }
        let protected = crate::protected_data::protect(&serialized.0)?;
        let envelope = WindowsEnvelope {
            version: 1,
            encrypted_payload: protected,
        };
        let envelope_bytes = serde_json::to_vec(&envelope)
            .map_err(|e| AppError::configuration(format!("failed to serialize envelope: {e}")))?;
        if envelope_bytes.len() as u64 > MAX_ENVELOPE_BYTES {
            return Err(AppError::configuration(
                "envelope exceeds maximum size limit",
            ));
        }
        crate::private_file::publish(path, &envelope_bytes).map_err(|e| {
            AppError::configuration(format!("failed to save proxy configuration: {:?}", e))
        })?;
        Ok(())
    }
}
