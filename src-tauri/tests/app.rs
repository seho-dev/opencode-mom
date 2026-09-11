use std::fs;
use std::path::PathBuf;

use opencode_mom_tauri::agents::{AgentCreate, AgentMutation, AgentStorage};
use opencode_mom_tauri::document::write_file;
use opencode_mom_tauri::error::ErrorCode;
use opencode_mom_tauri::groups;
use opencode_mom_tauri::models::{
    AgentModelBinding, AppConfig, AppSelectionState, GroupType, ModelDef, ModelGroup,
    OmoCategoryMapping, ProviderDef, ProviderOptions,
};
use opencode_mom_tauri::paths::ConfigPaths;
use opencode_mom_tauri::providers;
use time::OffsetDateTime;
use uuid::Uuid;

struct TestHome {
    root: PathBuf,
    paths: ConfigPaths,
}

impl Drop for TestHome {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn temp_home(name: &str) -> TestHome {
    let root = std::env::temp_dir().join(format!(
        "opencode-mom-test-{name}-{}-{}",
        std::process::id(),
        Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&root).unwrap();
    let paths = ConfigPaths::for_home(&root);
    TestHome { root, paths }
}

fn provider(id: &str, models: &[&str]) -> ProviderDef {
    ProviderDef {
        name: id.to_owned(),
        npm: Some("@ai-sdk/openai-compatible".to_owned()),
        options: None,
        models: models
            .iter()
            .map(|model_id| ((*model_id).to_owned(), model_def(model_id)))
            .collect(),
    }
}

fn model_def(id: &str) -> ModelDef {
    serde_json::from_value(serde_json::json!({ "id": id })).unwrap()
}

fn binding(agent: &str, model: &str) -> AgentModelBinding {
    AgentModelBinding {
        agent_name: agent.to_owned(),
        model_ref: model.to_owned(),
        variant: None,
    }
}

fn group(group_type: GroupType, name: &str) -> ModelGroup {
    ModelGroup {
        id: Uuid::new_v4(),
        name: name.to_owned(),
        description: String::new(),
        group_type,
        open_code_agent_overrides: Vec::new(),
        slim_agent_overrides: None,
        omo_agent_overrides: None,
        omo_category_mappings: None,
        is_enabled: true,
        updated_at: OffsetDateTime::now_utc(),
    }
}

fn load_config(home: &TestHome) -> AppConfig {
    let content = fs::read_to_string(home.paths.config_file()).unwrap();
    serde_json::from_str(&content).unwrap()
}

fn write_opencode(home: &TestHome) {
    let path = home.paths.opencode_file();
    write_file(
        &path,
        br#"{
  "$schema": "https://opencode.ai/config.json",
  "agent": {
    "reviewer": { "model": "acme/old" }
  },
  "provider": {
    "acme": {
      "name": "Acme",
      "models": {
        "old": { "name": "Old" },
        "new": { "name": "New" },
        "fast": { "name": "Fast" },
        "slow": { "name": "Slow" }
      }
    }
  }
}
"#,
    )
    .unwrap();
}

#[test]
fn provider_crud_round_trip() {
    let home = temp_home("providers");
    let opencode = home.paths.opencode_file();

    let created =
        providers::create_provider(&opencode, provider("acme", &["fast", "slow"])).unwrap();
    assert_eq!(created.name, "acme");
    let listed = providers::list_providers(&opencode).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].models.len(), 2);

    // Duplicate create fails.
    let error = providers::create_provider(&opencode, provider("acme", &[])).unwrap_err();
    assert_eq!(error.code, ErrorCode::ValidationFailed);

    // Model CRUD.
    let mut model = model_def("turbo");
    model.name = Some("Turbo".to_owned());
    providers::create_model(&opencode, "acme", model).unwrap();
    let mut updated = model_def("turbo");
    updated.name = Some("Turbo v2".to_owned());
    providers::update_model(
        &opencode,
        &providers::ModelRef::new("acme", "turbo").unwrap(),
        updated,
    )
    .unwrap();
    let acme = providers::get_provider(&opencode, "acme").unwrap();
    assert_eq!(acme.models["turbo"].name.as_deref(), Some("Turbo v2"));

    providers::delete_model(
        &opencode,
        &providers::ModelRef::new("acme", "turbo").unwrap(),
    );
    assert!(providers::get_provider(&opencode, "acme")
        .unwrap()
        .models
        .get("turbo")
        .is_none());

    // Delete with models is blocked.
    let error = providers::delete_provider(&opencode, "acme").unwrap_err();
    assert_eq!(error.code, ErrorCode::ValidationFailed);

    // apiKey round-trips across IPC as-is (no redaction).
    let mut secret = provider("secretco", &[]);
    secret.options = Some(ProviderOptions {
        api_key: Some("super-secret".to_owned()),
        base_url: None,
        headers: None,
    });
    providers::create_provider(&opencode, secret).unwrap();
    assert_eq!(
        providers::get_provider(&opencode, "secretco")
            .unwrap()
            .options
            .unwrap()
            .api_key
            .as_deref(),
        Some("super-secret")
    );
    let mut rotated = providers::get_provider(&opencode, "secretco").unwrap();
    rotated.options.as_mut().unwrap().api_key = Some("new-secret".to_owned());
    providers::update_provider(&opencode, rotated).unwrap();
    assert_eq!(
        providers::get_provider(&opencode, "secretco")
            .unwrap()
            .options
            .unwrap()
            .api_key
            .as_deref(),
        Some("new-secret")
    );
}

