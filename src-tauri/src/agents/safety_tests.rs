use std::os::unix::fs::symlink;

use super::*;

struct Fixture {
    home: PathBuf,
    paths: crate::paths::ConfigPaths,
    logical: PathBuf,
    target: PathBuf,
    outside: PathBuf,
    content: String,
    content_snapshot: String,
}

impl Fixture {
    fn new(storage: AgentStorage) -> Self {
        let home = std::env::temp_dir().join(format!(
            "opencode-mom-agent-safety-{}",
            uuid::Uuid::new_v4()
        ));
        let paths = crate::paths::ConfigPaths::for_home(&home);
        let (logical, content, content_snapshot) = match storage {
            AgentStorage::Inline => (
                paths.opencode_file(),
                r#"{"agents":{"helper":{"description":"original"}},"keep":true}"#.to_owned(),
                serde_json::json!({"description": "original"}).to_string(),
            ),
            AgentStorage::GlobalMarkdown => (
                paths.global_agents_dir().join("helper.md"),
                "---\ndescription: original\n---\nbody".to_owned(),
                "---\ndescription: original\n---\nbody".to_owned(),
            ),
        };
        let target = home.join("target/source");
        let outside = home.join("outside");
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, &content).unwrap();
        fs::write(&outside, &content).unwrap();
        fs::create_dir_all(logical.parent().unwrap()).unwrap();
        symlink(&target, &logical).unwrap();
        Self {
            home,
            paths,
            logical,
            target,
            outside,
            content,
            content_snapshot,
        }
    }

    fn mutation(&self) -> AgentMutation {
        AgentMutation {
            fields: serde_json::from_value(serde_json::json!({"description": "edited"})).unwrap(),
            expected_path: Some(fs::canonicalize(&self.target).unwrap()),
            expected_content: Some(self.content_snapshot.clone()),
            ..Default::default()
        }
    }

    fn retarget(&self, path: &Path) {
        fs::remove_file(path).unwrap();
        symlink(&self.outside, path).unwrap();
    }

    fn assert_unchanged(&self) {
        assert_eq!(fs::read_to_string(&self.target).unwrap(), self.content);
        assert_eq!(fs::read_to_string(&self.outside).unwrap(), self.content);
        let recovery = fs::read_dir(self.target.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        assert!(!recovery
            .iter()
            .any(|path| path.extension().is_some_and(|ext| ext == "tmp")));
        for backup in recovery
            .iter()
            .filter(|path| path.extension().is_some_and(|ext| ext == "bak"))
        {
            assert_eq!(fs::read_to_string(backup).unwrap(), self.content);
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.home);
    }
}

#[test]
fn update_rejects_logical_retarget_after_snapshot_before_publication() {
    for storage in [AgentStorage::Inline, AgentStorage::GlobalMarkdown] {
        let fixture = Fixture::new(storage);
        let error = update_inner(
            &fixture.paths,
            "helper",
            storage,
            fixture.mutation(),
            |_| {
                fixture.retarget(&fixture.logical);
            },
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::ConfigurationFailed);
        fixture.assert_unchanged();
    }
}

#[test]
fn update_rejects_resolved_target_replaced_by_identical_content_symlink() {
    for storage in [AgentStorage::Inline, AgentStorage::GlobalMarkdown] {
        let fixture = Fixture::new(storage);
        let error = update_inner(
            &fixture.paths,
            "helper",
            storage,
            fixture.mutation(),
            |resolved| {
                fixture.retarget(resolved);
            },
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::ConfigurationFailed);
        fixture.assert_unchanged();
    }
}

#[test]
fn update_rejects_external_content_change_after_snapshot() {
    for storage in [AgentStorage::Inline, AgentStorage::GlobalMarkdown] {
        let fixture = Fixture::new(storage);
        let changed = fixture.content.replace("original", "external");
        let error = update_inner(
            &fixture.paths,
            "helper",
            storage,
            fixture.mutation(),
            |resolved| {
                fs::write(resolved, &changed).unwrap();
            },
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::ConfigurationFailed);
        assert_eq!(fs::read_to_string(&fixture.target).unwrap(), changed);
        assert_eq!(
            fs::read_to_string(&fixture.outside).unwrap(),
            fixture.content
        );
    }
}

#[test]
fn markdown_update_rejects_duplicate_alias_retarget_before_publication() {
    let fixture = Fixture::new(AgentStorage::GlobalMarkdown);
    let singular = fixture.home.join(".config/opencode/agent/helper.md");
    fs::create_dir_all(singular.parent().unwrap()).unwrap();
    symlink(&fixture.target, &singular).unwrap();
    assert!(update_inner(
        &fixture.paths,
        "helper",
        AgentStorage::GlobalMarkdown,
        fixture.mutation(),
        |_| {
            fixture.retarget(&singular);
        }
    )
    .is_err());
    fixture.assert_unchanged();
}

#[test]
fn creation_refuses_raced_destination_symlink_for_both_storages() {
    for storage in [AgentStorage::Inline, AgentStorage::GlobalMarkdown] {
        let fixture = Fixture::new(storage);
        fs::remove_file(&fixture.logical).unwrap();
        assert!(create_inner(
            &fixture.paths,
            AgentCreate {
                id: "helper".to_owned(),
                storage,
                fields: Map::new(),
                prompt: Some("new".to_owned()),
            },
            |logical| {
                symlink(&fixture.outside, logical).unwrap();
            }
        )
        .is_err());
        fixture.assert_unchanged();
    }
}

#[test]
fn creation_refuses_parent_directory_retarget_for_both_storages() {
    for storage in [AgentStorage::Inline, AgentStorage::GlobalMarkdown] {
        let fixture = Fixture::new(storage);
        fs::remove_file(&fixture.logical).unwrap();
        let outside = fixture.home.join("outside-directory");
        fs::create_dir(&outside).unwrap();
        assert!(create_inner(
            &fixture.paths,
            AgentCreate {
                id: "helper".to_owned(),
                storage,
                fields: Map::new(),
                prompt: Some("new".to_owned()),
            },
            |logical| {
                let parent = logical.parent().unwrap();
                fs::rename(parent, parent.with_extension("held")).unwrap();
                symlink(&outside, parent).unwrap();
            }
        )
        .is_err());
        fixture.assert_unchanged();
        assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
    }
}
