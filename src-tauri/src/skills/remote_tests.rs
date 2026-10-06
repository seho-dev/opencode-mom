use std::io::Write;
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

use super::*;
use crate::error::ErrorCode;
use serde_json::json;

struct Home(PathBuf);

impl Home {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("opencode-mom-remote-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&path).unwrap();
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
        crate::document::write_file(&path, content.as_bytes()).unwrap();
        path
    }

    fn configure(&self, sources: &[&str]) {
        self.write(
            ".config/opencode/opencode.jsonc",
            &json!({"skills": sources}).to_string(),
        );
    }
}

impl Drop for Home {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[derive(Clone)]
struct Response {
    status: u16,
    body: String,
    delay: Duration,
}

#[derive(Default)]
struct ServerState {
    responses: BTreeMap<String, Response>,
    requests: Vec<String>,
}

struct Server {
    url: String,
    prefix: String,
    state: Arc<Mutex<ServerState>>,
    stop: Arc<AtomicBool>,
    maximum: Arc<AtomicUsize>,
    worker: Option<thread::JoinHandle<()>>,
}

impl Server {
    fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let prefix = format!("/catalog/{}/", uuid::Uuid::new_v4());
        let url = format!("http://{}{prefix}", listener.local_addr().unwrap());
        let state = Arc::new(Mutex::new(ServerState::default()));
        let stop = Arc::new(AtomicBool::new(false));
        let maximum = Arc::new(AtomicUsize::new(0));
        let worker = {
            let state = state.clone();
            let stop = stop.clone();
            let maximum = maximum.clone();
            thread::spawn(move || {
                let active = Arc::new(AtomicUsize::new(0));
                let mut workers = Vec::new();
                while !stop.load(Ordering::SeqCst) {
                    let (mut stream, _) = match listener.accept() {
                        Ok(stream) => stream,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(1));
                            continue;
                        }
                        Err(error) => panic!("loopback accept: {error}"),
                    };
                    let state = state.clone();
                    let active = active.clone();
                    let maximum = maximum.clone();
                    workers.push(thread::spawn(move || {
                        stream.set_nonblocking(false).unwrap();
                        stream
                            .set_read_timeout(Some(Duration::from_secs(2)))
                            .unwrap();
                        let mut request = Vec::new();
                        while !request.windows(4).any(|part| part == b"\r\n\r\n") {
                            let mut buffer = [0; 1024];
                            let count = stream.read(&mut buffer).unwrap();
                            assert_ne!(count, 0);
                            request.extend_from_slice(&buffer[..count]);
                        }
                        let request = String::from_utf8(request).unwrap();
                        let path = request.split_whitespace().nth(1).unwrap().to_owned();
                        let response = {
                            let mut state = state.lock().unwrap();
                            state.requests.push(path.clone());
                            state.responses.get(&path).cloned().unwrap_or(Response {
                                status: 404,
                                body: "missing".to_owned(),
                                delay: Duration::ZERO,
                            })
                        };
                        let markdown = path.ends_with(".md");
                        if markdown {
                            maximum.fetch_max(
                                active.fetch_add(1, Ordering::SeqCst) + 1,
                                Ordering::SeqCst,
                            );
                        }
                        thread::sleep(response.delay);
                        let _ = write!(
                            stream,
                            "HTTP/1.1 {} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            response.status,
                            response.body.len(),
                            response.body
                        );
                        if markdown {
                            active.fetch_sub(1, Ordering::SeqCst);
                        }
                    }));
                }
                for worker in workers {
                    worker.join().unwrap();
                }
            })
        };
        Self {
            url,
            prefix,
            state,
            stop,
            maximum,
            worker: Some(worker),
        }
    }

    fn respond(&self, relative: &str, status: u16, body: &str, delay: Duration) {
        self.state.lock().unwrap().responses.insert(
            format!("{}{relative}", self.prefix),
            Response {
                status,
                body: body.to_owned(),
                delay,
            },
        );
    }

    fn index(&self, skills: serde_json::Value) {
        self.respond(
            "index.json",
            200,
            &json!({"skills": skills}).to_string(),
            Duration::ZERO,
        );
    }

    fn requests(&self) -> Vec<String> {
        self.state.lock().unwrap().requests.clone()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let result = self.worker.take().unwrap().join();
        if !thread::panicking() {
            result.unwrap();
        }
    }
}

