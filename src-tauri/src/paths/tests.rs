use super::*;

#[test]
fn opencode_file_prefers_jsonc_then_falls_back_to_json() {
    let tmp = std::env::temp_dir().join(format!("opencode-mom-paths-{}", std::process::id()));
    let dir = tmp.join(".config").join("opencode");
    std::fs::create_dir_all(&dir).unwrap();
    let paths = || ConfigPaths::for_home(&tmp);
    // No file at all -> fall back to .json (opencode reads it too).
    assert_eq!(paths().opencode_file(), dir.join("opencode.json"));
    // Only opencode.json exists -> use it (the bug this guards against).
    std::fs::write(dir.join("opencode.json"), "{}").unwrap();
    assert_eq!(paths().opencode_file(), dir.join("opencode.json"));
    // Both exist -> .jsonc wins (opencode's primary file).
    std::fs::write(dir.join("opencode.jsonc"), "{}").unwrap();
    assert_eq!(paths().opencode_file(), dir.join("opencode.jsonc"));
    std::fs::write(dir.join("oh-my-opencode-slim.jsonc"), "{}").unwrap();
    assert_eq!(paths().slim_file(), dir.join("oh-my-opencode-slim.json"));
    // Explicit OPENCODE_CONFIG always wins.
    let explicit = tmp.join("custom.json");
    assert_eq!(
        ConfigPaths::with_parts(tmp.clone(), Some(explicit.clone()), None).opencode_file(),
        explicit
    );
    std::fs::remove_dir_all(&tmp).unwrap();
}

#[test]
fn arbitrary_explicit_config_is_last_and_relative_paths_use_working_directory() {
    let home = std::env::temp_dir().join("opencode-mom-path-contract");
    let cwd = home.join("cwd");
    let paths = ConfigPaths::with_resource_parts(
        home.clone(),
        Some(PathBuf::from("selected/custom.jsonc")),
        Some(PathBuf::from("extra")),
        home.join("xdg"),
        cwd.clone(),
    );
    let explicit = cwd.join("selected/custom.jsonc");
    assert_eq!(paths.opencode_file(), explicit);
    assert_eq!(
        paths.global_config_files(),
        &[
            home.join("xdg/opencode/opencode.json"),
            home.join("xdg/opencode/opencode.jsonc"),
            cwd.join("extra/opencode.json"),
            cwd.join("extra/opencode.jsonc"),
            explicit,
        ]
    );
    assert_eq!(paths.resolve_skill_path("relative"), cwd.join("relative"));
}

#[test]
fn duplicate_explicit_source_keeps_its_last_precedence() {
    let home = std::env::temp_dir().join("opencode-mom-path-contract");
    let global = home.join(".config/opencode");
    let explicit = global.join("opencode.json");
    let paths = ConfigPaths::with_resource_parts(
        home.clone(),
        Some(explicit.clone()),
        Some(global.clone()),
        home.join(".config"),
        home.join("cwd"),
    );
    assert_eq!(
        paths.global_config_files(),
        &[global.join("opencode.jsonc"), explicit]
    );
    assert_eq!(
        paths
            .global_skill_dirs()
            .iter()
            .filter(|path| **path == global.join("skills"))
            .count(),
        1
    );
}
