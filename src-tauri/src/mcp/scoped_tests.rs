use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};

use super::*;
use crate::error::ErrorCode;

fn project_paths() -> (tempfile::TempDir, ConfigPaths, ConfigPaths) {
    let home = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(home.path());
    let root = home.path().join("project");
    fs::create_dir(&root).unwrap();
    let project = crate::models::add_project(&paths.config_file(), root.to_str().unwrap()).unwrap();
    let scoped = paths.for_project(Some(project.id)).unwrap();
    (home, paths, scoped)
}

fn update_draft(server: &McpServer, config: Map<String, Value>) -> McpUpdate {
    McpUpdate {
        name: server.name.clone(),
        config,
        expected_config: server.config.clone(),
        expected_source_path: server.source_path.clone(),
        expected_source_snapshot: server.source_snapshot.clone(),
    }
}

fn toggle_draft(server: &McpServer, disabled: bool) -> McpDisabledUpdate {
    McpDisabledUpdate {
        name: server.name.clone(),
        disabled,
        expected_disabled: server.disabled,
        expected_source_path: server.source_path.clone(),
        expected_source_snapshot: server.source_snapshot.clone(),
    }
}

const ORIGINAL: &str = r#"{
  // outside comment
  "model": "keep",
  "mcp": { "servers": { "secret": {
    "type": "local",
    "command": ["run"], // target comment
    "disabled": false, // state comment
    "environment": { "TOKEN": "PRIVATE_TOKEN" },
    "future": { "exact": 9007199254740993 }
  } } }
}
"#;

fn delete_server(paths: &ConfigPaths, server: &McpServer) -> Result<(), AppError> {
    delete_project(
        paths,
        &server.name,
        &server.source_path,
        &server.source_snapshot,
    )
}

#[test]
fn project_edit_and_toggle_are_exact_root_and_leave_global_and_siblings_untouched() {
    let (_home, global, scoped) = project_paths();
    let root = scoped.project_directory().unwrap();
    fs::create_dir_all(global.opencode_file().parent().unwrap()).unwrap();
    fs::write(global.opencode_file(), ORIGINAL).unwrap();
    fs::write(root.join("opencode.jsonc"), ORIGINAL).unwrap();
    fs::create_dir(root.join("child")).unwrap();
    fs::write(root.join("child/opencode.jsonc"), ORIGINAL).unwrap();
    let original = get(&scoped, "secret").unwrap();
    assert_eq!(original.source_snapshot.len(), 64);
    let mut config = original.config.clone();
    config.insert("command".into(), serde_json::json!(["edited"]));
    let saved = update(&scoped, update_draft(&original, config)).unwrap();
    assert_eq!(get(&scoped, "secret").unwrap(), saved);
    let edited = fs::read_to_string(root.join("opencode.jsonc")).unwrap();
    assert!(edited.contains("// outside comment"));
    let disabled = set_disabled(&scoped, toggle_draft(&saved, true)).unwrap();
    assert!(disabled.disabled);
    assert_eq!(get(&scoped, "secret").unwrap(), disabled);
    assert_eq!(disabled.config["environment"]["TOKEN"], "PRIVATE_TOKEN");
    assert_eq!(disabled.config["future"]["exact"], 9007199254740993_u64);
    assert_eq!(
        fs::read_to_string(global.opencode_file()).unwrap(),
        ORIGINAL
    );
    assert_eq!(
        fs::read_to_string(root.join("child/opencode.jsonc")).unwrap(),
        ORIGINAL
    );
    assert!(!root.join("opencode.json").exists());
}

#[test]
fn global_toggle_changes_only_disabled_bytes_and_preserves_permissions() {
    let home = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(home.path());
    let path = paths.opencode_file();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, ORIGINAL).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    let current = get(&paths, "secret").unwrap();
    let saved = set_disabled(&paths, toggle_draft(&current, true)).unwrap();
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        ORIGINAL.replace("\"disabled\": false", "\"disabled\": true")
    );
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o640
    );
    assert_eq!(get(&paths, "secret").unwrap(), saved);
    assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
    set_disabled(&paths, toggle_draft(&saved, true)).unwrap();
    assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
    let enabled = set_disabled(&paths, toggle_draft(&saved, false)).unwrap();
    assert!(!enabled.disabled);
    assert_eq!(fs::read_to_string(&path).unwrap(), ORIGINAL);
}

