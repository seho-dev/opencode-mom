use omo_switch_tauri::core::draft_state::{
    current_group_model_match_counts, open_code_agent_override_section_counts,
    replacing_current_group_model_matches,
};
use omo_switch_tauri::core::models::{ModelGroupAgentOverride, ModelGroupCategoryMapping};

#[test]
fn draft_state_when_counting_opencode_overrides_uses_discovered_names_only() {
    let overrides = vec![
        agent_override("alpha", " openai/gpt-5.4 "),
        agent_override("beta", "   "),
        agent_override("stale", "openai/o3"),
    ];

    let counts = open_code_agent_override_section_counts(&overrides, &["alpha", "beta"]);

    assert_eq!(counts.filled, 1);
    assert_eq!(counts.total, 2);
}

#[test]
fn draft_state_when_replacing_models_uses_trimmed_exact_matches_across_current_draft_only() {
    let category_mappings = vec![
        category_mapping("quick", "openai/gpt-5.4"),
        category_mapping("slow", "openai/gpt-5.4-mini"),
    ];
    let agent_overrides = vec![
        agent_override("general", " openai/gpt-5.4 "),
        agent_override("oracle", "openai/o3"),
    ];
    let open_code_overrides = vec![
        agent_override("alpha", "openai/gpt-5.4"),
        agent_override("beta", "openai/gpt-5.4-mini"),
    ];

    let counts = current_group_model_match_counts(
        "  openai/gpt-5.4  ",
        &category_mappings,
        &agent_overrides,
        &open_code_overrides,
    );
    let result = replacing_current_group_model_matches(
        " openai/gpt-5.4 ",
        " cliproxyapi/gpt-5.5 ",
        &category_mappings,
        &agent_overrides,
        &open_code_overrides,
    );

    assert_eq!(counts.category_mappings, 1);
    assert_eq!(counts.agent_overrides, 1);
    assert_eq!(counts.open_code_agent_overrides, 1);
    assert_eq!(counts.total(), 3);
    assert_eq!(result.match_counts.total(), 3);
    assert_eq!(
        result.category_mappings,
        vec![
            category_mapping("quick", "cliproxyapi/gpt-5.5"),
            category_mapping("slow", "openai/gpt-5.4-mini"),
        ]
    );
    assert_eq!(
        result.agent_overrides,
        vec![
            agent_override("general", "cliproxyapi/gpt-5.5"),
            agent_override("oracle", "openai/o3"),
        ]
    );
    assert_eq!(
        result.open_code_agent_overrides,
        vec![
            agent_override("alpha", "cliproxyapi/gpt-5.5"),
            agent_override("beta", "openai/gpt-5.4-mini"),
        ]
    );
}

#[test]
fn draft_state_when_search_is_blank_leaves_draft_unchanged() {
    let category_mappings = vec![category_mapping("quick", "openai/gpt-5.4")];
    let agent_overrides = vec![agent_override("general", "openai/gpt-5.4")];
    let open_code_overrides = vec![agent_override("alpha", "openai/gpt-5.4")];

    let result = replacing_current_group_model_matches(
        "   ",
        "cliproxyapi/gpt-5.5",
        &category_mappings,
        &agent_overrides,
        &open_code_overrides,
    );

    assert_eq!(result.match_counts.total(), 0);
    assert_eq!(result.category_mappings, category_mappings);
    assert_eq!(result.agent_overrides, agent_overrides);
    assert_eq!(result.open_code_agent_overrides, open_code_overrides);
}

#[test]
fn draft_state_when_replacement_is_blank_or_consecutive_matches_compatibility_behavior() {
    let category_mappings = vec![category_mapping("unspecified-high", "openai/gpt-5.4")];
    let agent_overrides = vec![agent_override("general", "openai/gpt-5.4")];
    let open_code_overrides = vec![agent_override("alpha", "openai/gpt-5.4")];

    let blank = replacing_current_group_model_matches(
        "openai/gpt-5.4",
        "   ",
        &category_mappings,
        &agent_overrides,
        &open_code_overrides,
    );
    let first = replacing_current_group_model_matches(
        "openai/gpt-5.4",
        "cliproxyapi/gpt-5.5",
        &category_mappings,
        &agent_overrides,
        &open_code_overrides,
    );
    let second = replacing_current_group_model_matches(
        "cliproxyapi/gpt-5.5",
        "anthropic/claude-sonnet-4.5",
        &first.category_mappings,
        &first.agent_overrides,
        &first.open_code_agent_overrides,
    );

    assert_eq!(blank.match_counts.total(), 3);
    assert_eq!(blank.category_mappings[0].model_ref, "");
    assert_eq!(blank.agent_overrides[0].model_ref, "");
    assert_eq!(blank.open_code_agent_overrides[0].model_ref, "");
    assert_eq!(first.match_counts.total(), 3);
    assert_eq!(second.match_counts.total(), 3);
    assert_eq!(
        second.category_mappings[0].model_ref,
        "anthropic/claude-sonnet-4.5"
    );
    assert_eq!(
        second.agent_overrides[0].model_ref,
        "anthropic/claude-sonnet-4.5"
    );
    assert_eq!(
        second.open_code_agent_overrides[0].model_ref,
        "anthropic/claude-sonnet-4.5"
    );
}

fn category_mapping(category_name: &str, model_ref: &str) -> ModelGroupCategoryMapping {
    ModelGroupCategoryMapping {
        category_name: category_name.to_owned(),
        model_ref: model_ref.to_owned(),
    }
}

fn agent_override(agent_name: &str, model_ref: &str) -> ModelGroupAgentOverride {
    ModelGroupAgentOverride {
        agent_name: agent_name.to_owned(),
        model_ref: model_ref.to_owned(),
    }
}
