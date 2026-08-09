use omo_switch_tauri::core::draft_state::{
    agent_duplicate_warnings, category_duplicate_warnings, open_code_agent_mapping_presentation,
    persisted_draft_group, retained_open_code_agent_overrides, unique_group_name,
    AgentMappingDraftRow, CategoryMappingDraftRow, OpenCodeAgentMappingEdit,
    OpenCodeDiscoveryState,
};
use omo_switch_tauri::core::models::{
    ModelGroup, ModelGroupAgentOverride, ModelGroupCategoryMapping,
};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

#[test]
fn draft_state_when_opencode_discovery_fails_retains_and_preserves_saved_overrides() {
    let expected = vec![
        agent_override("stale", "openai/o3"),
        agent_override("alpha", "openai/gpt-5.4"),
    ];
    let group = make_group(expected.clone());

    let retained = retained_open_code_agent_overrides(
        &group,
        &["alpha"],
        OpenCodeDiscoveryState::Failure("OpenCode config is malformed.".to_owned()),
    );
    let persisted = persisted_draft_group(
        &group,
        &group.category_mappings,
        &group.agent_overrides,
        &retained,
        fixed_time(),
    );
    let presentation = open_code_agent_mapping_presentation(
        &expected,
        &["alpha"],
        OpenCodeDiscoveryState::Failure("OpenCode config is malformed.".to_owned()),
    );
    let edited = OpenCodeAgentMappingEdit {
        agent_name: "alpha".to_owned(),
        model_ref: "openai/o3".to_owned(),
    }
    .apply(
        &expected,
        &["alpha"],
        OpenCodeDiscoveryState::Failure("boom".to_owned()),
    );

    assert_eq!(retained, expected);
    assert_eq!(persisted.open_code_agent_overrides, expected);
    assert!(presentation.is_read_only);
    assert_eq!(
        presentation.discovery_error.as_deref(),
        Some("OpenCode config is malformed.")
    );
    assert!(presentation.discovered_rows.is_empty());
    assert_eq!(
        presentation
            .preserved_overrides
            .iter()
            .map(|row| row.agent_name.as_str())
            .collect::<Vec<_>>(),
        vec!["stale", "alpha"]
    );
    assert!(!presentation.allows_custom_agent_creation);
    assert_eq!(edited, expected);
}

#[test]
fn draft_state_when_opencode_discovery_succeeds_filters_stale_and_deduplicates_rows() {
    let saved = vec![
        agent_override("stale", "openai/o3"),
        agent_override("alpha", "openai/gpt-5.4"),
    ];
    let group = make_group(saved.clone());

    let retained = retained_open_code_agent_overrides(
        &group,
        &["beta", "alpha", "alpha"],
        OpenCodeDiscoveryState::Success,
    );
    let presentation = open_code_agent_mapping_presentation(
        &saved,
        &["beta", "alpha", "alpha"],
        OpenCodeDiscoveryState::Success,
    );
    let edited = OpenCodeAgentMappingEdit {
        agent_name: "beta".to_owned(),
        model_ref: " openai/o3 ".to_owned(),
    }
    .apply(
        &retained,
        &["beta", "alpha"],
        OpenCodeDiscoveryState::Success,
    );

    assert_eq!(retained, vec![agent_override("alpha", "openai/gpt-5.4")]);
    assert!(!presentation.is_read_only);
    assert_eq!(
        presentation
            .discovered_rows
            .iter()
            .map(|row| (
                row.agent_name.as_str(),
                row.model_ref.as_str(),
                row.is_editable
            ))
            .collect::<Vec<_>>(),
        vec![("beta", "", true), ("alpha", "openai/gpt-5.4", true)]
    );
    assert!(presentation.preserved_overrides.is_empty());
    assert_eq!(
        edited,
        vec![
            agent_override("alpha", "openai/gpt-5.4"),
            agent_override("beta", "openai/o3")
        ]
    );
}

#[test]
fn draft_state_when_custom_rows_have_duplicate_names_returns_stable_warnings() {
    let categories = vec![
        CategoryMappingDraftRow::custom("cat-1", " custom ", "model/a"),
        CategoryMappingDraftRow::custom("cat-2", "CUSTOM", "model/b"),
        CategoryMappingDraftRow::custom("cat-3", " ", "model/c"),
        CategoryMappingDraftRow::known("known:quick", "quick", "model/d"),
    ];
    let agents = vec![
        AgentMappingDraftRow::custom("agent-1", " reviewer ", "model/a"),
        AgentMappingDraftRow::custom("agent-2", "REVIEWER", "model/b"),
        AgentMappingDraftRow::custom("agent-3", " ", "model/c"),
        AgentMappingDraftRow::known("known:oracle", "oracle", "model/d"),
    ];

    let category_warnings = category_duplicate_warnings(&categories);
    let agent_warnings = agent_duplicate_warnings(&agents);

    assert_eq!(category_warnings.len(), 2);
    assert_eq!(category_warnings[0].row_index, 0);
    assert_eq!(
        category_warnings[0].message,
        "Duplicate category name \"custom\"."
    );
    assert_eq!(category_warnings[1].row_index, 1);
    assert_eq!(
        category_warnings[1].message,
        "Duplicate category name \"CUSTOM\"."
    );
    assert_eq!(agent_warnings.len(), 2);
    assert_eq!(agent_warnings[0].row_index, 0);
    assert_eq!(
        agent_warnings[0].message,
        "Duplicate agent name \"reviewer\"."
    );
    assert_eq!(agent_warnings[1].row_index, 1);
    assert_eq!(
        agent_warnings[1].message,
        "Duplicate agent name \"REVIEWER\"."
    );
}

#[test]
fn draft_state_when_copying_group_name_generates_trimmed_unique_name() {
    let existing_names = ["Primary", "Primary Copy", "primary copy 2", " Other "];

    assert_eq!(unique_group_name("   ", &existing_names), "Untitled Group");
    assert_eq!(unique_group_name("Research", &existing_names), "Research");
    assert_eq!(
        unique_group_name(" Primary Copy ", &existing_names),
        "Primary Copy 3"
    );
}

fn make_group(open_code_agent_overrides: Vec<ModelGroupAgentOverride>) -> ModelGroup {
    ModelGroup {
        id: "11111111-1111-1111-1111-111111111111"
            .parse()
            .expect("Given: UUID fixture parses"),
        name: " Primary ".to_owned(),
        description: Some("   ".to_owned()),
        category_mappings: vec![ModelGroupCategoryMapping {
            category_name: "unspecified-high".to_owned(),
            model_ref: "cliproxyapi/gpt-5.4".to_owned(),
        }],
        agent_overrides: vec![agent_override("general", "cliproxyapi/gpt-5.4")],
        open_code_agent_overrides,
        is_enabled: true,
        updated_at: OffsetDateTime::parse("2023-11-14T22:13:20Z", &Rfc3339)
            .expect("Given: date fixture parses"),
    }
}

fn agent_override(agent_name: &str, model_ref: &str) -> ModelGroupAgentOverride {
    ModelGroupAgentOverride {
        agent_name: agent_name.to_owned(),
        model_ref: model_ref.to_owned(),
    }
}

fn fixed_time() -> OffsetDateTime {
    OffsetDateTime::parse("2027-01-15T08:00:00Z", &Rfc3339).expect("Given: date fixture parses")
}
