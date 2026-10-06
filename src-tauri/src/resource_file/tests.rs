use std::os::unix::fs::{symlink, PermissionsExt};

use super::*;

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "opencode-mom-resource-create-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn create_refuses_directory_retarget_and_same_path_inode_replacement() {
    for symlink_change in [false, true] {
        let fixture = Fixture::new();
        let logical = fixture.0.join("skill");
        let directory = Directory::ensure(&logical).unwrap();
        let outside = fixture.0.join("outside");
        fs::create_dir(&outside).unwrap();
        let moved = fixture.0.join("snapshot-directory");
        fs::rename(&logical, &moved).unwrap();
        if symlink_change {
            symlink(&outside, &logical).unwrap();
        } else {
            fs::create_dir(&logical).unwrap();
        }
        assert!(directory.create_file("SKILL.md", "private").is_err());
        assert!(!outside.join("SKILL.md").exists());
        assert!(!logical.join("SKILL.md").exists());
        assert_eq!(fs::read_dir(&moved).unwrap().count(), 0);
    }
}

#[test]
fn exclusive_publication_refuses_raced_destination_symlink_and_cleans_temporary() {
    let fixture = Fixture::new();
    let directory = Directory::ensure(&fixture.0.join("skill")).unwrap();
    let outside = fixture.0.join("outside.md");
    fs::write(&outside, "unchanged").unwrap();
    assert!(directory
        .create_file_checked("SKILL.md", "private", || {
            symlink(&outside, directory.resolved.join("SKILL.md")).unwrap();
            Ok(())
        })
        .is_err());
    assert_eq!(fs::read_to_string(outside).unwrap(), "unchanged");
    assert_eq!(fs::read_dir(directory.resolved).unwrap().count(), 1);
}

fn drift_file(path: &Path, replace_inode: bool, mode: u32) {
    if replace_inode {
        let content = fs::read(path).unwrap();
        fs::rename(path, path.with_extension("held")).unwrap();
        let replacement = path.with_extension("replacement");
        fs::write(&replacement, content).unwrap();
        fs::set_permissions(&replacement, fs::Permissions::from_mode(mode)).unwrap();
        fs::rename(replacement, path).unwrap();
    } else {
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
    }
}

fn snapshot_fixture(fixture: &Fixture) -> (PathBuf, Snapshot) {
    let path = fixture.0.join("secret.md");
    fs::write(&path, "private original body").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    let snapshot = Snapshot::read(&path).unwrap().unwrap();
    (path, snapshot)
}

#[test]
fn snapshot_rejects_mode_and_identity_drift_during_initial_read() {
    for (replace_inode, mode) in [(false, 0o600), (true, 0o600), (true, 0o644)] {
        let fixture = Fixture::new();
        let (path, _) = snapshot_fixture(&fixture);
        let error = Snapshot::read_inner(
            &path,
            |_| {},
            |resolved| {
                drift_file(resolved, replace_inode, mode);
            },
        )
        .err()
        .unwrap();
        assert!(!error.message.contains("private original body"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "private original body");
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            mode
        );
    }
}

#[test]
fn snapshot_rejects_mode_and_identity_drift_before_staging_without_artifacts() {
    for (replace_inode, mode) in [(false, 0o600), (true, 0o600), (true, 0o644)] {
        let fixture = Fixture::new();
        let (path, snapshot) = snapshot_fixture(&fixture);
        drift_file(&path, replace_inode, mode);
        assert!(snapshot.validate().is_err());
        assert!(replace(&[(snapshot, "edited".to_owned())], |_| Ok(())).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "private original body");
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            mode
        );
        assert!(!fs::read_dir(&fixture.0).unwrap().any(|entry| entry
            .unwrap()
            .path()
            .extension()
            .is_some_and(|extension| extension == "bak" || extension == "tmp")));
    }
}

#[test]
fn snapshot_rejects_final_mode_and_identity_drift_keeps_private_backup_and_removes_temp() {
    for (replace_inode, mode) in [(false, 0o600), (true, 0o600), (true, 0o644)] {
        let fixture = Fixture::new();
        let (path, snapshot) = snapshot_fixture(&fixture);
        assert!(replace(&[(snapshot, "edited".to_owned())], |resolved| {
            drift_file(resolved, replace_inode, mode);
            Ok(())
        })
        .is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "private original body");
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            mode
        );
        let mut backups = 0;
        for entry in fs::read_dir(&fixture.0).unwrap() {
            let path = entry.unwrap().path();
            assert!(!path.extension().is_some_and(|extension| extension == "tmp"));
            if path.extension().is_some_and(|extension| extension == "bak") {
                backups += 1;
                assert_eq!(fs::read_to_string(&path).unwrap(), "private original body");
                assert_eq!(
                    fs::metadata(path).unwrap().permissions().mode() & 0o777,
                    0o600
                );
            }
        }
        assert_eq!(backups, 1);
    }
}

