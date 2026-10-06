use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(rename = "baseURL", skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDef {
    pub name: String,
    pub package: Option<String>,
    pub settings: Option<ProviderSettings>,
    pub headers: Option<BTreeMap<String, String>>,
    #[serde(default)]
    pub models: BTreeMap<String, ModelDef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ModelDef {
    pub id: String,
    pub name: Option<String>,
    pub family: Option<String>,
    pub disabled: Option<bool>,
    pub capabilities: Option<ModelCapabilities>,
    pub cost: Option<ModelCost>,
    pub limit: Option<ModelLimit>,
    pub settings: Option<BTreeMap<String, Value>>,
    pub headers: Option<BTreeMap<String, String>>,
    pub variants: Option<Vec<ModelVariant>>,
}

/// V2 model capabilities: tool support plus accepted input/output media types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ModelCapabilities {
    pub tools: Option<bool>,
    pub input: Option<Vec<String>>,
    pub output: Option<Vec<String>>,
}

/// V2 cost per million tokens; cache pricing nests under `cache.read` / `cache.write`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ModelCost {
    pub input: Option<f64>,
    pub output: Option<f64>,
    pub cache: Option<ModelCacheCost>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ModelCacheCost {
    pub read: Option<f64>,
    pub write: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ModelLimit {
    pub context: Option<u64>,
    pub input: Option<u64>,
    pub output: Option<u64>,
}

/// One V2 variant entry: `{ "id", "settings"?, "headers"?, "body"? }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ModelVariant {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<BTreeMap<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentModelBinding {
    pub agent_name: String,
    pub model_ref: String,
    pub variant: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OmoCategoryMapping {
    pub category_name: String,
    pub model_ref: String,
    pub variant: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GroupType {
    // Variant names use the canonical native/slim/omo vocabulary. The serde
    // strings are frozen persisted values required for config compatibility.
    #[serde(rename = "opencode")]
    Native,
    #[serde(rename = "slim")]
    Slim,
    #[serde(rename = "oh-my-openagent")]
    Omo,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelGroup {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    #[serde(rename = "type")]
    pub group_type: GroupType,
    #[serde(default)]
    pub open_code_agent_overrides: Vec<AgentModelBinding>,
    #[serde(default)]
    pub slim_agent_overrides: Option<Vec<AgentModelBinding>>,
    #[serde(default)]
    pub omo_agent_overrides: Option<Vec<AgentModelBinding>>,
    #[serde(default)]
    pub omo_category_mappings: Option<Vec<OmoCategoryMapping>>,
    pub is_enabled: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ThemePreference {
    Light,
    #[default]
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum LocalePreference {
    #[default]
    En,
    Zh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppPreferences {
    pub theme: ThemePreference,
    pub locale: LocalePreference,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppSelectionState {
    #[serde(rename = "selectedGroupID")]
    pub selected_group_id: Option<Uuid>,
    pub selected_group_name: Option<String>,
    #[serde(default)]
    pub preferences: AppPreferences,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    #[serde(default)]
    pub groups: Vec<ModelGroup>,
    #[serde(default)]
    pub state: AppSelectionState,
}

impl AppConfig {
    /// Lightweight structural validation. Type checking is already done by serde; this only
    /// rejects values serde cannot see.
    pub fn validate(&self) -> Result<(), AppError> {
        for group in &self.groups {
            if group.name.trim().is_empty() {
                return Err(AppError::validation(format!(
                    "group '{}' has an empty name",
                    group.id
                )));
            }
        }
        Ok(())
    }
}

/// Reads the application config file. A missing file is an empty default configuration.
pub fn load_config(path: &Path) -> Result<AppConfig, AppError> {
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let content = fs::read_to_string(path).map_err(|error| AppError::io("read", path, error))?;
    let config = serde_json::from_str(&content).map_err(|error| {
        AppError::validation(format!("application config: invalid JSON: {error}"))
    })?;
    Ok(config)
}

/// Serializes the application config and writes it in one plain write.
pub fn save_config(path: &Path, config: &AppConfig) -> Result<(), AppError> {
    let data = serde_json::to_vec_pretty(config)
        .map_err(|error| AppError::validation(format!("application config: {error}")))?;
    crate::document::write_file(path, &data)
}

#[cfg(test)]
mod tests;