#[test]
fn toggle_bootstraps_missing_disabled_and_rejects_stale_state_path_and_token() {
    let (_home, _global, scoped) = project_paths();
    let path = scoped.project_directory().unwrap().join("opencode.json");
    fs::write(
        &path,
        ORIGINAL.replace("    \"disabled\": false, // state comment\n", ""),
    )
    .unwrap();
    let current = get(&scoped, "secret").unwrap();
    for variant in 0..3 {
        let mut draft = toggle_draft(&current, true);
        match variant {
            0 => draft.expected_disabled = true,
            1 => draft.expected_source_path.push_str(".invalid"),
            _ => draft.expected_source_snapshot.clear(),
        }
        let before = fs::read(&path).unwrap();
        let error = set_disabled(&scoped, draft).unwrap_err();
        assert_eq!(error.code, ErrorCode::ConfigurationFailed);
        assert!(!error.to_string().contains("PRIVATE_TOKEN"));
        assert_eq!(fs::read(&path).unwrap(), before);
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
    }
    let saved = set_disabled(&scoped, toggle_draft(&current, true)).unwrap();
    assert_eq!(get(&scoped, "secret").unwrap(), saved);
    assert!(saved.disabled);
    assert_eq!(
        set_disabled(&scoped, toggle_draft(&current, true))
            .unwrap_err()
            .code,
        ErrorCode::ConfigurationFailed
    );
}

#[test]
fn project_sources_cannot_escape_or_write_other_resource_roots() {
    for target_kind in [
        "outside", "global", "agents", "skills", "commands", "plugins",
    ] {
        let (home, global, scoped) = project_paths();
        let root = scoped.project_directory().unwrap();
        let target = match target_kind {
            "outside" => home.path().join("outside.jsonc"),
            "global" => global.opencode_file(),
            directory => root.join(".opencode").join(directory).join("target.jsonc"),
        };
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, ORIGINAL).unwrap();
        symlink(&target, root.join("opencode.jsonc")).unwrap();
        let entries_before = fs::read_dir(target.parent().unwrap()).unwrap().count();
        if let Ok(server) = get(&scoped, "secret") {
            assert!(
                update(&scoped, update_draft(&server, server.config.clone())).is_err(),
                "{target_kind}"
            );
            assert!(
                set_disabled(&scoped, toggle_draft(&server, true)).is_err(),
                "{target_kind}"
            );
            assert!(delete_server(&scoped, &server).is_err(), "{target_kind}");
        }
        assert_eq!(fs::read_to_string(&target).unwrap(), ORIGINAL);
        assert_eq!(
            fs::read_dir(target.parent().unwrap()).unwrap().count(),
            entries_before
        );
    }
}

#[test]
fn project_config_directory_symlink_cannot_escape() {
    let (home, _global, scoped) = project_paths();
    let root = scoped.project_directory().unwrap();
    let outside = home.path().join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("opencode.jsonc"), ORIGINAL).unwrap();
    symlink(&outside, root.join(".opencode")).unwrap();
    let current = get(&scoped, "secret").unwrap();
    assert!(set_disabled(&scoped, toggle_draft(&current, true)).is_err());
    assert!(delete_server(&scoped, &current).is_err());
    assert_eq!(
        fs::read_to_string(outside.join("opencode.jsonc")).unwrap(),
        ORIGINAL
    );
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 1);
}

#[test]
fn project_aliases_preserve_symlinks_and_reject_same_content_retarget_after_list() {
    let (_home, _global, scoped) = project_paths();
    let root = scoped.project_directory().unwrap();
    let target = root.join("managed.jsonc");
    fs::write(&target, ORIGINAL).unwrap();
    let first = root.join("opencode.json");
    let second = root.join("opencode.jsonc");
    symlink(&target, &first).unwrap();
    symlink(&target, &second).unwrap();
    let current = get(&scoped, "secret").unwrap();
    let saved = set_disabled(&scoped, toggle_draft(&current, true)).unwrap();
    assert_eq!(get(&scoped, "secret").unwrap(), saved);
    assert_eq!(saved.source_paths.len(), 2);
    assert!(fs::symlink_metadata(&second)
        .unwrap()
        .file_type()
        .is_symlink());
    let other = root.join("other.jsonc");
    fs::write(&other, fs::read(&target).unwrap()).unwrap();
    for alias in [&first, &second] {
        fs::remove_file(alias).unwrap();
        symlink(&other, alias).unwrap();
    }
    let before = fs::read(&target).unwrap();
    assert_eq!(
        set_disabled(&scoped, toggle_draft(&saved, false))
            .unwrap_err()
            .code,
        ErrorCode::ConfigurationFailed
    );
    assert_eq!(fs::read(&other).unwrap(), before);
    assert_eq!(fs::read(&target).unwrap(), before);
}

