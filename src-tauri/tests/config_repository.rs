use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use omo_switch_tauri::core::document::{OhMyOpenAgentDocument, OpenCodeDocument};
use omo_switch_tauri::core::paths::{ConfigPaths, HomeEnv};
use omo_switch_tauri::core::repository::{OhMyOpenAgentConfigRepository, OpenCodeConfigRepository};

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

fn temp_home(label: &str) -> PathBuf {
    let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("task-7-temp")
        .join(format!("omo-switch-{label}-{}-{id}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("Given: temp home directory is created");
    path
}

fn remove_temp(path: &Path) {
    let _ = fs::remove_dir_all(path);
}

fn config_paths(home: &Path) -> ConfigPaths {
    ConfigPaths::from_home_env(HomeEnv::new(Some(home), None)).expect("Given: HOME path resolves")
}

fn fixture(relative_path: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Given: src-tauri has workspace parent")
        .join("fixtures")
        .join("compatibility")
        .join(relative_path);
    fs::read_to_string(path).expect("Given: fixture reads")
}

#[test]
fn config_repository_when_oh_my_config_is_missing_bootstraps_valid_empty_document() {
    let home = temp_home("oh-my-missing");
    let paths = config_paths(&home);
    let repository = OhMyOpenAgentConfigRepository::new(paths.oh_my_openagent_file());

    let document = repository.load().expect("When: missing Oh My config loads");

    assert_eq!(document, OhMyOpenAgentDocument::bootstrap());
    assert!(document.agents().is_empty());
    assert!(document.categories().is_empty());
    println!(
        "missing Oh My bootstrap path: {}",
        paths.oh_my_openagent_file().display()
    );

    remove_temp(&home);
}

#[test]
fn config_repository_when_opencode_config_is_missing_returns_missing_file_error() {
    let home = temp_home("opencode-missing");
    let paths = config_paths(&home);
    let repository = OpenCodeConfigRepository::new(paths.opencode_file());

    let error = repository
        .load()
        .expect_err("When: missing OpenCode config fails");

    assert_eq!(error.code().as_str(), "fileNotFound");
    println!("missing OpenCode path: {}", paths.opencode_file().display());

    remove_temp(&home);
}

#[test]
fn config_repository_when_jsonc_config_exists_loads_comments_for_both_targets() {
    let home = temp_home("jsonc-load");
    let paths = config_paths(&home);
    fs::create_dir_all(paths.opencode_dir()).expect("Given: opencode dir exists");
    fs::write(
        paths.oh_my_openagent_file(),
        fixture("json/with-jsonc-markers.jsonc"),
    )
    .expect("Given: Oh My JSONC fixture writes");
    fs::write(
        paths.opencode_file(),
        r#"{
  // schema
  "$schema": "https://opencode.ai/config.json",
  /* agents */
  "agent": { "reviewer": { "model": "provider/model/ref" } },
  "provider": { "cliproxyapi": { "name": "CLIProxyAPI" } }
}"#,
    )
    .expect("Given: OpenCode JSONC fixture writes");
    let oh_my_repository = OhMyOpenAgentConfigRepository::new(paths.oh_my_openagent_file());
    let opencode_repository = OpenCodeConfigRepository::new(paths.opencode_file());

    let oh_my = oh_my_repository.load().expect("When: Oh My JSONC loads");
    let opencode = opencode_repository
        .load()
        .expect("When: OpenCode JSONC loads");

    assert!(oh_my.agents().contains_key("test"));
    assert!(oh_my.categories().contains_key("test"));
    assert!(opencode.agents().contains_key("reviewer"));

    remove_temp(&home);
}

#[test]
fn config_repository_when_config_is_malformed_returns_target_malformed_error_without_writing() {
    let home = temp_home("malformed");
    let paths = config_paths(&home);
    fs::create_dir_all(paths.opencode_dir()).expect("Given: opencode dir exists");
    fs::write(paths.oh_my_openagent_file(), "{ // malformed").expect("Given: Oh My writes");
    fs::write(paths.opencode_file(), "NOT VALID JSON {{{").expect("Given: OpenCode writes");
    let oh_my_repository = OhMyOpenAgentConfigRepository::new(paths.oh_my_openagent_file());
    let opencode_repository = OpenCodeConfigRepository::new(paths.opencode_file());

    let oh_my_error = oh_my_repository
        .load()
        .expect_err("When: malformed Oh My fails");
    let opencode_error = opencode_repository
        .load()
        .expect_err("When: malformed OpenCode fails");

    assert_eq!(oh_my_error.code().as_str(), "malformedConfig");
    assert_eq!(opencode_error.code().as_str(), "malformedConfig");
    assert_eq!(
        fs::read_to_string(paths.oh_my_openagent_file()).expect("Then: Oh My file still reads"),
        "{ // malformed"
    );

    remove_temp(&home);
}

#[test]
fn config_repository_when_saving_documents_preserves_unrelated_keys_and_reparsable_output() {
    let home = temp_home("save-round-trip");
    let paths = config_paths(&home);
    let oh_my_repository = OhMyOpenAgentConfigRepository::new(paths.oh_my_openagent_file());
    let opencode_repository = OpenCodeConfigRepository::new(paths.opencode_file());
    let oh_my = OhMyOpenAgentDocument::parse_jsonc(&fixture("json/with-unknown-fields.json"))
        .expect("Given: Oh My unknown-field fixture parses");
    let opencode = OpenCodeDocument::parse_jsonc(&fixture("opencode/current-opencode.json"))
        .expect("Given: OpenCode fixture parses");

    fs::create_dir_all(paths.opencode_dir()).expect("Given: target config directory exists");
    fs::write(paths.oh_my_openagent_file(), br#"{"legacy":true}"#)
        .expect("Given: existing Oh My bytes persist");
    fs::write(paths.opencode_file(), br#"{"legacy":true}"#)
        .expect("Given: existing OpenCode bytes persist");
    oh_my_repository.save(&oh_my).expect("When: Oh My saves");
    opencode_repository
        .save(&opencode)
        .expect("When: OpenCode saves");
    let oh_my_round_trip = oh_my_repository.load().expect("Then: Oh My reloads");
    let opencode_round_trip = opencode_repository.load().expect("Then: OpenCode reloads");

    assert_eq!(
        oh_my_round_trip.raw()["unknownField"],
        "should be preserved"
    );
    assert_eq!(oh_my_round_trip.raw()["someExtraData"]["key"], "value");
    assert!(opencode_round_trip.raw().get("plugin").is_some());
    assert!(opencode_round_trip.raw().get("provider").is_some());
    assert!(opencode_round_trip.agents().contains_key("karen"));
    assert!(!fs::read_to_string(paths.oh_my_openagent_file())
        .expect("Then: Oh My replacement bytes read")
        .contains("legacy"));
    assert!(!fs::read_to_string(paths.opencode_file())
        .expect("Then: OpenCode replacement bytes read")
        .contains("legacy"));

    remove_temp(&home);
}
