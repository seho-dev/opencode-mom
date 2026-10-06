use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use opencode_mom_tauri::document::{write_file, JsoncDoc};
use opencode_mom_tauri::error::ErrorCode;
use opencode_mom_tauri::paths::ConfigPaths;
use opencode_mom_tauri::{mcp, skills};
use serde_json::json;

struct Home(PathBuf);

impl Home {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("opencode-mom-resources-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn paths(&self) -> ConfigPaths {
        ConfigPaths::with_resource_parts(
            self.0.clone(),
            None,
            None,
            self.0.join(".config"),
            self.0.join("cwd"),
        )
    }

    fn write(&self, relative: &str, content: &str) -> PathBuf {
        let path = self.0.join(relative);
        write_file(&path, content.as_bytes()).unwrap();
        path
    }
}

impl Drop for Home {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn global_paths_include_all_sources_in_override_order_and_anchor_relative_skills() {
    let home = Home::new();
    let paths = ConfigPaths::with_resource_parts(
        home.0.clone(),
        Some(PathBuf::from("override.jsonc")),
        Some(PathBuf::from("extra")),
        home.0.join("xdg"),
        home.0.join("cwd"),
    );
    assert_eq!(
        paths.global_config_files(),
        &[
            home.0.join("xdg/opencode/opencode.json"),
            home.0.join("xdg/opencode/opencode.jsonc"),
            home.0.join("cwd/extra/opencode.json"),
            home.0.join("cwd/extra/opencode.jsonc"),
            home.0.join("cwd/override.jsonc"),
        ]
    );
    assert_eq!(paths.opencode_file(), home.0.join("cwd/override.jsonc"));
    assert_eq!(paths.resolve_skill_path("~/custom"), home.0.join("custom"));
    assert_eq!(
        paths.resolve_skill_path("relative"),
        home.0.join("cwd/relative")
    );
    assert_eq!(
        paths.resolve_skill_path(&home.0.display().to_string()),
        home.0
    );
    assert_eq!(
        paths.global_skill_dirs(),
        &[
            home.0.join(".claude/skills"),
            home.0.join(".agents/skills"),
            home.0.join("xdg/opencode/skills"),
            home.0.join("cwd/extra/skills"),
        ]
    );
}

#[test]
fn explicit_selected_config_is_managed_but_unselected_projects_are_not_discovered() {
    let home = Home::new();
    let managed = home.0.join(".config/opencode/opencode.json");
    let paths = ConfigPaths::with_resource_parts(
        home.0.clone(),
        Some(managed.clone()),
        None,
        home.0.join(".config"),
        home.0.join("cwd"),
    );
    assert_eq!(
        paths.global_config_files(),
        &[home.0.join(".config/opencode/opencode.jsonc"), managed]
    );
    home.write(
        ".config/opencode/opencode.json",
        r#"{"mcp":{"servers":{"shared":{"type":"local","command":["global"]}}},"skills":["relative"]}"#,
    );
    home.write("cwd/relative/Global.md", "global configured skill");
    let project = home.write(
        "cwd/opencode.jsonc",
        r#"{"mcp":{"servers":{"shared":{"type":"local","command":["project"]},"project-only":{"type":"local","command":["project"]}}},"skills":["project-skills"]}"#,
    );
    home.write("cwd/project-skills/Project.md", "project configured skill");
    let before = fs::read_to_string(&project).unwrap();
    let paths = ConfigPaths::with_resource_parts(
        home.0.clone(),
        None,
        None,
        home.0.join(".config"),
        home.0.join("cwd"),
    );
    let servers = mcp::list(&paths).unwrap();
    assert_eq!(servers.data.len(), 1);
    assert_eq!(servers.data[0].target, "global");
    let listed_skills = skills::list(&paths).unwrap();
    assert_eq!(listed_skills.data.len(), 1);
    assert_eq!(listed_skills.data[0].id, "Global");
    mcp::delete(&paths, "shared").unwrap();
    assert_eq!(fs::read_to_string(&project).unwrap(), before);
    assert_eq!(
        mcp::delete(&paths, "project-only").unwrap_err().code,
        ErrorCode::NotFound
    );
    assert_eq!(fs::read_to_string(&project).unwrap(), before);
    home.write(".config/opencode/opencode.json", r#"{"mcp":{"servers":{"shared":{"type":"local","command":["global"]}}},"skills":["relative"]}"#);
    let selected = ConfigPaths::with_resource_parts(
        home.0.clone(),
        Some(PathBuf::from("opencode.jsonc")),
        None,
        home.0.join(".config"),
        home.0.join("cwd"),
    );
    assert_eq!(selected.opencode_file(), project);
    let shared = mcp::get(&selected, "shared").unwrap();
    assert_eq!(shared.target, "project");
    assert_eq!(shared.source_path, project.display().to_string());
    let listed = skills::list(&selected).unwrap();
    assert_eq!(
        listed
            .data
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        ["Global", "Project"]
    );
    let current = skills::get(&selected, "Project").unwrap();
    skills::update(&selected, skill_update(&current, "selected source edit")).unwrap();
    assert_eq!(
        fs::read_to_string(home.0.join("cwd/project-skills/Project.md")).unwrap(),
        "selected source edit"
    );
    mcp::delete(&selected, "shared").unwrap();
    assert!(mcp::get(&selected, "shared").is_err());
    assert_ne!(fs::read_to_string(&project).unwrap(), before);
}

#[test]
fn mcp_overlay_reports_provenance_and_delete_removes_every_shadow_without_touching_projects() {
    let home = Home::new();
    let json = home.write(".config/opencode/opencode.json", r#"{
  // unrelated comment
  "model": "keep/me",
  "mcp": {"servers": {
    "shared": {"type":"local", "command":["node","old.js"], "environment":{"KEEP":"yes","OVERRIDE":"old"}},
    "remote": {"type":"remote", "url":"https://example.test/mcp", "disabled":true, "oauth":false}
  }}
}"#);
    let jsonc = home.write(
        ".config/opencode/opencode.jsonc",
        r#"{
  /* preserved */
  "mcp": {"servers": {"shared": {"type":"local", "command":["node","new.js"], "environment":{"OVERRIDE":"new"}}}},
  "other": {"raw": 1,},
}"#,
    );
    let project = home.write(
        "cwd/opencode.jsonc",
        r#"{"mcp":{"servers":{"shared":{"type":"remote","url":"https://project.test"}}}}"#,
    );
    let project_before = fs::read_to_string(&project).unwrap();
    let listed = mcp::list(&home.paths()).unwrap();
    assert!(listed.diagnostics.is_empty());
    assert_eq!(listed.data.len(), 2);
    let shared = mcp::get(&home.paths(), "shared").unwrap();
    assert_eq!(shared.r#type, "local");
    assert_eq!(shared.target, "node new.js");
    assert!(!shared.disabled);
    assert_eq!(shared.source_path, jsonc.display().to_string());
    assert_eq!(
        shared.source_paths,
        vec![json.display().to_string(), jsonc.display().to_string()]
    );
    assert_eq!(shared.config["environment"], json!({"OVERRIDE":"new"}));
    assert_eq!(shared.config["command"], json!(["node", "new.js"]));
    let remote = mcp::get(&home.paths(), "remote").unwrap();
    assert!(remote.disabled);
    assert_eq!(remote.config["oauth"], false);
    // A fresh read retains changes made after the listing.
    let fresh = fs::read_to_string(&jsonc)
        .unwrap()
        .replace("\"raw\": 1", "\"raw\": 2");
    fs::write(&jsonc, &fresh).unwrap();
    mcp::delete(&home.paths(), "shared").unwrap();
    assert_eq!(
        mcp::get(&home.paths(), "shared").unwrap_err().code,
        ErrorCode::NotFound
    );
    let after_json = fs::read_to_string(&json).unwrap();
    let after_jsonc = fs::read_to_string(&jsonc).unwrap();
    assert!(after_json.contains("// unrelated comment"));
    assert!(after_jsonc.contains("/* preserved */"));
    assert_eq!(
        JsoncDoc::parse(&after_jsonc).unwrap().raw()["other"]["raw"],
        2
    );
    assert_eq!(mcp::list(&home.paths()).unwrap().data.len(), 1);
    assert_eq!(fs::read_to_string(project).unwrap(), project_before);
    let backups = fs::read_dir(json.parent().unwrap())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "bak"))
        .collect::<Vec<_>>();
    assert!(backups.is_empty());
    assert!(!fs::read_dir(json.parent().unwrap())
        .unwrap()
        .any(|entry| entry
            .unwrap()
            .path()
            .extension()
            .is_some_and(|extension| extension == "tmp")));
}

