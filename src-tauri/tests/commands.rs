#[path = "common/switching.rs"]
mod switching;

use std::fs;

use omo_switch_tauri::application::{
    ApplicationError, GroupApplicationService, GroupSwitchOutcome,
};
use omo_switch_tauri::commands::AppStateResponse;
use omo_switch_tauri::core::models::ModelGroup;
use omo_switch_tauri::core::switching::SwitchError;
use switching::{
    category_row, group, override_row, paths, remove_temp, seed_groups, seed_oh_my, seed_opencode,
    seed_state, temp_home,
};
use uuid::Uuid;

#[test]
fn commands_when_loading_app_state_returns_groups_state_discovery_and_literal_config_paths() {
    let home = temp_home("commands-load-state");
    let target = dual_target_group();
    seed_groups(&home, &[target.clone()]);
    seed_state(&home, Some(target.id), Some("Dual Target"));
    seed_oh_my(&home);
    seed_opencode(&home);

    let backend = GroupApplicationService::for_home(&home)
        .expect("Given: command backend resolves fake HOME");
    let state = backend.load_app_state().expect("When: app state loads");

    assert_eq!(state.groups, vec![target]);
    assert_eq!(
        state.app_state.selected_group_name.as_deref(),
        Some("Dual Target")
    );
    assert!(state
        .discovered_open_code_agent_names
        .contains(&"Jenny".to_owned()));
    assert_eq!(state.open_code_agent_discovery_error, None);
    remove_temp(&home);
}

#[test]
fn commands_when_groups_are_saved_copied_and_deleted_persist_typed_results() {
    let home = temp_home("commands-group-crud");
    let target = dual_target_group();
    seed_groups(&home, &[target.clone()]);
    seed_state(&home, Some(target.id), Some("Dual Target"));
    seed_oh_my(&home);
    seed_opencode(&home);

    let backend = GroupApplicationService::for_home(&home)
        .expect("Given: command backend resolves fake HOME");
    let mut renamed = target.clone();
    renamed.name = "Renamed".to_owned();

    let saved = backend
        .save_group(renamed.clone())
        .expect("When: group saves");
    assert_eq!(saved.group.name, "Renamed");
    assert_eq!(saved.groups, vec![renamed.clone()]);

    let copied = backend.copy_group(renamed.id).expect("When: group copies");
    assert_eq!(copied.group.name, "Renamed Copy");
    assert_ne!(copied.group.id, renamed.id);
    assert_eq!(copied.groups.len(), 2);

    let remaining = backend
        .delete_group(renamed.id)
        .expect("When: active group deletes");
    assert_eq!(remaining.groups, vec![copied.group]);
    assert_eq!(remaining.app_state.selected_group_id, None);

    remove_temp(&home);
}

#[test]
fn commands_when_switching_missing_disabled_or_missing_opencode_returns_typed_errors() {
    let home = temp_home("commands-switch-errors");
    let enabled = dual_target_group();
    let disabled = group(
        "77777777-7777-4777-8777-777777777777",
        "Disabled",
        false,
        vec![category_row(
            "unspecified-high",
            "cliproxyapi/gpt-5.4-xhigh",
        )],
        Vec::new(),
        Vec::new(),
    );
    seed_groups(&home, &[enabled.clone(), disabled.clone()]);
    seed_state(&home, None, None);
    seed_oh_my(&home);

    let backend = GroupApplicationService::for_home(&home)
        .expect("Given: command backend resolves fake HOME");
    let missing = backend
        .switch_group(Uuid::nil())
        .expect_err("When: missing group switch fails");
    assert!(matches!(
        missing,
        ApplicationError::Switch(SwitchError::GroupNotFound)
    ));

    let disabled_error = backend
        .switch_group(disabled.id)
        .expect_err("When: disabled group switch fails");
    assert!(matches!(
        disabled_error,
        ApplicationError::Switch(SwitchError::GroupDisabled)
    ));

    let missing_opencode = backend
        .switch_group(enabled.id)
        .expect_err("When: OpenCode-required switch lacks opencode.json");
    assert!(matches!(
        missing_opencode,
        ApplicationError::Switch(SwitchError::LoadOpenCodeConfigFailed)
    ));

    remove_temp(&home);
}

#[test]
fn commands_when_switching_valid_group_persists_state_and_warnings() {
    let home = temp_home("commands-switch-success");
    let target = dual_target_group();
    seed_groups(&home, &[target.clone()]);
    seed_state(&home, None, None);
    seed_oh_my(&home);
    seed_opencode(&home);

    let backend = GroupApplicationService::for_home(&home)
        .expect("Given: command backend resolves fake HOME");
    let result = backend
        .switch_group(target.id)
        .expect("When: switch command runs");

    assert_eq!(result.outcome, GroupSwitchOutcome::Success);
    assert_eq!(result.app_state.selected_group_id, Some(target.id));
    assert!(result.warnings.is_empty());

    remove_temp(&home);
}

