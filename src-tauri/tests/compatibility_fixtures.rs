use std::fs;
use std::path::{Path, PathBuf};

use omo_switch_tauri::core::backup::BackupRepository;
use omo_switch_tauri::core::document::{OhMyOpenAgentDocument, OpenCodeDocument};
use omo_switch_tauri::core::models::OmoSwitchConfig;
use omo_switch_tauri::core::paths::{ConfigPaths, HomeEnv};
use omo_switch_tauri::core::repository::ConfigRepository;
use omo_switch_tauri::core::switching::recover_pending_transaction;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

fn fixture(relative_path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Given: src-tauri has workspace parent")
        .join("fixtures")
        .join("compatibility")
        .join(relative_path)
}

fn fixture_text(relative_path: &str) -> String {
    fs::read_to_string(fixture(relative_path)).expect("Given: compatibility fixture reads")
}

fn fixed_now() -> OffsetDateTime {
    OffsetDateTime::parse("2023-11-14T22:21:40Z", &Rfc3339)
        .expect("Given: fixed fixture timestamp parses")
}

#[test]
fn compatibility_fixtures_when_config_schema_loads_pin_current_defaults() {
    // Given: a current language-neutral single-file config payload.
    let config: OmoSwitchConfig =
        serde_json::from_str(&fixture_text("config/current.json")).expect("Given: config parses");

    // Then: UUID/date encodings and defaults remain compatible.
    let current_group = &config.groups[0];
    assert_eq!(current_group.name, "Primary");
    assert_eq!(
        current_group.updated_at,
        OffsetDateTime::parse("2023-11-14T22:13:20Z", &Rfc3339)
            .expect("Then: group timestamp parses")
    );
    assert_eq!(config.state.selected_group_name.as_deref(), Some("Primary"));
    assert!(
        serde_json::from_str::<OmoSwitchConfig>(&fixture_text("config/malformed.json")).is_err()
    );
}

#[test]
fn compatibility_fixtures_when_temp_home_is_resolved_pin_literal_config_paths() {
    // Given: a full fixture home and a conditional home without opencode.json.
    let full_home = fixture("temp-homes/full");
    let conditional_home = fixture("temp-homes/conditional-missing-opencode");

    // When: current path and repository seams load those homes.
    let full_paths = ConfigPaths::from_home_env(HomeEnv::new(Some(&full_home), None))
        .expect("When: full HOME resolves");
    let conditional_paths = ConfigPaths::from_home_env(HomeEnv::new(Some(&conditional_home), None))
        .expect("When: conditional HOME resolves");
    let config = ConfigRepository::new(full_paths.config_file())
        .load()
        .expect("When: config loads");

    // Then: paths stay under HOME/.config and conditional OpenCode absence is represented on disk.
    assert!(config.groups.is_empty());
    assert_eq!(config.state.migration_version, 2);
    assert!(full_paths.opencode_file().exists());
    assert!(conditional_paths.oh_my_openagent_file().exists());
    assert!(!conditional_paths.opencode_file().exists());
    assert!(full_paths
        .config_file()
        .ends_with(".config/omo-switch/config.json"));
}

#[test]
fn compatibility_fixtures_when_jsonc_is_parsed_pin_structural_projection_inputs() {
    // Given: comments, trailing commas, unrelated keys, and malformed target fixtures.
    let oh_my = OhMyOpenAgentDocument::parse_jsonc(&fixture_text("targets/oh-my-openagent.jsonc"))
        .expect("When: Oh My JSONC parses");
    let opencode = OpenCodeDocument::parse_jsonc(&fixture_text("targets/opencode.jsonc"))
        .expect("When: OpenCode JSONC parses");

    // When: malformed variants are parsed through the same document boundaries.
    let malformed_oh_my = OhMyOpenAgentDocument::parse_jsonc(&fixture_text(
        "targets/malformed-oh-my-openagent.jsonc",
    ));
    let malformed_opencode =
        OpenCodeDocument::parse_jsonc(&fixture_text("targets/malformed-opencode.jsonc"));
    let trailing_comma = OhMyOpenAgentDocument::parse_jsonc(&fixture_text(
        "targets/trailing-comma-oh-my-openagent.jsonc",
    ));

    // Then: valid structure is preserved and malformed inputs are rejected before writes.
    assert_eq!(oh_my.agents()["oracle"]["variant"], "xhigh");
    assert_eq!(oh_my.raw()["unrelated"]["preserve"], true);
    assert_eq!(opencode.agents()["reviewer"]["mode"], "primary");
    assert!(opencode.raw().get("provider").is_some());
    assert!(malformed_oh_my.is_err());
    assert!(malformed_opencode.is_err());
    assert!(trailing_comma.is_err());
}

#[test]
fn compatibility_fixtures_when_backup_directory_is_loaded_pin_retention_order() {
    // Given: six deterministic target-specific backup files.
    let fixture_root = fixture("");
    let repository = BackupRepository::new(fixture_root, fixed_now);

    // When: current backup metadata listing reads the committed directory.
    let backups = repository
        .list_backups(Some("oh-my-openagent"))
        .expect("When: backup fixtures list");

    // Then: metadata is newest-first and exposes all pre-cleanup retention inputs.
    assert_eq!(backups.len(), 6);
    assert!(backups[0]
        .file_path
        .ends_with("20231114T221325Z-oh-my-openagent.json"));
    assert!(backups[5]
        .file_path
        .ends_with("20231114T221320Z-oh-my-openagent.json"));
}

#[test]
fn transaction_guarantee_when_stale_artifacts_exist_recovers_on_startup() {
    let home = fixture("temp-homes/stale-transaction");
    let copy = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/task-7-fixture-stale");
    let _ = fs::remove_dir_all(&copy);
    copy_tree(&home, &copy);
    let root = copy.join(".config/omo-switch");
    let config_before = fs::read(root.join("config.json")).expect("Given: config fixture reads");
    let stale_paths = [
        root.join(".config.json.omo-txn-11111111-1111-1111-1111-111111111111.staged"),
        root.join(".transaction-journal.json.tmp"),
    ];
    assert!(stale_paths.iter().all(|path| path.exists()));

    recover_pending_transaction(&root).expect("When: stale artifacts recover on startup");

    assert!(stale_paths.iter().all(|path| !path.exists()));
    assert_eq!(
        fs::read(root.join("config.json")).expect("Then: config fixture reads"),
        config_before
    );
    let _ = fs::remove_dir_all(copy);
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("Given: fixture copy directory creates");
    for entry in fs::read_dir(source).expect("Given: fixture directory reads") {
        let entry = entry.expect("Given: fixture entry reads");
        let target = destination.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).expect("Given: fixture file copies");
        }
    }
}
