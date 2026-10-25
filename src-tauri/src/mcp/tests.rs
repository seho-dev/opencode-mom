use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};

use super::*;
use crate::error::ErrorCode;

#[test]
fn saved_server_rejects_malformed_ipc_targets_without_panicking() {
    for config in [
        serde_json::json!({}),
        serde_json::json!({"type": false}),
        serde_json::json!({"type": "unknown"}),
        serde_json::json!({"type": "local"}),
        serde_json::json!({"type": "local", "command": "run"}),
        serde_json::json!({"type": "local", "command": []}),
        serde_json::json!({"type": "local", "command": ["run", 1]}),
        serde_json::json!({"type": "remote"}),
        serde_json::json!({"type": "remote", "url": false}),
    ] {
        let error = saved_server(
            "test".to_owned(),
            config.as_object().unwrap().clone(),
            Vec::new(),
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::ValidationFailed);
    }
    for (config, target) in [
        (
            serde_json::json!({"type": "local", "command": ["run", "--test"]}),
            "run --test",
        ),
        (
            serde_json::json!({"type": "remote", "url": "https://example.com/mcp"}),
            "https://example.com/mcp",
        ),
    ] {
        let server = saved_server(
            "test".to_owned(),
            config.as_object().unwrap().clone(),
            vec!["config.jsonc".to_owned()],
        )
        .unwrap();
        assert_eq!(server.target, target);
        assert_eq!(server.source_path, "config.jsonc");
    }
}

#[test]
fn persist_document_rejects_invalid_indices_without_panicking() {
    assert_eq!(
        persist_document(&[], 0).unwrap_err().code,
        ErrorCode::ConfigurationFailed,
    );
    let home = tempfile::tempdir().unwrap();
    let path = home.path().join("config.jsonc");
    let documents = vec![(
        path.clone(),
        None,
        JsoncDoc::bootstrap(crate::document::OPENCODE_SCHEMA),
    )];
    assert_eq!(
        persist_document(&documents, 1).unwrap_err().code,
        ErrorCode::ConfigurationFailed,
    );
    assert!(!path.exists());
}

#[test]
fn create_rejects_non_utf8_config_filename_before_creating_directories() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let home = tempfile::tempdir().unwrap();
    let parent = home.path().join("missing-parent");
    let target = parent.join(OsString::from_vec(b"config-\xff.jsonc".to_vec()));
    let paths = ConfigPaths::with_resource_parts(
        home.path().to_owned(),
        Some(target.clone()),
        None,
        home.path().join(".config"),
        home.path().join("cwd"),
    );
    let error = create(
        &paths,
        McpDraft {
            name: "new".to_owned(),
            config: serde_json::from_value(serde_json::json!({
                "type": "local",
                "command": ["run"]
            }))
            .unwrap(),
        },
    )
    .unwrap_err();

    assert_eq!(error.code, ErrorCode::ValidationFailed);
    assert!(!target.exists());
    assert!(!parent.exists());
    assert_eq!(fs::read_dir(home.path()).unwrap().count(), 0);
}

#[test]
fn managed_document_error_retains_safe_jsonc_position() {
    let home = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(home.path());
    let path = paths.opencode_file();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        &path,
        "{\n  \"secret\": \"PRIVATE_TOKEN\",\n  \"mcp\" []\n}\n",
    )
    .unwrap();

    let error = managed_documents(&paths).err().unwrap();

    assert_eq!(error.code, ErrorCode::ValidationFailed);
    assert!(error.message.contains(&path.display().to_string()));
    assert!(error.message.contains("expected colon"), "{error}");
    assert!(error.message.contains("line 3"), "{error}");
    assert!(error.message.contains("column "), "{error}");
    assert!(!format!("{error:?}").contains("PRIVATE_TOKEN"));
}

const ORIGINAL: &str = r#"{"mcp":{"servers":{"secret":{"type":"local","command":["run"],"environment":{"TOKEN":"private"}}}},"model":"keep"}"#;

struct Fixture {
    home: PathBuf,
    aliases: Vec<PathBuf>,
    targets: Vec<PathBuf>,
}