fn update_draft(path: &Path, content: &str) -> SkillUpdate {
    SkillUpdate {
        id: "Wanted".to_owned(),
        content: content.to_owned(),
        expected_path: path.display().to_string(),
        expected_content: "local original".to_owned(),
    }
}

#[test]
fn local_only_discovery_never_fetches_an_unselected_catalog() {
    let home = Home::new();
    let server = Server::new();
    home.write(
        "cwd/opencode.jsonc",
        &json!({"skills": [server.url]}).to_string(),
    );
    home.write(".config/opencode/skills/Wanted.md", "local original");
    assert_eq!(list(&home.paths()).unwrap().data.len(), 1);
    assert_eq!(
        get(&home.paths(), "Wanted").unwrap().content,
        "local original"
    );
    assert!(server.requests().is_empty());
}

#[test]
fn repeated_lists_cache_full_text_metadata_and_source_but_get_reads_fresh_body_and_index() {
    let home = Home::new();
    let server = Server::new();
    home.configure(&[&server.url]);
    server.index(json!([{"name":"Wanted","version":"1","files":["Wanted.md"]}]));
    let original = "---\nname: Display\ndescription: Full metadata\nmetadata:\n  opencode/autoinvoke: false\n---\noriginal body";
    server.respond("Wanted/Wanted.md", 200, original, Duration::ZERO);
    let first = list(&home.paths()).unwrap();
    assert!(first.diagnostics.is_empty());
    assert_eq!(first.data[0].description.as_deref(), Some("Full metadata"));
    assert!(!first.data[0].autoinvoke);
    assert_eq!(first.data[0].source, server.url);
    assert_eq!(first.data[0].content, original);
    assert_eq!(server.requests().len(), 2);
    assert_eq!(list(&home.paths()).unwrap().data, first.data);
    assert_eq!(server.requests().len(), 2);
    server.respond("Wanted/Wanted.md", 200, "fresh body", Duration::ZERO);
    assert_eq!(get(&home.paths(), "Wanted").unwrap().content, "fresh body");
    assert_eq!(server.requests().len(), 4);
    server.index(json!([]));
    assert_eq!(
        get(&home.paths(), "Wanted").unwrap_err().code,
        ErrorCode::NotFound
    );
    assert_eq!(server.requests().len(), 5);
    assert_eq!(list(&home.paths()).unwrap().data, first.data);
    assert_eq!(server.requests().len(), 5);
}

#[test]
fn create_and_update_bypass_warm_list_cache_for_new_remote_collision_or_failed_index() {
    for operation in ["create", "update"] {
        for failed_index in [false, true] {
            let home = Home::new();
            let server = Server::new();
            home.configure(&[&server.url]);
            let path = home.0.join(".config/opencode/skills/Wanted.md");
            if operation == "update" {
                home.write(".config/opencode/skills/Wanted.md", "local original");
            }
            server.index(json!([]));
            assert!(list(&home.paths()).unwrap().diagnostics.is_empty());
            assert_eq!(server.requests().len(), 1);
            if failed_index {
                server.respond("index.json", 503, "unavailable", Duration::ZERO);
            } else {
                server.index(json!([{"name":"Wanted","files":["Wanted.md"]}]));
                server.respond(
                    "Wanted/Wanted.md",
                    200,
                    "new remote definition",
                    Duration::ZERO,
                );
            }
            let error = if operation == "create" {
                create(
                    &home.paths(),
                    SkillDraft {
                        id: "Wanted".to_owned(),
                        content: "draft secret".to_owned(),
                    },
                )
                .unwrap_err()
            } else {
                update(&home.paths(), update_draft(&path, "draft secret")).unwrap_err()
            };
            assert!(!error.message.contains("draft secret"));
            assert_eq!(server.requests().len(), if failed_index { 2 } else { 3 });
            assert!(!home.0.join(".config/opencode/skills/Wanted").exists());
            if operation == "update" {
                assert_eq!(fs::read_to_string(&path).unwrap(), "local original");
                assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
            }
        }
    }
}

