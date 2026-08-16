use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::models::{AppSelectionState, ModelGroup, ModelGroupAgentOverride};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStateResponse {
    pub groups: Vec<ModelGroup>,
    pub app_state: AppSelectionState,
    pub discovered_open_code_agent_names: Vec<String>,
    pub open_code_agent_discovery_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupMutationResponse {
    pub group: ModelGroup,
    pub groups: Vec<ModelGroup>,
    pub app_state: AppSelectionState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyGroupRequest {
    pub id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteGroupRequest {
    pub id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveGroupRequest {
    pub group: ModelGroup,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchGroupRequest {
    pub id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchGroupResponse {
    pub outcome: SwitchGroupOutcome,
    pub warnings: Vec<String>,
    pub app_state: AppSelectionState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SwitchGroupOutcome {
    Success,
    NoOp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverOpenCodeAgentsRequest {
    pub saved_overrides: Vec<ModelGroupAgentOverride>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverOpenCodeAgentsResponse {
    pub agent_names: Vec<String>,
    pub error: Option<String>,
    pub presentation: OpenCodeAgentMappingPresentation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenCodeAgentMappingPresentation {
    pub discovered_rows: Vec<OpenCodeAgentDiscoveredRow>,
    pub stale_overrides: Vec<OpenCodeAgentOverrideInfoRow>,
    pub preserved_overrides: Vec<OpenCodeAgentOverrideInfoRow>,
    pub discovery_error: Option<String>,
    pub is_read_only: bool,
    pub allows_custom_agent_creation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenCodeAgentDiscoveredRow {
    pub id: String,
    pub agent_name: String,
    pub model_ref: String,
    pub is_editable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenCodeAgentOverrideInfoRow {
    pub id: String,
    pub agent_name: String,
    pub model_ref: String,
    pub status: String,
    pub message: String,
}

pub(super) fn open_code_agent_mapping_presentation(
    overrides: &[ModelGroupAgentOverride],
    discovered_agent_names: &[String],
    discovery_error: Option<&str>,
) -> OpenCodeAgentMappingPresentation {
    match discovery_error {
        Some(message) => degraded_presentation(overrides, message),
        None => discovered_presentation(overrides, discovered_agent_names),
    }
}

fn degraded_presentation(
    overrides: &[ModelGroupAgentOverride],
    message: &str,
) -> OpenCodeAgentMappingPresentation {
    OpenCodeAgentMappingPresentation {
        discovered_rows: Vec::new(),
        stale_overrides: Vec::new(),
        preserved_overrides: overrides
            .iter()
            .map(|agent_override| OpenCodeAgentOverrideInfoRow {
                id: format!("preserved:{}", agent_override.agent_name),
                agent_name: agent_override.agent_name.clone(),
                model_ref: agent_override.model_ref.clone(),
                status: "Preserved".to_owned(),
                message: "Editing disabled until OpenCode agent discovery succeeds.".to_owned(),
            })
            .collect(),
        discovery_error: Some(message.to_owned()),
        is_read_only: true,
        allows_custom_agent_creation: false,
    }
}

fn discovered_presentation(
    overrides: &[ModelGroupAgentOverride],
    discovered_agent_names: &[String],
) -> OpenCodeAgentMappingPresentation {
    let discovered_agent_names = unique_names_preserving_order(discovered_agent_names);

    OpenCodeAgentMappingPresentation {
        discovered_rows: discovered_agent_names
            .into_iter()
            .map(|name| OpenCodeAgentDiscoveredRow {
                id: format!("discovered:{name}"),
                agent_name: name.clone(),
                model_ref: model_ref_for(&name, overrides).to_owned(),
                is_editable: true,
            })
            .collect(),
        stale_overrides: Vec::new(),
        preserved_overrides: Vec::new(),
        discovery_error: None,
        is_read_only: false,
        allows_custom_agent_creation: false,
    }
}

fn model_ref_for<'a>(agent_name: &str, overrides: &'a [ModelGroupAgentOverride]) -> &'a str {
    overrides
        .iter()
        .find(|agent_override| agent_override.agent_name == agent_name)
        .map_or("", |agent_override| agent_override.model_ref.as_str())
}

fn unique_names_preserving_order(names: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut unique = Vec::new();
    for name in names {
        if seen.insert(name.as_str()) {
            unique.push(name.clone());
        }
    }
    unique
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CommandErrorCode {
    MissingHome,
    LoadGroupsFailed,
    SaveGroupsFailed,
    LoadAppStateFailed,
    SaveAppStateFailed,
    GroupNotFound,
    DuplicateGroupName,
    GroupDisabled,
    MissingOpenCodeConfig,
    MalformedOpenCodeConfig,
    LoadOhMyConfigFailed,
    BackupFailed,
    WriteFailed,
    RollbackFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: CommandErrorCode,
    pub message: String,
    pub detail: Option<String>,
}

impl CommandError {
    pub fn new(code: CommandErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            detail: None,
        }
    }

    pub fn with_detail(
        code: CommandErrorCode,
        message: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            detail: Some(detail.into()),
        }
    }
}
