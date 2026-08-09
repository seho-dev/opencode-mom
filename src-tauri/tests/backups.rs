use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use omo_switch_tauri::core::backup::BackupRepository;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

fn temp_root(label: &str) -> PathBuf {
    let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("task-8-temp")
        .join(format!("omo-switch-{label}-{}-{id}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("Given: temp root directory is created");
    path
}

fn remove_temp(path: &Path) {
    let _ = fs::remove_dir_all(path);
}

fn instant(value: &str) -> OffsetDateTime {
    OffsetDateTime::parse(value, &Rfc3339).expect("Given: RFC3339 fixture parses")
}

#[test]
fn backups_when_source_exists_create_timestamped_target_specific_artifact() {
    // Given: a target file has current bytes that must be protected before switching.
    let root = temp_root("create");
    let source = root.join("opencode.json");
    fs::write(&source, "original opencode").expect("Given: source writes");
    let repository =
        BackupRepository::new(root.join("omo-switch"), || instant("2023-11-14T22:13:20Z"));

    // When: Rust creates a backup for the OpenCode target.
    let backup = repository
        .create_backup("opencode", &source, b"original opencode")
        .expect("When: backup creates");

    // Then: the path and contents match established backup repository semantics.
    assert_eq!(backup.target, "opencode");
    assert!(backup.file_path.ends_with("20231114T221320Z-opencode.json"));
    assert_eq!(
        fs::read_to_string(&backup.file_path).expect("Then: backup reads"),
        "original opencode"
    );
    println!("created backup: {}", backup.file_path.display());

    remove_temp(&root);
}

#[test]
fn backups_when_more_than_five_exist_cleanup_keeps_latest_five_by_filename() {
    // Given: six dated backups exist for the same target.
    let root = temp_root("cleanup");
    let repository =
        BackupRepository::new(root.join("omo-switch"), || instant("2023-11-14T22:13:20Z"));
    let source = root.join("oh-my-openagent.json");
    fs::write(&source, "source").expect("Given: source writes");
    for index in 0..6 {
        let now = instant(&format!("2023-11-14T22:13:2{index}Z"));
        let repo = BackupRepository::new(root.join("omo-switch"), move || now);
        repo.create_backup(
            "oh-my-openagent",
            &source,
            format!("backup-{index}").as_bytes(),
        )
        .expect("Given: backup creates");
    }

    // When: cleanup runs with the established retention limit used by switching.
    repository
        .cleanup("oh-my-openagent", 5)
        .expect("When: cleanup succeeds");

    // Then: only the latest five backup filenames remain.
    let backups = repository
        .list_backups(Some("oh-my-openagent"))
        .expect("Then: backups list");
    let names: Vec<String> = backups
        .iter()
        .map(|backup| {
            backup
                .file_path
                .file_name()
                .expect("Then: backup has filename")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    assert_eq!(backups.len(), 5);
    assert!(names[0].starts_with("20231114T221325Z"));
    assert!(names[4].starts_with("20231114T221321Z"));

    remove_temp(&root);
}
