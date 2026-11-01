use super::*;

#[test]
fn opencode_file_prefers_jsonc_then_falls_back_to_json() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join(".config").join("opencode");
    std::fs::create_dir_all(&dir).unwrap();
    let paths = || ConfigPaths::for_home(tmp.path());
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
    let explicit = tmp.path().join("custom.json");
    assert_eq!(
        ConfigPaths::with_parts(tmp.path().to_path_buf(), Some(explicit.clone()), None)
            .opencode_file(),
        explicit
    );
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
    assert_eq!(paths.resolve_skill_path("relative"), None);
}

#[test]
fn duplicate_explicit_source_keeps_its_last_precedence() {
    let home = std::env::temp_dir().join("opencode-mom-path-contract");
    let global = home.join(".config/opencode");
    let explicit = global.join("opencode.json");
    let paths = ConfigPaths::with_resource_parts(
        home.clone(),
        Some(PathBuf::from("../.config/opencode/./opencode.json")),
        Some(PathBuf::from("../.config/opencode/nested/..")),
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

#[test]
fn parent_relative_overrides_normalize_existing_and_missing_config_and_command_paths() {
    for existing in [false, true] {
        for absolute in [false, true] {
            let home = tempfile::tempdir().unwrap();
            let cwd = home.path().join("cwd");
            let directory = home.path().join("override");
            let explicit = directory.join("custom.jsonc");
            if existing {
                fs::create_dir_all(&cwd).unwrap();
                fs::create_dir_all(directory.join("nested")).unwrap();
                fs::write(&explicit, "{}").unwrap();
            }
            let config = PathBuf::from("../override/nested/../custom.jsonc");
            let override_dir = PathBuf::from("../override/./nested/..");
            let paths = ConfigPaths::with_resource_parts(
                home.path().into(),
                Some(if absolute { cwd.join(config) } else { config }),
                Some(if absolute {
                    cwd.join(override_dir)
                } else {
                    override_dir
                }),
                home.path().join("xdg"),
                cwd.clone(),
            );
            assert_eq!(paths.opencode_file(), explicit);
            assert_eq!(
                paths.global_config_files(),
                &[
                    home.path().join("xdg/opencode/opencode.json"),
                    home.path().join("xdg/opencode/opencode.jsonc"),
                    directory.join("opencode.json"),
                    directory.join("opencode.jsonc"),
                    explicit.clone(),
                ]
            );
            assert!(paths.validate_plugin_config_source(&explicit).is_ok());
            assert_eq!(paths.commands_write_dir(), directory.join("commands"));
            assert_eq!(
                paths.global_instructions_file(),
                directory.join("AGENTS.md")
            );
            assert!(paths
                .validate_managed_write(&paths.commands_write_dir().join("team/new.md"))
                .is_ok());
            assert!(paths
                .validate_managed_write(&paths.global_instructions_file())
                .is_ok());
            assert!(paths
                .validate_managed_write(&directory.join("commands/../escape.md"))
                .is_err());
            assert!(paths
                .validate_managed_write(&directory.join("plugins/new.ts"))
                .is_err());
            let command = crate::custom_commands::create(
                &paths,
                crate::custom_commands::SlashCommandDraft {
                    name: "team/review".into(),
                    content: "Review the changes".into(),
                },
            )
            .unwrap();
            assert_eq!(
                command.source_path,
                directory.join("commands/team/review.md")
            );
            assert_eq!(
                crate::custom_commands::list(&paths).unwrap().data,
                vec![command]
            );
            let behavior = crate::behavior::get(&paths).unwrap();
            let saved = crate::behavior::save(
                &paths,
                crate::behavior::BehaviorUpdate {
                    content: "Use the relative override".into(),
                    expected_path: behavior.resolved_path,
                    expected_content: behavior.exists.then_some(behavior.content),
                },
            )
            .unwrap();
            assert_eq!(saved.path, directory.join("AGENTS.md"));
            assert_eq!(saved.content, "Use the relative override");
            assert!(!home.path().join("xdg/opencode/AGENTS.md").exists());
        }
    }
}

#[test]
fn override_parent_segments_do_not_pop_the_filesystem_root() {
    let home = tempfile::tempdir().unwrap();
    let canonical = fs::canonicalize(home.path()).unwrap();
    let root = canonical.ancestors().last().unwrap();
    let paths = ConfigPaths::with_resource_parts(
        home.path().into(),
        Some(root.join("../../opencode-override/../custom.jsonc")),
        Some(root.join("../../opencode-override/nested/..")),
        home.path().join("xdg"),
        home.path().into(),
    );
    assert_eq!(paths.opencode_file(), root.join("custom.jsonc"));
    assert_eq!(
        paths.commands_write_dir(),
        root.join("opencode-override/commands")
    );
}

#[test]
fn invalid_explicit_config_filename_is_not_normalized_to_a_directory() {
    let home = tempfile::tempdir().unwrap();
    let missing = home.path().join("missing");
    let explicit = missing.join("..");
    let paths = ConfigPaths::with_parts(home.path().into(), Some(explicit.clone()), None);
    assert_eq!(paths.opencode_file(), explicit);
    let error = crate::agents::create(
        &paths,
        crate::agents::AgentCreate {
            id: "helper".into(),
            storage: crate::agents::AgentStorage::Inline,
            fields: Default::default(),
            prompt: None,
        },
    )
    .unwrap_err();
    assert_eq!(error.code, crate::error::ErrorCode::ValidationFailed);
    assert!(!missing.exists());
}

#[test]
fn missing_project_root_is_not_found_but_files_are_invalid() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let file = root.join("file");
    fs::write(&file, "not a directory").unwrap();
    assert_eq!(
        canonical_project_directory(&root.join("missing"))
            .unwrap_err()
            .code,
        crate::error::ErrorCode::NotFound
    );
    assert_eq!(
        canonical_project_directory(&file).unwrap_err().code,
        crate::error::ErrorCode::ValidationFailed
    );
}

#[test]
fn filesystem_root_is_invalid_without_imposing_a_project_depth_requirement() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let canonical = fs::canonicalize(root).unwrap();
    let volume_root = canonical.ancestors().last().unwrap();
    assert_eq!(
        canonical_project_directory(volume_root).unwrap_err().code,
        crate::error::ErrorCode::ValidationFailed
    );
    assert_eq!(canonical_project_directory(root).unwrap(), canonical);
}

