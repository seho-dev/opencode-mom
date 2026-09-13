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
    pub temperature: Option<bool>,
    pub tool_call: Option<bool>,
    pub attachment: Option<bool>,
    pub interleaved: Option<Value>,
    pub cost: Option<ModelCost>,
    pub limit: Option<ModelLimit>,
    pub modalities: Option<ModelModalities>,
    pub experimental: Option<bool>,
    pub options: Option<BTreeMap<String, Value>>,
    pub headers: Option<BTreeMap<String, String>>,
    pub variants: Option<BTreeMap<String, Value>>,
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
    pub context_over_200k: Option<ContextOver200k>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ContextOver200k {
    pub input: f64,
    pub output: f64,
    pub cache_read: Option<f64>,
    pub cache_write: Option<f64>,
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
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn legacy_config_without_preferences_yields_defaults() {
        let config: AppConfig = serde_json::from_value(json!({
            "groups": [],
            "state": { "selectedGroupID": null, "selectedGroupName": null }
        }))
        .expect("legacy config should deserialize");
        assert_eq!(config.state.preferences, AppPreferences::default());
        assert_eq!(config.state.preferences.theme, ThemePreference::Dark);
        assert_eq!(config.state.preferences.locale, LocalePreference::En);
    }

    #[test]
    fn explicit_preferences_round_trip_unchanged() {
        let config: AppConfig = serde_json::from_value(json!({
            "groups": [],
            "state": {
                "selectedGroupID": null,
                "selectedGroupName": null,
                "preferences": { "theme": "light", "locale": "zh" }
            }
        }))
        .expect("config with preferences should deserialize");
        let encoded = serde_json::to_string(&config).expect("serialize config");
        let decoded: AppConfig = serde_json::from_str(&encoded).expect("deserialize config");
        assert_eq!(decoded, config);
        assert_eq!(decoded.state.preferences.theme, ThemePreference::Light);
        assert_eq!(decoded.state.preferences.locale, LocalePreference::Zh);
    }

    #[test]
    fn default_config_serializes_preferences() {
        let value = serde_json::to_value(AppConfig::default()).expect("serialize default config");
        assert_eq!(
            value["state"]["preferences"],
            json!({ "theme": "dark", "locale": "en" })
        );
    }

    #[test]
    fn empty_object_deserializes_to_defaults() {
        let config: AppConfig = serde_json::from_value(json!({})).expect("empty object");
        assert_eq!(config, AppConfig::default());
    }
}