#[test]
fn create_final_discovery_bypasses_warm_cache_and_removes_owned_directory_on_source_change() {
    for change in ["remote collision", "failed index", "configured source"] {
        let home = Home::new();
        let server = Server::new();
        home.configure(&[&server.url]);
        server.index(json!([]));
        let listed = list(&home.paths()).unwrap();
        assert!(listed.data.is_empty());
        assert!(listed.diagnostics.is_empty());
        assert_eq!(server.requests().len(), 1);
        let mut hook_called = false;
        let error = create_inner(
            &home.paths(),
            SkillDraft {
                id: "Wanted".to_owned(),
                content: "draft secret".to_owned(),
            },
            |directory| {
                hook_called = true;
                assert!(!directory.resolved.join("SKILL.md").exists());
                assert_eq!(server.requests().len(), 2);
                match change {
                    "remote collision" => {
                        server.index(json!([{"name":"Wanted","files":["Wanted.md"]}]));
                        server.respond("Wanted/Wanted.md", 200, "remote winner", Duration::ZERO);
                    }
                    "failed index" => {
                        server.respond("index.json", 503, "unavailable", Duration::ZERO)
                    }
                    "configured source" => {
                        home.write("cwd/last/Wanted.md", "configured winner");
                        home.configure(&[&server.url, "last"]);
                    }
                    _ => unreachable!(),
                }
                Ok(())
            },
        )
        .unwrap_err();
        assert!(hook_called);
        assert_eq!(
            error.code,
            if change == "failed index" {
                ErrorCode::ConfigurationFailed
            } else {
                ErrorCode::ValidationFailed
            }
        );
        assert!(!error.message.contains("draft secret"));
        let root = home.paths().native_global_skills_dir();
        assert!(fs::symlink_metadata(root.join("Wanted")).is_err());
        assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
        assert_eq!(
            server.requests().len(),
            if change == "remote collision" { 4 } else { 3 }
        );
        if change == "configured source" {
            assert_eq!(
                fs::read_to_string(home.0.join("cwd/last/Wanted.md")).unwrap(),
                "configured winner"
            );
        }
    }
}

#[test]
fn successful_create_rechecks_fresh_catalog_and_does_not_discover_its_own_temporary() {
    let home = Home::new();
    let server = Server::new();
    home.configure(&[&server.url]);
    server.index(json!([]));
    assert!(list(&home.paths()).unwrap().diagnostics.is_empty());
    let saved = create(
        &home.paths(),
        SkillDraft {
            id: "Wanted".to_owned(),
            content: "complete body".to_owned(),
        },
    )
    .unwrap();
    assert_eq!(server.requests().len(), 3);
    assert_eq!(fs::read_to_string(&saved.path).unwrap(), "complete body");
    assert_eq!(saved.kind, SkillKind::Local);
    let directory = home.paths().native_global_skills_dir().join("Wanted");
    assert_eq!(fs::read_dir(directory).unwrap().count(), 1);
}

#[test]
fn update_final_discovery_also_bypasses_warm_cache_and_retains_recovery_on_new_override() {
    for failed_index in [false, true] {
        let home = Home::new();
        let server = Server::new();
        home.configure(&[&server.url]);
        let path = home.write(".config/opencode/skills/Wanted.md", "local original");
        server.index(json!([]));
        assert!(list(&home.paths()).unwrap().diagnostics.is_empty());
        let error = update_inner(&home.paths(), update_draft(&path, "draft secret"), || {
            if failed_index {
                server.respond("index.json", 503, "unavailable", Duration::ZERO);
            } else {
                server.index(json!([{"name":"Wanted","files":["Wanted.md"]}]));
                server.respond(
                    "Wanted/Wanted.md",
                    200,
                    "new remote override",
                    Duration::ZERO,
                );
            }
        })
        .unwrap_err();
        assert!(!error.message.contains("draft secret"));
        assert_eq!(server.requests().len(), if failed_index { 3 } else { 4 });
        assert_eq!(fs::read_to_string(&path).unwrap(), "local original");
        let artifacts = fs::read_dir(path.parent().unwrap())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "bak" || extension == "tmp")
            })
            .collect::<Vec<_>>();
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].extension().unwrap(), "bak");
        assert_eq!(fs::read_to_string(&artifacts[0]).unwrap(), "local original");
    }
}

