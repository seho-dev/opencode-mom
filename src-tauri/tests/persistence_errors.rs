use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use omo_switch_tauri::core::document::OpenCodeDocument;
use omo_switch_tauri::core::error::{
    ConfigPathError, PersistenceOperation, RepositoryErrorCode, TargetConfigErrorCode,
};
use omo_switch_tauri::core::models::{
    AppSelectionState, LastSuccessfulWriteMetadata, ModelGroup, OmoSwitchConfig,
};
use omo_switch_tauri::core::paths::{ConfigPaths, HomeEnv};
use omo_switch_tauri::core::repository::{ConfigRepository, OpenCodeConfigRepository};

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

fn temp_home(label: &str) -> PathBuf {
    let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("task-6-temp")
        .join(format!("omo-switch-{label}-{}-{id}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("Given: isolated temp home exists");
    path
}

#[test]
fn persistence_error_when_home_variables_are_missing_is_stable_and_typed() {
    // Given: neither compatibility home variable is available.
    let home_env = HomeEnv::new(None, None);

    // When: compatibility paths are resolved.
    let error = ConfigPaths::from_home_env(home_env).expect_err("When: path resolution fails");

    // Then: callers receive the pinned owned error contract.
    assert_eq!(error, ConfigPathError::MissingHome);
    assert_eq!(error.code(), "missingHome");
    assert_eq!(error.detail(), "HOME or USERPROFILE must be set");
}

#[test]
fn persistence_error_when_config_is_malformed_includes_operation_and_path() {
    // Given: malformed config at the literal compatibility location.
    let home = temp_home("malformed-config");
    let paths = ConfigPaths::from_home_env(HomeEnv::new(Some(&home), None))
        .expect("Given: fake HOME resolves");
    fs::create_dir_all(paths.omo_switch_dir()).expect("Given: app config directory exists");
    fs::write(paths.config_file(), "{ malformed").expect("Given: malformed config writes");
    let repository = ConfigRepository::new(paths.config_file());

    // When: config is loaded through its repository boundary.
    let error = repository.load().expect_err("When: malformed config fails");

    // Then: the error identifies the stable class, operation, and exact path.
    assert_eq!(error.code(), RepositoryErrorCode::MalformedJson);
    assert_eq!(error.operation(), PersistenceOperation::Parse);
    assert_eq!(error.path(), paths.config_file());
    assert!(error.detail().contains("config.json"));

    fs::remove_dir_all(home).expect("Then: temp home is removed");
}

#[test]
fn persistence_error_when_target_jsonc_is_malformed_includes_operation_and_path() {
    // Given: malformed OpenCode JSONC at the literal compatibility location.
    let home = temp_home("malformed-target");
    let paths = ConfigPaths::from_home_env(HomeEnv::new(Some(&home), None))
        .expect("Given: fake HOME resolves");
    fs::create_dir_all(paths.opencode_dir()).expect("Given: target config directory exists");
    fs::write(paths.opencode_file(), "{ // malformed").expect("Given: malformed target writes");
    let repository = OpenCodeConfigRepository::new(paths.opencode_file());

    // When: target JSONC is loaded through its repository boundary.
    let error = repository.load().expect_err("When: malformed target fails");

    // Then: the target repository exposes the same stable context shape.
    assert_eq!(error.code(), TargetConfigErrorCode::MalformedConfig);
    assert_eq!(error.operation(), PersistenceOperation::Parse);
    assert_eq!(error.path(), paths.opencode_file());
    assert!(error.detail().contains("opencode.json"));

    fs::remove_dir_all(home).expect("Then: temp home is removed");
}

#[test]
fn persistence_error_when_parent_is_a_file_returns_stable_write_context() {
    // Given: a file occupies the directory required by config persistence.
    let home = temp_home("unwritable-parent");
    let blocked_parent = home.join("blocked");
    fs::write(&blocked_parent, "not a directory").expect("Given: blocking file exists");
    let config_file = blocked_parent.join("config.json");
    let repository = ConfigRepository::new(config_file.clone());

    // When: config persistence attempts to create the parent directory.
    let error = repository
        .save(&OmoSwitchConfig::default())
        .expect_err("When: config save fails");

    // Then: the failure is deterministic on Windows and Unix fake homes.
    assert_eq!(error.code(), RepositoryErrorCode::WriteFailed);
    assert_eq!(error.operation(), PersistenceOperation::CreateDirectory);
    assert_eq!(error.path(), blocked_parent);
    assert!(error.detail().contains("blocked"));

    fs::remove_dir_all(home).expect("Then: temp home is removed");
}

#[test]
fn persistence_error_when_target_parent_is_a_file_keeps_target_write_context() {
    let home = temp_home("unwritable-target-parent");
    let blocked_parent = home.join("blocked");
    fs::write(&blocked_parent, "not a directory").expect("Given: blocking file exists");
    let target_file = blocked_parent.join("opencode.json");
    let repository = OpenCodeConfigRepository::new(target_file);
    let document =
        OpenCodeDocument::parse_jsonc(r#"{"agent":{}}"#).expect("Given: target document parses");

    let error = repository
        .save(&document)
        .expect_err("When: target parent cannot be created");

    assert_eq!(error.code(), TargetConfigErrorCode::WriteFailed);
    assert_eq!(error.operation(), PersistenceOperation::CreateDirectory);
    assert_eq!(error.path(), blocked_parent);
    assert!(error.detail().contains("blocked"));

    fs::remove_dir_all(home).expect("Then: temp home is removed");
}

#[test]
fn persistence_error_when_config_serialization_fails_keeps_existing_bytes() {
    let home = temp_home("serialization-retention");
    let paths = ConfigPaths::from_home_env(HomeEnv::new(Some(&home), None))
        .expect("Given: fake HOME resolves");
    fs::create_dir_all(paths.omo_switch_dir()).expect("Given: app config directory exists");
    let config_bytes = b"original config bytes";
    fs::write(paths.config_file(), config_bytes).expect("Given: original config bytes persist");
    let repository = ConfigRepository::new(paths.config_file());
    let invalid_timestamp = time::Date::from_calendar_date(-1, time::Month::January, 1)
        .expect("Given: negative year is representable by the domain time type")
        .with_time(time::Time::MIDNIGHT)
        .assume_utc();
    let group = ModelGroup {
        id: uuid::Uuid::nil(),
        name: "Cannot serialize".to_owned(),
        description: None,
        category_mappings: Vec::new(),
        agent_overrides: Vec::new(),
        open_code_agent_overrides: Vec::new(),
        is_enabled: true,
        updated_at: invalid_timestamp,
    };
    let state_with_invalid_timestamp = AppSelectionState {
        last_successful_write: Some(LastSuccessfulWriteMetadata {
            target: "test".to_owned(),
            wrote_at: invalid_timestamp,
            backup_path: None,
        }),
        ..AppSelectionState::default()
    };

    let error = repository
        .save(&OmoSwitchConfig {
            groups: vec![group],
            state: state_with_invalid_timestamp,
        })
        .expect_err("When: an out-of-range config timestamp cannot serialize");

    assert_eq!(error.code(), RepositoryErrorCode::MalformedJson);
    assert_eq!(error.operation(), PersistenceOperation::Serialize);
    assert_eq!(error.path(), paths.config_file());
    assert_eq!(
        fs::read(paths.config_file()).expect("Then: original config bytes read"),
        config_bytes
    );

    fs::remove_dir_all(home).expect("Then: temp home is removed");
}