#[test]
fn duplicate_project_definitions_and_global_toggles_are_rejected_without_writes() {
    let (_home, global, scoped) = project_paths();
    for paths in [&global, &scoped] {
        let files = &paths.global_config_files()[..2];
        fs::create_dir_all(files[0].parent().unwrap()).unwrap();
        for path in files {
            fs::write(path, ORIGINAL).unwrap();
        }
        let current = get(paths, "secret").unwrap();
        assert_eq!(
            set_disabled(paths, toggle_draft(&current, true))
                .unwrap_err()
                .code,
            ErrorCode::ValidationFailed
        );
        if paths.is_project() {
            assert_eq!(
                update(paths, update_draft(&current, current.config.clone()))
                    .unwrap_err()
                    .code,
                ErrorCode::ValidationFailed
            );
            assert_eq!(
                delete_server(paths, &current).unwrap_err().code,
                ErrorCode::ValidationFailed
            );
        }
        for path in files {
            assert_eq!(fs::read_to_string(path).unwrap(), ORIGINAL);
        }
        assert_eq!(fs::read_dir(files[0].parent().unwrap()).unwrap().count(), 2);
    }
}

#[test]
fn raw_source_changes_and_new_shadow_files_invalidate_snapshots() {
    let (_home, _global, scoped) = project_paths();
    let root = scoped.project_directory().unwrap();
    let path = root.join("opencode.jsonc");
    fs::write(&path, ORIGINAL).unwrap();
    let current = get(&scoped, "secret").unwrap();
    fs::write(&path, ORIGINAL.replace("outside comment", "external edit")).unwrap();
    assert_eq!(
        update(&scoped, update_draft(&current, current.config.clone()))
            .unwrap_err()
            .code,
        ErrorCode::ConfigurationFailed
    );
    let current = get(&scoped, "secret").unwrap();
    fs::write(root.join("opencode.json"), "{\"model\":\"new\"}").unwrap();
    assert_eq!(
        set_disabled(&scoped, toggle_draft(&current, true))
            .unwrap_err()
            .code,
        ErrorCode::ConfigurationFailed
    );
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        ORIGINAL.replace("outside comment", "external edit")
    );
}

#[test]
fn prepublication_project_alias_swap_is_rejected_before_staging() {
    let (_home, _global, scoped) = project_paths();
    let root = scoped.project_directory().unwrap();
    let path = root.join("opencode.jsonc");
    fs::write(&path, ORIGINAL).unwrap();
    let mut documents = managed_documents(&scoped).unwrap();
    let index = 1;
    documents[index]
        .2
        .patch(
            &["mcp", "servers", "secret", "disabled"],
            Some(Value::Bool(true)),
        )
        .unwrap();
    let other = root.join("other.jsonc");
    fs::write(&other, ORIGINAL).unwrap();
    fs::remove_file(&path).unwrap();
    symlink(&other, &path).unwrap();
    assert_eq!(
        persist_document(&scoped, &documents, index)
            .unwrap_err()
            .code,
        ErrorCode::ConfigurationFailed
    );
    assert_eq!(fs::read_to_string(&other).unwrap(), ORIGINAL);
    assert_eq!(fs::read_dir(root).unwrap().count(), 2);
}

#[test]
fn scoped_mcp_ipc_supports_project_update_and_disabled_only_toggle() {
    use tauri::Manager;

    let (_home, paths, scoped) = project_paths();
    let path = scoped.project_directory().unwrap().join("opencode.jsonc");
    fs::write(&path, ORIGINAL).unwrap();
    let project_id = crate::models::load_config(&paths.config_file())
        .unwrap()
        .projects[0]
        .id;
    let app = tauri::test::mock_app();
    app.manage(paths);
    let current = get(&scoped, "secret").unwrap();
    let saved = tauri::async_runtime::block_on(crate::commands::update_mcp(
        app.state(),
        update_draft(&current, current.config.clone()),
        Some(project_id),
    ))
    .unwrap();
    let after_update = fs::read_to_string(&path).unwrap();
    let disabled = tauri::async_runtime::block_on(crate::commands::set_mcp_disabled(
        app.state(),
        toggle_draft(&saved, true),
        Some(project_id),
    ))
    .unwrap();
    assert_eq!(get(&scoped, "secret").unwrap(), disabled);
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        after_update.replace("\"disabled\": false", "\"disabled\": true")
    );
    assert!(
        serde_json::from_value::<McpDisabledUpdate>(serde_json::json!({
            "name":"secret", "disabled":true, "expectedDisabled":false,
        "expectedSourcePath":"source", "expectedSourceSnapshot":"token",
            "config":{"command":["malicious"]}
        }))
        .is_err()
    );
}