#[test]
fn cache_expiry_is_fixed_and_fifo_entry_and_byte_bounds_include_url_keys() {
    let now = Instant::now();
    let mut cache = TextCache::default();
    cache.insert("first", "body", now);
    assert_eq!(
        cache
            .get("first", now + CACHE_TTL - Duration::from_nanos(1))
            .as_deref(),
        Some("body")
    );
    cache.insert("first", "body", now + Duration::from_secs(20));
    assert!(cache.get("first", now + CACHE_TTL).is_none());
    for index in 0..=CACHE_ENTRIES {
        cache.insert(&format!("url-{index}"), "body", now);
    }
    assert_eq!(cache.entries.len(), CACHE_ENTRIES);
    assert!(cache.get("url-0", now).is_none());
    assert!(cache.get("url-1", now).is_some());
    assert!(cache.bytes <= CACHE_BYTES);
    assert!(
        cache.bytes
            + cache.entries.capacity() * std::mem::size_of::<CachedText>()
            + std::mem::size_of::<TextCache>()
            <= CACHE_BYTES
    );
    let body = "x".repeat(MAX_BODY as usize);
    for index in 0..5 {
        cache.insert(&format!("big-{index}"), &body, now);
    }
    assert_eq!(cache.entries.len(), 3);
    assert_eq!(cache.bytes, 3 * (body.len() + "big-0".len()));
    assert!(cache.get("big-1", now).is_none());
    assert!(cache.get("big-2", now).is_some());
    assert!(cache.bytes <= CACHE_BYTES);
    assert!(
        cache.bytes
            + cache.entries.capacity() * std::mem::size_of::<CachedText>()
            + std::mem::size_of::<TextCache>()
            <= CACHE_BYTES
    );
    cache.insert(&"u".repeat(CACHE_BYTES), "x", now);
    cache.insert("oversized", &"x".repeat(MAX_BODY as usize + 1), now);
    assert_eq!(cache.entries.len(), 3);
    assert!(cache.get("big-4", now + CACHE_TTL).is_none());
    assert_eq!(cache.bytes, 0);
}

#[test]
fn expired_list_cache_refetches_index_and_markdown() {
    let home = Home::new();
    let server = Server::new();
    home.configure(&[&server.url]);
    server.index(json!([{"name":"Wanted","files":["Wanted.md"]}]));
    server.respond("Wanted/Wanted.md", 200, "original", Duration::ZERO);
    let cache = Mutex::new(TextCache::default());
    assert_eq!(
        discover(&home.paths(), None, Some(&cache)).unwrap().0.data[0].content,
        "original"
    );
    server.respond("Wanted/Wanted.md", 200, "refreshed", Duration::ZERO);
    for entry in &mut cache.lock().unwrap().entries {
        entry.stored = Instant::now() - CACHE_TTL;
    }
    assert_eq!(
        discover(&home.paths(), None, Some(&cache)).unwrap().0.data[0].content,
        "refreshed"
    );
    assert_eq!(server.requests().len(), 4);
}