#[test]
fn mcp_replacements_do_not_inherit_local_fields_or_validate_using_shadow_defaults() {
    let home = Home::new();
    let lower = home.write(
        ".config/opencode/opencode.json",
        r#"{"mcp":{"servers":{"shared":{"type":"local","command":["old"],"environment":{"OLD":"yes"},"disabled":true}}}}"#,
    );
    let higher = home.write(
        ".config/opencode/opencode.jsonc",
        r#"{"mcp":{"servers":{"shared":{"type":"remote","url":"https://replacement.test/mcp"}}}}"#,
    );
    let server = mcp::get(&home.paths(), "shared").unwrap();
    assert_eq!(server.r#type, "remote");
    assert!(!server.disabled);
    assert_eq!(
        server.config,
        json!({"type":"remote","url":"https://replacement.test/mcp"})
            .as_object()
            .unwrap()
            .clone()
    );
    assert_eq!(
        server.source_paths,
        vec![lower.display().to_string(), higher.display().to_string()]
    );
    for replacement in [
        json!({"url":"https://invalid.test"}),
        json!({"type":"local"}),
        json!(false),
    ] {
        fs::write(
            &higher,
            json!({"mcp":{"servers":{"shared":replacement}}}).to_string(),
        )
        .unwrap();
        let listed = mcp::list(&home.paths()).unwrap();
        assert!(listed.data.is_empty());
        assert_eq!(listed.diagnostics.len(), 1);
        assert!(listed.diagnostics[0].contains("shared"));
    }
    mcp::delete(&home.paths(), "shared").unwrap();
    assert!(mcp::list(&home.paths()).unwrap().data.is_empty());
}

#[test]
fn malformed_or_unreadable_global_document_blocks_delete_before_any_write() {
    let home = Home::new();
    let json = home.write(
        ".config/opencode/opencode.json",
        r#"{"mcp":{"servers":{"ok":{"type":"local","command":["ok"]}}}}"#,
    );
    let original = fs::read_to_string(&json).unwrap();
    let malformed = home.write(".config/opencode/opencode.jsonc", "{broken");
    let list = mcp::list(&home.paths()).unwrap();
    assert_eq!(list.data.len(), 1);
    assert_eq!(list.diagnostics.len(), 1);
    assert!(mcp::delete(&home.paths(), "ok").is_err());
    assert_eq!(fs::read_to_string(&json).unwrap(), original);
    fs::remove_file(&malformed).unwrap();
    fs::create_dir(&malformed).unwrap();
    assert!(mcp::delete(&home.paths(), "ok").is_err());
    assert_eq!(fs::read_to_string(&json).unwrap(), original);
    assert_eq!(fs::read_dir(json.parent().unwrap()).unwrap().count(), 2);
}

#[test]
fn skill_discovery_has_case_sensitive_file_ids_precedence_frontmatter_and_diagnostics() {
    let home = Home::new();
    home.write(".claude/skills/Shared.md", "claude");
    home.write(".claude/skills/OnlyClaude.md", "claude only");
    home.write(".agents/skills/Shared.md", "agents");
    home.write(".config/opencode/skills/Shared.md", "opencode");
    home.write("lowercase-source/shared.md", "lowercase");
    home.write(".config/opencode/skills/SKILL.md", "root skill");
    home.write(".config/opencode/skills/nested/Directory/SKILL.md", "---\nname: Display Name\ndescription: A description\nmetadata:\n  opencode/autoinvoke: false\n---\n# Directory body\n");
    home.write(
        ".config/opencode/skills/nested/ignored.md",
        "not a skill entry",
    );
    home.write(
        ".config/opencode/skills/bad.md",
        "---\nname: [\n---\ncorrupt YAML",
    );
    home.write(".config/opencode/skills/unclosed.md", "---\nname: Broken\n");
    home.write("first/Shared.md", "first configured");
    home.write("cwd/second/Shared.md", "---\nname: Final Display\nmetadata:\n  opencode/autoinvoke: 'false'\n---\nsecond configured");
    home.write(
        ".config/opencode/opencode.json",
        r#"{"skills":["~/first"]}"#,
    );
    home.write(
        ".config/opencode/opencode.jsonc",
        r#"{"skills":["second","~/lowercase-source","missing"]}"#,
    );
    home.write("cwd/.opencode/skills/Project.md", "project must not appear");
    let paths = home.paths();
    let list = skills::list(&paths).unwrap();
    assert_eq!(
        list.data
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        vec!["Directory", "OnlyClaude", "SKILL", "Shared", "shared"]
    );
    assert_eq!(list.diagnostics.len(), 2);
    let shared = skills::get(&paths, "Shared").unwrap();
    assert_eq!(shared.kind, skills::SkillKind::Local);
    assert_eq!(shared.name, "Final Display");
    assert!(!shared.autoinvoke);
    assert_eq!(
        shared.source,
        home.0.join("cwd/second").display().to_string()
    );
    assert!(shared.content.contains("metadata:"));
    assert_eq!(skills::get(&paths, "shared").unwrap().content, "lowercase");
    let directory = skills::get(&paths, "Directory").unwrap();
    assert_eq!(directory.description.as_deref(), Some("A description"));
    assert!(!directory.autoinvoke);
    assert!(skills::get(&paths, "SKILL").unwrap().autoinvoke);
    assert_eq!(
        skills::get(&paths, "Display Name").unwrap_err().code,
        ErrorCode::NotFound
    );
    assert_eq!(
        skills::get(&paths, "../../first/Shared.md")
            .unwrap_err()
            .code,
        ErrorCode::NotFound
    );
    home.write("cwd/second/Shared.md", "fresh body");
    assert_eq!(skills::get(&paths, "Shared").unwrap().content, "fresh body");
}

#[test]
fn get_skill_reports_invalid_requested_winners_without_hiding_unrelated_valid_details() {
    let home = Home::new();
    home.write(".claude/skills/Shared.md", "lower-priority body");
    home.write(".claude/skills/Visible.md", "visible");
    let winner = home.write(".config/opencode/skills/Shared.md", "---\nname: Broken\n");
    let only_invalid = home.write(
        ".config/opencode/skills/Broken.md",
        "---\nname: [\n---\ninvalid YAML",
    );
    let error = skills::get(&home.paths(), "Shared").unwrap_err();
    assert_eq!(error.code, ErrorCode::ValidationFailed);
    assert!(error.message.contains(&winner.display().to_string()));
    assert!(error.message.contains("unclosed skill frontmatter"));
    let error = skills::get(&home.paths(), "Broken").unwrap_err();
    assert_eq!(error.code, ErrorCode::ValidationFailed);
    assert!(error.message.contains(&only_invalid.display().to_string()));
    fs::write(&winner, [0xff]).unwrap();
    let error = skills::get(&home.paths(), "Shared").unwrap_err();
    assert_eq!(error.code, ErrorCode::ConfigurationFailed);
    assert!(error.message.contains(&winner.display().to_string()));
    assert_eq!(
        skills::get(&home.paths(), "Visible").unwrap().content,
        "visible"
    );
    home.write("last/Shared.md", "valid final winner");
    home.write(
        ".config/opencode/opencode.jsonc",
        r#"{"skills":["~/last"]}"#,
    );
    assert_eq!(
        skills::get(&home.paths(), "Shared").unwrap().content,
        "valid final winner"
    );
}

#[cfg(unix)]
#[test]
fn get_skill_reports_broken_requested_symlink_instead_of_returning_shadow() {
    use std::os::unix::fs::symlink;
    let home = Home::new();
    home.write(".claude/skills/Shared.md", "lower-priority body");
    let path = home.0.join(".config/opencode/skills/Shared.md");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    symlink(home.0.join("missing.md"), &path).unwrap();
    let error = skills::get(&home.paths(), "Shared").unwrap_err();
    assert_eq!(error.code, ErrorCode::ConfigurationFailed);
    assert!(error.message.contains(&path.display().to_string()));
}

#[cfg(unix)]
#[test]
fn skill_symlinks_are_followed_without_recursion_loops_and_use_link_directory_ids() {
    use std::os::unix::fs::symlink;
    let home = Home::new();
    home.write("installed/SKILL.md", "symlink body");
    home.write(".config/opencode/skills/ordinary.md", "ordinary");
    let root = home.0.join(".config/opencode/skills");
    symlink(home.0.join("installed"), root.join("Linked")).unwrap();
    symlink(&root, home.0.join("installed/loop")).unwrap();
    let list = skills::list(&home.paths()).unwrap();
    assert_eq!(
        list.data
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        vec!["Linked", "ordinary"]
    );
    assert_eq!(
        list.data[0].path,
        root.join("Linked/SKILL.md").display().to_string()
    );
    assert_eq!(list.diagnostics.len(), 1);
    assert!(list.diagnostics[0].contains("recursion loop"));
}

fn serve(
    responses: Vec<(&str, u16, String, Option<usize>)>,
) -> (String, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let prefix = format!("/catalog/{}", uuid::Uuid::new_v4());
    let url = format!("http://{}{prefix}/", listener.local_addr().unwrap());
    let responses = responses
        .into_iter()
        .map(|(path, status, body, length)| (path.to_owned(), (status, body, length)))
        .collect::<BTreeMap<_, _>>();
    let expected = responses.len();
    let handle = thread::spawn(move || {
        let mut requests = Vec::new();
        for _ in 0..expected {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            loop {
                let mut buffer = [0; 1024];
                let count = stream.read(&mut buffer).unwrap();
                assert_ne!(count, 0);
                request.extend_from_slice(&buffer[..count]);
                if request.windows(4).any(|part| part == b"\r\n\r\n") {
                    break;
                }
            }
            let request = String::from_utf8(request).unwrap();
            let path = format!(
                "/catalog{}",
                request
                    .split_whitespace()
                    .nth(1)
                    .unwrap()
                    .strip_prefix(&prefix)
                    .unwrap()
            );
            let (status, body, length) = responses.get(&path).unwrap();
            write!(
                stream,
                "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                length.unwrap_or(body.len())
            )
            .unwrap();
            requests.push(path);
        }
        requests
    });
    (url, handle)
}

fn configure_catalog(home: &Home, source: &str) {
    home.write(
        ".config/opencode/opencode.jsonc",
        &json!({"skills":[source]}).to_string(),
    );
}

fn assert_catalog_requests(mut actual: Vec<String>, expected: &[&str]) {
    assert_eq!(
        actual.first().map(String::as_str),
        Some("/catalog/index.json")
    );
    actual.sort();
    let mut expected = expected
        .iter()
        .map(|path| path.to_string())
        .collect::<Vec<_>>();
    expected.sort();
    assert_eq!(actual, expected);
}

#[test]
fn http_catalog_fetches_only_skill_markdown_and_keeps_url_provenance() {
    let home = Home::new();
    home.write(".config/opencode/skills/SKILL.md", "filesystem copy");
    let index = json!({"skills":[
        {"name":"Remote","version":"1.0","files":["script.sh","assets/logo.png","SKILL.md"]},
        {"name":"Later","version":"1.0","files":["SKILL.md"]},
        {"name":"Flat","version":"1.0","files":["Flat.md","ignore.py"]},
        {"name":"NoEntry","version":"1.0","files":["readme.md"]},
    ]})
    .to_string();
    let (url, server) = serve(vec![
        ("/catalog/index.json", 200, index, None),
        ("/catalog/Remote/SKILL.md", 200, "earlier body".to_owned(), None),
        ("/catalog/Later/SKILL.md", 200, "---\nname: Remote Display\nmetadata:\n  opencode/autoinvoke: 'false'\n---\nremote body".to_owned(), None),
        ("/catalog/Flat/Flat.md", 200, "flat body".to_owned(), None),
    ]);
    configure_catalog(&home, &url);
    let list = skills::list(&home.paths()).unwrap();
    assert!(list.diagnostics.is_empty(), "{:?}", list.diagnostics);
    assert_eq!(list.data.len(), 2);
    let remote = list.data.iter().find(|entry| entry.id == "SKILL").unwrap();
    assert_eq!(remote.kind, skills::SkillKind::Remote);
    assert_eq!(remote.path, format!("{url}Later/SKILL.md"));
    assert_eq!(remote.source, url);
    assert_eq!(remote.name, "Remote Display");
    assert!(!remote.autoinvoke);
    assert!(remote.content.ends_with("remote body"));
    let flat = list.data.iter().find(|entry| entry.id == "Flat").unwrap();
    assert_eq!(flat.path, format!("{url}Flat/Flat.md"));
    assert_eq!(flat.source, url);
    assert_catalog_requests(
        server.join().unwrap(),
        &[
            "/catalog/index.json",
            "/catalog/Remote/SKILL.md",
            "/catalog/Later/SKILL.md",
            "/catalog/Flat/Flat.md",
        ],
    );
}

#[test]
fn get_named_skill_resolves_entry_file_id_and_fetches_only_the_requested_body() {
    let home = Home::new();
    let index = json!({"skills":[
        {"name":"Wanted","version":"1","files":["Wanted.md"]},
        {"name":"Unrelated","version":"1","files":["SKILL.md"]},
    ]})
    .to_string();
    let (url, server) = serve(vec![
        ("/catalog/index.json", 200, index, None),
        (
            "/catalog/Wanted/Wanted.md",
            200,
            "wanted body".to_owned(),
            None,
        ),
    ]);
    configure_catalog(&home, &url);
    let skill = skills::get(&home.paths(), "Wanted").unwrap();
    assert_eq!(skill.id, "Wanted");
    assert_eq!(skill.content, "wanted body");
    assert_eq!(skill.path, format!("{url}Wanted/Wanted.md"));
    assert_eq!(skill.source, url);
    assert_eq!(
        server.join().unwrap(),
        vec!["/catalog/index.json", "/catalog/Wanted/Wanted.md"]
    );
}

#[test]
fn get_root_skill_reads_all_matching_catalog_entries_and_later_sources_win() {
    let home = Home::new();
    home.write(".config/opencode/skills/SKILL.md", "filesystem copy");
    let index = json!({"skills":[
        {"name":"First","files":["SKILL.md"]},
        {"name":"Second","files":["SKILL.md"]},
        {"name":"Other","files":["Other.md"]},
    ]})
    .to_string();
    let (first_url, first_server) = serve(vec![
        ("/catalog/index.json", 200, index, None),
        ("/catalog/First/SKILL.md", 200, "first".to_owned(), None),
        ("/catalog/Second/SKILL.md", 200, "second".to_owned(), None),
    ]);
    let (last_url, last_server) = serve(vec![
        (
            "/catalog/index.json",
            200,
            json!({"skills":[{"name":"Final","files":["SKILL.md"]}]}).to_string(),
            None,
        ),
        ("/catalog/Final/SKILL.md", 200, "final".to_owned(), None),
    ]);
    home.write(
        ".config/opencode/opencode.jsonc",
        &json!({"skills":[first_url,last_url]}).to_string(),
    );
    let entry = skills::get(&home.paths(), "SKILL").unwrap();
    assert_eq!(entry.content, "final");
    assert_eq!(entry.path, format!("{last_url}Final/SKILL.md"));
    assert_eq!(entry.source, last_url);
    assert_catalog_requests(
        first_server.join().unwrap(),
        &[
            "/catalog/index.json",
            "/catalog/First/SKILL.md",
            "/catalog/Second/SKILL.md",
        ],
    );
    assert_eq!(
        last_server.join().unwrap(),
        vec!["/catalog/index.json", "/catalog/Final/SKILL.md"]
    );
}

#[test]
fn get_skill_reports_matching_http_winner_failures_and_ignores_unrelated_catalog_entries() {
    for (status, body, expected) in [
        (200, "---\nname: Broken\n", ErrorCode::ValidationFailed),
        (404, "missing", ErrorCode::ConfigurationFailed),
    ] {
        let home = Home::new();
        home.write(".config/opencode/skills/SKILL.md", "filesystem shadow");
        let index = json!({"skills":[
            {"name":"First","files":["SKILL.md"]},
            {"name":"Broken","files":["SKILL.md"]},
            {"name":"Unrelated","files":["Unrelated.md","../unsafe"]},
        ]})
        .to_string();
        let (url, server) = serve(vec![
            ("/catalog/index.json", 200, index, None),
            (
                "/catalog/First/SKILL.md",
                200,
                "HTTP shadow".to_owned(),
                None,
            ),
            ("/catalog/Broken/SKILL.md", status, body.to_owned(), None),
        ]);
        configure_catalog(&home, &url);
        let error = skills::get(&home.paths(), "SKILL").unwrap_err();
        assert_eq!(error.code, expected);
        assert!(error.message.contains(&format!("{url}Broken/SKILL.md")));
        assert_catalog_requests(
            server.join().unwrap(),
            &[
                "/catalog/index.json",
                "/catalog/First/SKILL.md",
                "/catalog/Broken/SKILL.md",
            ],
        );
    }
    let home = Home::new();
    home.write(".config/opencode/skills/Visible.md", "visible");
    let (url, server) = serve(vec![(
        "/catalog/index.json",
        200,
        json!({"skills":[{"name":"Other","files":["SKILL.md","../unsafe"]}]}).to_string(),
        None,
    )]);
    configure_catalog(&home, &url);
    assert_eq!(
        skills::get(&home.paths(), "Visible").unwrap().content,
        "visible"
    );
    assert_eq!(server.join().unwrap(), vec!["/catalog/index.json"]);
}

#[test]
fn get_skill_fails_honestly_when_http_index_cannot_rule_out_a_requested_override() {
    for (status, body) in [(404, "missing"), (200, "{broken")] {
        for local in [false, true] {
            let home = Home::new();
            if local {
                home.write(".config/opencode/skills/Wanted.md", "possible shadow");
            }
            let (url, server) = serve(vec![("/catalog/index.json", status, body.to_owned(), None)]);
            configure_catalog(&home, &url);
            let error = skills::get(&home.paths(), "Wanted").unwrap_err();
            assert_ne!(error.code, ErrorCode::NotFound);
            assert!(error.message.contains(&url));
            assert_eq!(server.join().unwrap(), vec!["/catalog/index.json"]);
        }
    }
    let home = Home::new();
    home.write("last/Wanted.md", "verified final winner");
    let (url, server) = serve(vec![(
        "/catalog/index.json",
        404,
        "missing".to_owned(),
        None,
    )]);
    home.write(
        ".config/opencode/opencode.jsonc",
        &json!({"skills":[url,"~/last"]}).to_string(),
    );
    assert_eq!(
        skills::get(&home.paths(), "Wanted").unwrap().content,
        "verified final winner"
    );
    assert_eq!(server.join().unwrap(), vec!["/catalog/index.json"]);
}

#[test]
fn inaccessible_skill_source_and_malformed_config_leave_successful_sources_visible() {
    let home = Home::new();
    home.write(".claude/skills/Visible.md", "visible");
    home.write("not-a-directory", "plain file");
    home.write(
        ".config/opencode/opencode.json",
        r#"{"skills":["~/not-a-directory"]}"#,
    );
    home.write(".config/opencode/opencode.jsonc", "{malformed");
    let list = skills::list(&home.paths()).unwrap();
    assert_eq!(list.data.len(), 1);
    assert_eq!(list.data[0].id, "Visible");
    assert_eq!(list.diagnostics.len(), 2);
}

#[cfg(unix)]
#[test]
fn deleting_mcp_keeps_original_modes_and_removes_owned_backups_and_temp_files() {
    use std::os::unix::fs::PermissionsExt;
    for mode in [0o644, 0o600] {
        let home = Home::new();
        let original = r#"{
  // keep this comment
  "model":"keep",
  "mcp":{"servers":{"secret":{"type":"local","command":["run"],"environment":{"TOKEN":"private"}}}}
}"#;
        let path = home.write(".config/opencode/opencode.jsonc", original);
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
        mcp::delete(&home.paths(), "secret").unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            mode
        );
        let replacement = fs::read_to_string(&path).unwrap();
        assert!(replacement.contains("// keep this comment"));
        assert_eq!(
            JsoncDoc::parse(&replacement).unwrap().raw()["model"],
            "keep"
        );
        assert!(mcp::list(&home.paths()).unwrap().data.is_empty());
        let files = fs::read_dir(path.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        let backups = files
            .iter()
            .filter(|path| path.extension().is_some_and(|extension| extension == "bak"))
            .collect::<Vec<_>>();
        assert!(backups.is_empty());
        assert!(!files
            .iter()
            .any(|path| path.extension().is_some_and(|extension| extension == "tmp")));
    }
}