#[test]
fn is_within_boundary_detects_descendant_and_escapes() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let child = root.join("child/sub");
    fs::create_dir_all(&child).unwrap();

    assert!(is_within_boundary(root, &child));
    assert!(is_within_boundary(root, root));

    let outside = tmp.path().parent().unwrap();
    assert!(!is_within_boundary(root, outside));

    let escape = root.join("../escape");
    assert!(!is_within_boundary(root, &escape));
}

#[test]
fn command_plugin_and_instruction_paths_are_scoped_without_redirecting_existing_writes() {
    let home = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::with_resource_parts(
        home.path().into(),
        Some(home.path().join("custom.jsonc")),
        Some(home.path().join("override")),
        home.path().join("xdg"),
        home.path().into(),
    );
    assert_eq!(
        paths.command_dirs(),
        &[
            home.path().join("xdg/opencode/command"),
            home.path().join("xdg/opencode/commands"),
            home.path().join("override/command"),
            home.path().join("override/commands"),
        ]
    );
    assert_eq!(
        paths.commands_write_dir(),
        home.path().join("override/commands")
    );
    assert_eq!(
        paths.global_instructions_file(),
        home.path().join("override/AGENTS.md")
    );
    let project_root = home.path().join("project");
    fs::create_dir(&project_root).unwrap();
    let project =
        crate::models::add_project(&paths.config_file(), project_root.to_str().unwrap()).unwrap();
    let scoped = paths.for_project(Some(project.id)).unwrap();
    let root = fs::canonicalize(project_root).unwrap();
    assert_eq!(scoped.project_directory(), Some(root.as_path()));
    assert_eq!(scoped.commands_write_dir(), root.join(".opencode/commands"));
    assert_eq!(scoped.opencode_file(), paths.opencode_file());
    assert_eq!(
        scoped.global_instructions_file(),
        paths.global_instructions_file()
    );
    fs::write(root.join("opencode.json"), "{}").unwrap();
    assert!(scoped
        .validate_managed_write(&root.join("opencode.json"))
        .is_err());
    assert!(scoped
        .validate_managed_write(&root.join(".opencode/plugins/new.ts"))
        .is_err());
    for directory in ["agents", "mcp", "skills"] {
        assert!(scoped
            .validate_managed_write(&root.join(".opencode").join(directory).join("new.md"))
            .is_err());
    }
    assert!(scoped
        .validate_managed_write(&root.join(".opencode/commands/team/review.md"))
        .is_ok());
    assert!(scoped
        .validate_managed_write(&paths.opencode_file())
        .is_err());
    assert!(scoped
        .validate_managed_write(&root.join("other.md"))
        .is_err());
    assert!(scoped
        .validate_managed_write(&root.join(".opencode/commands/../escape.md"))
        .is_err());
}

