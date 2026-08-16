use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

const fn default_migration_version() -> u16 {
    1
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelGroupCategoryMapping {
    pub category_name: String,
    pub model_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelGroupAgentOverride {
    pub agent_name: String,
    pub model_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelGroup {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub category_mappings: Vec<ModelGroupCategoryMapping>,
    pub agent_overrides: Vec<ModelGroupAgentOverride>,
    #[serde(default)]
    pub open_code_agent_overrides: Vec<ModelGroupAgentOverride>,
    pub is_enabled: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSelectionState {
    #[serde(rename = "selectedGroupID")]
    pub selected_group_id: Option<Uuid>,
    pub selected_group_name: Option<String>,
    pub last_successful_write: Option<LastSuccessfulWriteMetadata>,
    pub last_warning_summary: Option<ProjectionIssueSummary>,
    pub last_error_summary: Option<ProjectionIssueSummary>,
    #[serde(default = "default_migration_version")]
    pub migration_version: u16,
}

impl AppSelectionState {
    pub const CURRENT_SCHEMA_VERSION: u16 = 2;
}

impl Default for AppSelectionState {
    fn default() -> Self {
        Self {
            selected_group_id: None,
            selected_group_name: None,
            last_successful_write: None,
            last_warning_summary: None,
            last_error_summary: None,
            migration_version: Self::CURRENT_SCHEMA_VERSION,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OmoSwitchConfig {
    #[serde(default)]
    pub groups: Vec<ModelGroup>,
    #[serde(default)]
    pub state: AppSelectionState,
}

impl Default for OmoSwitchConfig {
    fn default() -> Self {
        Self {
            groups: Vec::new(),
            state: AppSelectionState::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LastSuccessfulWriteMetadata {
    pub target: String,
    #[serde(with = "time::serde::rfc3339")]
    pub wrote_at: OffsetDateTime,
    pub backup_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectionIssueSummary {
    pub message: String,
    pub count: u32,
}
