use super::*;
use serde_json::json;

#[test]
fn legacy_config_without_preferences_yields_defaults() {
    let config: AppConfig = serde_json::from_value(json!({
        "groups": [],
        "state": { "selectedGroupID": null, "selectedGroupName": null }
    }))
    .expect("legacy config should deserialize");
    assert_eq!(config.state.preferences, AppPreferences::default());
    assert_eq!(config.state.preferences.theme, ThemePreference::Dark);
    assert_eq!(config.state.preferences.locale, LocalePreference::En);
    assert!(!config.state.island_enabled);
    assert!(config.projects.is_empty());
    assert_eq!(config.state.selected_project_id, None);
}

#[test]
fn explicit_preferences_round_trip_unchanged() {
    let config: AppConfig = serde_json::from_value(json!({
        "groups": [],
        "state": {
            "selectedGroupID": null,
            "selectedGroupName": null,
            "preferences": { "theme": "light", "locale": "zh" }
        }
    }))
    .expect("config with preferences should deserialize");
    let encoded = serde_json::to_string(&config).expect("serialize config");
    let decoded: AppConfig = serde_json::from_str(&encoded).expect("deserialize config");
    assert_eq!(decoded, config);
    assert_eq!(decoded.state.preferences.theme, ThemePreference::Light);
    assert_eq!(decoded.state.preferences.locale, LocalePreference::Zh);
}

#[test]
fn default_config_serializes_preferences() {
    let value = serde_json::to_value(AppConfig::default()).expect("serialize default config");
    assert_eq!(
        value["state"]["preferences"],
        json!({ "theme": "dark", "locale": "en" })
    );
    assert_eq!(value["projects"], json!([]));
    assert_eq!(value["state"]["selectedProjectId"], Value::Null);
    assert_eq!(value["state"]["islandEnabled"], false);
}

#[test]
fn project_selection_roundtrip_preserves_frozen_group_schema() {
    let id = Uuid::new_v4();
    let config: AppConfig = serde_json::from_value(json!({
        "projects": [{"id": id, "path": "/project"}],
        "state": {"selectedGroupID": id, "selectedProjectId": id}
    }))
    .unwrap();
    let encoded = serde_json::to_value(&config).unwrap();
    assert_eq!(encoded["state"]["selectedGroupID"], json!(id));
    assert_eq!(encoded["state"]["selectedProjectId"], json!(id));
    assert!(encoded["projects"][0].get("name").is_none());
    assert_eq!(
        serde_json::from_value::<AppConfig>(encoded).unwrap(),
        config
    );
    assert!(serde_json::from_value::<AppConfig>(
        json!({"projects": [{"id":"invalid", "path":"/project"}]})
    )
    .is_err());
}

#[test]
fn empty_object_deserializes_to_defaults() {
    let config: AppConfig = serde_json::from_value(json!({})).expect("empty object");
    assert_eq!(config, AppConfig::default());
}

#[test]
fn island_flag_is_independent_of_preferences() {
    let mut config: AppConfig = serde_json::from_value(json!({
        "state": { "islandEnabled": true }
    }))
    .unwrap();
    config.state.preferences = AppPreferences {
        theme: ThemePreference::Light,
        locale: LocalePreference::Zh,
    };
    let encoded = serde_json::to_value(&config).unwrap();
    assert_eq!(encoded["state"]["islandEnabled"], true);
    assert!(encoded["state"]["preferences"]
        .get("islandEnabled")
        .is_none());
    assert!(
        serde_json::from_value::<AppConfig>(encoded)
            .unwrap()
            .state
            .island_enabled
    );
}

struct ConfigDirectory(tempfile::TempDir);

impl ConfigDirectory {
    fn new() -> Self {
        Self(tempfile::tempdir().unwrap())
    }

    fn path(&self) -> &Path {
        self.0.path()
    }
}

#[test]
fn save_config_creates_and_replaces_without_backups() {
    let directory = ConfigDirectory::new();
    let path = directory.path().join("nested/config.json");
    let mut config = AppConfig::default();
    save_config(&path, &config).unwrap();
    assert_eq!(load_config(&path).unwrap(), config);
    config.state.island_enabled = true;
    config.state.preferences.locale = LocalePreference::Zh;
    save_config(&path, &config).unwrap();
    assert_eq!(load_config(&path).unwrap(), config);
    assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
}

