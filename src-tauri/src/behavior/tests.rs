use super::*;
use std::fs;

fn update(document: &BehaviorDocument, content: &str) -> BehaviorUpdate {
    BehaviorUpdate {
        content: content.into(),
        expected_path: document.resolved_path.clone(),
        expected_content: document.exists.then(|| document.content.clone()),
    }
}

#[test]
fn missing_create_replace_empty_and_stale_snapshots() {
    let home = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(home.path());
    let missing = get(&paths).unwrap();
    assert!(!missing.exists);
    assert_eq!(missing.content, "");
    assert_eq!(missing.resolved_path, None);
    let wire = serde_json::to_value(&missing).unwrap();
    assert!(wire["resolvedPath"].is_null());
    let created = save(&paths, update(&missing, "# Rules\r\nUse 中文 $ARGUMENTS\n")).unwrap();
    assert_eq!(created.content, "# Rules\r\nUse 中文 $ARGUMENTS\n");
    assert!(save(&paths, update(&missing, "clobber")).is_err());
    assert!(save(
        &paths,
        BehaviorUpdate {
            content: "clobber".into(),
            expected_path: created.resolved_path.clone(),
            expected_content: None,
        }
    )
    .is_err());
    let empty = save(&paths, update(&created, "")).unwrap();
    assert!(empty.exists);
    assert_eq!(empty.content, "");
    assert!(save(&paths, update(&created, "stale")).is_err());
    assert_eq!(fs::read_to_string(&empty.path).unwrap(), "");
    assert!(!fs::read_dir(empty.path.parent().unwrap())
        .unwrap()
        .any(|entry| entry
            .unwrap()
            .path()
            .extension()
            .is_some_and(|extension| extension == "bak")));
}

#[test]
fn global_instructions_honor_config_directory_not_explicit_config_file_parent() {
    let home = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::with_resource_parts(
        home.path().into(),
        Some(home.path().join("elsewhere.jsonc")),
        Some(home.path().join("override")),
        home.path().join("xdg"),
        home.path().into(),
    );
    let missing = get(&paths).unwrap();
    assert_eq!(missing.path, home.path().join("override/AGENTS.md"));
    let saved = save(&paths, update(&missing, "rules")).unwrap();
    assert_eq!(saved.content, "rules");
    assert!(!home.path().join("xdg/opencode/AGENTS.md").exists());
    assert!(!paths.opencode_file().exists());

    let paths = ConfigPaths::with_resource_parts(
        home.path().into(),
        Some(home.path().join("custom/opencode.jsonc")),
        None,
        home.path().join("xdg"),
        home.path().into(),
    );
    let missing = get(&paths).unwrap();
    assert_eq!(missing.path, home.path().join("xdg/opencode/AGENTS.md"));
    assert!(!missing.exists);
    let saved = save(&paths, update(&missing, "xdg rules")).unwrap();
    assert_eq!(saved.content, "xdg rules");
    assert!(!home.path().join("custom/AGENTS.md").exists());
    assert!(!paths.opencode_file().exists());
}

#[test]
fn relative_config_directory_uses_captured_working_directory() {
    let home = tempfile::tempdir().unwrap();
    let working_directory = home.path().join("cwd");
    let paths = ConfigPaths::with_resource_parts(
        home.path().into(),
        Some(PathBuf::from("config/custom.jsonc")),
        Some(PathBuf::from("override")),
        home.path().join("xdg"),
        working_directory.clone(),
    );
    let missing = get(&paths).unwrap();
    assert_eq!(missing.path, working_directory.join("override/AGENTS.md"));
    let saved = save(&paths, update(&missing, "relative rules")).unwrap();
    assert_eq!(saved.content, "relative rules");
    assert!(!working_directory.join("config/AGENTS.md").exists());
    assert!(!home.path().join("xdg/opencode/AGENTS.md").exists());
}

#[test]
fn checked_creation_preserves_competing_file() {
    let home = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(home.path());
    let missing = get(&paths).unwrap();
    assert!(save_inner(&paths, update(&missing, "ours"), || {
        fs::write(&missing.path, "theirs").unwrap();
        Ok(())
    })
    .is_err());
    assert_eq!(fs::read_to_string(missing.path).unwrap(), "theirs");
}

#[cfg(unix)]
#[test]
fn symlink_target_mode_preserved_and_retarget_rejected() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let home = tempfile::tempdir().unwrap();
    let target = home.path().join("shared.md");
    fs::write(&target, "old").unwrap();
    fs::set_permissions(&target, fs::Permissions::from_mode(0o640)).unwrap();
    let logical = home.path().join(".config/opencode/AGENTS.md");
    fs::create_dir_all(logical.parent().unwrap()).unwrap();
    symlink(&target, &logical).unwrap();
    let paths = ConfigPaths::for_home(home.path());
    let document = get(&paths).unwrap();
    let saved = save(&paths, update(&document, "new")).unwrap();
    assert_eq!(fs::read_to_string(&target).unwrap(), "new");
    assert_eq!(
        fs::metadata(&target).unwrap().permissions().mode() & 0o777,
        0o640
    );
    assert!(fs::symlink_metadata(&logical)
        .unwrap()
        .file_type()
        .is_symlink());
    let other = home.path().join("other.md");
    fs::write(&other, "new").unwrap();
    fs::remove_file(&logical).unwrap();
    symlink(&other, &logical).unwrap();
    assert!(save(&paths, update(&saved, "clobber")).is_err());
    assert_eq!(fs::read_to_string(other).unwrap(), "new");
}