#[test]
fn agent_inline_and_markdown_lifecycle() {
    let home = temp_home("agents");
    write_opencode(&home);

    // Inline create/update/delete through the OpenCode config file.
    let mut fields = serde_json::Map::new();
    fields.insert(
        "model".to_owned(),
        serde_json::Value::String("acme/fast".to_owned()),
    );
    let created = opencode_mom_tauri::agents::create(
        &home.paths,
        AgentCreate {
            id: "helper".to_owned(),
            storage: AgentStorage::Inline,
            fields: fields.clone(),
            prompt: Some("do things".to_owned()),
        },
    )
    .unwrap();
    assert_eq!(
        created.source,
        opencode_mom_tauri::agents::AgentSource::Inline
    );

    let listed = opencode_mom_tauri::agents::list(&home.paths).unwrap();
    assert!(listed.iter().any(|agent| agent.id == "helper"));

    opencode_mom_tauri::agents::update(
        &home.paths,
        "helper",
        AgentStorage::Inline,
        AgentMutation {
            fields: serde_json::Map::new(),
            prompt: Some("updated prompt".to_owned()),
            clear_fields: vec!["model".to_owned()],
        },
    )
    .unwrap();

    // Markdown create writes an agents/*.md file with frontmatter.
    let created_md = opencode_mom_tauri::agents::create(
        &home.paths,
        AgentCreate {
            id: "docs-writer".to_owned(),
            storage: AgentStorage::GlobalMarkdown,
            fields: fields.clone(),
            prompt: Some("write docs".to_owned()),
        },
    )
    .unwrap();
    let markdown_path = home.paths.global_agents_dir().join("docs-writer.md");
    assert!(markdown_path.exists());
    let raw = fs::read_to_string(&markdown_path).unwrap();
    assert!(raw.starts_with("---"));
    assert_eq!(
        created_md.source,
        opencode_mom_tauri::agents::AgentSource::Markdown
    );

    // Built-in deletion protection.
    let references = Default::default();
    let error =
        opencode_mom_tauri::agents::delete(&home.paths, "build", AgentStorage::Inline, &references)
            .unwrap_err();
    assert_eq!(error.code, ErrorCode::ReferencesBlocked);

    // Give the helper a second (markdown) source so deleting the inline one leaves the
    // markdown source as replacement.
    opencode_mom_tauri::agents::create(
        &home.paths,
        AgentCreate {
            id: "helper".to_owned(),
            storage: AgentStorage::GlobalMarkdown,
            fields: serde_json::Map::new(),
            prompt: Some("markdown prompt".to_owned()),
        },
    )
    .unwrap();
    let result = opencode_mom_tauri::agents::delete(
        &home.paths,
        "helper",
        AgentStorage::Inline,
        &references,
    )
    .unwrap();
    assert!(result.another_source_takes_over);
    let replacement = result.replacement.unwrap();
    assert_eq!(replacement.id, "helper");
    assert_eq!(
        replacement.source,
        opencode_mom_tauri::agents::AgentSource::Markdown
    );
}