#[cfg(unix)]
#[test]
fn deleting_mcp_through_config_symlink_preserves_the_symlink() {
    use std::os::unix::fs::symlink;
    let home = Home::new();
    let real = home.write(
        "managed/config.jsonc",
        r#"{"mcp":{"servers":{"ok":{"type":"local","command":["ok"]}}}}"#,
    );
    let link = home.0.join(".config/opencode/opencode.jsonc");
    fs::create_dir_all(link.parent().unwrap()).unwrap();
    symlink(&real, &link).unwrap();
    mcp::delete(&home.paths(), "ok").unwrap();
    assert!(fs::symlink_metadata(&link)
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(
        fs::read_to_string(&link).unwrap(),
        fs::read_to_string(&real).unwrap()
    );
    assert!(mcp::list(&home.paths()).unwrap().data.is_empty());
}

#[test]
fn unsafe_http_catalog_paths_fail_the_source_before_fetching_entries() {
    for unsafe_file in [
        "../SKILL.md",
        "%2e%2e/SKILL.md",
        "%252e%252e/SKILL.md",
        "a/%2f..%2fSKILL.md",
        "https://other.test/SKILL.md",
        "//other.test/SKILL.md",
        "C:\\SKILL.md",
    ] {
        let home = Home::new();
        home.write(".config/opencode/skills/Kept.md", "kept");
        let index =
            json!({"skills":[{"name":"Remote","version":"1","files":["SKILL.md",unsafe_file]}]})
                .to_string();
        let (url, server) = serve(vec![("/catalog/index.json", 200, index, None)]);
        configure_catalog(&home, &url);
        let list = skills::list(&home.paths()).unwrap();
        assert_eq!(list.data.len(), 1);
        assert_eq!(list.data[0].id, "Kept");
        assert_eq!(list.diagnostics.len(), 1, "{unsafe_file}");
        assert!(
            list.diagnostics[0].contains("unsafe catalog path"),
            "{:?}",
            list.diagnostics
        );
        assert_eq!(server.join().unwrap(), vec!["/catalog/index.json"]);
    }
}

#[test]
fn failed_or_oversized_http_source_is_a_diagnostic_without_hiding_local_skills() {
    for (status, body, length) in [
        (302, String::new(), None),
        (404, "missing".to_owned(), None),
        (200, "{broken".to_owned(), None),
        (200, String::new(), Some(2 * 1024 * 1024 + 1)),
    ] {
        let home = Home::new();
        home.write(".claude/skills/Local.md", "local");
        let (url, server) = serve(vec![("/catalog/index.json", status, body, length)]);
        configure_catalog(&home, &url);
        let list = skills::list(&home.paths()).unwrap();
        assert_eq!(list.data.len(), 1);
        assert_eq!(list.data[0].id, "Local");
        assert_eq!(list.diagnostics.len(), 1);
        server.join().unwrap();
    }
}

#[test]
fn invalid_mcp_entries_produce_diagnostics_not_a_failed_catalog() {
    let home = Home::new();
    home.write(
        ".config/opencode/opencode.json",
        &json!({"mcp":{"servers":{
            "ok":{"type":"local","command":["ok"],"disabled":false},
            "bad-type":{"type":"unknown"},
            "bad-command":{"type":"local","command":["ok",1]},
            "bad-disabled":{"type":"remote","url":"https://example.test","disabled":"false"},
            "bad-entry":false,
        }}})
        .to_string(),
    );
    let list = mcp::list(&home.paths()).unwrap();
    assert_eq!(list.data.len(), 1);
    assert_eq!(list.diagnostics.len(), 4);
}

#[test]
fn empty_global_sources_are_empty_and_do_not_create_files() {
    let home = Home::new();
    assert!(mcp::list(&home.paths()).unwrap().data.is_empty());
    let skills = skills::list(&home.paths()).unwrap();
    assert!(skills.data.is_empty());
    assert!(skills.diagnostics.is_empty());
    assert_eq!(fs::read_dir(&home.0).unwrap().count(), 0);
    assert_eq!(
        mcp::delete(&home.paths(), "missing").unwrap_err().code,
        ErrorCode::NotFound
    );
    assert_eq!(fs::read_dir(&home.0).unwrap().count(), 0);
}

fn mcp_draft(name: &str, config: serde_json::Value) -> mcp::McpDraft {
    mcp::McpDraft {
        name: name.to_owned(),
        config: config.as_object().unwrap().clone(),
    }
}

fn mcp_update(server: &mcp::McpServer, config: serde_json::Value) -> mcp::McpUpdate {
    mcp::McpUpdate {
        name: server.name.clone(),
        config: config.as_object().unwrap().clone(),
        expected_config: server.config.clone(),
        expected_source_path: server.source_path.clone(),
    }
}

fn mcp_duplicate_documents(config: &serde_json::Value) -> [String; 3] {
    let shadow =
        json!({"type":"local","command":["shadow"],"environment":{"TOKEN":"shadow-secret"}});
    let shadow_servers = json!({"secret":shadow});
    let effective_servers = json!({"secret":config});
    [
        format!("{{\n// keep comment\n\"model\":\"keep\",\"mcp\":{{\"servers\":{{\"secret\":{shadow},\"secret\":{config}}}}}}}"),
        format!("{{\n// keep comment\n\"model\":\"keep\",\"mcp\":{{\"servers\":{shadow_servers}}},\"mcp\":{{\"servers\":{effective_servers}}}}}"),
        format!("{{\n// keep comment\n\"model\":\"keep\",\"mcp\":{{\"servers\":{shadow_servers},\"servers\":{effective_servers}}}}}"),
    ]
}

fn mcp_directory_snapshot(home: &Home) -> BTreeMap<PathBuf, Vec<u8>> {
    fs::read_dir(home.0.join(".config/opencode"))
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            let content = fs::read(&path).unwrap();
            (path, content)
        })
        .collect()
}

