use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use omo_switch_tauri::core::models::{
    AppSelectionState, LastSuccessfulWriteMetadata, ModelGroup, ModelGroupAgentOverride,
    ModelGroupCategoryMapping, OmoSwitchConfig, ProjectionIssueSummary,
};
use omo_switch_tauri::core::paths::{ConfigPaths, HomeEnv};
use omo_switch_tauri::core::repository::ConfigRepository;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

fn temp_home(label: &str) -> PathBuf {
    let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("task-6-temp")
        .join(format!("omo-switch-{label}-{}-{id}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("Given: temp home directory is created");
    path
}

fn remove_temp(path: &Path) {
    let _ = fs::remove_dir_all(path);
}

fn iso8601(value: &str) -> OffsetDateTime {
    OffsetDateTime::parse(value, &Rfc3339).expect("Given: ISO8601 date fixture parses")
}

#[test]
fn schemas_when_legacy_unknown_fields_are_read_without_becoming_state_projection() {
    // Given: a prior state payload retains an unknown feature field.
    let payload = r#"{"migrationVersion":1,"selectedGroupID":"4F648C2A-1D48-44D6-B0C0-4D340816E1F3","selectedGroupName":"Primary","removedFeature":{"enabled":true}}"#;

    // When: Rust decodes the persisted state.
    let state: AppSelectionState =
        serde_json::from_str(payload).expect("When: legacy state decodes");

    // Then: unknown fields are ignored and cannot be emitted by the retained state model.
    let encoded = serde_json::to_string(&state).expect("Then: retained state encodes");
    assert_eq!(state.migration_version, 1);
    assert_eq!(state.selected_group_name.as_deref(), Some("Primary"));
    assert!(!encoded.contains("removedFeature"));
}

#[test]
fn schemas_when_legacy_server_fields_are_read_do_not_become_app_state_projection() {
    let state: AppSelectionState = serde_json::from_str(
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
  "migrationVersion": 2
}"#,
    )
    .expect("When: legacy server state decodes");

    let encoded = serde_json::to_value(&state).expect("Then: retained state encodes");

    assert_eq!(state.selected_group_name.as_deref(), Some("Primary"));
    assert_eq!(state.migration_version, 2);
    assert!(encoded.get("launchAtLoginEnabled").is_none());
    assert!(encoded.get("openCodeServeConfig").is_none());
}

#[test]
fn schemas_when_config_round_trip_preserves_uuid_and_iso8601_dates() {
    // Given: persisted compatible config values with fixed UUID/date strings.
    let config_payload = r#"{
  "groups" : [
    {
      "agentOverrides" : [
        {
          "agentName" : "oracle",
          "modelRef" : "openai/gpt-5.4"
        }
      ],
      "categoryMappings" : [
        {
          "categoryName" : "quick",
          "modelRef" : "anthropic/claude-sonnet-4"
        }
      ],
      "description" : "Primary group",
      "id" : "4F648C2A-1D48-44D6-B0C0-4D340816E1F3",
      "isEnabled" : true,
      "name" : "Primary",
      "updatedAt" : "2023-11-14T22:13:20Z"
    }
  ],
  "state" : {
  "lastSuccessfulWrite" : {
    "backupPath" : "/tmp/backup.json",
    "target" : "oh-my-openagent.categories",
    "wroteAt" : "2023-11-14T22:21:40Z"
  },
  "migrationVersion" : 2,
  "selectedGroupID" : "4F648C2A-1D48-44D6-B0C0-4D340816E1F3",
  "selectedGroupName" : "Primary"
  }
}"#;

    // When: Rust decodes and re-encodes the persisted config schema.
    let config: OmoSwitchConfig =
        serde_json::from_str(config_payload).expect("When: config decodes");
    let encoded = serde_json::to_string_pretty(&config).expect("Then: config encodes");

    // Then: UUID and ISO8601 date strings survive round trip.
    assert_eq!(
        config.groups[0].id.to_string().to_uppercase(),
        "4F648C2A-1D48-44D6-B0C0-4D340816E1F3"
    );
    assert!(encoded.contains("2023-11-14T22:13:20Z"));
    assert_eq!(
        config
            .state
            .selected_group_id
            .expect("selected group id")
            .to_string()
            .to_uppercase(),
        "4F648C2A-1D48-44D6-B0C0-4D340816E1F3"
    );
    assert!(encoded.contains("2023-11-14T22:21:40Z"));
}

#[test]
fn config_paths_schemas_when_repository_saves_complete_config_under_temp_home() {
    // Given: Rust repositories are rooted at fake HOME-derived config paths.
    let home = temp_home("repository-save");
    let paths = ConfigPaths::from_home_env(HomeEnv::new(Some(home.as_path()), None))
        .expect("Given: HOME path should resolve");
    let repository = ConfigRepository::new(paths.config_file());
    let group_id = "4F648C2A-1D48-44D6-B0C0-4D340816E1F3"
        .parse()
        .expect("Given: UUID fixture parses");

    let group = ModelGroup {
        id: group_id,
        name: "Primary".to_owned(),
        description: Some("Primary group".to_owned()),
        category_mappings: vec![ModelGroupCategoryMapping {
            category_name: "quick".to_owned(),
            model_ref: "anthropic/claude-sonnet-4".to_owned(),
        }],
        agent_overrides: vec![ModelGroupAgentOverride {
            agent_name: "oracle".to_owned(),
            model_ref: "openai/gpt-5.4".to_owned(),
        }],
        open_code_agent_overrides: vec![],
        is_enabled: true,
        updated_at: iso8601("2023-11-14T22:13:20Z"),
    };
    let state = AppSelectionState {
        selected_group_id: Some(group_id),
        selected_group_name: Some("Primary".to_owned()),
        last_successful_write: Some(LastSuccessfulWriteMetadata {
            target: "oh-my-openagent.categories".to_owned(),
            wrote_at: iso8601("2023-11-14T22:21:40Z"),
            backup_path: Some("/tmp/backup.json".to_owned()),
        }),
        last_warning_summary: Some(ProjectionIssueSummary {
            message: "undeclared refs".to_owned(),
            count: 2,
        }),
        last_error_summary: Some(ProjectionIssueSummary {
            message: "write failed".to_owned(),
            count: 1,
        }),
        migration_version: AppSelectionState::CURRENT_SCHEMA_VERSION,
    };

    fs::create_dir_all(paths.omo_switch_dir()).expect("Given: repository directory exists");
    fs::write(paths.config_file(), b"old config bytes")
        .expect("Given: existing config bytes persist");

    // When: the repository saves and reloads its complete payload.
    let config = OmoSwitchConfig {
        groups: vec![group.clone()],
        state: state.clone(),
    };
    repository.save(&config).expect("When: config saves");
    println!(
        "repository config.json path: {}",
        paths.config_file().display()
    );

    // Then: observable file, load behavior, defaults, and path match the persisted contract.
    assert_eq!(repository.load().expect("Then: config loads"), config);
    assert!(paths.config_file().exists());
    let encoded = fs::read_to_string(paths.config_file()).expect("Then: config file reads");
    assert!(encoded.contains("\"groups\":"));
    assert!(encoded.contains("\"state\":"));
    assert!(encoded.contains("\"migrationVersion\": 2"));

    remove_temp(&home);
}