#[test]
fn successful_replace_removes_only_owned_backups_and_preserves_original_modes() {
    let fixture = Fixture::new();
    let old_backup = fixture.0.join(".secret.older.bak");
    fs::write(&old_backup, "unrelated recovery").unwrap();
    let mut updates = Vec::new();
    for (name, mode) in [("first.md", 0o644), ("second.md", 0o600)] {
        let path = fixture.0.join(name);
        fs::write(&path, "original").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
        updates.push((Snapshot::read(&path).unwrap().unwrap(), "edited".to_owned()));
    }
    replace(&updates, |_| Ok(())).unwrap();
    for (snapshot, _) in updates {
        assert_eq!(fs::read_to_string(&snapshot.path).unwrap(), "edited");
        assert_eq!(
            fs::metadata(snapshot.path).unwrap().permissions().mode(),
            snapshot.identity.permissions().mode()
        );
    }
    assert_eq!(
        fs::read_to_string(&old_backup).unwrap(),
        "unrelated recovery"
    );
    assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 3);
}

#[test]
fn failed_multi_file_publish_retains_all_private_complete_backups() {
    for fail_at in [0, 1] {
        let fixture = Fixture::new();
        let mut updates = Vec::new();
        for name in ["first.md", "second.md"] {
            let path = fixture.0.join(name);
            fs::write(&path, name).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
            updates.push((Snapshot::read(&path).unwrap().unwrap(), "edited".to_owned()));
        }
        let mut calls = 0;
        assert!(replace(&updates, |_| {
            let current = calls;
            calls += 1;
            if current == fail_at {
                Err(AppError::configuration("Injected prepublish failure"))
            } else {
                Ok(())
            }
        })
        .is_err());
        assert_eq!(
            fs::read_to_string(fixture.0.join("first.md")).unwrap(),
            if fail_at == 0 { "first.md" } else { "edited" }
        );
        assert_eq!(
            fs::read_to_string(fixture.0.join("second.md")).unwrap(),
            "second.md"
        );
        for name in ["first.md", "second.md"] {
            assert_eq!(
                fs::metadata(fixture.0.join(name))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o644
            );
        }
        let mut originals = Vec::new();
        for entry in fs::read_dir(&fixture.0).unwrap() {
            let path = entry.unwrap().path();
            assert!(!path.extension().is_some_and(|extension| extension == "tmp"));
            if path.extension().is_some_and(|extension| extension == "bak") {
                originals.push(fs::read_to_string(&path).unwrap());
                assert_eq!(
                    fs::metadata(path).unwrap().permissions().mode() & 0o777,
                    0o600
                );
            }
        }
        originals.sort();
        assert_eq!(originals, ["first.md", "second.md"]);
    }
}

#[test]
fn staging_failure_keeps_prior_recovery_backup_and_cleans_temporary() {
    let fixture = Fixture::new();
    let (path, snapshot) = snapshot_fixture(&fixture);
    let other = fixture.0.join("other.md");
    fs::write(&other, "other original").unwrap();
    let stale = Snapshot::read(&other).unwrap().unwrap();
    fs::write(&other, "changed before staging").unwrap();
    assert!(replace(
        &[
            (snapshot, "edited".to_owned()),
            (stale, "edited".to_owned())
        ],
        |_| Ok(())
    )
    .is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), "private original body");
    let backups = fs::read_dir(&fixture.0)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "bak"))
        .collect::<Vec<_>>();
    assert_eq!(backups.len(), 1);
    assert_eq!(
        fs::read_to_string(&backups[0]).unwrap(),
        "private original body"
    );
    assert_eq!(
        fs::metadata(&backups[0]).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert!(!fs::read_dir(&fixture.0).unwrap().any(|entry| entry
        .unwrap()
        .path()
        .extension()
        .is_some_and(|extension| extension == "tmp")));
}

#[test]
fn backup_cleanup_error_after_commit_does_not_report_save_failure() {
    let fixture = Fixture::new();
    let (path, snapshot) = snapshot_fixture(&fixture);
    replace(&[(snapshot, "edited".to_owned())], |_| {
        let backup = fs::read_dir(&fixture.0)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|path| path.extension().is_some_and(|extension| extension == "bak"))
            .unwrap();
        fs::remove_file(&backup).unwrap();
        fs::create_dir(backup).unwrap();
        Ok(())
    })
    .unwrap();
    assert_eq!(fs::read_to_string(path).unwrap(), "edited");
}
