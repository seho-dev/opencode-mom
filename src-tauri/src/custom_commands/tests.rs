use super::*;

fn draft(name: &str, content: &str) -> SlashCommandDraft {
    SlashCommandDraft {
        name: name.into(),
        content: content.into(),
    }
}

fn edit(entry: &SlashCommandEntry, content: &str) -> SlashCommandUpdate {
    SlashCommandUpdate {
        id: entry.id.clone(),
        content: content.into(),
        expected_path: entry.resolved_path.clone(),
        expected_content: entry.content.clone(),
    }
}

fn deletion(entry: &SlashCommandEntry) -> SlashCommandDelete {
    SlashCommandDelete {
        id: entry.id.clone(),
        expected_path: entry.resolved_path.clone(),
        expected_content: entry.content.clone(),
    }
}

fn write(path: &Path, content: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

#[test]
fn global_and_project_crud_preserve_templates_and_json_config() {
    let home = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(home.path());
    write(
        &paths.opencode_file(),
        "{\"command\":{\"ignored\":{\"template\":\"json\"}}}",
    );
    let root = home.path().join("project");
    fs::create_dir(&root).unwrap();
    let project = crate::models::add_project(&paths.config_file(), root.to_str().unwrap()).unwrap();
    let project_paths = paths.for_project(Some(project.id)).unwrap();
    let original = fs::read(paths.opencode_file()).unwrap();
    let content = "---\ndescription: |\n  Review: shell\nagent: build\nmodel: openai/gpt-4o\nsubagent: true\ncustom: {keep: true}\n---\n!`git diff`\n$ARGUMENTS $1 @src/file\nC:\\work\\file\n";
    for scope in [&paths, &project_paths] {
        let created = create(scope, draft("team/review.v2", content)).unwrap();
        assert_eq!(created.name, "team/review.v2");
        assert_eq!(
            created.source_path,
            scope.commands_write_dir().join("team/review.v2.md")
        );
        assert_eq!(created.content, content);
        assert_eq!(created.description.as_deref(), Some("Review: shell\n"));
        assert_eq!(created.agent.as_deref(), Some("build"));
        let wire = serde_json::to_value(&created).unwrap();
        assert_eq!(wire["sourcePath"], created.source_path.to_str().unwrap());
        assert_eq!(
            wire["resolvedPath"],
            created.resolved_path.to_str().unwrap()
        );
        assert_eq!(wire["description"], "Review: shell\n");
        assert_eq!(wire["agent"], "build");
        assert_eq!(wire["model"], "openai/gpt-4o");
        assert_eq!(wire["subagent"], true);
        assert_eq!(wire.as_object().unwrap().len(), 9);
        assert_ne!(created.id, created.source_path.to_str().unwrap());
        assert!(wire.get("template").is_none());
        assert!(wire.get("frontmatter").is_none());
        assert!(create(scope, draft("team/review.v2", "clobber")).is_err());
        let updated = update(scope, edit(&created, "updated $ARGUMENTS")).unwrap();
        assert_eq!(updated.content, "updated $ARGUMENTS");
        let updated_wire = serde_json::to_value(&updated).unwrap();
        assert_eq!(updated_wire.as_object().unwrap().len(), 5);
        assert!(update(scope, edit(&created, "stale")).is_err());
        assert!(delete(scope, deletion(&created)).is_err());
        delete(scope, deletion(&updated)).unwrap();
        assert!(!updated.source_path.exists());
    }
    assert_eq!(fs::read(paths.opencode_file()).unwrap(), original);
    assert!(!root.join(".opencode/opencode.jsonc").exists());
}

#[test]
fn scans_singular_plural_override_and_diagnostics_without_cascading_get_failure() {
    let home = tempfile::tempdir().unwrap();
    let override_dir = home.path().join("override");
    let paths = ConfigPaths::with_parts(
        home.path().into(),
        Some(home.path().join("elsewhere/config.jsonc")),
        Some(override_dir.clone()),
    );
    for dir in paths.command_dirs() {
        write(
            &dir.join("team/review.v2.md"),
            "---\ndescription: 'Review: all'\n---\n$ARGUMENTS",
        );
        write(&dir.join("broken.md"), "---\nbroken: [\n---\nbody");
        write(&dir.join("ignore.txt"), "ignore");
    }
    let list = list(&paths).unwrap();
    assert_eq!(list.data.len(), 4);
    assert_eq!(list.diagnostics.len(), 4);
    assert!(list.data.iter().all(|entry| entry.name == "team/review.v2"));
    let entry = &list.data[3];
    assert_eq!(get(&paths, &entry.id).unwrap(), *entry);
    let created = create(&paths, draft("review.v2.md", "literal suffix")).unwrap();
    assert!(created.source_path.ends_with("commands/review.v2.md.md"));
    assert_eq!(
        created.source_path,
        override_dir.join("commands/review.v2.md.md")
    );
    assert!(!paths.opencode_file().exists());
}

#[test]
fn invalid_names_and_outside_ids_do_not_touch_files() {
    let home = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(home.path());
    for name in [
        "",
        ".",
        "..",
        "../escape",
        "a/../../escape",
        "/absolute",
        "a//b",
        "a/",
        "C:/drive",
        "C:\\drive",
        "a\\b",
        "bad\0name",
    ] {
        assert!(create(&paths, draft(name, "body")).is_err(), "{name:?}");
    }
    let outside = home.path().join("outside.md");
    write(&outside, "outside");
    assert!(get(&paths, outside.to_str().unwrap()).is_err());
    assert!(delete(
        &paths,
        SlashCommandDelete {
            id: outside.to_str().unwrap().into(),
            expected_path: fs::canonicalize(&outside).unwrap(),
            expected_content: "outside".into(),
        }
    )
    .is_err());
    assert_eq!(fs::read_to_string(outside).unwrap(), "outside");
    assert!(!paths.commands_write_dir().exists());
}

#[test]
fn command_depth_matches_discovery_and_rejects_before_directory_creation() {
    let home = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(home.path());
    for depth in [64, 65] {
        let name = format!("{}review", "a/".repeat(depth));
        let error = create(&paths, draft(&name, "body")).unwrap_err();
        assert_eq!(error.code, crate::error::ErrorCode::ValidationFailed);
        assert_eq!(fs::read_dir(home.path()).unwrap().count(), 0);
    }

    let name = format!("{}review", "a/".repeat(63));
    let created = create(&paths, draft(&name, "body")).unwrap();
    assert_eq!(created.name, name);
    let listed = list(&paths).unwrap();
    assert_eq!(listed.data, vec![created.clone()]);
    assert!(listed.diagnostics.is_empty());
    assert_eq!(get(&paths, &created.id).unwrap(), created);
    let updated = update(&paths, edit(&created, "updated")).unwrap();
    assert_eq!(get(&paths, &updated.id).unwrap(), updated);
    assert_eq!(updated.content, "updated");
    delete(&paths, deletion(&updated)).unwrap();
    assert!(!updated.source_path.exists());
    assert!(list(&paths).unwrap().data.is_empty());
}

#[test]
fn basename_byte_limit_reserves_staging_and_backup_names() {
    let home = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(home.path());
    for name in ["a".repeat(215), format!("{}a", "é".repeat(107))] {
        assert_eq!(name.len(), 215);
        let error = create(&paths, draft(&name, "body")).unwrap_err();
        assert_eq!(error.code, crate::error::ErrorCode::ValidationFailed);
        assert_eq!(fs::read_dir(home.path()).unwrap().count(), 0);
    }
    assert!(validate_name(&format!("{}/review", "a".repeat(240))).is_ok());
    assert!(validate_name(&format!("{}/review", "a".repeat(241))).is_err());

    for name in ["a".repeat(214), "é".repeat(107)] {
        assert_eq!(name.len(), 214);
        let created = create(&paths, draft(&name, "body")).unwrap();
        assert_eq!(created.name, name);
        assert_eq!(list(&paths).unwrap().data, vec![created.clone()]);
        assert_eq!(get(&paths, &created.id).unwrap(), created);
        let updated = update(&paths, edit(&created, "updated")).unwrap();
        assert_eq!(updated.content, "updated");
        delete(&paths, deletion(&updated)).unwrap();
        assert_eq!(fs::read_dir(paths.commands_write_dir()).unwrap().count(), 0);
    }
}

#[cfg(unix)]
#[test]
fn symlink_delete_removes_only_entry_and_retargeted_drafts_fail() {
    use std::os::unix::fs::symlink;
    let home = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(home.path());
    let shared = home.path().join("shared.md");
    write(&shared, "shared body");
    let entry_path = paths.commands_write_dir().join("review.md");
    fs::create_dir_all(entry_path.parent().unwrap()).unwrap();
    symlink(&shared, &entry_path).unwrap();
    let entry = list(&paths)
        .unwrap()
        .data
        .into_iter()
        .find(|entry| entry.source_path == entry_path)
        .unwrap();
    let updated = update(&paths, edit(&entry, "new body")).unwrap();
    assert!(fs::symlink_metadata(&entry_path)
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(fs::read_to_string(&shared).unwrap(), "new body");
    let other = home.path().join("other.md");
    write(&other, "new body");
    fs::remove_file(&entry_path).unwrap();
    symlink(&other, &entry_path).unwrap();
    assert!(update(&paths, edit(&updated, "stale")).is_err());
    assert!(delete(&paths, deletion(&updated)).is_err());
    let fresh = get(&paths, &entry.id).unwrap();
    delete(&paths, deletion(&fresh)).unwrap();
    assert!(!entry_path.exists());
    assert_eq!(fs::read_to_string(shared).unwrap(), "new body");
    assert_eq!(fs::read_to_string(other).unwrap(), "new body");
}

#[cfg(unix)]
#[test]
fn project_command_aliases_cannot_expose_or_mutate_readonly_resources() {
    use std::os::unix::fs::symlink;

    for resource in [
        ".opencode/agent",
        ".opencode/agents",
        ".opencode/skill",
        ".opencode/skills",
        ".opencode/plugins",
        "opencode.json",
        "opencode.jsonc",
        ".opencode/opencode.json",
        ".opencode/opencode.jsonc",
    ] {
        let config = resource.ends_with(".json") || resource.ends_with(".jsonc");
        for command_dir in ["command", "commands"] {
            for alias in ["root", "nested", "leaf"] {
                if config && alias != "leaf" {
                    continue;
                }
                let home = tempfile::tempdir().unwrap();
                let paths = ConfigPaths::for_home(home.path());
                let root = home.path().join("project");
                fs::create_dir(&root).unwrap();
                let project =
                    crate::models::add_project(&paths.config_file(), root.to_str().unwrap())
                        .unwrap();
                let scoped = paths.for_project(Some(project.id)).unwrap();
                let root = scoped.project_directory().unwrap();
                let commands = root.join(".opencode").join(command_dir);
                let name = if alias == "nested" {
                    "team/review"
                } else {
                    "review"
                };
                let source = commands.join(format!("{name}.md"));
                write(&source, "body");
                let entry = list(&scoped)
                    .unwrap()
                    .data
                    .into_iter()
                    .find(|entry| entry.source_path == source)
                    .unwrap();
                fs::remove_file(&source).unwrap();
                let target = if config {
                    root.join(resource)
                } else {
                    root.join(resource).join("review.md")
                };
                write(&target, "body");
                let link = match alias {
                    "root" => commands.clone(),
                    "nested" => commands.join("team"),
                    _ => source.clone(),
                };
                if alias != "leaf" {
                    fs::remove_dir(&link).unwrap();
                }
                let destination = if alias == "leaf" {
                    &target
                } else {
                    target.parent().unwrap()
                };
                symlink(destination, &link).unwrap();
                let before = fs::read_dir(target.parent().unwrap()).unwrap().count();
                assert!(
                    list(&scoped).unwrap().data.is_empty(),
                    "{resource}/{command_dir}/{alias}"
                );
                assert!(get(&scoped, &entry.id).is_err());
                assert_eq!(
                    scoped.validate_managed_write(&source).unwrap_err().code,
                    crate::error::ErrorCode::ValidationFailed
                );
                let new_name = if alias == "nested" {
                    "team/new/nested"
                } else {
                    "new/nested"
                };
                if command_dir == "commands" && alias != "leaf" {
                    assert_eq!(
                        create(&scoped, draft(new_name, "ours")).unwrap_err().code,
                        crate::error::ErrorCode::ValidationFailed
                    );
                }
                if command_dir == "commands" {
                    assert!(create(&scoped, draft(name, "ours")).is_err());
                }
                assert!(update(&scoped, edit(&entry, "ours")).is_err());
                assert!(delete(&scoped, deletion(&entry)).is_err());
                assert_eq!(fs::read_to_string(&target).unwrap(), "body");
                assert_eq!(
                    fs::read_dir(target.parent().unwrap()).unwrap().count(),
                    before
                );
                assert!(fs::symlink_metadata(&link)
                    .unwrap()
                    .file_type()
                    .is_symlink());
            }
        }
    }
}

#[cfg(unix)]
#[test]
fn project_command_to_command_aliases_support_crud() {
    use std::os::unix::fs::symlink;

    for alias in ["root", "nested", "leaf"] {
        let home = tempfile::tempdir().unwrap();
        let paths = ConfigPaths::for_home(home.path());
        let root = home.path().join("project");
        fs::create_dir(&root).unwrap();
        let project =
            crate::models::add_project(&paths.config_file(), root.to_str().unwrap()).unwrap();
        let scoped = paths.for_project(Some(project.id)).unwrap();
        let singular = scoped.command_dirs()[0].clone();
        fs::create_dir_all(&singular).unwrap();
        let name = if alias == "nested" {
            "team/review"
        } else {
            "review"
        };
        let source = scoped.commands_write_dir().join(format!("{name}.md"));
        let link = match alias {
            "root" => scoped.commands_write_dir(),
            "nested" => scoped.commands_write_dir().join("team"),
            _ => source.clone(),
        };
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        let target = singular.join("target.md");
        if alias == "leaf" {
            write(&target, "body");
        }
        symlink(if alias == "leaf" { &target } else { &singular }, &link).unwrap();
        if alias != "leaf" {
            create(&scoped, draft(name, "body")).unwrap();
        }
        let entry = list(&scoped)
            .unwrap()
            .data
            .into_iter()
            .find(|entry| entry.source_path == source)
            .unwrap();
        let updated = update(&scoped, edit(&entry, "updated")).unwrap();
        assert_eq!(updated.content, "updated");
        delete(&scoped, deletion(&updated)).unwrap();
        assert!(!source.exists());
        if alias == "leaf" {
            assert_eq!(fs::read_to_string(target).unwrap(), "updated");
        }
    }
}

#[cfg(unix)]
#[test]
fn project_command_retarget_into_agents_is_rejected_before_backups() {
    use std::os::unix::fs::symlink;

    let home = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(home.path());
    let root = home.path().join("project");
    fs::create_dir(&root).unwrap();
    let project = crate::models::add_project(&paths.config_file(), root.to_str().unwrap()).unwrap();
    let scoped = paths.for_project(Some(project.id)).unwrap();
    let command = create(&scoped, draft("review", "body")).unwrap();
    let agents = scoped.project_directory().unwrap().join(".opencode/agents");
    write(&agents.join("review.md"), "body");
    let original = scoped.command_dirs()[0].clone();
    assert!(update_inner(&scoped, edit(&command, "ours"), |_| {
        fs::rename(scoped.commands_write_dir(), &original).unwrap();
        symlink(&agents, scoped.commands_write_dir()).unwrap();
        Ok(())
    })
    .is_err());
    for directory in [&original, &agents] {
        assert_eq!(
            fs::read_to_string(directory.join("review.md")).unwrap(),
            "body"
        );
        assert_eq!(fs::read_dir(directory).unwrap().count(), 1);
    }
    assert!(create(&scoped, draft("new/nested", "ours")).is_err());
    assert!(delete(&scoped, deletion(&command)).is_err());
    assert!(!agents.join("new").exists());
}

#[cfg(unix)]
#[test]
fn cycles_are_bounded_and_project_escapes_fail_before_backups() {
    use std::os::unix::fs::symlink;
    let home = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(home.path());
    let root = home.path().join("project");
    fs::create_dir(&root).unwrap();
    let project = crate::models::add_project(&paths.config_file(), root.to_str().unwrap()).unwrap();
    let scoped = paths.for_project(Some(project.id)).unwrap();
    let command = create(&scoped, draft("review", "body")).unwrap();
    symlink(
        scoped.commands_write_dir(),
        scoped.commands_write_dir().join("cycle"),
    )
    .unwrap();
    assert_eq!(list(&scoped).unwrap().data.len(), 1);
    let outside = home.path().join("outside.md");
    write(&outside, "body");
    fs::remove_file(&command.source_path).unwrap();
    symlink(&outside, &command.source_path).unwrap();
    assert!(get(&scoped, &command.id).is_err());
    assert!(update(&scoped, edit(&command, "clobber")).is_err());
    assert!(delete(&scoped, deletion(&command)).is_err());
    assert_eq!(fs::read_to_string(&outside).unwrap(), "body");
    assert!(!fs::read_dir(home.path()).unwrap().any(|entry| entry
        .unwrap()
        .path()
        .extension()
        .is_some_and(|extension| extension == "bak")));
    fs::remove_file(&command.source_path).unwrap();
    fs::remove_file(scoped.commands_write_dir().join("cycle")).unwrap();
    fs::remove_dir(scoped.commands_write_dir()).unwrap();
    let outside_dir = home.path().join("outside-dir");
    fs::create_dir(&outside_dir).unwrap();
    symlink(&outside_dir, scoped.commands_write_dir()).unwrap();
    assert!(create(&scoped, draft("escape", "body")).is_err());
    assert!(!outside_dir.join("escape.md").exists());
}

#[test]
fn checked_create_rejects_competing_file_without_clobber() {
    let home = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(home.path());
    assert!(create_inner(&paths, draft("review", "ours"), |path| {
        fs::write(path, "theirs").unwrap();
        Ok(())
    })
    .is_err());
    assert_eq!(
        fs::read_to_string(paths.commands_write_dir().join("review.md")).unwrap(),
        "theirs"
    );
}

#[test]
fn invalid_frontmatter_and_empty_templates_fail_before_creation() {
    let home = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(home.path());
    for content in [
        "",
        "   \n",
        "---\ndescription: missing end",
        "---\ndescription: 1\n---\nbody",
        "---\nagent: [build]\n---\nbody",
        "---\nmodel: false\n---\nbody",
        "---\nsubagent: yes\n---\nbody",
        "---\ndescription: valid\n---\n\n",
    ] {
        assert!(
            create(&paths, draft("review", content)).is_err(),
            "{content:?}"
        );
    }
    assert!(!paths.commands_write_dir().exists());
}

#[cfg(unix)]
#[test]
fn project_alias_escape_is_rejected_before_directory_creation_or_backup() {
    use std::os::unix::fs::symlink;

    let home = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(home.path());
    let root = home.path().join("project");
    fs::create_dir(&root).unwrap();
    let project = crate::models::add_project(&paths.config_file(), root.to_str().unwrap()).unwrap();
    let scoped = paths.for_project(Some(project.id)).unwrap();
    let command = create(&scoped, draft("review", "body")).unwrap();
    let outside = home.path().join("outside");
    fs::create_dir(&outside).unwrap();
    write(&outside.join("review.md"), "body");
    let original = scoped
        .commands_write_dir()
        .with_file_name("original-commands");

    assert!(update_inner(&scoped, edit(&command, "ours"), |_| {
        fs::rename(scoped.commands_write_dir(), &original).unwrap();
        symlink(&outside, scoped.commands_write_dir()).unwrap();
        Ok(())
    })
    .is_err());
    assert_eq!(
        fs::read_to_string(original.join("review.md")).unwrap(),
        "body"
    );
    assert_eq!(
        fs::read_to_string(outside.join("review.md")).unwrap(),
        "body"
    );
    assert_eq!(fs::read_dir(&original).unwrap().count(), 1);
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 1);

    assert!(create(&scoped, draft("new/nested", "ours")).is_err());
    assert!(!outside.join("new").exists());
    assert!(delete(&scoped, deletion(&command)).is_err());
    assert!(outside.join("review.md").exists());
}

#[test]
fn checked_replacement_runs_guards_before_backup_and_before_publication() {
    let home = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(home.path());
    let entry = create(&paths, draft("review", "body")).unwrap();
    let snapshot = Snapshot::read(&entry.source_path).unwrap().unwrap();
    let parent = entry.source_path.parent().unwrap();

    assert!(
        resource_file::replace_checked(&[(snapshot.clone(), "ours".into())], |_| {
            assert_eq!(fs::read_dir(parent).unwrap().count(), 1);
            Err(AppError::validation("blocked before staging"))
        })
        .is_err()
    );
    assert_eq!(fs::read_dir(parent).unwrap().count(), 1);

    let mut staged = false;
    assert!(
        resource_file::replace_checked(&[(snapshot, "ours".into())], |_| {
            if fs::read_dir(parent).unwrap().count() > 1 {
                staged = true;
                return Err(AppError::validation("blocked before publication"));
            }
            Ok(())
        })
        .is_err()
    );
    assert!(staged);
    assert_eq!(fs::read_to_string(&entry.source_path).unwrap(), "body");
    let leftovers = fs::read_dir(parent)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(leftovers.len(), 2);
    let backup = leftovers
        .iter()
        .find(|path| path.extension().is_some_and(|extension| extension == "bak"))
        .unwrap();
    assert_eq!(fs::read_to_string(backup).unwrap(), "body");
}
