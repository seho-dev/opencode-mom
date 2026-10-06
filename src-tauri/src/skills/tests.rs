use std::os::unix::fs::symlink;

use super::*;

#[test]
fn update_rejects_identical_content_alias_retarget_after_discovery() {
    let home = std::env::temp_dir().join(format!(
        "opencode-mom-skill-rebound-{}",
        uuid::Uuid::new_v4()
    ));
    let root = home.join(".config/opencode/skills");
    fs::create_dir_all(&root).unwrap();
    let a = home.join("a.md");
    let b = home.join("b.md");
    fs::write(&a, "same original").unwrap();
    fs::write(&b, "same original").unwrap();
    let alias = root.join("Linked.md");
    symlink(&a, &alias).unwrap();
    let draft = SkillUpdate {
        id: "Linked".to_owned(),
        content: "edited".to_owned(),
        expected_content: "same original".to_owned(),
        expected_path: alias.display().to_string(),
    };
    let result = update_inner(&ConfigPaths::for_home(&home), draft, || {
        fs::remove_file(&alias).unwrap();
        symlink(&b, &alias).unwrap();
    });
    assert!(result.is_err());
    assert_eq!(fs::read_to_string(a).unwrap(), "same original");
    assert_eq!(fs::read_to_string(b).unwrap(), "same original");
    fs::remove_dir_all(home).unwrap();
}

#[test]
fn update_rejects_moved_parent_before_staging_secret_backup_or_temporary() {
    let home = std::env::temp_dir().join(format!(
        "opencode-mom-skill-parent-rebound-{}",
        uuid::Uuid::new_v4()
    ));
    let root = home.join(".config/opencode/skills");
    fs::create_dir_all(&root).unwrap();
    let original = "---\nname: Private\n---\nold secret body";
    let path = root.join("Private.md");
    fs::write(&path, original).unwrap();
    let moved = home.join("moved-skills");
    let outside = home.join("outside");
    fs::create_dir(&outside).unwrap();
    let draft = SkillUpdate {
        id: "Private".to_owned(),
        content: "new secret body".to_owned(),
        expected_content: original.to_owned(),
        expected_path: path.display().to_string(),
    };
    let result = update_inner(&ConfigPaths::for_home(&home), draft, || {
        fs::rename(&root, &moved).unwrap();
        symlink(&outside, &root).unwrap();
    });
    assert_eq!(
        result.unwrap_err().code,
        crate::error::ErrorCode::ConfigurationFailed
    );
    assert_eq!(
        fs::read_to_string(moved.join("Private.md")).unwrap(),
        original
    );
    assert_eq!(fs::read_dir(&moved).unwrap().count(), 1);
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
    fs::remove_dir_all(home).unwrap();
}

#[test]
fn failed_create_removes_owned_empty_directory_and_allows_same_id_retry() {
    let home = std::env::temp_dir().join(format!(
        "opencode-mom-skill-create-retry-{}",
        uuid::Uuid::new_v4()
    ));
    fs::create_dir(&home).unwrap();
    let paths = ConfigPaths::for_home(&home);
    let failure = AppError::configuration("Injected publication failure");
    let error = create_inner(
        &paths,
        SkillDraft {
            id: "Retry".to_owned(),
            content: "private body".to_owned(),
        },
        |_| Err(failure.clone()),
    )
    .unwrap_err();
    assert_eq!(error, failure);
    let root = paths.native_global_skills_dir();
    assert!(fs::symlink_metadata(root.join("Retry")).is_err());
    assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
    let saved = create(
        &paths,
        SkillDraft {
            id: "Retry".to_owned(),
            content: "private body".to_owned(),
        },
    )
    .unwrap();
    assert_eq!(fs::read_to_string(&saved.path).unwrap(), "private body");
    assert_eq!(fs::read_dir(root.join("Retry")).unwrap().count(), 1);
    fs::remove_dir_all(home).unwrap();
}

#[test]
fn failed_create_does_not_delete_nonempty_or_replaced_directory() {
    for replacement in ["nonempty", "directory", "symlink"] {
        let home = std::env::temp_dir().join(format!(
            "opencode-mom-skill-create-cleanup-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir(&home).unwrap();
        let paths = ConfigPaths::for_home(&home);
        let failure = AppError::configuration("Injected publication failure");
        let error = create_inner(
            &paths,
            SkillDraft {
                id: "Keep".to_owned(),
                content: "private body".to_owned(),
            },
            |directory| {
                if replacement != "nonempty" {
                    fs::rename(&directory.resolved, home.join("moved-owned-directory")).unwrap();
                    if replacement == "symlink" {
                        let outside = home.join("outside");
                        fs::create_dir(&outside).unwrap();
                        symlink(&outside, &directory.resolved).unwrap();
                    } else {
                        fs::create_dir(&directory.resolved).unwrap();
                    }
                } else {
                    fs::write(directory.resolved.join("keep.txt"), "unrelated").unwrap();
                }
                Err(failure.clone())
            },
        )
        .unwrap_err();
        assert_eq!(error, failure);
        let directory = paths.native_global_skills_dir().join("Keep");
        assert!(directory.is_dir());
        if replacement != "nonempty" {
            assert_eq!(fs::read_dir(&directory).unwrap().count(), 0);
            if replacement == "symlink" {
                assert!(fs::symlink_metadata(&directory)
                    .unwrap()
                    .file_type()
                    .is_symlink());
                assert_eq!(fs::read_dir(home.join("outside")).unwrap().count(), 0);
            }
        } else {
            assert_eq!(
                fs::read_to_string(directory.join("keep.txt")).unwrap(),
                "unrelated"
            );
            assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
        }
        fs::remove_dir_all(home).unwrap();
    }
}
