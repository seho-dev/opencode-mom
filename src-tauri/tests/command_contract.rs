#[path = "common/command_contract_sets.rs"]
mod command_contract_sets;
#[path = "common/switching.rs"]
mod switching;

use std::fs;
use std::path::Path;

use command_contract_sets::{exact_set_mismatch, runtime_commands, BACKEND_COMMANDS};
use omo_switch_tauri::application::{ApplicationError, GroupApplicationService};
use omo_switch_tauri::commands::AppStateResponse;
use omo_switch_tauri::core::models::ModelGroup;
use omo_switch_tauri::core::switching::SwitchError;
use switching::{
    category_row, group, override_row, paths, remove_temp, seed_groups, seed_opencode, seed_state,
    temp_home,
};

#[test]
fn commands_when_saving_duplicate_name_returns_stable_error() {
    let home = temp_home("commands-duplicate-name");
    let first = dual_target_group();
    let second = group(
        "55555555-5555-4555-8555-555555555555",
        "Second",
        true,
        Vec::new(),
        Vec::new(),
        Vec::new(),
    );
    seed_groups(&home, &[first.clone(), second.clone()]);
    seed_state(&home, None, None);
    let service = GroupApplicationService::for_home(&home).expect("Given: fake HOME resolves");
    let mut duplicate = second;
    duplicate.name = first.name;

    let error = service
        .save_group(duplicate)
        .expect_err("When: duplicate group name is saved");

    assert!(matches!(error, ApplicationError::DuplicateGroupName));
    remove_temp(&home);
}

#[test]
fn commands_when_active_group_save_reapply_fails_returns_transaction_error() {
    let home = temp_home("commands-active-save-reapply");
    let target = dual_target_group();
    seed_groups(&home, &[target.clone()]);
    seed_state(&home, Some(target.id), Some(&target.name));
    seed_opencode(&home);
    let target_paths = paths(&home);
    fs::create_dir_all(target_paths.opencode_dir()).expect("Given: target directory exists");
    fs::write(target_paths.oh_my_openagent_file(), "{ malformed: ")
        .expect("Given: active target is malformed");
    let service = GroupApplicationService::for_home(&home).expect("Given: fake HOME resolves");
    let mut edited = target;
    edited.description = Some("Requires active reapply".to_owned());

    let error = service
        .save_group(edited)
        .expect_err("When: active group save attempts transactional reapply");

    assert!(matches!(
        error,
        ApplicationError::Switch(SwitchError::LoadOhMyConfigFailed)
    ));
    remove_temp(&home);
}

#[test]
fn commands_when_serializing_app_state_does_not_expose_backend_paths() {
    let home = temp_home("commands-owned-dto");
    seed_groups(&home, &[]);
    seed_state(&home, None, None);
    let service = GroupApplicationService::for_home(&home).expect("Given: fake HOME resolves");

    let state = service
        .load_app_state()
        .expect("When: app state loads through application service");
    let serialized = serde_json::to_value(AppStateResponse {
        groups: state.groups,
        app_state: state.app_state,
        discovered_open_code_agent_names: state.discovered_open_code_agent_names,
        open_code_agent_discovery_error: state.open_code_agent_discovery_error,
    })
    .expect("Then: command response serializes");

    assert!(serialized.get("paths").is_none());
    remove_temp(&home);
}

#[test]
fn commands_runtime_handler_matches_approved_command_set() {
    let actual = runtime_commands(include_str!("../src/lib.rs"));

    if let Some(mismatch) = exact_set_mismatch("runtime handler", &actual, BACKEND_COMMANDS) {
        panic!("{mismatch}");
    }
}

#[test]
fn commands_exact_contract_rejects_unknown_and_missing_names() {
    let actual = vec![
        "load_app_state".to_owned(),
        "injected_unknown_command".to_owned(),
    ];

    let mismatch = exact_set_mismatch(
        "mutation probe",
        &actual,
        &["load_app_state", "switch_group"],
    )
    .expect("Then: an injected unknown command cannot produce a success result");

    assert!(mismatch.contains("missing=[\"switch_group\"]"));
    assert!(mismatch.contains("unexpected=[\"injected_unknown_command\"]"));
}

#[test]
fn commands_use_tauri_defaults_without_application_acl_configuration() {
    let tauri_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let tauri_config: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(tauri_root.join("tauri.conf.json"))
            .expect("Given: tauri.conf.json reads"),
    )
    .expect("Given: tauri.conf.json is valid JSON");

    assert_eq!(
        tauri_config
            .pointer("/app/windows")
            .and_then(serde_json::Value::as_array)
            .expect("tauri.conf.json must configure application windows")
            .iter()
            .filter_map(|window| window.get("label"))
            .filter_map(serde_json::Value::as_str)
            .collect::<Vec<_>>(),
        ["quick-switch", "settings"],
        "tauri.conf.json must configure exactly the quick-switch and settings window labels"
    );
    assert!(
        tauri_config.pointer("/app/security/csp").is_some(),
        "tauri.conf.json must retain its CSP"
    );
    assert!(
        tauri_config.pointer("/app/security/capabilities").is_none(),
        "tauri.conf.json must not configure application capabilities"
    );
    assert!(
        configuration_files(&tauri_root.join("capabilities"), "json").is_empty(),
        "application capability config files must not exist"
    );
    assert!(
        configuration_files(&tauri_root.join("permissions"), "toml").is_empty(),
        "application permission manifest/config files must not exist"
    );
}

fn configuration_files(directory: &Path, extension: &str) -> Vec<std::path::PathBuf> {
    if !directory.exists() {
        return Vec::new();
    }

    fs::read_dir(directory)
        .expect("Given: application ACL configuration directory reads")
        .map(|entry| {
            entry
                .expect("Given: application ACL configuration entry reads")
                .path()
        })
        .flat_map(|path| {
            if path.is_dir() {
                configuration_files(&path, extension)
            } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
                vec![path]
            } else {
                Vec::new()
            }
        })
        .collect()
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