#[test]
fn group_switch_projects_targets_and_state() {
    let home = temp_home("switch-slim");
    write_opencode(&home);

    let mut slim_group = group(GroupType::Slim, "work");
    slim_group.slim_agent_overrides = Some(vec![binding("reviewer", "acme/slow")]);
    let (saved, _) = groups::save_group(&home.paths, slim_group).unwrap();

    groups::switch_group(&home.paths, saved.id).unwrap();

    let config = load_config(&home);
    assert_eq!(config.state.selected_group_id, Some(saved.id));

    let slim_content = fs::read_to_string(home.paths.slim_file()).unwrap();
    assert!(slim_content.contains("\"preset\": \"work\""));
    assert!(slim_content.contains("acme/slow"));
}

#[test]
fn group_delete_clears_selected_omo_mappings() {
    let home = temp_home("delete-omo");
    write_opencode(&home);

    let mut omo_group = group(GroupType::OhMyOpenagent, "omo-main");
    omo_group.omo_agent_overrides = Some(vec![binding("planner", "acme/slow")]);
    omo_group.omo_category_mappings = Some(vec![OmoCategoryMapping {
        category_name: "coding".to_owned(),
        model_ref: "acme/fast".to_owned(),
        variant: None,
    }]);
    let (saved, _) = groups::save_group(&home.paths, omo_group).unwrap();
    groups::switch_group(&home.paths, saved.id).unwrap();

    let omo_path = home.paths.omo_file();
    assert!(omo_path.exists());
    let content = fs::read_to_string(&omo_path).unwrap();
    assert!(content.contains("acme/slow"));

    groups::delete_group(&home.paths, saved.id).unwrap();

    let config = load_config(&home);
    assert_eq!(config.state.selected_group_id, None);
    assert!(!config
        .groups
        .iter()
        .any(|candidate| candidate.id == saved.id));
    let content = fs::read_to_string(&omo_path).unwrap();
    assert!(!content.contains("acme/slow"));
}

#[test]
fn save_group_validates_model_references_and_names() {
    let home = temp_home("group-validation");
    write_opencode(&home);

    let mut invalid = group(GroupType::OpenCode, "bad-refs");
    invalid.open_code_agent_overrides = vec![binding("ghost", "missing/model")];
    let error = groups::save_group(&home.paths, invalid).unwrap_err();
    assert_eq!(error.code, ErrorCode::ValidationFailed);

    let first = group(GroupType::OpenCode, "dup");
    groups::save_group(&home.paths, first).unwrap();
    let second = group(GroupType::OpenCode, "DUP");
    let error = groups::save_group(&home.paths, second).unwrap_err();
    assert_eq!(error.code, ErrorCode::ValidationFailed);
}

#[test]
fn selection_state_survives_save_of_unselected_group() {
    let home = temp_home("state");
    let selected = group(GroupType::OpenCode, "selected");
    let (selected_saved, _) = groups::save_group(&home.paths, selected).unwrap();
    groups::switch_group(&home.paths, selected_saved.id).unwrap();

    let other = group(GroupType::Slim, "other");
    let (_, config) = groups::save_group(&home.paths, other).unwrap();
    assert_eq!(
        config.state,
        AppSelectionState {
            selected_group_id: Some(selected_saved.id),
            selected_group_name: Some("selected".to_owned()),
        }
    );
}