#[test]
fn global_resource_writes_default_to_xdg_and_ignore_explicit_config_parent() {
    let home = tempfile::tempdir().unwrap();
    let xdg = home.path().join("xdg");
    let working_directory = home.path().join("cwd");
    let explicit = working_directory.join("custom/config.jsonc");
    for config in [None, Some(explicit.clone())] {
        let paths = ConfigPaths::with_resource_parts(
            home.path().into(),
            config.clone(),
            None,
            xdg.clone(),
            working_directory.clone(),
        );
        assert_eq!(paths.commands_write_dir(), xdg.join("opencode/commands"));
        assert_eq!(
            paths.global_instructions_file(),
            xdg.join("opencode/AGENTS.md")
        );
        assert_eq!(
            paths.command_dirs(),
            &[xdg.join("opencode/command"), xdg.join("opencode/commands")]
        );
        assert!(paths
            .validate_managed_write(&xdg.join("opencode/plugins/new.ts"))
            .is_err());
        if let Some(config) = config {
            assert_eq!(paths.opencode_file(), config);
        }
    }

    let paths = ConfigPaths::with_resource_parts(
        home.path().into(),
        Some(explicit.clone()),
        Some(PathBuf::from("override")),
        xdg,
        working_directory.clone(),
    );
    assert_eq!(
        paths.commands_write_dir(),
        working_directory.join("override/commands")
    );
    assert_eq!(
        paths.global_instructions_file(),
        working_directory.join("override/AGENTS.md")
    );
    assert_eq!(paths.opencode_file(), explicit);
    assert!(paths
        .validate_managed_write(&paths.commands_write_dir().join("new.md"))
        .is_ok());
    assert!(paths
        .validate_managed_write(&working_directory.join("override/plugins/new.ts"))
        .is_err());
}