#[test]
fn mcp_duplicate_ancestors_reject_create_without_writes_or_artifacts() {
    let config = json!({"type":"local","command":["run"],"environment":{"TOKEN":"real-secret"},"extension":{"future":[null,{"keep":true}]}});
    for original in mcp_duplicate_documents(&config).into_iter().skip(1) {
        let home = Home::new();
        home.write(".config/opencode/opencode.jsonc", &original);
        let before = mcp_directory_snapshot(&home);
        let error = mcp::create(&home.paths(), mcp_draft("new", config.clone())).unwrap_err();
        assert_eq!(error.code, ErrorCode::ValidationFailed);
        assert!(!error.message.contains("real-secret"));
        assert!(!error.message.contains("shadow-secret"));
        assert_eq!(mcp_directory_snapshot(&home), before);
        assert_eq!(
            mcp::get(&home.paths(), "new").unwrap_err().code,
            ErrorCode::NotFound
        );
    }
}

#[test]
fn mcp_duplicate_keys_reject_update_when_unknown_values_differ_without_artifacts() {
    let config = json!({"type":"local","command":["run"],"environment":{"TOKEN":"real-secret"},"extension":{"future":[null,{"keep":true}]}});
    for original in mcp_duplicate_documents(&config) {
        let home = Home::new();
        home.write(".config/opencode/opencode.jsonc", &original);
        let current = mcp::get(&home.paths(), "secret").unwrap();
        assert_eq!(current.config, config.as_object().unwrap().clone());
        let mut edited = config.clone();
        edited["extension"]["future"] = json!([null, {"keep":false}]);
        let before = mcp_directory_snapshot(&home);
        let error = mcp::update(&home.paths(), mcp_update(&current, edited)).unwrap_err();
        assert_eq!(error.code, ErrorCode::ValidationFailed);
        assert!(!error.message.contains("real-secret"));
        assert!(!error.message.contains("shadow-secret"));
        assert_eq!(mcp_directory_snapshot(&home), before);
        assert_eq!(mcp::get(&home.paths(), "secret").unwrap(), current);
    }
}