#[test]
fn project_delete_removes_only_selected_definition_and_preserves_config_and_other_scopes() {
    let (_home, global, scoped) = project_paths();
    let root = scoped.project_directory().unwrap();
    fs::create_dir_all(global.opencode_file().parent().unwrap()).unwrap();
    fs::write(global.opencode_file(), ORIGINAL).unwrap();
    fs::create_dir(root.join(".opencode")).unwrap();
    let path = root.join(".opencode/opencode.jsonc");
    let content = ORIGINAL.replace(
        "\"secret\": {",
        "\"other\": {\"type\":\"local\",\"command\":[\"keep\"]}, \"secret\": {",
    );
    fs::write(&path, &content).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    let ancestor = root.join("opencode.jsonc");
    fs::write(&ancestor, "{\"model\":\"root\"}").unwrap();
    fs::create_dir(root.join("child")).unwrap();
    fs::write(root.join("child/opencode.jsonc"), ORIGINAL).unwrap();
    let current = get(&scoped, "secret").unwrap();
    delete_server(&scoped, &current).unwrap();
    assert_eq!(
        get(&scoped, "secret").unwrap_err().code,
        ErrorCode::NotFound
    );
    assert_eq!(get(&scoped, "other").unwrap().target, "keep");
    let saved = fs::read_to_string(&path).unwrap();
    assert!(saved.contains("// outside comment"));
    assert!(!saved.contains("PRIVATE_TOKEN"));
    assert_eq!(JsoncDoc::parse(&saved).unwrap().raw()["model"], "keep");
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o640
    );
    assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
    assert_eq!(
        fs::read_to_string(&ancestor).unwrap(),
        "{\"model\":\"root\"}"
    );
    assert_eq!(
        fs::read_to_string(root.join("child/opencode.jsonc")).unwrap(),
        ORIGINAL
    );
    assert_eq!(
        fs::read_to_string(global.opencode_file()).unwrap(),
        ORIGINAL
    );
    assert!(!root.join("opencode.json").exists());
}

#[test]
fn project_delete_preserves_shared_symlink_aliases_without_fallback() {
    let (_home, _global, scoped) = project_paths();
    let root = scoped.project_directory().unwrap();
    let target = root.join("managed.jsonc");
    fs::write(&target, ORIGINAL).unwrap();
    for name in ["opencode.json", "opencode.jsonc"] {
        symlink(&target, root.join(name)).unwrap();
    }
    let current = get(&scoped, "secret").unwrap();
    assert_eq!(current.source_paths.len(), 2);
    delete_server(&scoped, &current).unwrap();
    assert_eq!(
        get(&scoped, "secret").unwrap_err().code,
        ErrorCode::NotFound
    );
    for name in ["opencode.json", "opencode.jsonc"] {
        assert!(fs::symlink_metadata(root.join(name))
            .unwrap()
            .file_type()
            .is_symlink());
    }
    assert_eq!(fs::read_dir(root).unwrap().count(), 3);
    assert_eq!(
        JsoncDoc::parse(&fs::read_to_string(target).unwrap())
            .unwrap()
            .raw()["model"],
        "keep"
    );
}