#[cfg(unix)]
#[test]
fn normalized_overrides_preserve_logical_symlinks_and_reject_retargeted_sources() {
    use std::os::unix::fs::symlink;
    let home = tempfile::tempdir().unwrap();
    let first = home.path().join("first");
    let second = home.path().join("second");
    let alias = home.path().join("alias");
    for target in [&first, &second] {
        fs::create_dir_all(target.join("commands")).unwrap();
        fs::write(target.join("custom.jsonc"), "{}").unwrap();
    }
    symlink(&first, &alias).unwrap();
    let paths = ConfigPaths::with_resource_parts(
        home.path().into(),
        Some(PathBuf::from("../alias/custom.jsonc")),
        Some(PathBuf::from("../alias/nested/..")),
        home.path().join("xdg"),
        home.path().join("cwd"),
    );
    assert_eq!(paths.opencode_file(), alias.join("custom.jsonc"));
    assert_eq!(paths.commands_write_dir(), alias.join("commands"));
    assert!(paths
        .validate_managed_write(&paths.commands_write_dir().join("new.md"))
        .is_ok());
    assert!(paths
        .validate_managed_write(&paths.global_instructions_file())
        .is_ok());
    assert!(paths
        .validate_plugin_config_source(&fs::canonicalize(first.join("custom.jsonc")).unwrap())
        .is_ok());
    let outside = home.path().join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("static.md"), "Static Global symlink").unwrap();
    symlink(outside.join("static.md"), first.join("commands/static.md")).unwrap();
    assert!(paths
        .validate_managed_write(&paths.commands_write_dir().join("static.md"))
        .is_ok());
    symlink(&outside, first.join("commands/escape")).unwrap();
    assert!(paths
        .validate_managed_write(&paths.commands_write_dir().join("escape/new.md"))
        .is_err());
    fs::remove_file(&alias).unwrap();
    symlink(&second, &alias).unwrap();
    for target in [
        paths.commands_write_dir().join("new.md"),
        paths.global_instructions_file(),
    ] {
        assert_eq!(
            paths.validate_managed_write(&target).unwrap_err().code,
            crate::error::ErrorCode::ConfigurationFailed
        );
    }
    assert_eq!(
        paths
            .validate_plugin_config_source(&paths.opencode_file())
            .unwrap_err()
            .code,
        crate::error::ErrorCode::ConfigurationFailed
    );
}

#[cfg(unix)]
#[test]
fn override_parent_segments_follow_symlinks_without_losing_retarget_checks() {
    use std::os::unix::fs::symlink;
    let home = tempfile::tempdir().unwrap();
    let first = home.path().join("first/target");
    let second = home.path().join("second/target");
    let alias = home.path().join("alias");
    fs::create_dir_all(&first).unwrap();
    fs::create_dir_all(&second).unwrap();
    symlink(&first, &alias).unwrap();
    let paths = ConfigPaths::with_resource_parts(
        home.path().into(),
        Some(PathBuf::from("../alias/../override/custom.jsonc")),
        Some(PathBuf::from("../alias/../override")),
        home.path().join("xdg"),
        home.path().join("cwd"),
    );
    let directory = fs::canonicalize(&first)
        .unwrap()
        .parent()
        .unwrap()
        .join("override");
    assert_eq!(paths.opencode_file(), directory.join("custom.jsonc"));
    assert_eq!(paths.commands_write_dir(), directory.join("commands"));
    let command = paths.commands_write_dir().join("new.md");
    assert!(paths.validate_managed_write(&command).is_ok());
    assert!(paths
        .validate_plugin_config_source(&paths.opencode_file())
        .is_ok());
    fs::remove_file(&alias).unwrap();
    symlink(&second, &alias).unwrap();
    assert_eq!(
        paths.validate_managed_write(&command).unwrap_err().code,
        crate::error::ErrorCode::ConfigurationFailed
    );
    assert_eq!(
        paths
            .validate_plugin_config_source(&paths.opencode_file())
            .unwrap_err()
            .code,
        crate::error::ErrorCode::ConfigurationFailed
    );
}

#[cfg(unix)]
#[test]
fn override_parent_segments_do_not_collapse_files_or_unresolved_symlinks() {
    use std::os::unix::fs::symlink;
    let home = tempfile::tempdir().unwrap();
    fs::write(home.path().join("file"), "not a directory").unwrap();
    symlink(home.path().join("missing"), home.path().join("dangling")).unwrap();
    for component in ["file", "dangling"] {
        let paths = ConfigPaths::with_resource_parts(
            home.path().into(),
            Some(PathBuf::from(format!(
                "../{component}/../override/custom.jsonc"
            ))),
            Some(PathBuf::from(format!("../{component}/../override"))),
            home.path().join("xdg"),
            home.path().join("cwd"),
        );
        assert!(paths
            .validate_managed_write(&paths.commands_write_dir().join("new.md"))
            .is_err());
        assert!(paths
            .validate_plugin_config_source(&paths.opencode_file())
            .is_err());
    }
}

