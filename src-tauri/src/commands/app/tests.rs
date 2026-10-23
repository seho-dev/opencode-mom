use super::*;

#[test]
fn app_state_serializes_selected_group_id_as_camel_case_or_null() {
    for selected_group_id in [Some(Uuid::from_u128(1)), None] {
        let response = AppStateResponse {
            providers: Vec::new(),
            agents: Vec::new(),
            groups: Vec::new(),
            selected_group_id,
            projects: Vec::new(),
            selected_project_id: None,
            preferences: crate::models::AppPreferences::default(),
        };
        let value = serde_json::to_value(response).unwrap();
        assert_eq!(
            value.get("selectedGroupId"),
            Some(&serde_json::to_value(selected_group_id).unwrap())
        );
        assert!(value.get("selectedGroupID").is_none());
        assert!(value.get("selected_group_id").is_none());
        assert_eq!(value["projects"], serde_json::json!([]));
        assert_eq!(value["selectedProjectId"], serde_json::Value::Null);
    }
}

#[test]
fn add_project_ipc_rejects_global_overlaps_before_persistence_without_opening_picker() {
    use std::fs;
    use std::path::{Path, PathBuf};

    struct Home(PathBuf);
    impl Drop for Home {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    let home =
        Home(std::env::temp_dir().join(format!("opencode-mom-project-ipc-{}", Uuid::new_v4())));
    let global = home.0.join(".config/opencode");
    let override_root = home.0.join("override");
    let pinned_root = home.0.join("pinned");
    let valid = home.0.join("project");
    let other = home.0.join("other-project");
    for root in [&global, &override_root, &pinned_root, &valid, &other] {
        fs::create_dir_all(root).unwrap();
    }
    for root in [&global, &override_root, &pinned_root] {
        fs::write(root.join("opencode.jsonc"), "{}").unwrap();
    }
    let paths = ConfigPaths::with_resource_parts(
        home.0.clone(),
        Some(pinned_root.join("opencode.jsonc")),
        Some(override_root.clone()),
        home.0.join(".config"),
        home.0.join("cwd"),
    );
    // No dialog plugin or picker handler: only add_project is dispatched through mock IPC.
    let app = tauri::test::mock_builder()
        .manage(paths.clone())
        .invoke_handler(tauri::generate_handler![super::add_project])
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .unwrap();
    let invoke = |path: &Path| {
        tauri::test::get_ipc_response(
            &webview,
            tauri::webview::InvokeRequest {
                cmd: "add_project".into(),
                callback: tauri::ipc::CallbackFn(0),
                error: tauri::ipc::CallbackFn(1),
                url: "tauri://localhost".parse().unwrap(),
                body: tauri::ipc::InvokeBody::Json(serde_json::json!({"path": path})),
                headers: Default::default(),
                invoke_key: tauri::test::INVOKE_KEY.into(),
            },
        )
    };
    let invalid = vec![global, override_root, pinned_root];
    #[cfg(unix)]
    let invalid = {
        let alias = home.0.join("global-alias");
        std::os::unix::fs::symlink(&invalid[0], &alias).unwrap();
        let mut invalid = invalid;
        invalid.push(alias);
        invalid
    };
    for root in &invalid {
        let error = invoke(root).unwrap_err();
        assert_eq!(error["code"], "validation_failed", "{}", root.display());
        assert!(!paths.app_config_dir().exists());
    }
    let project = invoke(&valid)
        .unwrap()
        .deserialize::<models::ProjectFolder>()
        .unwrap();
    assert_eq!(
        project.path,
        fs::canonicalize(&valid).unwrap().to_str().unwrap()
    );
    let mut config = models::load_config(&paths.config_file()).unwrap();
    assert_eq!(config.projects.as_slice(), std::slice::from_ref(&project));
    assert_eq!(config.state.selected_project_id, Some(project.id));
    config.state.island_enabled = true;
    config.state.preferences.locale = models::LocalePreference::Zh;
    models::save_config(&paths.config_file(), &config).unwrap();
    let before = fs::read(paths.config_file()).unwrap();
    for root in &invalid {
        assert_eq!(invoke(root).unwrap_err()["code"], "validation_failed");
        assert_eq!(fs::read(paths.config_file()).unwrap(), before);
        assert_eq!(models::load_config(&paths.config_file()).unwrap(), config);
    }
    let duplicate = invoke(&valid.join("."))
        .unwrap()
        .deserialize::<models::ProjectFolder>()
        .unwrap();
    assert_eq!(duplicate, project);
    assert_eq!(fs::read(paths.config_file()).unwrap(), before);
    let added = invoke(&other)
        .unwrap()
        .deserialize::<models::ProjectFolder>()
        .unwrap();
    let after = models::load_config(&paths.config_file()).unwrap();
    assert_eq!(after.projects, [project, added.clone()]);
    assert_eq!(after.state.selected_project_id, Some(added.id));
    assert!(after.state.island_enabled);
    assert_eq!(after.state.preferences, config.state.preferences);
    for root in invalid.iter().take(3) {
        assert_eq!(
            fs::read_to_string(root.join("opencode.jsonc")).unwrap(),
            "{}"
        );
    }
}