#[test]
fn failed_or_partial_catalogs_are_not_cached_and_poisoned_cache_bypasses_safely() {
    let home = Home::new();
    let server = Server::new();
    home.configure(&[&server.url]);
    let cache = Mutex::new(TextCache::default());
    server.index(json!([{"name":"Wanted","files":["Wanted.md"]}]));
    server.respond(
        "Wanted/Wanted.md",
        200,
        "---\nname: Broken\n",
        Duration::ZERO,
    );
    let listed = discover(&home.paths(), None, Some(&cache)).unwrap().0;
    assert_eq!(listed.diagnostics.len(), 1);
    assert!(cache.lock().unwrap().entries.is_empty());
    server.respond("Wanted/Wanted.md", 200, "valid body", Duration::ZERO);
    assert_eq!(
        discover(&home.paths(), None, Some(&cache))
            .unwrap()
            .0
            .data
            .len(),
        1
    );
    assert_eq!(server.requests().len(), 4);
    assert_eq!(cache.lock().unwrap().entries.len(), 2);
    let poisoned = Mutex::new(TextCache::default());
    let _ = std::panic::catch_unwind(|| {
        let _guard = poisoned.lock().unwrap();
        panic!("injected cache failure");
    });
    assert_eq!(
        discover(&home.paths(), None, Some(&poisoned))
            .unwrap()
            .0
            .data
            .len(),
        1
    );
    assert_eq!(server.requests().len(), 6);
}

#[test]
fn cold_catalog_fetches_at_most_four_markdown_bodies_and_keeps_index_and_source_overlay_order() {
    let home = Home::new();
    let first = Server::new();
    let last = Server::new();
    home.write(".config/opencode/skills/SKILL.md", "local shadow");
    home.configure(&[&first.url, &last.url]);
    first.index(json!((0..6)
        .map(|index| json!({"name":format!("Entry{index}"),"files":["SKILL.md"]}))
        .collect::<Vec<_>>()));
    for index in 0..6 {
        first.respond(
            &format!("Entry{index}/SKILL.md"),
            200,
            &format!("body {index}"),
            Duration::from_millis(if index == 0 || index == 4 { 150 } else { 50 }),
        );
    }
    last.index(json!([{"name":"Final","files":["SKILL.md"]}]));
    last.respond("Final/SKILL.md", 200, "final source", Duration::ZERO);
    let cache = Mutex::new(TextCache::default());
    let catalog = scan_catalog(
        &first.url,
        None,
        Some(&cache),
        Instant::now() + Duration::from_secs(3),
    )
    .unwrap();
    assert_eq!(
        catalog.last().unwrap().1.as_ref().unwrap().content,
        "body 5"
    );
    assert_eq!(
        catalog
            .iter()
            .map(|(_, entry)| entry.as_ref().unwrap().content.as_str())
            .collect::<Vec<_>>(),
        ["body 0", "body 1", "body 2", "body 3", "body 4", "body 5"]
    );
    assert_eq!(first.maximum.load(Ordering::SeqCst), 4);
    let listed = discover(&home.paths(), None, None).unwrap().0;
    assert!(listed.diagnostics.is_empty());
    assert_eq!(listed.data.len(), 1);
    assert_eq!(listed.data[0].content, "final source");
    assert_eq!(listed.data[0].source, last.url);
    assert_eq!(first.maximum.load(Ordering::SeqCst), 4);
}

#[test]
fn scan_deadline_is_shared_across_catalogs_preserves_local_list_and_never_claims_absence() {
    let home = Home::new();
    let first = Server::new();
    let never_fetched = Server::new();
    home.configure(&[&first.url, &never_fetched.url]);
    home.write(".config/opencode/skills/Wanted.md", "local original");
    first.respond(
        "index.json",
        200,
        &json!({"skills":[]}).to_string(),
        Duration::from_millis(400),
    );
    for only in [None, Some("Wanted"), Some("Absent")] {
        let started = Instant::now();
        let result = discover_until(
            &home.paths(),
            only,
            None,
            started + Duration::from_millis(100),
        );
        assert!(started.elapsed() < Duration::from_millis(350));
        if only.is_none() {
            let listed = result.unwrap().0;
            assert_eq!(listed.data[0].content, "local original");
            assert_eq!(listed.diagnostics.len(), 2);
            assert!(listed.diagnostics[1].contains("deadline exceeded"));
        } else {
            assert_eq!(result.err().unwrap().code, ErrorCode::ConfigurationFailed);
        }
    }
    assert!(never_fetched.requests().is_empty());
}