#[test]
fn project_delete_rejects_wrong_source_scope_stale_content_and_missing_guards_without_writes() {
    use tauri::Manager;

    let (_home, global, scoped) = project_paths();
    let root = scoped.project_directory().unwrap();
    let path = root.join("opencode.jsonc");
    fs::write(&path, ORIGINAL).unwrap();
    let current = get(&scoped, "secret").unwrap();
    let project_id = crate::models::load_config(&global.config_file())
        .unwrap()
        .projects[0]
        .id;
    let app = tauri::test::mock_app();
    app.manage(global.clone());
    for (source, token, project_id, expected_code) in [
        (
            Some(current.source_path.clone()),
            None,
            Some(project_id),
            ErrorCode::ValidationFailed,
        ),
        (
            None,
            Some(current.source_snapshot.clone()),
            Some(project_id),
            ErrorCode::ValidationFailed,
        ),
        (
            Some(current.source_path.clone()),
            Some(String::new()),
            Some(project_id),
            ErrorCode::ConfigurationFailed,
        ),
        (
            Some(root.join("../outside.jsonc").display().to_string()),
            Some(current.source_snapshot.clone()),
            Some(project_id),
            ErrorCode::ConfigurationFailed,
        ),
        (
            Some(current.source_path.clone()),
            Some(current.source_snapshot.clone()),
            None,
            ErrorCode::ValidationFailed,
        ),
        (
            Some(current.source_path.clone()),
            Some(current.source_snapshot.clone()),
            Some(uuid::Uuid::new_v4()),
            ErrorCode::NotFound,
        ),
    ] {
        let error = tauri::async_runtime::block_on(crate::commands::delete_mcp(
            app.state(),
            current.name.clone(),
            project_id,
            source,
            token,
        ))
        .unwrap_err();
        assert_eq!(error.code, expected_code);
        assert!(!error.to_string().contains("PRIVATE_TOKEN"));
        assert_eq!(fs::read_to_string(&path).unwrap(), ORIGINAL);
        assert_eq!(fs::read_dir(root).unwrap().count(), 1);
    }
    assert_eq!(
        delete_server(&global, &current).unwrap_err().code,
        ErrorCode::ValidationFailed
    );
    fs::write(&path, ORIGINAL.replace("outside comment", "external edit")).unwrap();
    assert_eq!(
        delete_server(&scoped, &current).unwrap_err().code,
        ErrorCode::ConfigurationFailed
    );
    let current = get(&scoped, "secret").unwrap();
    tauri::async_runtime::block_on(crate::commands::delete_mcp(
        app.state(),
        current.name,
        Some(project_id),
        Some(current.source_path),
        Some(current.source_snapshot),
    ))
    .unwrap();
    assert!(path.is_file());
    assert_eq!(
        get(&scoped, "secret").unwrap_err().code,
        ErrorCode::NotFound
    );
    assert!(!global.opencode_file().exists());
}

#[test]
fn project_delete_rejects_concurrent_document_shadow_and_symlink_changes_before_staging() {
    for change in ["content", "shadow", "symlink"] {
        let (_home, _global, scoped) = project_paths();
        let root = scoped.project_directory().unwrap();
        let path = root.join("opencode.jsonc");
        fs::write(&path, ORIGINAL).unwrap();
        let current = get(&scoped, "secret").unwrap();
        let other = root.join(if change == "shadow" {
            "opencode.json"
        } else {
            "other.jsonc"
        });
        let changed = ORIGINAL.replace("outside comment", "external edit");
        let error = delete_project_inner(
            &scoped,
            &current.name,
            &current.source_path,
            &current.source_snapshot,
            || match change {
                "content" => fs::write(&path, &changed).unwrap(),
                "shadow" => fs::write(&other, ORIGINAL).unwrap(),
                _ => {
                    fs::write(&other, ORIGINAL).unwrap();
                    fs::remove_file(&path).unwrap();
                    symlink(&other, &path).unwrap();
                }
            },
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::ConfigurationFailed, "{change}");
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            if change == "content" {
                &changed
            } else {
                ORIGINAL
            }
        );
        if other.exists() {
            assert_eq!(fs::read_to_string(&other).unwrap(), ORIGINAL);
        }
        assert_eq!(
            fs::read_dir(root).unwrap().count(),
            if change == "content" { 1 } else { 2 }
        );
    }
}

#[test]
fn project_delete_duplicate_keys_reject_without_artifacts_or_exposing_secrets() {
    let config = r#"{"type":"local","command":["run"],"environment":{"TOKEN":"PRIVATE_TOKEN"}}"#;
    for content in [
        format!("{{\"mcp\":{{\"servers\":{{\"secret\":{config},\"secret\":{config}}}}}}}"),
        format!("{{\"mcp\":{{\"servers\":{{\"secret\":{config}}},\"servers\":{{\"secret\":{config}}}}}}}"),
        format!("{{\"mcp\":{{\"servers\":{{\"secret\":{config}}}}},\"mcp\":{{\"servers\":{{\"secret\":{config}}}}}}}"),
    ] {
        let (_home, _global, scoped) = project_paths();
        let root = scoped.project_directory().unwrap();
        let path = root.join("opencode.jsonc");
        fs::write(&path, &content).unwrap();
        let current = get(&scoped, "secret").unwrap();
        let error = delete_server(&scoped, &current).unwrap_err();
        assert_eq!(error.code, ErrorCode::ValidationFailed);
        assert!(!error.to_string().contains("PRIVATE_TOKEN"));
        assert_eq!(fs::read_to_string(&path).unwrap(), content);
        assert_eq!(fs::read_dir(root).unwrap().count(), 1);
    }
}
