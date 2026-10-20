use std::fs;
use std::path::PathBuf;

use opencode_mom_tauri::agents::{AgentCreate, AgentMutation, AgentStorage};
use opencode_mom_tauri::document::write_file;
use opencode_mom_tauri::error::ErrorCode;
use opencode_mom_tauri::groups;
use opencode_mom_tauri::models::{
    AgentModelBinding, AppConfig, AppSelectionState, GroupType, ModelDef, ModelGroup,
    OmoCategoryMapping, ProviderDef, ProviderSettings,
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
        package: Some("aisdk:@ai-sdk/openai-compatible".to_owned()),
        settings: None,
        headers: None,
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
  "agents": {
    "reviewer": { "model": "acme/old" }
  },
  "providers": {
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
    // The model id is the source of truth for `name` after the provider refactor.
    assert_eq!(acme.models["turbo"].name.as_deref(), Some("turbo"));

    providers::delete_model(
        &opencode,
        &providers::ModelRef::new("acme", "turbo").unwrap(),
    )
    .unwrap();
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
    secret.settings = Some(ProviderSettings {
        api_key: Some("super-secret".to_owned()),
        base_url: None,
    });
    providers::create_provider(&opencode, secret).unwrap();
    assert_eq!(
        providers::get_provider(&opencode, "secretco")
            .unwrap()
            .settings
            .unwrap()
            .api_key
            .as_deref(),
        Some("super-secret")
    );
    let mut rotated = providers::get_provider(&opencode, "secretco").unwrap();
    rotated.settings.as_mut().unwrap().api_key = Some("new-secret".to_owned());
    providers::update_provider(&opencode, rotated).unwrap();
    assert_eq!(
        providers::get_provider(&opencode, "secretco")
            .unwrap()
            .settings
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
            expected_path: Some(created.inline.as_ref().unwrap().resolved_path.clone()),
            expected_content: Some(created.inline.as_ref().unwrap().raw.to_string()),
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
fn native_overrides_are_ignored_for_non_native_groups() {
    let home = temp_home("native-gate");
    write_opencode(&home);

    let mut slim_group = group(GroupType::Slim, "slim-with-native");
    slim_group.open_code_agent_overrides = vec![binding("reviewer", "acme/new")];
    slim_group.slim_agent_overrides = Some(vec![binding("reviewer", "acme/slow")]);
    let (saved, _) = groups::save_group(&home.paths, slim_group).unwrap();
    groups::switch_group(&home.paths, saved.id).unwrap();

    let content = fs::read_to_string(home.paths.opencode_file()).unwrap();
    assert!(content.contains("acme/old"));
    assert!(!content.contains("acme/new"));
}

#[test]
fn switching_type_away_from_native_removes_owned_overrides() {
    let home = temp_home("native-remove");
    write_opencode(&home);

    let mut native_group = group(GroupType::Native, "switch-me");
    native_group.open_code_agent_overrides = vec![binding("reviewer", "acme/new")];
    let (saved, _) = groups::save_group(&home.paths, native_group).unwrap();
    groups::switch_group(&home.paths, saved.id).unwrap();
    assert!(fs::read_to_string(home.paths.opencode_file())
        .unwrap()
        .contains("acme/new"));

    let mut changed = saved;
    changed.group_type = GroupType::Slim;
    groups::save_group(&home.paths, changed).unwrap();

    let content = fs::read_to_string(home.paths.opencode_file()).unwrap();
    assert!(!content.contains("acme/new"));
}

#[test]
fn group_delete_keeps_target_file_untouched() {
    let home = temp_home("delete-omo");
    write_opencode(&home);

    let mut omo_group = group(GroupType::Omo, "omo-main");
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
    // Deleting a group only touches opencode-mom's own config; target files are left as-is.
    let content = fs::read_to_string(&omo_path).unwrap();
    assert!(content.contains("acme/slow"));
}

#[test]
fn omo_switch_merges_and_preserves_residual_mappings() {
    let home = temp_home("omo-merge");
    write_opencode(&home);

    let omo_path = home.paths.omo_file();
    write_file(
        &omo_path,
        br#"{"$schema":"x","opencode":{"agents":{"existing":{"model":"acme/old"}},"categories":{"existing-cat":{"model":"acme/old"}}}}"#,
    )
    .unwrap();

    let mut omo_group = group(GroupType::Omo, "omo-main");
    omo_group.omo_agent_overrides = Some(vec![binding("planner", "acme/slow")]);
    omo_group.omo_category_mappings = Some(vec![OmoCategoryMapping {
        category_name: "coding".to_owned(),
        model_ref: "acme/fast".to_owned(),
        variant: None,
    }]);
    let (saved, _) = groups::save_group(&home.paths, omo_group).unwrap();
    groups::switch_group(&home.paths, saved.id).unwrap();

    let content = fs::read_to_string(&omo_path).unwrap();
    let value: serde_json::Value = serde_json::from_str(&content).unwrap();
    let agents = value["opencode"]["agents"].as_object().unwrap();
    assert_eq!(agents["existing"]["model"], "acme/old");
    assert_eq!(agents["planner"]["model"], "acme/slow");
    let categories = value["opencode"]["categories"].as_object().unwrap();
    assert_eq!(categories["existing-cat"]["model"], "acme/old");
    assert_eq!(categories["coding"]["model"], "acme/fast");
}

#[test]
fn slim_switch_merges_and_preserves_residual_entries() {
    let home = temp_home("slim-merge");
    write_opencode(&home);

    let slim_path = home.paths.slim_file();
    write_file(
        &slim_path,
        br#"{"$schema":"x","presets":{"work":{"other":{"model":"acme/old"}}}}"#,
    )
    .unwrap();

    let mut slim_group = group(GroupType::Slim, "work");
    slim_group.slim_agent_overrides = Some(vec![binding("reviewer", "acme/slow")]);
    let (saved, _) = groups::save_group(&home.paths, slim_group).unwrap();
    groups::switch_group(&home.paths, saved.id).unwrap();

    let content = fs::read_to_string(&slim_path).unwrap();
    let value: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(value["preset"], "work");
    let preset = value["presets"]["work"].as_object().unwrap();
    assert_eq!(preset["other"]["model"], "acme/old");
    assert_eq!(preset["reviewer"]["model"], "acme/slow");
}

#[test]
fn omo_delete_leaves_target_file_untouched() {
    let home = temp_home("omo-delete-residual");
    write_opencode(&home);

    let omo_path = home.paths.omo_file();
    write_file(
        &omo_path,
        br#"{"$schema":"x","opencode":{"agents":{"existing":{"model":"acme/old"}}}}"#,
    )
    .unwrap();

    let mut omo_group = group(GroupType::Omo, "omo-main");
    omo_group.omo_agent_overrides = Some(vec![binding("planner", "acme/slow")]);
    let (saved, _) = groups::save_group(&home.paths, omo_group).unwrap();
    groups::switch_group(&home.paths, saved.id).unwrap();

    groups::delete_group(&home.paths, saved.id).unwrap();

    let content = fs::read_to_string(&omo_path).unwrap();
    let value: serde_json::Value = serde_json::from_str(&content).unwrap();
    let agents = value["opencode"]["agents"].as_object().unwrap();
    assert_eq!(agents["existing"]["model"], "acme/old");
    assert_eq!(agents["planner"]["model"], "acme/slow");
}

#[test]
fn slim_delete_leaves_preset_untouched() {
    let home = temp_home("slim-delete");
    write_opencode(&home);

    let mut slim_group = group(GroupType::Slim, "work");
    slim_group.slim_agent_overrides = Some(vec![binding("reviewer", "acme/slow")]);
    let (saved, _) = groups::save_group(&home.paths, slim_group).unwrap();
    groups::switch_group(&home.paths, saved.id).unwrap();

    let slim_path = home.paths.slim_file();
    assert!(fs::read_to_string(&slim_path)
        .unwrap()
        .contains("acme/slow"));

    groups::delete_group(&home.paths, saved.id).unwrap();

    let content = fs::read_to_string(&slim_path).unwrap();
    assert!(content.contains("\"preset\": \"work\""));
    assert!(content.contains("acme/slow"));
}

#[test]
fn save_group_validates_model_references_and_names() {
    let home = temp_home("group-validation");
    write_opencode(&home);

    let mut invalid = group(GroupType::Native, "bad-refs");
    invalid.open_code_agent_overrides = vec![binding("ghost", "missing/model")];
    let error = groups::save_group(&home.paths, invalid).unwrap_err();
    assert_eq!(error.code, ErrorCode::ValidationFailed);

    let first = group(GroupType::Native, "dup");
    groups::save_group(&home.paths, first).unwrap();
    let second = group(GroupType::Native, "DUP");
    let error = groups::save_group(&home.paths, second).unwrap_err();
    assert_eq!(error.code, ErrorCode::ValidationFailed);
}

#[test]
fn native_overrides_join_variant_into_selector() {
    let home = temp_home("native-variant");
    write_opencode(&home);

    let mut native = group(GroupType::Native, "variant-group");
    let mut binding = binding("reviewer", "acme/new");
    binding.variant = Some("high".to_owned());
    native.open_code_agent_overrides = vec![binding];
    let (saved, _) = groups::save_group(&home.paths, native).unwrap();
    groups::switch_group(&home.paths, saved.id).unwrap();

    let content = fs::read_to_string(home.paths.opencode_file()).unwrap();
    let value: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(value["agents"]["reviewer"]["model"], "acme/new#high");
    assert!(value["agents"]["reviewer"].get("variant").is_none());
}

/// Read-only smoke test: parses the machine's real native V2 config when present.
/// Skipped on machines without `~/.config/opencode/opencode.json`.
#[test]
fn readonly_smoke_against_real_v2_config() {
    let Ok(home) = std::env::var("HOME") else {
        return;
    };
    let paths = ConfigPaths::for_home(std::path::Path::new(&home));
    let opencode = paths.opencode_file();
    if !opencode.exists() {
        return;
    }
    let providers = providers::list_providers(&opencode).expect("real V2 providers parse");
    assert!(!providers.is_empty());
    let agents = opencode_mom_tauri::agents::list(&paths).expect("real V2 agents parse");
    let references = opencode_mom_tauri::refs::collect_model_references(&paths)
        .expect("real V2 model references parse");
    let _ = (agents, references);
}

#[test]
fn selection_state_survives_save_of_unselected_group() {
    let home = temp_home("state");
    let selected = group(GroupType::Native, "selected");
    let (selected_saved, _) = groups::save_group(&home.paths, selected).unwrap();
    let mut enabled_config = load_config(&home);
    enabled_config.state.island_enabled = true;
    opencode_mom_tauri::models::save_config(&home.paths.config_file(), &enabled_config).unwrap();
    groups::switch_group(&home.paths, selected_saved.id).unwrap();

    let other = group(GroupType::Slim, "other");
    let (_, config) = groups::save_group(&home.paths, other).unwrap();
    assert_eq!(
        config.state,
        AppSelectionState {
            selected_group_id: Some(selected_saved.id),
            selected_group_name: Some("selected".to_owned()),
            selected_project_id: None,
            island_enabled: true,
            ..Default::default()
        }
    );
    let config = groups::delete_group(&home.paths, selected_saved.id).unwrap();
    assert!(config.state.island_enabled);
    assert!(config.state.selected_group_id.is_none());
}

#[test]
fn project_folder_lifecycle_preserves_files_and_normalizes_stale_selection() {
    use opencode_mom_tauri::models;
    let home = temp_home("projects");
    let root = home.root.join("project");
    fs::create_dir(&root).unwrap();
    let file = root.join("opencode.json");
    fs::write(&file, "{}").unwrap();
    let config_file = home.paths.config_file();
    let added = models::add_project(&config_file, root.to_str().unwrap()).unwrap();
    assert_eq!(
        added.path,
        fs::canonicalize(&root).unwrap().to_str().unwrap()
    );
    assert_eq!(
        models::add_project(&config_file, root.join(".").to_str().unwrap()).unwrap(),
        added
    );
    let config = load_config(&home);
    assert_eq!(config.projects, [added.clone()]);
    assert_eq!(config.state.selected_project_id, Some(added.id));
    models::select_project(&config_file, None).unwrap();
    assert_eq!(load_config(&home).state.selected_project_id, None);
    assert_eq!(
        models::add_project(&config_file, root.to_str().unwrap()).unwrap(),
        added
    );
    models::remove_project(&config_file, added.id).unwrap();
    assert!(root.is_dir());
    assert_eq!(fs::read_to_string(&file).unwrap(), "{}");
    assert!(load_config(&home).projects.is_empty());
    assert_eq!(load_config(&home).state.selected_project_id, None);
    let unknown = Uuid::new_v4();
    assert_eq!(
        models::select_project(&config_file, Some(unknown))
            .unwrap_err()
            .code,
        ErrorCode::NotFound
    );
    assert_eq!(
        models::remove_project(&config_file, unknown)
            .unwrap_err()
            .code,
        ErrorCode::NotFound
    );
    assert_eq!(
        home.paths.for_project(Some(unknown)).unwrap_err().code,
        ErrorCode::NotFound
    );
    for invalid in [PathBuf::from("relative"), home.root.join("missing"), file] {
        assert!(models::add_project(&config_file, invalid.to_str().unwrap()).is_err());
    }
    let mut config = load_config(&home);
    config.state.selected_project_id = Some(unknown);
    models::save_config(&config_file, &config).unwrap();
    assert_eq!(
        models::load_config(&config_file)
            .unwrap()
            .state
            .selected_project_id,
        None
    );
}

#[cfg(unix)]
#[test]
fn project_folder_symlink_dedup_unreadable_and_non_utf8_validation() {
    use opencode_mom_tauri::models;
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    use std::os::unix::fs::{symlink, PermissionsExt};
    let home = temp_home("project-alias");
    let root = home.root.join("project");
    fs::create_dir(&root).unwrap();
    let alias = home.root.join("alias");
    symlink(&root, &alias).unwrap();
    let added = models::add_project(&home.paths.config_file(), root.to_str().unwrap()).unwrap();
    assert_eq!(
        models::add_project(&home.paths.config_file(), alias.to_str().unwrap()).unwrap(),
        added
    );
    let scoped = home.paths.for_project(Some(added.id)).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o0)).unwrap();
    let privileged = unsafe { libc::geteuid() } == 0;
    let unreadable_add = models::add_project(&home.paths.config_file(), root.to_str().unwrap());
    let unreadable_read = opencode_mom_tauri::agents::list(&scoped);
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    if !privileged {
        assert!(unreadable_add.is_err());
        assert!(unreadable_read.is_err());
    }
    let invalid = home.root.join(OsString::from_vec(vec![0xff]));
    if fs::create_dir(&invalid).is_ok() {
        let invalid_alias = home.root.join("invalid-alias");
        symlink(&invalid, &invalid_alias).unwrap();
        assert_eq!(
            models::add_project(&home.paths.config_file(), invalid_alias.to_str().unwrap())
                .unwrap_err()
                .code,
            ErrorCode::ValidationFailed
        );
    }
    assert_eq!(load_config(&home).projects, [added]);
}

#[test]
fn group_lifecycle_preserves_selected_project_and_preferences() {
    use opencode_mom_tauri::models;
    let home = temp_home("group-project-selection");
    write_opencode(&home);
    let root = home.root.join("project");
    fs::create_dir(&root).unwrap();
    let project = models::add_project(&home.paths.config_file(), root.to_str().unwrap()).unwrap();
    let mut config = load_config(&home);
    config.state.preferences.locale = models::LocalePreference::Zh;
    models::save_config(&home.paths.config_file(), &config).unwrap();
    let (saved, _) = groups::save_group(&home.paths, group(GroupType::Native, "selected")).unwrap();
    groups::switch_group(&home.paths, saved.id).unwrap();
    groups::copy_group(&home.paths, saved.id, None).unwrap();
    groups::delete_group(&home.paths, saved.id).unwrap();
    let config = load_config(&home);
    assert_eq!(config.projects, [project.clone()]);
    assert_eq!(config.state.selected_project_id, Some(project.id));
    assert_eq!(
        config.state.preferences.locale,
        models::LocalePreference::Zh
    );
    assert_eq!(config.state.selected_group_id, None);
}

#[test]
fn global_agent_aliases_retain_paths_and_refuse_ambiguous_mutations() {
    use opencode_mom_tauri::agents;
    let home = temp_home("agent-aliases");
    let singular = home.root.join(".config/opencode/agent/nested/helper.md");
    write_file(&singular, b"---\nmodel: acme/old\n---\nsingular").unwrap();
    write_file(
        &home.root.join(".config/opencode/mode/ignored.md"),
        b"ignored",
    )
    .unwrap();
    write_file(
        &home.root.join(".config/opencode/modes/ignored.md"),
        b"ignored",
    )
    .unwrap();
    assert_eq!(agents::list(&home.paths).unwrap().len(), 1);
    assert_eq!(
        agents::collect_markdown_files_in_roots(&home.paths).unwrap(),
        [singular.clone()]
    );
    assert_eq!(agents::model_references(&home.paths).unwrap().len(), 1);
    agents::update(
        &home.paths,
        "nested/helper",
        AgentStorage::GlobalMarkdown,
        AgentMutation {
            prompt: Some("edited".to_owned()),
            expected_path: Some(fs::canonicalize(&singular).unwrap()),
            expected_content: Some(fs::read_to_string(&singular).unwrap()),
            ..Default::default()
        },
    )
    .unwrap();
    let plural = home.paths.global_agents_dir().join("nested/helper.md");
    assert!(!plural.exists());
    assert!(fs::read_to_string(&singular).unwrap().ends_with("edited"));
    write_file(&plural, b"plural").unwrap();
    let agent = agents::get(&home.paths, "nested/helper").unwrap();
    assert_eq!(agent.markdown.len(), 2);
    assert_eq!(agent.effective["system"], "plural");
    let before = fs::read(&singular).unwrap();
    let error = agents::update(
        &home.paths,
        "nested/helper",
        AgentStorage::GlobalMarkdown,
        AgentMutation {
            expected_path: Some(fs::canonicalize(&singular).unwrap()),
            expected_content: Some(fs::read_to_string(&singular).unwrap()),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::ValidationFailed);
    assert!(error.message.contains("multiple Markdown paths"));
    assert!(agents::delete(
        &home.paths,
        "nested/helper",
        AgentStorage::GlobalMarkdown,
        &Default::default()
    )
    .is_err());
    assert!(agents::create(
        &home.paths,
        AgentCreate {
            id: "nested/helper".to_owned(),
            storage: AgentStorage::GlobalMarkdown,
            fields: Default::default(),
            prompt: None
        }
    )
    .is_err());
    assert_eq!(fs::read(&singular).unwrap(), before);
    assert_eq!(fs::read_to_string(&plural).unwrap(), "plural");
    fs::remove_file(&plural).unwrap();
    agents::delete(
        &home.paths,
        "nested/helper",
        AgentStorage::GlobalMarkdown,
        &Default::default(),
    )
    .unwrap();
    assert!(!singular.exists());
    assert!(!plural.exists());
}

#[cfg(unix)]
#[test]
fn agent_alias_symlinks_to_one_file_are_not_ambiguous() {
    use opencode_mom_tauri::agents;
    use std::os::unix::fs::symlink;
    let home = temp_home("agent-symlink-aliases");
    let singular = home.root.join(".config/opencode/agent/helper.md");
    write_file(&singular, b"body").unwrap();
    let plural = home.paths.global_agents_dir().join("helper.md");
    fs::create_dir_all(plural.parent().unwrap()).unwrap();
    symlink(&singular, &plural).unwrap();
    agents::update(
        &home.paths,
        "helper",
        AgentStorage::GlobalMarkdown,
        AgentMutation {
            prompt: Some("edited".to_owned()),
            expected_path: Some(fs::canonicalize(&singular).unwrap()),
            expected_content: Some(fs::read_to_string(&singular).unwrap()),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(fs::read_to_string(&singular).unwrap().ends_with("edited"));
    assert!(fs::symlink_metadata(plural)
        .unwrap()
        .file_type()
        .is_symlink());
}

#[cfg(unix)]
#[test]
fn project_scope_rejects_saved_root_retarget_but_ignores_added_alias_retarget() {
    use opencode_mom_tauri::{agents, mcp, models, skills};
    use std::os::unix::fs::symlink;
    let home = temp_home("project-root-retarget");
    let root = home.root.join("project");
    let target = home.root.join("target");
    for (directory, marker) in [(&root, "original"), (&target, "retargeted")] {
        write_file(
            &directory.join("opencode.json"),
            serde_json::json!({
                "agents": {"helper": {"description": marker}},
                "mcp": {"servers": {"server": {"type": "local", "command": [marker]}}}
            })
            .to_string()
            .as_bytes(),
        )
        .unwrap();
        write_file(
            &directory.join(".opencode/skills/Shared.md"),
            marker.as_bytes(),
        )
        .unwrap();
    }
    let alias = home.root.join("alias");
    symlink(&root, &alias).unwrap();
    let project = models::add_project(&home.paths.config_file(), alias.to_str().unwrap()).unwrap();
    assert_eq!(
        PathBuf::from(&project.path),
        fs::canonicalize(&root).unwrap()
    );
    let scoped = home.paths.for_project(Some(project.id)).unwrap();
    let config_before = fs::read(home.paths.config_file()).unwrap();
    fs::remove_file(&alias).unwrap();
    symlink(&target, &alias).unwrap();
    for paths in [&scoped, &home.paths.for_project(Some(project.id)).unwrap()] {
        assert_eq!(agents::list(paths).unwrap().len(), 1);
        assert_eq!(
            agents::get(paths, "helper").unwrap().effective["description"],
            "original"
        );
        assert_eq!(mcp::list(paths).unwrap().data.len(), 1);
        assert_eq!(mcp::get(paths, "server").unwrap().target, "original");
        assert_eq!(skills::list(paths).unwrap().data.len(), 1);
        assert_eq!(skills::get(paths, "Shared").unwrap().content, "original");
    }
    fs::rename(&root, home.root.join("moved-project")).unwrap();
    symlink(&target, &root).unwrap();
    assert_eq!(
        home.paths.for_project(Some(project.id)).unwrap_err().code,
        ErrorCode::ConfigurationFailed
    );
    for error in [
        agents::list(&scoped).unwrap_err(),
        agents::get(&scoped, "helper").unwrap_err(),
        mcp::list(&scoped).unwrap_err(),
        mcp::get(&scoped, "server").unwrap_err(),
        skills::list(&scoped).unwrap_err(),
        skills::get(&scoped, "Shared").unwrap_err(),
    ] {
        assert_eq!(error.code, ErrorCode::ConfigurationFailed);
    }
    assert_eq!(fs::read(home.paths.config_file()).unwrap(), config_before);
}

#[cfg(unix)]
#[test]
fn agent_delete_refuses_same_target_logical_aliases_in_both_directions() {
    use opencode_mom_tauri::agents;
    use std::os::unix::fs::symlink;
    for singular_is_link in [false, true] {
        let home = temp_home("agent-delete-aliases");
        let singular = home.root.join(".config/opencode/agent/helper.md");
        let plural = home.paths.global_agents_dir().join("helper.md");
        let (real, alias) = if singular_is_link {
            (&plural, &singular)
        } else {
            (&singular, &plural)
        };
        write_file(real, b"---\ndescription: keep\n---\noriginal body").unwrap();
        fs::create_dir_all(alias.parent().unwrap()).unwrap();
        symlink(real, alias).unwrap();
        let original = fs::read(real).unwrap();
        let original_link = fs::read_link(alias).unwrap();
        assert_eq!(
            agents::get(&home.paths, "helper").unwrap().markdown.len(),
            2
        );
        let error = agents::delete(
            &home.paths,
            "helper",
            AgentStorage::GlobalMarkdown,
            &Default::default(),
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::ValidationFailed);
        assert!(error.message.contains("multiple Markdown paths"));
        assert_eq!(fs::read(real).unwrap(), original);
        assert_eq!(fs::read(alias).unwrap(), original);
        assert_eq!(fs::read_link(alias).unwrap(), original_link);
        assert!(fs::symlink_metadata(alias)
            .unwrap()
            .file_type()
            .is_symlink());
        assert!(!fs::symlink_metadata(real).unwrap().file_type().is_symlink());
        assert_eq!(
            agents::get(&home.paths, "helper").unwrap().markdown.len(),
            2
        );
    }
}

#[test]
fn agent_update_rejects_stale_inline_and_markdown_drafts_without_writes() {
    use opencode_mom_tauri::agents;
    use serde_json::json;
    for storage in [AgentStorage::Inline, AgentStorage::GlobalMarkdown] {
        let home = temp_home("agent-stale-draft");
        let (path, original, changed, expected_content) = match storage {
            AgentStorage::Inline => (
                home.paths.opencode_file(),
                r#"{"agents":{"helper":{"description":"original"}},"unrelated":"keep"}"#,
                r#"{"agents":{"helper":{"description":"external edit"}},"unrelated":"keep"}"#,
                json!({"description": "original"}).to_string(),
            ),
            AgentStorage::GlobalMarkdown => (
                home.paths.global_agents_dir().join("helper.md"),
                "---\ndescription: original\n---\noriginal body",
                "---\ndescription: external edit\n---\nexternal body",
                "---\ndescription: original\n---\noriginal body".to_owned(),
            ),
        };
        write_file(&path, original.as_bytes()).unwrap();
        let mutation: AgentMutation = serde_json::from_value(json!({
            "fields": {"description": "draft"},
            "expectedPath": fs::canonicalize(&path).unwrap(),
            "expectedContent": expected_content,
        }))
        .unwrap();
        fs::write(&path, changed).unwrap();
        let error = agents::update(&home.paths, "helper", storage, mutation).unwrap_err();
        assert_eq!(error.code, ErrorCode::ConfigurationFailed);
        assert_eq!(fs::read_to_string(&path).unwrap(), changed);
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
    }
}

#[test]
fn agent_update_requires_expected_source_but_creation_stays_compatible() {
    use opencode_mom_tauri::agents;
    use serde_json::json;
    for storage in [AgentStorage::Inline, AgentStorage::GlobalMarkdown] {
        let home = temp_home("agent-required-snapshot");
        agents::create(
            &home.paths,
            AgentCreate {
                id: "helper".to_owned(),
                storage,
                fields: Default::default(),
                prompt: Some("keep".to_owned()),
            },
        )
        .unwrap();
        let path = match storage {
            AgentStorage::Inline => home.paths.opencode_file(),
            AgentStorage::GlobalMarkdown => home.paths.global_agents_dir().join("helper.md"),
        };
        let before = fs::read(&path).unwrap();
        for payload in [
            json!({}),
            json!({"expectedPath": fs::canonicalize(&path).unwrap()}),
            json!({"expectedContent": "keep"}),
            json!({"expectedPath": fs::canonicalize(&path).unwrap(), "expectedRaw": {"system": "keep"}}),
        ] {
            let mutation = serde_json::from_value(payload).unwrap();
            let error = agents::update(&home.paths, "helper", storage, mutation).unwrap_err();
            assert_eq!(error.code, ErrorCode::ValidationFailed);
            assert_eq!(fs::read(&path).unwrap(), before);
        }
        assert_eq!(
            agents::get(&home.paths, "helper").unwrap().effective["system"],
            "keep"
        );
    }
}

#[cfg(unix)]
#[test]
fn agent_update_rejects_identical_content_symlink_retarget_for_both_storages() {
    use opencode_mom_tauri::agents;
    use serde_json::json;
    use std::os::unix::fs::symlink;
    for storage in [AgentStorage::Inline, AgentStorage::GlobalMarkdown] {
        let home = temp_home("agent-draft-retarget");
        let (logical, content, expected_content) = match storage {
            AgentStorage::Inline => (
                home.paths.opencode_file(),
                r#"{"agents":{"helper":{"description":"original"}}}"#,
                json!({"description": "original"}).to_string(),
            ),
            AgentStorage::GlobalMarkdown => (
                home.paths.global_agents_dir().join("helper.md"),
                "---\ndescription: original\n---\nbody",
                "---\ndescription: original\n---\nbody".to_owned(),
            ),
        };
        let original = home.root.join("original");
        let outside = home.root.join("outside");
        fs::write(&original, content).unwrap();
        fs::write(&outside, content).unwrap();
        fs::create_dir_all(logical.parent().unwrap()).unwrap();
        symlink(&original, &logical).unwrap();
        let mutation: AgentMutation = serde_json::from_value(json!({
            "fields": {"description": "draft"},
            "expectedPath": fs::canonicalize(&original).unwrap(),
            "expectedContent": expected_content,
        }))
        .unwrap();
        fs::remove_file(&logical).unwrap();
        symlink(&outside, &logical).unwrap();
        let error = agents::update(&home.paths, "helper", storage, mutation).unwrap_err();
        assert_eq!(error.code, ErrorCode::ConfigurationFailed);
        assert_eq!(fs::read_to_string(&original).unwrap(), content);
        assert_eq!(fs::read_to_string(&outside).unwrap(), content);
        assert_eq!(fs::read_link(&logical).unwrap(), outside);
    }
}

#[cfg(unix)]
#[test]
fn agent_markdown_symlink_update_replaces_target_without_truncating_other_hardlinks() {
    use opencode_mom_tauri::agents;
    use serde_json::json;
    use std::os::unix::fs::{symlink, PermissionsExt};
    let home = temp_home("agent-atomic-symlink");
    let logical = home.paths.global_agents_dir().join("helper.md");
    let target = home.root.join("linked/helper.md");
    let held = home.root.join("held.md");
    let content = "---\ndescription: original\n---\nbody";
    write_file(&target, content.as_bytes()).unwrap();
    fs::set_permissions(&target, fs::Permissions::from_mode(0o640)).unwrap();
    fs::hard_link(&target, &held).unwrap();
    fs::create_dir_all(logical.parent().unwrap()).unwrap();
    symlink(&target, &logical).unwrap();
    let mutation: AgentMutation = serde_json::from_value(json!({
        "fields": {"description": "edited"},
        "expectedPath": fs::canonicalize(&target).unwrap(),
        "expectedContent": content,
    }))
    .unwrap();
    let saved = agents::update(
        &home.paths,
        "helper",
        AgentStorage::GlobalMarkdown,
        mutation,
    )
    .unwrap();
    assert_eq!(saved.effective["description"], "edited");
    assert_eq!(fs::read_to_string(&held).unwrap(), content);
    assert_eq!(fs::read_link(&logical).unwrap(), target);
    assert_eq!(
        fs::metadata(&target).unwrap().permissions().mode() & 0o777,
        0o640
    );
    assert_eq!(fs::read_dir(target.parent().unwrap()).unwrap().count(), 1);
}

#[test]
fn agent_inline_expected_content_allows_unrelated_config_edits_and_preserves_jsonc() {
    use opencode_mom_tauri::agents;
    use serde_json::json;
    let home = temp_home("agent-inline-unrelated");
    let path = home.paths.opencode_file();
    let original = "{\n// keep comment\n\"agents\":{\"helper\":{\"description\":\"original\",\"custom\":true}},\n\"other\":\"original\",\n}\n";
    write_file(&path, original.as_bytes()).unwrap();
    let loaded = agents::get(&home.paths, "helper").unwrap().inline.unwrap();
    fs::write(
        &path,
        original.replace("\"other\":\"original\"", "\"other\":\"external\""),
    )
    .unwrap();
    let mutation: AgentMutation = serde_json::from_value(json!({
        "fields": {"description": "draft"},
        "expectedPath": loaded.resolved_path,
        "expectedContent": loaded.raw.to_string(),
    }))
    .unwrap();
    let saved = agents::update(&home.paths, "helper", AgentStorage::Inline, mutation).unwrap();
    assert_eq!(saved.effective["description"], "draft");
    assert_eq!(saved.effective["custom"], true);
    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("// keep comment"));
    assert!(content.contains("\"other\":\"external\""));
    assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
}

#[test]
fn agent_ipc_forwards_expected_source_and_rejects_stale_payloads() {
    use opencode_mom_tauri::commands::{self, AgentDefinitionDto};
    use serde_json::json;
    use tauri::Manager;
    for storage in ["inline", "global_markdown"] {
        let home = temp_home("agent-ipc-snapshot");
        let path = if storage == "inline" {
            home.paths.opencode_file()
        } else {
            home.paths.global_agents_dir().join("helper.md")
        };
        let original = if storage == "inline" {
            r#"{"agents":{"helper":{"description":"original"}}}"#
        } else {
            "---\ndescription: original\n---\nbody"
        };
        write_file(&path, original.as_bytes()).unwrap();
        let app = tauri::test::mock_app();
        app.manage(home.paths.clone());
        let loaded =
            commands::get_agent(app.state::<ConfigPaths>(), "helper".to_owned(), None).unwrap();
        let preview = &loaded.sources[0];
        assert_eq!(preview.path.as_deref(), path.to_str());
        let resolved = fs::canonicalize(&path).unwrap();
        assert_eq!(preview.resolved_path, resolved.to_str().unwrap());
        assert_eq!(
            preview.content,
            if storage == "inline" {
                preview.raw.to_string()
            } else {
                preview.raw.as_str().unwrap().to_owned()
            }
        );
        let draft: AgentDefinitionDto = serde_json::from_value(json!({
            "id": "helper", "source": loaded.source, "storage": storage,
            "mutation": {"fields": {"description": "draft"},
                "expectedPath": preview.resolved_path, "expectedContent": preview.content},
        }))
        .unwrap();
        let saved =
            commands::update_agent(app.state::<ConfigPaths>(), draft.clone(), None).unwrap();
        assert_eq!(saved.description.as_deref(), Some("draft"));
        let before = fs::read(&path).unwrap();
        let error = commands::update_agent(app.state::<ConfigPaths>(), draft, None).unwrap_err();
        assert_eq!(error.code, ErrorCode::ConfigurationFailed);
        assert_eq!(fs::read(&path).unwrap(), before);
    }
}

#[cfg(unix)]
#[test]
fn agent_markdown_delete_removes_only_the_leaf_symlink() {
    use opencode_mom_tauri::agents;
    use std::os::unix::fs::symlink;
    let home = temp_home("agent-delete-leaf-symlink");
    let logical = home.paths.global_agents_dir().join("helper.md");
    let target = home.root.join("outside.md");
    fs::write(&target, "keep external body").unwrap();
    fs::create_dir_all(logical.parent().unwrap()).unwrap();
    symlink(&target, &logical).unwrap();
    agents::delete(
        &home.paths,
        "helper",
        AgentStorage::GlobalMarkdown,
        &Default::default(),
    )
    .unwrap();
    assert!(fs::symlink_metadata(logical).is_err());
    assert_eq!(fs::read_to_string(target).unwrap(), "keep external body");
}

#[test]
fn agent_inline_create_rejects_invalid_config_filename_without_creating_directories() {
    let home = temp_home("agent-invalid-config-name");
    let missing = home.root.join("missing");
    let paths = ConfigPaths::with_parts(home.root.clone(), Some(missing.join("..")), None);
    let error = opencode_mom_tauri::agents::create(
        &paths,
        AgentCreate {
            id: "helper".to_owned(),
            storage: AgentStorage::Inline,
            fields: Default::default(),
            prompt: None,
        },
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::ValidationFailed);
    assert!(!missing.exists());
}

#[test]
fn agent_inline_javascript_roundtrip_preserves_numeric_snapshots_and_disk_fields() {
    use opencode_mom_tauri::commands::{self, AgentDefinitionDto};
    use std::io::Write;
    use std::process::{Command, Stdio};
    use tauri::Manager;

    let home = temp_home("agent-js-numeric-roundtrip");
    let path = home.paths.opencode_file();
    let original = r#"{
  // Numeric spelling and precision must survive an unrelated description edit.
  "agents": {"helper": {"description":"original","temperature":1.0,
    "nested":{"float":1e0,"large":9007199254740993},"array":[1.0,9007199254740995]}},
  "other":"keep"
}"#;
    write_file(&path, original.as_bytes()).unwrap();
    let app = tauri::test::mock_app();
    app.manage(home.paths.clone());
    let draft = |description: &str| {
        let loaded =
            commands::get_agent(app.state::<ConfigPaths>(), "helper".to_owned(), None).unwrap();
        let mut node = Command::new("node")
            .args([
                "--input-type=module",
                "-e",
                r#"import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
const agent = JSON.parse(readFileSync(0, 'utf8'));
const source = agent.sources[0];
assert.equal(JSON.stringify(source.raw.temperature), '1');
assert.equal(source.raw.nested.large, 9007199254740992);
assert.equal(typeof source.content, 'string');
assert.ok(source.content.includes('"temperature":1.0'));
assert.ok(source.content.includes('"large":9007199254740993'));
process.stdout.write(JSON.stringify({id:agent.id,source:agent.source,storage:'inline',
  mutation:{fields:{description:process.argv[1]},expectedPath:source.resolvedPath,expectedContent:source.content}}));"#,
                description,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("agent numeric roundtrip regression requires Node.js");
        node.stdin
            .take()
            .unwrap()
            .write_all(&serde_json::to_vec(&loaded).unwrap())
            .unwrap();
        let output = node.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "JavaScript roundtrip failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice::<AgentDefinitionDto>(&output.stdout).unwrap()
    };

    let saved = commands::update_agent(app.state::<ConfigPaths>(), draft("draft"), None).unwrap();
    assert_eq!(saved.description.as_deref(), Some("draft"));
    let saved_content = fs::read_to_string(&path).unwrap();
    assert!(saved_content.contains("\"temperature\":1.0"));
    assert!(saved_content.contains("\"float\":1e0"));
    assert!(saved_content.contains("\"large\":9007199254740993"));
    assert!(saved_content.contains("\"array\":[1.0,9007199254740995]"));

    for external in [
        saved_content.replace("9007199254740993", "9007199254740992"),
        saved_content.replace("\"temperature\":1.0", "\"temperature\":1"),
        saved_content.replace("\"description\":\"draft\"", "\"description\":\"external\""),
    ] {
        fs::write(&path, &saved_content).unwrap();
        let stale = draft("stale draft");
        fs::write(&path, &external).unwrap();
        let error = commands::update_agent(app.state::<ConfigPaths>(), stale, None).unwrap_err();
        assert_eq!(error.code, ErrorCode::ConfigurationFailed);
        assert_eq!(fs::read_to_string(&path).unwrap(), external);
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
    }

    fs::write(&path, &saved_content).unwrap();
    let unrelated = draft("second draft");
    fs::write(
        &path,
        saved_content.replace("\"other\":\"keep\"", "\"other\":\"external\""),
    )
    .unwrap();
    let saved = commands::update_agent(app.state::<ConfigPaths>(), unrelated, None).unwrap();
    assert_eq!(saved.description.as_deref(), Some("second draft"));
    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("\"other\":\"external\""));
    assert!(content.contains("\"temperature\":1.0"));
    assert!(content.contains("\"float\":1e0"));
    assert!(content.contains("\"large\":9007199254740993"));
    assert!(content.contains("\"array\":[1.0,9007199254740995]"));
    assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
}
