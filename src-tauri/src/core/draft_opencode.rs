use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::core::draft_state::trim_model_value;
use crate::core::models::{ModelGroup, ModelGroupAgentOverride};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenCodeAgentOverrideSectionCounts {
    pub filled: usize,
    pub total: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OpenCodeDiscoveryState {
    Success,
    Failure(String),
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenCodeAgentMappingEdit {
    pub agent_name: String,
    pub model_ref: String,
}

impl OpenCodeAgentMappingEdit {
    pub fn apply(
        &self,
        overrides: &[ModelGroupAgentOverride],
        discovered_agent_names: &[&str],
        discovery_state: OpenCodeDiscoveryState,
    ) -> Vec<ModelGroupAgentOverride> {
        match discovery_state {
            OpenCodeDiscoveryState::Failure(_) => overrides.to_vec(),
            OpenCodeDiscoveryState::Success => {
                update_opencode_model_ref(overrides, discovered_agent_names, self)
            }
        }
    }
}

pub fn open_code_agent_override_section_counts(
    overrides: &[ModelGroupAgentOverride],
    discovered_agent_names: &[&str],
) -> OpenCodeAgentOverrideSectionCounts {
    let discovered = discovered_agent_names
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    let filled = overrides
        .iter()
        .filter(|agent_override| {
            discovered.contains(agent_override.agent_name.as_str())
                && !trim_model_value(&agent_override.model_ref).is_empty()
        })
        .count();

    OpenCodeAgentOverrideSectionCounts {
        filled,
        total: discovered_agent_names.len(),
    }
}

pub fn retained_open_code_agent_overrides(
    group: &ModelGroup,
    discovered_agent_names: &[&str],
    discovery_state: OpenCodeDiscoveryState,
) -> Vec<ModelGroupAgentOverride> {
    match discovery_state {
        OpenCodeDiscoveryState::Failure(_) => group.open_code_agent_overrides.clone(),
        OpenCodeDiscoveryState::Success => {
            let discovered = discovered_agent_names
                .iter()
                .copied()
                .collect::<HashSet<_>>();
            group
                .open_code_agent_overrides
                .iter()
                .filter(|agent_override| discovered.contains(agent_override.agent_name.as_str()))
                .cloned()
                .collect()
        }
    }
}

pub fn open_code_agent_mapping_presentation(
    overrides: &[ModelGroupAgentOverride],
    discovered_agent_names: &[&str],
    discovery_state: OpenCodeDiscoveryState,
) -> OpenCodeAgentMappingPresentation {
    match discovery_state {
        OpenCodeDiscoveryState::Failure(message) => degraded_presentation(overrides, message),
        OpenCodeDiscoveryState::Success => {
            discovered_presentation(overrides, discovered_agent_names)
        }
    }
}

fn degraded_presentation(
    overrides: &[ModelGroupAgentOverride],
    message: String,
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
        discovery_error: Some(message),
        is_read_only: true,
        allows_custom_agent_creation: false,
    }
}

fn discovered_presentation(
    overrides: &[ModelGroupAgentOverride],
    discovered_agent_names: &[&str],
) -> OpenCodeAgentMappingPresentation {
    OpenCodeAgentMappingPresentation {
        discovered_rows: unique_names_preserving_order(discovered_agent_names)
            .into_iter()
            .map(|name| OpenCodeAgentDiscoveredRow {
                id: format!("discovered:{name}"),
                agent_name: name.to_owned(),
                model_ref: model_ref_for(name, overrides).to_owned(),
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

fn update_opencode_model_ref(
    overrides: &[ModelGroupAgentOverride],
    discovered_agent_names: &[&str],
    edit: &OpenCodeAgentMappingEdit,
) -> Vec<ModelGroupAgentOverride> {
    let discovered = discovered_agent_names
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    if !discovered.contains(edit.agent_name.as_str()) {
        return overrides.to_vec();
    }

    let trimmed_model_ref = trim_model_value(&edit.model_ref);
    let mut found_existing = false;
    let mut updated = Vec::new();
    for agent_override in overrides {
        if agent_override.agent_name != edit.agent_name {
            updated.push(agent_override.clone());
            continue;
        }

        found_existing = true;
        if !trimmed_model_ref.is_empty() {
            updated.push(ModelGroupAgentOverride {
                agent_name: edit.agent_name.clone(),
                model_ref: trimmed_model_ref.to_owned(),
            });
        }
    }

    if !found_existing && !trimmed_model_ref.is_empty() {
        updated.push(ModelGroupAgentOverride {
            agent_name: edit.agent_name.clone(),
            model_ref: trimmed_model_ref.to_owned(),
        });
    }

    updated
}

fn model_ref_for<'a>(agent_name: &str, overrides: &'a [ModelGroupAgentOverride]) -> &'a str {
    overrides
        .iter()
        .find(|agent_override| agent_override.agent_name == agent_name)
        .map_or("", |agent_override| agent_override.model_ref.as_str())
}

fn unique_names_preserving_order<'a>(names: &'a [&'a str]) -> Vec<&'a str> {
    let mut seen = HashSet::new();
    let mut unique = Vec::new();
    for name in names {
        if seen.insert(*name) {
            unique.push(*name);
        }
    }
    unique
}
