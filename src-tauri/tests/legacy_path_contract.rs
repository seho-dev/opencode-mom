use std::fs;
use std::io;
use std::path::{Path, PathBuf};

struct SourceRoot {
    path: PathBuf,
    required: bool,
}

#[test]
fn direct_cutover_when_product_and_test_sources_are_scanned_has_no_legacy_test_path() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Given: src-tauri has workspace parent");
    let legacy_path = ["OMO", "Switch", "Tests"].concat();
    let source_roots = [
        SourceRoot {
            path: workspace.join("src-tauri/src"),
            required: true,
        },
        SourceRoot {
            path: workspace.join("src-tauri/tests"),
            required: true,
        },
        SourceRoot {
            path: workspace.join("src"),
            required: true,
        },
        SourceRoot {
            path: workspace.join("e2e"),
            required: false,
        },
    ];

    let stale_paths = stale_paths(&source_roots, &legacy_path)
        .expect("When: present product or test sources are scanned");

    assert!(
        stale_paths.is_empty(),
        "Then: direct cutover has no stale legacy test paths: {stale_paths:?}"
    );
    assert!(
        !workspace.join(&legacy_path).exists(),
        "Then: direct cutover does not restore the retired test directory"
    );
}

fn stale_paths(source_roots: &[SourceRoot], legacy_path: &str) -> io::Result<Vec<PathBuf>> {
    let mut stale_paths = Vec::new();
    for root in source_roots {
        for path in source_files(&root.path, root.required)? {
            if fs::read_to_string(&path)?.contains(legacy_path) {
                stale_paths.push(path);
            }
        }
    }
    Ok(stale_paths)
}

fn source_files(root: &Path, required: bool) -> io::Result<Vec<PathBuf>> {
    if !root.exists() {
        return if required {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("required source root does not exist: {}", root.display()),
            ))
        } else {
            Ok(Vec::new())
        };
    }

    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();

    while let Some(path) = pending.pop() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();
            if entry_path.is_dir() {
                pending.push(entry_path);
            } else if matches!(
                entry_path
                    .extension()
                    .and_then(|extension| extension.to_str()),
                Some("rs" | "ts" | "tsx" | "js" | "mjs" | "cjs" | "svelte")
            ) {
                files.push(entry_path);
            }
        }
    }

    Ok(files)
}

#[test]
fn source_files_when_optional_root_is_absent_skips_it() {
    let missing_root = std::env::temp_dir().join(format!(
        "omo-switch-missing-optional-root-{}",
        uuid::Uuid::new_v4()
    ));

    let files = source_files(&missing_root, false).expect("When: optional root is scanned");

    assert!(files.is_empty(), "Then: absent optional root is skipped");
}

#[test]
fn source_files_when_required_root_is_absent_returns_not_found() {
    let missing_root = std::env::temp_dir().join(format!(
        "omo-switch-missing-required-root-{}",
        uuid::Uuid::new_v4()
    ));

    let error = source_files(&missing_root, true).expect_err("When: required root is scanned");

    assert_eq!(
        error.kind(),
        io::ErrorKind::NotFound,
        "Then: absent required root fails validation"
    );
}

#[test]
fn stale_paths_when_retired_path_is_injected_reports_existing_source() {
    let root = std::env::temp_dir().join(format!(
        "omo-switch-legacy-path-contract-{}",
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&root).expect("Given: scanned source root exists");
    let source = root.join("injected.rs");
    let legacy_path = ["OMO", "Switch", "Tests"].concat();
    fs::write(&source, format!("// {legacy_path}"))
        .expect("Given: retired path reference is injected");
    let source_roots = [SourceRoot {
        path: root.clone(),
        required: true,
    }];

    let paths = stale_paths(&source_roots, &legacy_path).expect("When: source root is scanned");

    assert_eq!(paths, [source], "Then: injected retired path is reported");
    fs::remove_dir_all(root).expect("Then: injected source fixture is removed");
}
