use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use omo_switch_tauri::core::paths::{ConfigPaths, HomeEnv};

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

#[test]
fn config_paths_when_home_is_available_resolve_exact_legacy_dot_config_locations() {
    // Given: a fake HOME directory controls path resolution.
    let home = temp_home("home-paths");

    // When: committed config paths are resolved.
    let paths = ConfigPaths::from_home_env(HomeEnv::new(Some(home.as_path()), None))
        .expect("When: HOME path should resolve");
    println!("HOME config root: {}", home.display());
    println!("groups.json path: {}", paths.groups_file().display());
    println!("state.json path: {}", paths.state_file().display());
    println!(
        "oh-my-openagent.json path: {}",
        paths.oh_my_openagent_file().display()
    );
    println!("opencode.json path: {}", paths.opencode_file().display());

    // Then: the backend preserves the documented ~/.config contract exactly.
    assert_eq!(
        paths.omo_switch_dir(),
        home.join(".config").join("omo-switch")
    );
    assert_eq!(
        paths.groups_file(),
        home.join(".config").join("omo-switch").join("groups.json")
    );
    assert_eq!(
        paths.state_file(),
        home.join(".config").join("omo-switch").join("state.json")
    );
    assert_eq!(
        paths.oh_my_openagent_file(),
        home.join(".config")
            .join("opencode")
            .join("oh-my-openagent.json")
    );
    assert_eq!(
        paths.opencode_file(),
        home.join(".config").join("opencode").join("opencode.json")
    );

    remove_temp(&home);
}

#[test]
fn config_paths_when_home_is_missing_use_userprofile_without_app_data_migration() {
    // Given: Windows-style fake USERPROFILE is the only home signal.
    let userprofile = temp_home("userprofile-paths");

    // When: committed config paths are resolved.
    let paths = ConfigPaths::from_home_env(HomeEnv::new(None, Some(userprofile.as_path())))
        .expect("When: USERPROFILE path should resolve");
    println!("USERPROFILE config root: {}", userprofile.display());
    println!(
        "USERPROFILE groups.json path: {}",
        paths.groups_file().display()
    );
    println!(
        "USERPROFILE state.json path: {}",
        paths.state_file().display()
    );

    // Then: even on Windows the app keeps literal ~/.config paths, not APPDATA.
    assert_eq!(
        paths.groups_file(),
        userprofile
            .join(".config")
            .join("omo-switch")
            .join("groups.json")
    );
    assert_eq!(
        paths.state_file(),
        userprofile
            .join(".config")
            .join("omo-switch")
            .join("state.json")
    );
    assert_eq!(
        paths.oh_my_openagent_file(),
        userprofile
            .join(".config")
            .join("opencode")
            .join("oh-my-openagent.json")
    );
    assert_eq!(
        paths.opencode_file(),
        userprofile
            .join(".config")
            .join("opencode")
            .join("opencode.json")
    );
    assert!(!paths.groups_file().to_string_lossy().contains("AppData"));

    remove_temp(&userprofile);
}

#[test]
fn source_when_committed_config_paths_are_authored_does_not_use_tauri_app_data_helpers() {
    // Given: committed config files must never move into Tauri app-data directories.
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut checked_files = Vec::new();
    for entry in fs::read_dir(source_root).expect("Given: src dir reads") {
        let entry = entry.expect("Given: src entry reads");
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) == Some("rs") {
            checked_files.push(path);
        }
    }

    // When: authored Rust source is scanned for app-data routing helpers.
    let forbidden = [
        "app_data",
        "appData",
        "app_config",
        "BaseDirectory::App",
        "app_local_data_dir",
    ];

    // Then: no committed config path implementation routes through platform app data.
    for file in checked_files {
        let source = fs::read_to_string(&file).expect("When: source file reads");
        for token in forbidden {
            assert!(
                !source.contains(token),
                "forbidden token {token} found in {}",
                file.display()
            );
        }
    }
}
