use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(rename = "baseURL", skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDef {
    pub name: String,
    pub npm: Option<String>,
    pub options: Option<ProviderOptions>,
    #[serde(default)]
    pub models: BTreeMap<String, ModelDef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ModelDef {
    pub id: String,
    pub name: Option<String>,
    pub family: Option<String>,
    pub release_date: Option<String>,
    pub status: Option<ModelStatus>,
    pub reasoning: Option<bool>,
    pub temperature: Option<f64>,
    pub tool_call: Option<bool>,
    pub attachment: Option<bool>,
    pub interleaved: Option<bool>,
    pub cost: Option<ModelCost>,
    pub limit: Option<ModelLimit>,
    pub modalities: Option<ModelModalities>,
    pub experimental: Option<bool>,
    pub options: Option<BTreeMap<String, Value>>,
    pub headers: Option<BTreeMap<String, String>>,
    pub variants: Option<BTreeMap<String, ModelVariant>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelStatus {
    Alpha,
    Beta,
    Deprecated,
    Active,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ModelCost {
    pub input: Option<f64>,
    pub output: Option<f64>,
    pub cache_read: Option<f64>,
    pub cache_write: Option<f64>,
    pub context_over_200k: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ModelLimit {
    pub context: Option<u64>,
    pub input: Option<u64>,
    pub output: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelModalities {
    pub input: Option<Vec<String>>,
    pub output: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ModelVariant {
    pub disabled: Option<bool>,
    pub options: Option<BTreeMap<String, Value>>,
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
    #[serde(rename = "opencode")]
    OpenCode,
    #[serde(rename = "slim")]
    Slim,
    #[serde(rename = "oh-my-openagent")]
    OhMyOpenagent,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppSelectionState {
    #[serde(rename = "selectedGroupID")]
    pub selected_group_id: Option<Uuid>,
    pub selected_group_name: Option<String>,
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