#[test]
fn commands_when_legacy_server_state_is_loaded_and_group_switched_drops_retired_fields() {
    let home = temp_home("commands-legacy-server-state");
    let target = dual_target_group();
    seed_groups(&home, &[target.clone()]);
    seed_oh_my(&home);
    seed_opencode(&home);
    let target_paths = paths(&home);
    fs::write(
        target_paths.state_file(),
        r#"{
  "selectedGroupID": "4f648c2a-1d48-44d6-b0c0-4d340816e1f3",
  "selectedGroupName": "Primary",
  "launchAtLoginEnabled": true,
  "openCodeServeConfig": {
    "port": 4096,
    "hostname": "127.0.0.1",
    "mdns": false,
    "mdnsDomain": "opencode.local",
    "cors": [],
    "executablePath": null,
    "autoStart": false
  },
  "lastSuccessfulWrite": {
    "target": "switch-group",
    "wroteAt": "2023-11-14T22:21:40Z",
    "backupPath": "legacy-backup.json"
  },
  "lastWarningSummary": null,
  "lastErrorSummary": null,
  "migrationVersion": 2
}"#,
    )
    .expect("Given: legacy state persists");
    let backend = GroupApplicationService::for_home(&home)
        .expect("Given: command backend resolves fake HOME");

    let loaded = backend
        .load_app_state()
        .expect("When: legacy app state loads");
    assert_eq!(loaded.groups, vec![target.clone()]);
    let response = serde_json::to_value(AppStateResponse {
        groups: loaded.groups,
        app_state: loaded.app_state,
        discovered_open_code_agent_names: loaded.discovered_open_code_agent_names,
        open_code_agent_discovery_error: loaded.open_code_agent_discovery_error,
    })
    .expect("Then: command response serializes");
    let response_state = response
        .get("appState")
        .expect("Then: command response includes app state");
    assert!(response_state.get("launchAtLoginEnabled").is_none());
    assert!(response_state.get("openCodeServeConfig").is_none());

    let switched = backend
        .switch_group(target.id)
        .expect("When: group switch persists the retained state");
    assert_eq!(switched.outcome, GroupSwitchOutcome::Success);
    let persisted_state: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(target_paths.state_file()).expect("Then: persisted state reads"),
    )
    .expect("Then: persisted state JSON parses");
    assert!(persisted_state.get("launchAtLoginEnabled").is_none());
    assert!(persisted_state.get("openCodeServeConfig").is_none());
    assert_eq!(persisted_state["selectedGroupID"], target.id.to_string());
    assert_eq!(persisted_state["selectedGroupName"], target.name);

    let reloaded = backend
        .load_app_state()
        .expect("Then: retained app state reloads");
    assert_eq!(reloaded.groups, vec![target.clone()]);
    assert_eq!(reloaded.app_state.selected_group_id, Some(target.id));
    assert_eq!(
        reloaded.app_state.selected_group_name.as_deref(),
        Some(target.name.as_str())
    );

    remove_temp(&home);
}

#[test]
fn commands_when_discovering_opencode_handles_success_missing_and_malformed_config() {
    let missing_home = temp_home("commands-discovery-missing");
    let backend =
        GroupApplicationService::for_home(&missing_home).expect("Given: missing backend resolves");
    let missing = backend.discover_open_code_agents(Vec::new());
    assert!(missing.agent_names.is_empty());
    assert_eq!(missing.error.as_deref(), Some("OpenCode config not found."));
    remove_temp(&missing_home);

    let malformed_home = temp_home("commands-discovery-malformed");
    let malformed_paths = paths(&malformed_home);
    fs::create_dir_all(malformed_paths.opencode_dir()).expect("Given: opencode dir exists");
    fs::write(malformed_paths.opencode_file(), "{ agent: ")
        .expect("Given: malformed config writes");
    let malformed_backend =
        GroupApplicationService::for_home(&malformed_home).expect("Given: backend resolves");
    let malformed =
        malformed_backend.discover_open_code_agents(vec![override_row("legacy", "model")]);
    assert_eq!(
        malformed.error.as_deref(),
        Some("OpenCode config is malformed.")
    );
    assert!(malformed.presentation.is_read_only);
    assert_eq!(
        malformed.presentation.preserved_overrides[0].agent_name,
        "legacy"
    );
    remove_temp(&malformed_home);

    let success_home = temp_home("commands-discovery-success");
    seed_opencode(&success_home);
    let success_backend =
        GroupApplicationService::for_home(&success_home).expect("Given: backend resolves");
    let success = success_backend.discover_open_code_agents(Vec::new());
    assert_eq!(success.error, None);
    assert!(success.agent_names.contains(&"Jenny".to_owned()));
    assert!(!success.presentation.is_read_only);
    remove_temp(&success_home);
}

fn dual_target_group() -> ModelGroup {
    group(
        "44444444-4444-4444-8444-444444444444",
        "Dual Target",
        true,
        vec![category_row(
            "unspecified-high",
            "cliproxyapi/gpt-5.4-xhigh",
        )],
        Vec::new(),
        vec![
            override_row("creative-ui-coder", "cliproxyapi/gpt-5.4"),
            override_row("Jenny", "cliproxyapi/gpt-5.4-xhigh"),
        ],
    )
}