#[test]
fn mcp_duplicate_keys_in_later_source_block_delete_before_any_write_or_artifact() {
    let config = json!({"type":"local","command":["run"],"environment":{"TOKEN":"real-secret"}});
    for original in mcp_duplicate_documents(&config) {
        let home = Home::new();
        home.write(
            ".config/opencode/opencode.json",
            r#"{"mcp":{"servers":{"secret":{"type":"local","command":["lower"]}}},"model":"keep"}"#,
        );
        home.write(".config/opencode/opencode.jsonc", &original);
        let current = mcp::get(&home.paths(), "secret").unwrap();
        assert_eq!(current.source_paths.len(), 2);
        let before = mcp_directory_snapshot(&home);
        let error = mcp::delete(&home.paths(), "secret").unwrap_err();
        assert_eq!(error.code, ErrorCode::ValidationFailed);
        assert!(!error.message.contains("real-secret"));
        assert!(!error.message.contains("shadow-secret"));
        assert_eq!(mcp_directory_snapshot(&home), before);
        assert_eq!(mcp::get(&home.paths(), "secret").unwrap(), current);
    }
}

#[test]
fn mcp_duplicate_keys_allow_update_when_exact_effective_config_already_matches() {
    let config = json!({"type":"local","command":["run"],"environment":{"TOKEN":"real-secret"},"extension":{"future":[null,{"keep":true}]}});
    for original in mcp_duplicate_documents(&config) {
        let home = Home::new();
        let path = home.write(".config/opencode/opencode.jsonc", &original);
        let current = mcp::get(&home.paths(), "secret").unwrap();
        let saved = mcp::update(&home.paths(), mcp_update(&current, config.clone())).unwrap();
        assert_eq!(saved.config, config.as_object().unwrap().clone());
        assert_eq!(mcp::get(&home.paths(), "secret").unwrap(), saved);
        let content = fs::read_to_string(path).unwrap();
        assert!(content.contains("// keep comment"));
        assert_eq!(JsoncDoc::parse(&content).unwrap().raw()["model"], "keep");
    }
}

fn skill_update(entry: &skills::SkillEntry, content: &str) -> skills::SkillUpdate {
    skills::SkillUpdate {
        id: entry.id.clone(),
        content: content.to_owned(),
        expected_content: entry.content.clone(),
        expected_path: entry.path.clone(),
    }
}

