use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};

use super::*;
use crate::error::ErrorCode;

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