#[cfg(unix)]
#[test]
fn save_config_publishes_new_bytes_without_truncating_the_previous_file() {
    use std::io::Read;
    use std::os::unix::fs::PermissionsExt;
    let directory = ConfigDirectory::new();
    let path = directory.path().join("config.json");
    let original = serde_json::to_vec_pretty(&AppConfig::default()).unwrap();
    fs::write(&path, &original).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    let mut previous = fs::File::open(&path).unwrap();
    let mut changed = AppConfig::default();
    changed.state.island_enabled = true;
    save_config(&path, &changed).unwrap();
    let mut previous_bytes = Vec::new();
    previous.read_to_end(&mut previous_bytes).unwrap();
    assert_eq!(previous_bytes, original);
    assert_eq!(load_config(&path).unwrap(), changed);
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o640
    );
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[cfg(unix)]
#[test]
fn save_config_staging_failure_keeps_the_previous_bytes() {
    use std::os::unix::fs::PermissionsExt;
    if unsafe { libc::geteuid() } == 0 {
        return;
    }
    let directory = ConfigDirectory::new();
    let path = directory.path().join("config.json");
    let original = serde_json::to_vec_pretty(&AppConfig::default()).unwrap();
    fs::write(&path, &original).unwrap();
    fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o500)).unwrap();
    let mut changed = AppConfig::default();
    changed.state.island_enabled = true;
    let result = save_config(&path, &changed);
    fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(
        result.unwrap_err().code,
        crate::error::ErrorCode::ConfigurationFailed
    );
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
fn save_config_rejects_non_regular_destination_without_artifacts() {
    let directory = ConfigDirectory::new();
    let path = directory.path().join("config.json");
    fs::create_dir(&path).unwrap();
    fs::write(path.join("keep"), "retained").unwrap();
    assert_eq!(
        save_config(&path, &AppConfig::default()).unwrap_err().code,
        crate::error::ErrorCode::ConfigurationFailed
    );
    assert_eq!(fs::read_to_string(path.join("keep")).unwrap(), "retained");
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[cfg(unix)]
#[test]
fn save_config_keeps_existing_symlink_and_target_permissions() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let directory = ConfigDirectory::new();
    let target = directory.path().join("target/config.json");
    fs::create_dir(target.parent().unwrap()).unwrap();
    fs::write(&target, "{}").unwrap();
    fs::set_permissions(&target, fs::Permissions::from_mode(0o600)).unwrap();
    let alias = directory.path().join("config.json");
    symlink(&target, &alias).unwrap();
    let mut config = AppConfig::default();
    config.state.island_enabled = true;
    save_config(&alias, &config).unwrap();
    assert!(fs::symlink_metadata(&alias)
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(load_config(&target).unwrap(), config);
    assert_eq!(
        fs::metadata(&target).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
    assert_eq!(fs::read_dir(target.parent().unwrap()).unwrap().count(), 1);
}

#[cfg(unix)]
#[test]
fn save_config_partial_write_failure_preserves_original_bytes() {
    const CHILD_DIRECTORY: &str = "MOM_CONFIG_WRITE_FAULT_DIRECTORY";
    let Some(directory) = std::env::var_os(CHILD_DIRECTORY) else {
        let directory = ConfigDirectory::new();
        fs::write(directory.path().join("config.json"), "{}").unwrap();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "models::tests::save_config_partial_write_failure_preserves_original_bytes",
                "--nocapture",
            ])
            .env(CHILD_DIRECTORY, directory.path())
            .status()
            .unwrap();
        assert!(status.success());
        return;
    };
    let directory = std::path::PathBuf::from(directory);
    let path = directory.join("config.json");
    let original = fs::read(&path).unwrap();
    let limit = libc::rlimit {
        rlim_cur: 64,
        rlim_max: 64,
    };
    // The file-size fault is process-local; never constrain the parent test runner.
    assert_ne!(
        unsafe { libc::signal(libc::SIGXFSZ, libc::SIG_IGN) },
        libc::SIG_ERR
    );
    assert_eq!(unsafe { libc::setrlimit(libc::RLIMIT_FSIZE, &limit) }, 0);
    let error = save_config(&path, &AppConfig::default()).unwrap_err();
    assert_eq!(error.code, crate::error::ErrorCode::ConfigurationFailed);
    assert!(error.message.starts_with("write "));
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(fs::read_dir(directory).unwrap().count(), 1);
    // Control: a raw in-place write loses the original under the same fault.
    assert!(fs::write(
        &path,
        &serde_json::to_vec_pretty(&AppConfig::default()).unwrap()
    )
    .is_err());
    assert_ne!(fs::read(&path).unwrap(), original);
}
