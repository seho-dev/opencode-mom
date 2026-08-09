use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::core::draft_state::trim_model_value;
use crate::core::models::{ModelGroup, ModelGroupAgentOverride, ModelGroupCategoryMapping};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentGroupModelMatchCounts {
    pub category_mappings: usize,
    pub agent_overrides: usize,
    pub open_code_agent_overrides: usize,
}

impl CurrentGroupModelMatchCounts {
    pub const fn total(self) -> usize {
        self.category_mappings + self.agent_overrides + self.open_code_agent_overrides
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrentGroupModelReplaceResult {
    pub category_mappings: Vec<ModelGroupCategoryMapping>,
    pub agent_overrides: Vec<ModelGroupAgentOverride>,
    pub open_code_agent_overrides: Vec<ModelGroupAgentOverride>,
    pub match_counts: CurrentGroupModelMatchCounts,
}

pub fn current_group_model_match_counts(
    search_value: &str,
    draft_category_mappings: &[ModelGroupCategoryMapping],
    draft_agent_overrides: &[ModelGroupAgentOverride],
    draft_open_code_agent_overrides: &[ModelGroupAgentOverride],
) -> CurrentGroupModelMatchCounts {
    let trimmed_search = trim_model_value(search_value);
    if trimmed_search.is_empty() {
        return CurrentGroupModelMatchCounts {
            category_mappings: 0,
            agent_overrides: 0,
            open_code_agent_overrides: 0,
        };
    }

    CurrentGroupModelMatchCounts {
        category_mappings: draft_category_mappings
            .iter()
            .filter(|mapping| trim_model_value(&mapping.model_ref) == trimmed_search)
            .count(),
        agent_overrides: draft_agent_overrides
            .iter()
            .filter(|agent_override| trim_model_value(&agent_override.model_ref) == trimmed_search)
            .count(),
        open_code_agent_overrides: draft_open_code_agent_overrides
            .iter()
            .filter(|agent_override| trim_model_value(&agent_override.model_ref) == trimmed_search)
            .count(),
    }
}

pub fn replacing_current_group_model_matches(
    search_value: &str,
    replace_value: &str,
    draft_category_mappings: &[ModelGroupCategoryMapping],
    draft_agent_overrides: &[ModelGroupAgentOverride],
    draft_open_code_agent_overrides: &[ModelGroupAgentOverride],
) -> CurrentGroupModelReplaceResult {
    let trimmed_search = trim_model_value(search_value);
    let trimmed_replace = trim_model_value(replace_value);
    let match_counts = current_group_model_match_counts(
        trimmed_search,
        draft_category_mappings,
        draft_agent_overrides,
        draft_open_code_agent_overrides,
    );

    if trimmed_search.is_empty() {
        return CurrentGroupModelReplaceResult {
            category_mappings: draft_category_mappings.to_vec(),
            agent_overrides: draft_agent_overrides.to_vec(),
            open_code_agent_overrides: draft_open_code_agent_overrides.to_vec(),
            match_counts,
        };
    }

    CurrentGroupModelReplaceResult {
        category_mappings: draft_category_mappings
            .iter()
            .map(|mapping| ModelGroupCategoryMapping {
                category_name: mapping.category_name.clone(),
                model_ref: replacement_for(&mapping.model_ref, trimmed_search, trimmed_replace),
            })
            .collect(),
        agent_overrides: draft_agent_overrides
            .iter()
            .map(|agent_override| ModelGroupAgentOverride {
                agent_name: agent_override.agent_name.clone(),
                model_ref: replacement_for(
                    &agent_override.model_ref,
                    trimmed_search,
                    trimmed_replace,
                ),
            })
            .collect(),
        open_code_agent_overrides: draft_open_code_agent_overrides
            .iter()
            .map(|agent_override| ModelGroupAgentOverride {
                agent_name: agent_override.agent_name.clone(),
                model_ref: replacement_for(
                    &agent_override.model_ref,
                    trimmed_search,
                    trimmed_replace,
                ),
            })
            .collect(),
        match_counts,
    }
}

pub fn persisted_draft_group(
    draft_group: &ModelGroup,
    draft_category_mappings: &[ModelGroupCategoryMapping],
    draft_agent_overrides: &[ModelGroupAgentOverride],
    draft_open_code_agent_overrides: &[ModelGroupAgentOverride],
    updated_at: OffsetDateTime,
) -> ModelGroup {
    let description = draft_group
        .description
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);

    ModelGroup {
        id: draft_group.id,
        name: draft_group.name.trim().to_owned(),
        description,
        category_mappings: draft_category_mappings.to_vec(),
        agent_overrides: draft_agent_overrides.to_vec(),
        open_code_agent_overrides: draft_open_code_agent_overrides.to_vec(),
        is_enabled: draft_group.is_enabled,
        updated_at,
    }
}

fn replacement_for(model_ref: &str, search: &str, replacement: &str) -> String {
    if trim_model_value(model_ref) == search {
        replacement.to_owned()
    } else {
        model_ref.to_owned()
    }
}