#[test]
fn markdown_deadline_stops_later_chunks_without_losing_valid_local_skills_or_caching_partial_scan()
{
    let home = Home::new();
    let server = Server::new();
    home.configure(&[&server.url]);
    home.write(".config/opencode/skills/SKILL.md", "valid local shadow");
    server.index(json!((0..6)
        .map(|index| json!({"name":format!("Entry{index}"),"files":["SKILL.md"]}))
        .collect::<Vec<_>>()));
    for index in 0..6 {
        server.respond(
            &format!("Entry{index}/SKILL.md"),
            200,
            "remote body",
            Duration::from_millis(400),
        );
    }
    let cache = Mutex::new(TextCache::default());
    let started = Instant::now();
    let listed = discover_until(
        &home.paths(),
        None,
        Some(&cache),
        started + Duration::from_millis(100),
    )
    .unwrap()
    .0;
    assert!(started.elapsed() < Duration::from_millis(350));
    assert_eq!(listed.data[0].content, "valid local shadow");
    assert_eq!(listed.diagnostics.len(), 6);
    assert!(cache.lock().unwrap().entries.is_empty());
    assert_eq!(server.requests().len(), 5);
    assert!(server.maximum.load(Ordering::SeqCst) <= 4);
}

#[test]
fn successive_catalogs_and_failed_requests_keep_the_same_shared_client() {
    let client = catalog_client().unwrap();
    let first = Server::new();
    let second = Server::new();
    first.index(json!([]));
    second.index(json!([]));
    for server in [&first, &second] {
        assert!(scan_catalog(
            &server.url,
            None,
            None,
            Instant::now() + Duration::from_secs(2)
        )
        .unwrap()
        .is_empty());
        assert!(std::ptr::eq(client, catalog_client().unwrap()));
        assert_eq!(server.requests().len(), 1);
    }
    second.respond("index.json", 503, "unavailable", Duration::ZERO);
    assert!(scan_catalog(
        &second.url,
        None,
        None,
        Instant::now() + Duration::from_secs(2)
    )
    .is_err());
    assert!(std::ptr::eq(client, catalog_client().unwrap()));
    second.respond(
        "index.json",
        200,
        &json!({"skills":[]}).to_string(),
        Duration::from_millis(400),
    );
    let error = scan_catalog(
        &second.url,
        None,
        None,
        Instant::now() + Duration::from_millis(100),
    )
    .err()
    .unwrap();
    assert_eq!(error.code, ErrorCode::ConfigurationFailed);
    assert!(std::ptr::eq(client, catalog_client().unwrap()));
    first.index(json!([]));
    assert!(scan_catalog(
        &first.url,
        None,
        None,
        Instant::now() + Duration::from_secs(2)
    )
    .unwrap()
    .is_empty());
    assert!(std::ptr::eq(client, catalog_client().unwrap()));
}

#[test]
fn scan_deadline_covers_slow_streamed_body_not_only_response_headers() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = Url::parse(&format!("http://{}/body", listener.local_addr().unwrap())).unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut request = Vec::new();
        while !request.windows(4).any(|part| part == b"\r\n\r\n") {
            let mut buffer = [0; 1024];
            let count = stream.read(&mut buffer).unwrap();
            assert_ne!(count, 0);
            request.extend_from_slice(&buffer[..count]);
        }
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: 8\r\nConnection: close\r\n\r\na"
        )
        .unwrap();
        for _ in 0..7 {
            thread::sleep(Duration::from_millis(40));
            if stream.write_all(b"a").is_err() {
                break;
            }
        }
    });
    let client = catalog_client().unwrap();
    let started = Instant::now();
    let error = fetch_text(client, &url, None, started + Duration::from_millis(100)).unwrap_err();
    assert_eq!(error.code, ErrorCode::ConfigurationFailed);
    assert!(started.elapsed() < Duration::from_millis(350));
    server.join().unwrap();
}