impl Fixture {
    fn new() -> Self {
        let home =
            std::env::temp_dir().join(format!("opencode-mom-mcp-safety-{}", uuid::Uuid::new_v4()));
        let global = home.join(".config/opencode");
        fs::create_dir_all(&global).unwrap();
        let targets = vec![home.join("a.jsonc"), home.join("b.jsonc")];
        for target in &targets {
            fs::write(target, ORIGINAL).unwrap();
        }
        let aliases = vec![global.join("opencode.json"), global.join("opencode.jsonc")];
        for alias in &aliases {
            symlink(&targets[0], alias).unwrap();
        }
        Self {
            home,
            aliases,
            targets,
        }
    }

    fn retarget(&self, alias: &Path) {
        fs::remove_file(alias).unwrap();
        symlink(&self.targets[1], alias).unwrap();
    }

    fn assert_unchanged(&self) {
        for target in &self.targets {
            assert_eq!(fs::read_to_string(target).unwrap(), ORIGINAL);
        }
        assert!(!fs::read_dir(&self.home).unwrap().any(|entry| entry
            .unwrap()
            .path()
            .extension()
            .is_some_and(|extension| extension == "tmp")));
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.home);
    }
}

#[test]
fn alias_retarget_between_resolution_and_snapshot_read_is_rejected() {
    let fixture = Fixture::new();
    let mut retargeted = false;
    let error = delete_inner(
        &ConfigPaths::for_home(&fixture.home),
        "secret",
        |alias| {
            if alias == fixture.aliases[0] {
                fixture.retarget(alias);
                retargeted = true;
            }
        },
        |_| {},
    )
    .unwrap_err();
    assert!(retargeted);
    assert_eq!(error.code, ErrorCode::ConfigurationFailed);
    assert!(error.message.contains("changed during deletion"));
    fixture.assert_unchanged();
}

#[test]
fn duplicate_alias_retarget_after_snapshot_is_rejected_before_rename() {
    let fixture = Fixture::new();
    let mut retargeted = false;
    let error = delete_inner(
        &ConfigPaths::for_home(&fixture.home),
        "secret",
        |_| {},
        |_| {
            let staged = fs::read_dir(&fixture.home)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .filter(|path| {
                    path.extension()
                        .is_some_and(|extension| extension == "tmp" || extension == "bak")
                })
                .collect::<Vec<_>>();
            assert_eq!(staged.len(), 2);
            for path in staged {
                assert_eq!(
                    fs::metadata(path).unwrap().permissions().mode() & 0o777,
                    0o600
                );
            }
            fixture.retarget(&fixture.aliases[1]);
            retargeted = true;
        },
    )
    .unwrap_err();
    assert!(retargeted);
    assert_eq!(error.code, ErrorCode::ConfigurationFailed);
    assert!(error.message.contains("changed during deletion"));
    fixture.assert_unchanged();
}

#[test]
fn resolved_target_replaced_by_identical_content_symlink_is_rejected() {
    let fixture = Fixture::new();
    let error = delete_inner(
        &ConfigPaths::for_home(&fixture.home),
        "secret",
        |_| {},
        |target| {
            fs::remove_file(target).unwrap();
            symlink(&fixture.targets[1], target).unwrap();
        },
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::ConfigurationFailed);
    fixture.assert_unchanged();
}

#[test]
fn resolved_snapshot_changed_after_read_is_rejected_before_rename() {
    let fixture = Fixture::new();
    let changed_content = ORIGINAL.replace("keep", "new");
    let error = delete_inner(
        &ConfigPaths::for_home(&fixture.home),
        "secret",
        |_| {},
        |target| fs::write(target, &changed_content).unwrap(),
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::ConfigurationFailed);
    assert_eq!(
        fs::read_to_string(&fixture.targets[0]).unwrap(),
        changed_content
    );
    assert_eq!(fs::read_to_string(&fixture.targets[1]).unwrap(), ORIGINAL);
}

#[test]
fn update_rejects_duplicate_alias_retarget_after_document_snapshot() {
    let fixture = Fixture::new();
    let paths = ConfigPaths::for_home(&fixture.home);
    let mut documents = managed_documents(&paths).unwrap();
    let index = documents.len() - 1;
    documents[index]
        .2
        .patch(
            &["mcp", "servers", "secret", "command"],
            Some(serde_json::json!(["edited"])),
        )
        .unwrap();
    fixture.retarget(&fixture.aliases[0]);
    assert!(persist_document(&documents, index).is_err());
    fixture.assert_unchanged();
}
