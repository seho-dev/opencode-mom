use std::path::PathBuf;

use crate::core::document::OhMyOpenAgentDocument;
use crate::core::repository::{
    AppStateRepository, ModelGroupRepository, OhMyOpenAgentConfigRepository,
    OpenCodeConfigRepository,
};

#[derive(Debug, Clone)]
pub struct SwitchGroupRepositories {
    pub model_groups: ModelGroupRepository,
    pub app_state: AppStateRepository,
    pub backups_root: PathBuf,
    pub opencode: OpenCodeConfigRepository,
    pub oh_my_openagent: OhMyOpenAgentConfigRepository,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SwitchOutcome {
    Success {
        oh_my_document: OhMyOpenAgentDocument,
        warnings: Vec<String>,
    },
    NoOp,
}
