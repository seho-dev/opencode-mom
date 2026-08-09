#[path = "draft_names.rs"]
mod draft_names;
#[path = "draft_opencode.rs"]
mod draft_opencode;
#[path = "draft_replace.rs"]
mod draft_replace;
#[path = "draft_rows.rs"]
mod draft_rows;

pub use draft_names::unique_group_name;
pub use draft_opencode::{
    open_code_agent_mapping_presentation, open_code_agent_override_section_counts,
    retained_open_code_agent_overrides, OpenCodeAgentDiscoveredRow, OpenCodeAgentMappingEdit,
    OpenCodeAgentMappingPresentation, OpenCodeAgentOverrideInfoRow,
    OpenCodeAgentOverrideSectionCounts, OpenCodeDiscoveryState,
};
pub use draft_replace::{
    current_group_model_match_counts, persisted_draft_group, replacing_current_group_model_matches,
    CurrentGroupModelMatchCounts, CurrentGroupModelReplaceResult,
};
pub use draft_rows::{
    agent_duplicate_warnings, category_duplicate_warnings, AgentMappingDraftRow,
    CategoryMappingDraftRow, DraftRowWarning,
};

pub(crate) fn trim_model_value(value: &str) -> &str {
    value.trim()
}
