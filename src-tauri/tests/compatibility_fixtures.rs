use std::fs;
use std::path::{Path, PathBuf};

use omo_switch_tauri::core::backup::BackupRepository;
use omo_switch_tauri::core::document::{OhMyOpenAgentDocument, OpenCodeDocument};
use omo_switch_tauri::core::models::{AppSelectionState, ModelGroupStore};
use omo_switch_tauri::core::paths::{ConfigPaths, HomeEnv};
use omo_switch_tauri::core::repository::{AppStateRepository, ModelGroupRepository};
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
fn compatibility_fixtures_when_schemas_load_pin_current_and_legacy_defaults() {
    // Given: current and legacy language-neutral persisted payloads.
    let current_state_source = fixture_text("state/current.json");
    let legacy_server_state_source = r#"{
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
}"#;
    let current_groups: ModelGroupStore =
        serde_json::from_str(&fixture_text("groups/current.json")).expect("Given: groups parse");
    let legacy_groups: ModelGroupStore =
        serde_json::from_str(&fixture_text("groups/legacy-missing-fields.json"))
            .expect("Given: legacy groups parse");
    let current_state: AppSelectionState =
        serde_json::from_str(&current_state_source).expect("Given: state parses");
    let legacy_server_state: AppSelectionState = serde_json::from_str(legacy_server_state_source)
        .expect("Given: legacy server state parses");
    let legacy_state: AppSelectionState =
        serde_json::from_str(&fixture_text("state/legacy-missing-fields.json"))
            .expect("Given: legacy state parses");

    // When: values are observed through current Rust schema types.
    let current_group = &current_groups.groups[0];
    let legacy_group = &legacy_groups.groups[0];

    // Then: UUID/date encodings and missing-field defaults remain compatible.
    assert_eq!(current_group.name, "Primary");
    assert_eq!(
        current_group.updated_at,
        OffsetDateTime::parse("2023-11-14T22:13:20Z", &Rfc3339)
            .expect("Then: group timestamp parses")
    );
    assert!(legacy_group.open_code_agent_overrides.is_empty());
    assert!(legacy_group.is_enabled);
    assert_eq!(
        current_state.selected_group_name.as_deref(),
        Some("Primary")
    );
    let persisted_legacy_server_state: serde_json::Value =
        serde_json::from_str(legacy_server_state_source)
            .expect("Given: legacy server state JSON parses");
    assert_eq!(persisted_legacy_server_state["launchAtLoginEnabled"], true);
    assert_eq!(
        persisted_legacy_server_state["openCodeServeConfig"],
        serde_json::json!({
            "port": 4096,
            "hostname": "127.0.0.1",
            "mdns": false,
            "mdnsDomain": "opencode.local",
            "cors": [],
            "executablePath": null,
            "autoStart": false
        })
    );
    assert_eq!(legacy_state.migration_version, 1);
    let encoded_state = serde_json::to_value(&legacy_server_state).expect("Then: state encodes");
    assert_eq!(
        encoded_state["selectedGroupID"],
        "4f648c2a-1d48-44d6-b0c0-4d340816e1f3"
    );
    assert!(encoded_state.get("launchAtLoginEnabled").is_none());
    assert!(encoded_state.get("openCodeServeConfig").is_none());
    assert!(
        serde_json::from_str::<ModelGroupStore>(&fixture_text("groups/malformed.json")).is_err()
    );
    assert!(
        serde_json::from_str::<AppSelectionState>(&fixture_text("state/malformed.json")).is_err()
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
    let groups = ModelGroupRepository::new(full_paths.groups_file())
        .load()
        .expect("When: groups load");
    let state = AppStateRepository::new(full_paths.state_file())
        .load()
        .expect("When: state loads");

    // Then: paths stay under HOME/.config and conditional OpenCode absence is represented on disk.
    assert!(groups.is_empty());
    assert_eq!(state.migration_version, 2);
    assert!(full_paths.opencode_file().exists());
    assert!(conditional_paths.oh_my_openagent_file().exists());
    assert!(!conditional_paths.opencode_file().exists());
    assert!(full_paths
        .groups_file()
        .ends_with(".config/omo-switch/groups.json"));
    assert!(full_paths
        .state_file()
        .ends_with(".config/omo-switch/state.json"));
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
    let groups_before = fs::read(root.join("groups.json")).expect("Given: groups fixture reads");
    let state_before = fs::read(root.join("state.json")).expect("Given: state fixture reads");
    let stale_paths = [
        root.join(".groups.json.omo-txn-11111111-1111-1111-1111-111111111111.staged"),
        root.join(".state.json.omo-txn-11111111-1111-1111-1111-111111111111.original"),
        root.join(".transaction-journal.json.tmp"),
    ];
    assert!(stale_paths.iter().all(|path| path.exists()));

    recover_pending_transaction(&root).expect("When: stale artifacts recover on startup");

    assert!(stale_paths.iter().all(|path| !path.exists()));
    assert_eq!(
        fs::read(root.join("groups.json")).expect("Then: groups fixture reads"),
        groups_before
    );
    assert_eq!(
        fs::read(root.join("state.json")).expect("Then: state fixture reads"),
        state_before
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
