use std::fs;
use std::path::Path;

use omo_switch_tauri::core::document::{OhMyOpenAgentDocument, OpenCodeDocument};
use omo_switch_tauri::core::models::{
    ModelGroup, ModelGroupAgentOverride, ModelGroupCategoryMapping,
};
use omo_switch_tauri::core::projection::{
    OhMyOpenAgentProjectionService, OpenCodeProjectionService,
};
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

fn fixture(relative_path: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Given: src-tauri has workspace parent")
        .join("fixtures")
        .join("compatibility")
        .join(relative_path);
    fs::read_to_string(path).expect("Given: fixture reads")
}

fn group(
    category_mappings: Vec<ModelGroupCategoryMapping>,
    agent_overrides: Vec<ModelGroupAgentOverride>,
    open_code_agent_overrides: Vec<ModelGroupAgentOverride>,
) -> ModelGroup {
    ModelGroup {
        id: Uuid::parse_str("99999999-9999-9999-9999-999999999999")
            .expect("Given: UUID fixture parses"),
        name: "Projection Fixture".to_owned(),
        description: None,
        category_mappings,
        agent_overrides,
        open_code_agent_overrides,
        is_enabled: true,
        updated_at: OffsetDateTime::parse("2023-11-14T22:13:20Z", &Rfc3339)
            .expect("Given: date fixture parses"),
    }
}

fn override_row(agent_name: &str, model_ref: &str) -> ModelGroupAgentOverride {
    ModelGroupAgentOverride {
        agent_name: agent_name.to_owned(),
        model_ref: model_ref.to_owned(),
    }
}

fn category_row(category_name: &str, model_ref: &str) -> ModelGroupCategoryMapping {
    ModelGroupCategoryMapping {
        category_name: category_name.to_owned(),
        model_ref: model_ref.to_owned(),
    }
}

#[test]
fn projection_when_oh_my_group_selects_subset_preserves_selected_siblings_and_removes_omitted_entries(
) {
    // Given: compatibility behavior uses selected group rows as the full desired Oh My section set.
    let existing = OhMyOpenAgentDocument::parse_jsonc(
        r#"{
  "$schema": "https://example.com/schema.json",
  "customTopLevel": { "keep": true },
  "agents": {
    "librarian": { "model": "legacy", "variant": "medium", "temperature": 0.2 },
    "oracle": { "model": "legacy", "variant": "xhigh" }
  },
  "categories": {
    "quick": { "model": "legacy", "variant": "balanced" },
    "deep": { "model": "legacy", "variant": "slow" }
  }
}"#,
    )
    .expect("Given: Oh My document parses");
    let group = group(
        vec![category_row("quick", "cliproxyapi/minimax-m2.7")],
        vec![override_row("librarian", "cliproxyapi/gpt-5.4")],
        vec![],
    );

    // When: Rust projects the group onto the existing document.
    let result = OhMyOpenAgentProjectionService::project(&group, &existing);

    // Then: selected rows are patched, selected sibling fields survive, omitted rows are removed.
    assert!(result.warnings.is_empty());
    assert_eq!(result.document.raw()["customTopLevel"]["keep"], true);
    assert_eq!(
        result.document.agents()["librarian"]["model"],
        "cliproxyapi/gpt-5.4"
    );
    assert_eq!(result.document.agents()["librarian"]["variant"], "medium");
    assert_eq!(result.document.agents()["librarian"]["temperature"], 0.2);
    assert!(result.document.agents().get("oracle").is_none());
    assert_eq!(
        result.document.categories()["quick"]["model"],
        "cliproxyapi/minimax-m2.7"
    );
    assert_eq!(result.document.categories()["quick"]["variant"], "balanced");
    assert!(result.document.categories().get("deep").is_none());
}

#[test]
fn projection_when_opencode_override_targets_existing_agent_preserves_unrelated_config_keys() {
    // Given: the current OpenCode fixture has schema, plugin, provider, and sibling agent data.
    let existing = OpenCodeDocument::parse_jsonc(&fixture("opencode/current-opencode.json"))
        .expect("Given: OpenCode fixture parses");
    let before_schema = existing.raw()["$schema"].clone();
    let before_plugin = existing.raw()["plugin"].clone();
    let before_provider = existing.raw()["provider"].clone();
    let before_karen = existing.agents()["karen"].clone();
    let before_description = existing.agents()["creative-ui-coder"]["description"].clone();
    let group = group(
        vec![],
        vec![],
        vec![override_row(
            "creative-ui-coder",
            "cliproxyapi/gpt-5.4-xhigh",
        )],
    );

    // When: Rust projects the OpenCode override.
    let result = OpenCodeProjectionService::project(&group, &existing);

    // Then: only the targeted agent model changes; unrelated top-level and sibling data survive.
    assert!(result.warnings.is_empty());
    assert_eq!(result.document.raw()["$schema"], before_schema);
    assert_eq!(result.document.raw()["plugin"], before_plugin);
    assert_eq!(result.document.raw()["provider"], before_provider);
    assert_eq!(result.document.agents()["karen"], before_karen);
    assert_eq!(
        result.document.agents()["creative-ui-coder"]["model"],
        "cliproxyapi/gpt-5.4-xhigh"
    );
    assert_eq!(
        result.document.agents()["creative-ui-coder"]["description"],
        before_description
    );
}

#[test]
fn projection_when_opencode_agent_section_is_missing_returns_warnings_without_rewriting_document() {
    // Given: compatibility behavior leaves a document unchanged when the top-level agent object is absent.
    let existing =
        OpenCodeDocument::parse_jsonc(r#"{ "$schema": "https://opencode.ai/config.json" }"#)
            .expect("Given: OpenCode document parses");
    let group = group(
        vec![],
        vec![],
        vec![
            override_row("creative-ui-coder", "cliproxyapi/gpt-5.4"),
            override_row("blank", "   "),
        ],
    );

    // When: Rust projects OpenCode overrides.
    let result = OpenCodeProjectionService::project(&group, &existing);

    // Then: only effective overrides warn and the document is unchanged.
    assert_eq!(result.document, existing);
    assert_eq!(
        result.warnings,
        vec![
            "OpenCode config has no valid top-level 'agent' object; skipped model override for 'creative-ui-coder'."
                .to_owned()
        ]
    );
}

#[test]
fn projection_when_opencode_agent_is_missing_warns_and_keeps_document_reparsable() {
    // Given: one effective override points at a missing OpenCode agent.
    let existing = OpenCodeDocument::parse_jsonc(&fixture("opencode/current-opencode.json"))
        .expect("Given: OpenCode fixture parses");
    let group = group(
        vec![],
        vec![],
        vec![override_row("missing-agent", "cliproxyapi/gpt-5.4")],
    );

    // When: Rust projects OpenCode overrides.
    let result = OpenCodeProjectionService::project(&group, &existing);

    // Then: compatibility behavior records a warning and leaves all agents structurally intact.
    assert_eq!(
        result.warnings,
        vec!["OpenCode agent 'missing-agent' was not found; skipped model override.".to_owned()]
    );
    let encoded = result
        .document
        .serialize()
        .expect("Then: projected document serializes");
    let reparsed: Value = serde_json::from_str(&encoded).expect("Then: output is valid JSON");
    assert!(reparsed["agent"].get("creative-ui-coder").is_some());
}