#[cfg(unix)]
#[test]
fn project_commands_reject_readonly_resource_aliases_into_command_paths() {
    use std::os::unix::fs::symlink;

    let home = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(home.path());
    let root = home.path().join("project");
    fs::create_dir(&root).unwrap();
    let project = crate::models::add_project(&paths.config_file(), root.to_str().unwrap()).unwrap();
    let scoped = paths.for_project(Some(project.id)).unwrap();
    let root = scoped.project_directory().unwrap();
    let command = scoped.commands_write_dir().join("review.md");
    fs::create_dir_all(command.parent().unwrap()).unwrap();
    fs::write(&command, "body").unwrap();
    for resource in ["agent", "agents", "skill", "skills", "plugins"] {
        let alias = root.join(".opencode").join(resource);
        symlink(scoped.commands_write_dir(), &alias).unwrap();
        assert!(
            scoped.validate_managed_write(&command).is_err(),
            "{resource}"
        );
        fs::remove_file(&alias).unwrap();
    }
    for resource in [
        "opencode.json",
        "opencode.jsonc",
        ".opencode/opencode.json",
        ".opencode/opencode.jsonc",
    ] {
        let alias = root.join(resource);
        symlink(&command, &alias).unwrap();
        assert!(
            scoped.validate_managed_write(&command).is_err(),
            "{resource}"
        );
        fs::remove_file(&alias).unwrap();
    }
    assert!(scoped.validate_managed_write(&command).is_ok());
}

#[cfg(unix)]
#[test]
fn project_command_plugin_roots_and_target_parents_cannot_overlap_global_or_escape() {
    use std::os::unix::fs::symlink;
    let home = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(home.path());
    let root = home.path().join("project");
    fs::create_dir_all(root.join(".opencode")).unwrap();
    fs::create_dir_all(paths.commands_write_dir()).unwrap();
    symlink(paths.commands_write_dir(), root.join(".opencode/commands")).unwrap();
    assert!(paths.validate_project_directory(&root).is_err());
    fs::remove_file(root.join(".opencode/commands")).unwrap();
    let project = crate::models::add_project(&paths.config_file(), root.to_str().unwrap()).unwrap();
    let scoped = paths.for_project(Some(project.id)).unwrap();
    let outside = home.path().join("outside");
    fs::create_dir(&outside).unwrap();
    symlink(&outside, root.join(".opencode/plugins")).unwrap();
    assert!(scoped
        .validate_managed_write(&root.join(".opencode/plugins/new.ts"))
        .is_err());
    fs::remove_file(root.join(".opencode/plugins")).unwrap();
    symlink(paths.commands_write_dir(), root.join(".opencode/plugins")).unwrap();
    assert!(scoped.validate_resource_root().is_err());
}

#[cfg(unix)]
#[test]
fn plugin_roots_remain_readonly_through_global_command_aliases() {
    use std::os::unix::fs::symlink;
    let home = tempfile::tempdir().unwrap();
    let plugin_root = home.path().join(".config/opencode/plugins");
    fs::create_dir_all(&plugin_root).unwrap();
    symlink(&plugin_root, home.path().join(".config/opencode/commands")).unwrap();
    let paths = ConfigPaths::for_home(home.path());
    assert!(paths
        .validate_managed_write(&paths.commands_write_dir().join("new.md"))
        .is_err());
    assert!(paths
        .validate_managed_write(&paths.opencode_file())
        .is_err());
}
