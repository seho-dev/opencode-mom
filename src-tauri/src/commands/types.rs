use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::draft_state::OpenCodeAgentMappingPresentation;
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