#[test]
fn mcp_management_bootstrap_roundtrip_preserves_comments_secrets_unknown_keys_and_shadows() {
    let home = Home::new();
    let created = mcp::create(&home.paths(), mcp_draft("punctuation/a:b", json!({
        "type":"local","command":["node","server.js"],"environment":{"TOKEN":"real-secret"},"extension":{"future":true}
    }))).unwrap();
    assert_eq!(
        created.source_path,
        home.0
            .join(".config/opencode/opencode.jsonc")
            .display()
            .to_string()
    );
    assert_eq!(mcp::get(&home.paths(), &created.name).unwrap(), created);
    let lower = home.write(".config/opencode/opencode.json", r#"{"mcp":{"servers":{"punctuation/a:b":{"type":"local","command":["shadow"]}}},"lower":"keep"}"#);
    let higher = home.write(".config/opencode/opencode.jsonc", &format!(
        "{{\n// keep comment\n\"model\":\"keep\",\"mcp\":{{\"servers\":{{\"punctuation/a:b\":{}}}}}}}",
        serde_json::to_string(&created.config).unwrap()
    ));
    let lower_before = fs::read_to_string(&lower).unwrap();
    let current = mcp::get(&home.paths(), &created.name).unwrap();
    let mut config = serde_json::Value::Object(current.config.clone());
    config["command"] = json!(["node", "edited.js"]);
    let saved = mcp::update(&home.paths(), mcp_update(&current, config.clone())).unwrap();
    assert_eq!(saved.config["environment"]["TOKEN"], "real-secret");
    assert_eq!(saved.config["extension"]["future"], true);
    assert_eq!(fs::read_to_string(&lower).unwrap(), lower_before);
    let content = fs::read_to_string(&higher).unwrap();
    assert!(content.contains("// keep comment"));
    assert_eq!(JsoncDoc::parse(&content).unwrap().raw()["model"], "keep");
    assert_eq!(mcp::get(&home.paths(), &created.name).unwrap(), saved);
    assert!(mcp::update(&home.paths(), mcp_update(&current, config)).is_err());
    assert_eq!(mcp::get(&home.paths(), &created.name).unwrap(), saved);
}

#[test]
fn mcp_create_uses_highest_existing_managed_file_and_blocks_hidden_duplicates_and_bad_docs() {
    let home = Home::new();
    let file = home.write(
        ".config/opencode/opencode.json",
        "{\n// existing\n\"model\":\"keep\"\n}",
    );
    let saved = mcp::create(&home.paths(), mcp_draft("new", json!({"type":"remote","url":"https://example.test/mcp","oauth":{"client_id":"id","scope":"read"}}))).unwrap();
    assert_eq!(saved.source_path, file.display().to_string());
    assert!(!home.0.join(".config/opencode/opencode.jsonc").exists());
    assert!(fs::read_to_string(&file).unwrap().contains("// existing"));
    let hidden = home.write(".config/opencode/opencode.jsonc", r#"{"mcp":{"servers":{"hidden":false,"disabled":{"type":"remote","url":"https://example.test","disabled":true}}}}"#);
    for name in ["hidden", "disabled"] {
        let before = fs::read_to_string(&hidden).unwrap();
        assert!(mcp::create(
            &home.paths(),
            mcp_draft(name, json!({"type":"local","command":["run"]}))
        )
        .is_err());
        assert_eq!(fs::read_to_string(&hidden).unwrap(), before);
    }
    fs::write(&hidden, "{broken").unwrap();
    let before = fs::read_to_string(&file).unwrap();
    assert!(mcp::create(
        &home.paths(),
        mcp_draft("another", json!({"type":"local","command":["run"]}))
    )
    .is_err());
    assert_eq!(fs::read_to_string(&file).unwrap(), before);
}

#[test]
fn mcp_update_refuses_changed_source_and_targets_explicit_selected_override() {
    let home = Home::new();
    let base = home.write(
        ".config/opencode/opencode.jsonc",
        r#"{"mcp":{"servers":{"same":{"type":"local","command":["base"]}}}}"#,
    );
    let current = mcp::get(&home.paths(), "same").unwrap();
    let project = home.write(
        "cwd/project.jsonc",
        r#"{"mcp":{"servers":{"same":{"type":"local","command":["project"]}}}}"#,
    );
    let paths = ConfigPaths::with_resource_parts(
        home.0.clone(),
        Some(project.clone()),
        Some(home.0.join("extra")),
        home.0.join(".config"),
        home.0.join("cwd"),
    );
    home.write(
        "extra/opencode.jsonc",
        r#"{"mcp":{"servers":{"same":{"type":"local","command":["base"]}}}}"#,
    );
    let before = fs::read_to_string(&base).unwrap();
    assert!(mcp::update(
        &paths,
        mcp_update(&current, json!({"type":"local","command":["edit"]}))
    )
    .is_err());
    assert_eq!(fs::read_to_string(&base).unwrap(), before);
    let current = mcp::get(&paths, "same").unwrap();
    let saved = mcp::update(
        &paths,
        mcp_update(&current, json!({"type":"local","command":["edit"]})),
    )
    .unwrap();
    assert_eq!(saved.source_path, project.display().to_string());
    assert_eq!(fs::read_to_string(&base).unwrap(), before);
    assert_eq!(mcp::get(&paths, "same").unwrap().target, "edit");
}

#[test]
fn invalid_mcp_drafts_do_not_write_files() {
    for config in [
        json!({"type":"local","command":[]}),
        json!({"type":"local","command":[" "]}),
        json!({"type":"local","command":["run",1]}),
        json!({"type":"other"}),
        json!({"type":"remote","url":"file:///secret"}),
        json!({"type":"remote","url":"https://example.test","disabled":"false"}),
        json!({"type":"local","command":["run"],"environment":{"KEY":1}}),
        json!({"type":"remote","url":"https://example.test","headers":[]}),
        json!({"type":"local","command":["run"],"cwd":1}),
        json!({"type":"local","command":["run"],"timeout":0}),
        json!({"type":"local","command":["run"],"timeout":1.5}),
        json!({"type":"remote","url":"https://example.test","oauth":true}),
        json!({"type":"remote","url":"https://example.test","oauth":{"client_secret":false}}),
    ] {
        let home = Home::new();
        let error = mcp::create(&home.paths(), mcp_draft("valid", config)).unwrap_err();
        assert_eq!(error.code, ErrorCode::ValidationFailed);
        assert_eq!(fs::read_dir(&home.0).unwrap().count(), 0);
    }
    for name in ["", " ", "bad\nname", &"a".repeat(257)] {
        let home = Home::new();
        assert!(mcp::create(
            &home.paths(),
            mcp_draft(name, json!({"type":"local","command":["run"]}))
        )
        .is_err());
        assert_eq!(fs::read_dir(&home.0).unwrap().count(), 0);
    }
}

#[test]
fn mcp_v2_timeout_protocol_codemode_and_oauth_roundtrip_preserve_secrets_and_unknown_fields() {
    let home = Home::new();
    let full = json!({
        "type":"remote", "url":"https://example.test/mcp", "protocol":"2026-07-28", "codemode":true,
        "timeout":{"startup":1000,"catalog":5000,"execution":30000,"future_budget":"keep"},
        "oauth":{"client_id":"id","client_secret":"actual-secret","scope":"read write",
            "redirect_uri":"http://localhost:49152/callback","auth_server_metadata_url":"https://example.test/.well-known/oauth-authorization-server",
            "callback_port":49152,"future_option":{"keep":true},"clientId":false},
        "headers":{"Authorization":"real-token"}, "extension":{"future":true}
    });
    let created = mcp::create(&home.paths(), mcp_draft("full-v2", full.clone())).unwrap();
    assert_eq!(created.config, full.as_object().unwrap().clone());
    let mut edited = full;
    edited["url"] = json!("https://example.test/edited");
    let saved = mcp::update(&home.paths(), mcp_update(&created, edited.clone())).unwrap();
    assert_eq!(saved.config, edited.as_object().unwrap().clone());
    assert_eq!(mcp::get(&home.paths(), "full-v2").unwrap(), saved);
    for (index, options) in [
        json!({"timeout":{},"oauth":{},"protocol":"auto","codemode":false}),
        json!({"timeout":{"startup":1},"oauth":false,"protocol":"legacy"}),
        json!({"timeout":{"catalog":1},"oauth":{"callback_port":1}}),
        json!({"timeout":{"execution":u64::MAX},"oauth":{"callback_port":65535}}),
    ]
    .into_iter()
    .enumerate()
    {
        let mut config = json!({"type":"remote","url":"https://example.test/mcp"});
        config
            .as_object_mut()
            .unwrap()
            .extend(options.as_object().unwrap().clone());
        let server = mcp::create(
            &home.paths(),
            mcp_draft(&format!("partial-{index}"), config.clone()),
        )
        .unwrap();
        assert_eq!(server.config, config.as_object().unwrap().clone());
    }
}

#[test]
fn invalid_documented_v2_mcp_fields_reject_create_and_update_without_writes() {
    let mut invalid = Vec::new();
    for timeout in [
        json!(5000),
        json!(null),
        json!([]),
        json!("5000"),
        json!(false),
    ] {
        invalid.push(json!({"timeout":timeout}));
    }
    for field in ["startup", "catalog", "execution"] {
        for value in [
            json!(0),
            json!(-1),
            json!(1.5),
            json!("1000"),
            json!(null),
            json!(false),
            json!({}),
        ] {
            let mut timeout = serde_json::Map::new();
            timeout.insert(field.to_owned(), value);
            invalid.push(json!({"timeout":timeout}));
        }
    }
    for protocol in [
        json!("2025-03-26"),
        json!("AUTO"),
        json!(""),
        json!(1),
        json!(null),
    ] {
        invalid.push(json!({"protocol":protocol}));
    }
    for codemode in [json!("true"), json!(1), json!(null)] {
        invalid.push(json!({"codemode":codemode}));
    }
    for field in [
        "client_id",
        "client_secret",
        "scope",
        "redirect_uri",
        "auth_server_metadata_url",
    ] {
        let mut oauth = serde_json::Map::new();
        oauth.insert(field.to_owned(), json!(false));
        invalid.push(json!({"oauth":oauth}));
    }
    for value in [
        json!(0),
        json!(65536),
        json!(-1),
        json!(1.5),
        json!("8080"),
        json!(null),
    ] {
        invalid.push(json!({"oauth":{"callback_port":value}}));
    }
    for oauth in [json!(true), json!(null), json!([]), json!("secret-value")] {
        invalid.push(json!({"oauth":oauth}));
    }
    let existing = Home::new();
    let current = mcp::create(&existing.paths(), mcp_draft("existing", json!({"type":"remote","url":"https://example.test","oauth":{"client_secret":"real-secret"},"future":"keep"}))).unwrap();
    let before = fs::read_to_string(&current.source_path).unwrap();
    let directory = PathBuf::from(&current.source_path)
        .parent()
        .unwrap()
        .to_owned();
    let count = fs::read_dir(&directory).unwrap().count();
    for options in invalid {
        let home = Home::new();
        let mut config = json!({"type":"remote","url":"https://example.test"});
        config
            .as_object_mut()
            .unwrap()
            .extend(options.as_object().unwrap().clone());
        let error = mcp::create(&home.paths(), mcp_draft("new", config.clone())).unwrap_err();
        assert_eq!(error.code, ErrorCode::ValidationFailed);
        assert!(!error.message.contains("secret-value"));
        assert_eq!(fs::read_dir(&home.0).unwrap().count(), 0);
        let error = mcp::update(&existing.paths(), mcp_update(&current, config)).unwrap_err();
        assert_eq!(error.code, ErrorCode::ValidationFailed);
        assert_eq!(fs::read_to_string(&current.source_path).unwrap(), before);
        assert_eq!(fs::read_dir(&directory).unwrap().count(), count);
    }
}

#[test]
fn skill_autoinvoke_alias_and_explicit_metadata_precedence_are_preserved_with_full_content() {
    let home = Home::new();
    for (id, fields, expected) in [
        ("alias-true", "disable-model-invocation: true\n", false),
        ("alias-false", "disable-model-invocation: false\n", true),
        ("default", "", true),
        (
            "metadata-true",
            "disable-model-invocation: true\nmetadata:\n  opencode/autoinvoke: true\n",
            true,
        ),
        (
            "metadata-false",
            "disable-model-invocation: false\nmetadata:\n  opencode/autoinvoke: false\n",
            false,
        ),
        (
            "quoted-false",
            "disable-model-invocation: false\nmetadata:\n  opencode/autoinvoke: 'false'\n",
            false,
        ),
        (
            "unknown-metadata",
            "disable-model-invocation: true\nmetadata:\n  opencode/autoinvoke: unknown\n",
            true,
        ),
    ] {
        let content = format!("---\nname: Display\n{fields}---\n# Body\n");
        let created = skills::create(
            &home.paths(),
            skills::SkillDraft {
                id: id.to_owned(),
                content: content.clone(),
            },
        )
        .unwrap();
        assert_eq!(created.autoinvoke, expected, "{id}");
        let fetched = skills::get(&home.paths(), id).unwrap();
        assert_eq!(fetched.autoinvoke, expected, "{id}");
        assert_eq!(fetched.content, content);
    }
    let listed = skills::list(&home.paths()).unwrap();
    assert_eq!(listed.data.len(), 7);
    assert!(listed.diagnostics.is_empty());
}

#[test]
fn local_skill_create_and_flat_nested_unicode_edits_preserve_full_markdown_and_stale_checks() {
    let home = Home::new();
    let content = "---\n# YAML comment\nname: Display Name\ndescription: Description\ncustom: keep\n---\n# Body\n";
    let created = skills::create(
        &home.paths(),
        skills::SkillDraft {
            id: "safe-id".to_owned(),
            content: content.to_owned(),
        },
    )
    .unwrap();
    assert_eq!(created.kind, skills::SkillKind::Local);
    assert_eq!(created.name, "Display Name");
    assert_eq!(
        created.path,
        home.0
            .join(".config/opencode/skills/safe-id/SKILL.md")
            .display()
            .to_string()
    );
    assert_eq!(skills::get(&home.paths(), "safe-id").unwrap(), created);
    assert!(skills::create(
        &home.paths(),
        skills::SkillDraft {
            id: "safe-id".to_owned(),
            content: "duplicate".to_owned()
        }
    )
    .is_err());
    for (relative, id) in [
        (".config/opencode/skills/Flat.md", "Flat"),
        (".config/opencode/skills/nested/深い/SKILL.md", "深い"),
    ] {
        home.write(relative, content);
        let current = skills::get(&home.paths(), id).unwrap();
        let edited = content.replace("# Body", "# Edited body");
        let saved = skills::update(&home.paths(), skill_update(&current, &edited)).unwrap();
        assert_eq!(saved.content, edited);
        assert_eq!(saved.name, "Display Name");
        assert_eq!(fs::read_to_string(home.0.join(relative)).unwrap(), edited);
        assert!(skills::update(&home.paths(), skill_update(&current, "stale")).is_err());
        assert_eq!(skills::get(&home.paths(), id).unwrap(), saved);
        let mut forged = skill_update(&saved, "forged");
        forged.expected_path = home.0.join("unmanaged.md").display().to_string();
        assert!(skills::update(&home.paths(), forged).is_err());
        assert_eq!(skills::get(&home.paths(), id).unwrap(), saved);
        let error = skills::update(
            &home.paths(),
            skill_update(&saved, "---\nname: [\n---\nbad"),
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::ValidationFailed);
        assert_eq!(skills::get(&home.paths(), id).unwrap(), saved);
    }
}

#[cfg(unix)]
#[test]
fn resource_management_preserves_managed_root_and_duplicate_config_symlinks() {
    use std::os::unix::fs::symlink;
    let home = Home::new();
    let global = home.0.join(".config/opencode");
    fs::create_dir_all(&global).unwrap();
    let installed = home.0.join("installed-skills");
    fs::create_dir(&installed).unwrap();
    let root_alias = global.join("skills");
    symlink(&installed, &root_alias).unwrap();
    let created = skills::create(
        &home.paths(),
        skills::SkillDraft {
            id: "RootLink".to_owned(),
            content: "body".to_owned(),
        },
    )
    .unwrap();
    assert_eq!(
        fs::read_to_string(installed.join("RootLink/SKILL.md")).unwrap(),
        "body"
    );
    assert!(fs::symlink_metadata(root_alias)
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(created.kind, skills::SkillKind::Local);
    let config = home.write("managed.jsonc", r#"{"mcp":{"servers":{"linked":{"type":"local","command":["old"],"future":"keep"}}},"model":"keep"}"#);
    for name in ["opencode.json", "opencode.jsonc"] {
        symlink(&config, global.join(name)).unwrap();
    }
    let current = mcp::get(&home.paths(), "linked").unwrap();
    let saved = mcp::update(
        &home.paths(),
        mcp_update(
            &current,
            json!({"type":"local","command":["new"],"future":"keep"}),
        ),
    )
    .unwrap();
    assert_eq!(saved.source_paths.len(), 2);
    for name in ["opencode.json", "opencode.jsonc"] {
        assert!(fs::symlink_metadata(global.join(name))
            .unwrap()
            .file_type()
            .is_symlink());
    }
    assert_eq!(mcp::get(&home.paths(), "linked").unwrap(), saved);
    let created = mcp::create(
        &home.paths(),
        mcp_draft(
            "created-via-aliases",
            json!({"type":"local","command":["run"]}),
        ),
    )
    .unwrap();
    assert_eq!(created.source_paths.len(), 2);
    assert_eq!(
        mcp::get(&home.paths(), "created-via-aliases").unwrap(),
        created
    );
    let config_before = fs::read_to_string(&config).unwrap();
    home.write("extra/opencode.jsonc", "{broken");
    let paths = ConfigPaths::with_resource_parts(
        home.0.clone(),
        None,
        Some(home.0.join("extra")),
        home.0.join(".config"),
        home.0.join("cwd"),
    );
    assert!(mcp::update(
        &paths,
        mcp_update(&saved, json!({"type":"local","command":["bad-write"]}))
    )
    .is_err());
    assert_eq!(fs::read_to_string(&config).unwrap(), config_before);
}

#[test]
fn skill_create_refuses_unavailable_remote_catalog_before_creating_destination() {
    let home = Home::new();
    let (url, server) = serve(vec![(
        "/catalog/index.json",
        404,
        "unavailable".to_owned(),
        None,
    )]);
    configure_catalog(&home, &url);
    assert!(skills::create(
        &home.paths(),
        skills::SkillDraft {
            id: "New".to_owned(),
            content: "body".to_owned()
        }
    )
    .is_err());
    assert!(!home.0.join(".config/opencode/skills/New").exists());
    assert_eq!(server.join().unwrap(), vec!["/catalog/index.json"]);
}

#[test]
fn skill_update_refuses_uncertain_managed_sources_without_writing_local_definition() {
    let home = Home::new();
    let file = home.write(".config/opencode/skills/Valid.md", "body");
    let current = skills::get(&home.paths(), "Valid").unwrap();
    home.write(".config/opencode/opencode.jsonc", "{broken");
    assert!(skills::update(&home.paths(), skill_update(&current, "changed")).is_err());
    assert_eq!(fs::read_to_string(file).unwrap(), "body");
}

#[test]
fn skill_create_blocks_invalid_ids_content_duplicates_and_uncertain_discovery() {
    for id in [
        "",
        ".",
        "..",
        "../escape",
        "a/b",
        "a\\b",
        "a:b",
        "bad\n",
        "CON",
        "nul",
        "COM1",
        "LPT9",
        "name.",
        "name ",
    ] {
        let home = Home::new();
        assert!(skills::create(
            &home.paths(),
            skills::SkillDraft {
                id: id.to_owned(),
                content: "body".to_owned()
            }
        )
        .is_err());
        assert_eq!(fs::read_dir(&home.0).unwrap().count(), 0);
    }
    for content in [
        "",
        " ",
        "nul\0body",
        "---\nname: Missing closure\n",
        "---\nname: [\n---\nbroken",
    ] {
        let home = Home::new();
        assert!(skills::create(
            &home.paths(),
            skills::SkillDraft {
                id: "valid".to_owned(),
                content: content.to_owned()
            }
        )
        .is_err());
        assert_eq!(fs::read_dir(&home.0).unwrap().count(), 0);
    }
    let home = Home::new();
    home.write(".claude/skills/Flat.md", "duplicate");
    home.write(".agents/skills/nested/Directory/SKILL.md", "duplicate");
    for id in ["Flat", "Directory"] {
        assert!(skills::create(
            &home.paths(),
            skills::SkillDraft {
                id: id.to_owned(),
                content: "body".to_owned()
            }
        )
        .is_err());
    }
    home.write(".config/opencode/opencode.jsonc", "{broken");
    assert!(skills::create(
        &home.paths(),
        skills::SkillDraft {
            id: "New".to_owned(),
            content: "body".to_owned()
        }
    )
    .is_err());
    assert!(!home.0.join(".config/opencode/skills/New").exists());
}

#[cfg(unix)]
#[test]
fn skill_create_refuses_destination_symlinks_including_dangling_and_update_preserves_link_mode() {
    use std::os::unix::fs::{symlink, PermissionsExt};
    let home = Home::new();
    let root = home.0.join(".config/opencode/skills");
    fs::create_dir_all(&root).unwrap();
    for id in ["Escape", "Dangling"] {
        let target = home
            .0
            .join(if id == "Escape" { "outside" } else { "missing" });
        if id == "Escape" {
            fs::create_dir(&target).unwrap();
        }
        symlink(&target, root.join(id)).unwrap();
        assert!(skills::create(
            &home.paths(),
            skills::SkillDraft {
                id: id.to_owned(),
                content: "body".to_owned()
            }
        )
        .is_err());
        assert!(!target.join("SKILL.md").exists());
        fs::remove_file(root.join(id)).unwrap();
    }
    let real = home.write("installed.md", "original body");
    fs::set_permissions(&real, fs::Permissions::from_mode(0o644)).unwrap();
    let alias = root.join("Linked.md");
    symlink(&real, &alias).unwrap();
    let current = skills::get(&home.paths(), "Linked").unwrap();
    let saved = skills::update(&home.paths(), skill_update(&current, "edited body")).unwrap();
    assert_eq!(saved.kind, skills::SkillKind::Local);
    assert!(fs::symlink_metadata(&alias)
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(fs::read_to_string(&real).unwrap(), "edited body");
    assert_eq!(
        fs::metadata(&real).unwrap().permissions().mode() & 0o777,
        0o644
    );
    assert!(!fs::read_dir(&home.0)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .any(|path| path.extension().is_some_and(|extension| extension == "bak")));
}

#[test]
fn absent_explicit_config_is_a_create_target_and_arbitrary_source_supports_roundtrip() {
    let home = Home::new();
    let explicit = home.0.join("selected/custom.jsonc");
    let paths = ConfigPaths::with_resource_parts(
        home.0.clone(),
        Some(explicit.clone()),
        None,
        home.0.join(".config"),
        home.0.join("cwd"),
    );
    assert!(!explicit.exists());
    let created = mcp::create(
        &paths,
        mcp_draft("selected", json!({"type":"local","command":["run"]})),
    )
    .unwrap();
    assert_eq!(created.source_path, explicit.display().to_string());
    assert!(!home.0.join(".config/opencode/opencode.jsonc").exists());
    let mut document = JsoncDoc::parse(&fs::read_to_string(&explicit).unwrap()).unwrap();
    document
        .patch(&["skills"], Some(json!(["relative"])))
        .unwrap();
    fs::write(&explicit, document.content()).unwrap();
    let skill = home.write("cwd/relative/Selected.md", "original selected skill");
    home.write(
        ".config/opencode/skills/Global.md",
        "global remains visible",
    );
    assert_eq!(skills::list(&paths).unwrap().data.len(), 2);
    let current = skills::get(&paths, "Selected").unwrap();
    skills::update(&paths, skill_update(&current, "edited selected skill")).unwrap();
    assert_eq!(fs::read_to_string(skill).unwrap(), "edited selected skill");
    let saved = mcp::update(
        &paths,
        mcp_update(&created, json!({"type":"local","command":["edit"]})),
    )
    .unwrap();
    assert_eq!(mcp::get(&paths, "selected").unwrap(), saved);
    mcp::delete(&paths, "selected").unwrap();
    assert!(mcp::get(&paths, "selected").is_err());
}

#[test]
fn remote_skill_duplicate_create_and_forged_update_are_rejected_without_local_writes() {
    for update in [false, true] {
        let home = Home::new();
        let local = home.write(".config/opencode/skills/SKILL.md", "local shadow");
        let (url, server) = serve(vec![
            (
                "/catalog/index.json",
                200,
                json!({"skills":[{"name":"Remote","files":["SKILL.md"]}]}).to_string(),
                None,
            ),
            (
                "/catalog/Remote/SKILL.md",
                200,
                "remote body".to_owned(),
                None,
            ),
        ]);
        configure_catalog(&home, &url);
        if update {
            let error = skills::update(
                &home.paths(),
                skills::SkillUpdate {
                    id: "SKILL".to_owned(),
                    content: "forged".to_owned(),
                    expected_path: local.display().to_string(),
                    expected_content: "local shadow".to_owned(),
                },
            )
            .unwrap_err();
            assert_eq!(error.code, ErrorCode::ValidationFailed);
            assert!(error.message.contains("Remote skills"));
        } else {
            assert!(skills::create(
                &home.paths(),
                skills::SkillDraft {
                    id: "SKILL".to_owned(),
                    content: "body".to_owned()
                }
            )
            .is_err());
        }
        assert_eq!(fs::read_to_string(local).unwrap(), "local shadow");
        assert!(!home.0.join(".config/opencode/skills/SKILL").exists());
        assert_eq!(
            server.join().unwrap(),
            vec!["/catalog/index.json", "/catalog/Remote/SKILL.md"]
        );
    }
}
