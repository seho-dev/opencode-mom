use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

use serde_json::Value;

use super::*;
use crate::opencode::target::{Selection, Snapshot, TargetRuntime};

struct Request {
    method: String,
    path: String,
    body: Value,
    authenticated: bool,
}

struct FakeHttp {
    address: std::net::SocketAddr,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl FakeHttp {
    fn global(
        paths: &ConfigPaths,
        plugins: Value,
        handler: impl Fn(Request, &mut TcpStream) + Send + 'static,
    ) -> Self {
        let path = paths.opencode_file();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            serde_json::to_vec(&json!({"plugins": plugins})).unwrap(),
        )
        .unwrap();
        Self::new(move |request, stream| {
            if request.path.starts_with("/api/config?") {
                assert_eq!(request.method, "GET");
                respond(
                    stream,
                    200,
                    &json!([{"type":"document","path":path,"info":{"plugins":plugins}}]),
                );
            } else {
                handler(request, stream);
            }
        })
    }

    fn new(handler: impl Fn(Request, &mut TcpStream) + Send + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let thread = thread::spawn(move || {
            for stream in listener.incoming() {
                let mut stream = stream.unwrap();
                if stopping.load(Ordering::SeqCst) {
                    break;
                }
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut bytes = Vec::new();
                while !bytes.ends_with(b"\r\n\r\n") {
                    let mut byte = [0];
                    stream.read_exact(&mut byte).unwrap();
                    bytes.push(byte[0]);
                    assert!(bytes.len() <= 8192);
                }
                let headers = String::from_utf8(bytes).unwrap();
                let mut words = headers.split_whitespace();
                let method = words.next().unwrap().to_owned();
                let path = words.next().unwrap().to_owned();
                let length = headers
                    .lines()
                    .find_map(|line| {
                        let (key, value) = line.split_once(':')?;
                        key.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                assert!(length <= 8192);
                let mut bytes = vec![0; length];
                stream.read_exact(&mut bytes).unwrap();
                let body = if bytes.is_empty() {
                    Value::Null
                } else {
                    serde_json::from_slice(&bytes).unwrap()
                };
                let authenticated = headers.lines().any(|line| {
                    line.eq_ignore_ascii_case("authorization: Basic b3BlbmNvZGU6cHJpdmF0ZQ==")
                });
                handler(
                    Request {
                        method,
                        path,
                        body,
                        authenticated,
                    },
                    &mut stream,
                );
            }
        });
        Self {
            address,
            stop,
            thread: Some(thread),
        }
    }

    fn url(&self) -> String {
        format!("http://{}", self.address)
    }
}

impl Drop for FakeHttp {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.address);
        self.thread.take().unwrap().join().unwrap();
    }
}

fn respond(stream: &mut TcpStream, status: u16, value: &Value) {
    let body = if status == 204 {
        Vec::new()
    } else {
        serde_json::to_vec(value).unwrap()
    };
    write!(stream, "HTTP/1.1 {status} test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).unwrap();
    stream.write_all(&body).unwrap();
}

fn info(stream: &mut TcpStream) {
    respond(stream, 200, &json!({"pid":42,"version":"2.0.22"}));
}

fn envelope(directory: &str, target: &str) -> Value {
    json!({"location":{"directory":directory},"data":[{
        "id":"custom.plugin", "source":{"type":"package","target":target,"version":"1.0.0","outdated":true},
        "features":{"server":true},"state":{"status":"active"},"options":{"PRIVATE":"must not leak"}
    }]})
}

fn connect(api: &FakeHttp, password: Option<&str>) -> ManagedConnection {
    transport::connect_target(
        Snapshot::detached(Selection {
            url: Some(api.url()),
            password: password.map(str::to_owned),
        }),
        Path::new("unused"),
        deadline(),
        |_| panic!("explicit endpoint must never bootstrap"),
    )
    .unwrap()
}

fn deadline() -> Instant {
    Instant::now() + Duration::from_secs(5)
}

#[test]
fn declaration_removal_preserves_other_jsonc_and_files_without_api_writes_or_cached_reappearance() {
    let temp = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(temp.path());
    let plugins =
        json!(["pkg", {"package":"pkg","options":{"token":"PRIVATE"}}, {"package":"keep"}]);
    let api = FakeHttp::global(&paths, plugins, |request, stream| {
        assert_eq!(
            request.method, "GET",
            "removal must not check, update or reload"
        );
        if request.path == "/api/info" {
            info(stream);
        } else {
            respond(stream, 200, &envelope("/runtime", "pkg"));
        }
    });
    let content = "{\n  // Keep header\n  \"plugins\": [\n    \"pkg\",\n    {\"package\":\"pkg\",\"options\":{\"token\":\"PRIVATE\"}},\n    // Keep comment\n    {\"package\":\"keep\",\"extra\":42},\n  ],\n  \"providers\": {\"token\":\"PRIVATE\"},\n}\n";
    fs::write(paths.opencode_file(), content).unwrap();
    let plugin_file = temp.path().join("plugin.js");
    fs::write(&plugin_file, "unchanged").unwrap();
    let connection = connect(&api, None);
    let target = PluginRemovalTarget::Package {
        target: "pkg".into(),
    };
    let plan = removal::prepare_with(&paths, target, &connection, deadline()).unwrap();
    assert_eq!(plan.sources.len(), 1);
    assert!(!serde_json::to_string(&plan).unwrap().contains("PRIVATE"));
    assert_eq!(plan.sources[0].expected_hash.len(), 64);
    removal::remove_with(&paths, plan, &connection, deadline()).unwrap();
    let result = fs::read_to_string(paths.opencode_file()).unwrap();
    let raw = crate::document::JsoncDoc::parse(&result).unwrap();
    assert_eq!(raw.raw()["plugins"], json!([{"package":"keep","extra":42}]));
    assert_eq!(raw.raw()["providers"], json!({"token":"PRIVATE"}));
    assert!(result.contains("// Keep header") && result.contains("// Keep comment"));
    assert_eq!(fs::read_to_string(plugin_file).unwrap(), "unchanged");
    assert!(list_with(&paths, &connection, deadline())
        .unwrap()
        .data
        .is_empty());
    assert!(removal::prepare_with(
        &paths,
        PluginRemovalTarget::Package {
            target: "pkg".into()
        },
        &connection,
        deadline()
    )
    .is_err());
}

#[test]
fn declaration_removal_rejects_changed_hash_forged_sources_and_stale_service() {
    let temp = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(temp.path());
    let api = FakeHttp::global(&paths, json!(["pkg"]), |request, stream| {
        assert_eq!(request.method, "GET");
        if request.path == "/api/info" {
            info(stream);
        } else {
            respond(stream, 200, &envelope("/runtime", "pkg"));
        }
    });
    let connection = connect(&api, None);
    let plan = removal::prepare_with(
        &paths,
        PluginRemovalTarget::Package {
            target: "pkg".into(),
        },
        &connection,
        deadline(),
    )
    .unwrap();
    let original = fs::read_to_string(paths.opencode_file()).unwrap();
    for mutation in 0..4 {
        let mut forged = plan.clone();
        match mutation {
            0 => forged.service_key = "0".repeat(64),
            1 => {
                forged.sources[0].path = temp
                    .path()
                    .join("project.jsonc")
                    .to_string_lossy()
                    .into_owned()
            }
            2 => forged.sources[0].expected_hash = "0".repeat(64),
            _ => {
                forged.sources[0].expected_path = temp
                    .path()
                    .join("outside.jsonc")
                    .to_string_lossy()
                    .into_owned()
            }
        }
        assert!(removal::remove_with(&paths, forged, &connection, deadline()).is_err());
        assert_eq!(fs::read_to_string(paths.opencode_file()).unwrap(), original);
    }
    let external = format!("{original}\n// external change\n");
    fs::write(paths.opencode_file(), &external).unwrap();
    assert!(removal::remove_with(&paths, plan, &connection, deadline()).is_err());
    assert_eq!(fs::read_to_string(paths.opencode_file()).unwrap(), external);
}

#[test]
fn local_declaration_removal_never_deletes_local_or_auto_discovered_files() {
    let temp = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(temp.path());
    let local = temp.path().join("declared");
    fs::create_dir(&local).unwrap();
    let local = fs::canonicalize(local).unwrap();
    let entry = local.join("index.js");
    fs::write(&entry, "keep").unwrap();
    let response_entry = entry.clone();
    let api = FakeHttp::global(&paths, json!([entry]), move |request, stream| {
        assert_eq!(request.method, "GET");
        if request.path == "/api/info" {
            info(stream);
        } else {
            respond(
                stream,
                200,
                &json!({"location":{"directory":"/runtime"},"data":[{"source":{"type":"local","path":response_entry},"features":{},"state":{"status":"active"}}]}),
            );
        }
    });
    let connection = connect(&api, None);
    assert!(list_with(&paths, &connection, deadline()).unwrap().data[0].can_remove);
    let plan = removal::prepare_with(
        &paths,
        PluginRemovalTarget::Local {
            path: entry.to_string_lossy().into_owned(),
        },
        &connection,
        deadline(),
    )
    .unwrap();
    removal::remove_with(&paths, plan, &connection, deadline()).unwrap();
    assert_eq!(fs::read_to_string(&entry).unwrap(), "keep");
    assert!(list_with(&paths, &connection, deadline())
        .unwrap()
        .data
        .is_empty());
}

#[test]
fn nested_local_removal_never_claims_ancestor_declarations_or_discovered_files() {
    let temp = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(temp.path());
    let bundle = paths.opencode_file().parent().unwrap().join("bundle");
    let nested = bundle.join("nested");
    fs::create_dir_all(&nested).unwrap();
    let bundle = fs::canonicalize(bundle).unwrap();
    let nested = fs::canonicalize(nested).unwrap();
    let parent_entry = bundle.join("index.js");
    let entry = nested.join("index.js");
    let discovered = nested.join("auto.js");
    for path in [&parent_entry, &entry, &discovered] {
        fs::write(path, "keep").unwrap();
    }
    let response_entry = entry.clone();
    let response_discovered = discovered.clone();
    let api = FakeHttp::global(
        &paths,
        json!(["./bundle", "./bundle/nested", entry]),
        move |request, stream| {
            assert_eq!(request.method, "GET");
            if request.path == "/api/info" {
                info(stream);
            } else {
                respond(
                    stream,
                    200,
                    &json!({"location":{"directory":"/runtime"},"data":[
                        {"source":{"type":"local","path":response_entry},"features":{},"state":{"status":"active"}},
                        {"source":{"type":"local","path":response_discovered},"features":{},"state":{"status":"active"}}
                    ]}),
                );
            }
        },
    );
    let connection = connect(&api, None);
    fs::write(
        paths.opencode_file(),
        r#"{"plugins":["./bundle","./bundle/nested"]}"#,
    )
    .unwrap();
    let original = fs::read_to_string(paths.opencode_file()).unwrap();
    let runtime = list_with(&paths, &connection, deadline()).unwrap();
    assert_eq!(runtime.data.len(), 2);
    assert!(runtime.data.iter().all(|item| !item.can_remove));
    for path in [&entry, &discovered] {
        assert!(removal::prepare_with(
            &paths,
            PluginRemovalTarget::Local {
                path: path.to_string_lossy().into_owned()
            },
            &connection,
            deadline()
        )
        .is_err());
        assert_eq!(fs::read_to_string(paths.opencode_file()).unwrap(), original);
    }
    let exact = json!({"plugins":["./bundle", "./bundle/nested", entry]});
    fs::write(paths.opencode_file(), serde_json::to_vec(&exact).unwrap()).unwrap();
    let runtime = list_with(&paths, &connection, deadline()).unwrap();
    assert!(runtime.data[0].can_remove);
    assert!(!runtime.data[1].can_remove);
    let plan = removal::prepare_with(
        &paths,
        PluginRemovalTarget::Local {
            path: entry.to_string_lossy().into_owned(),
        },
        &connection,
        deadline(),
    )
    .unwrap();
    removal::remove_with(&paths, plan, &connection, deadline()).unwrap();
    let text = fs::read_to_string(paths.opencode_file()).unwrap();
    assert_eq!(
        crate::document::JsoncDoc::parse(&text).unwrap().raw()["plugins"],
        json!(["./bundle", "./bundle/nested"])
    );
    for path in [&parent_entry, &entry, &discovered] {
        assert_eq!(fs::read_to_string(path).unwrap(), "keep");
    }
}

#[test]
fn declaration_removal_clears_all_global_documents_and_rejects_omitted_sources() {
    let temp = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(temp.path());
    let first = paths.opencode_file();
    let second = first.with_extension("jsonc");
    fs::create_dir_all(first.parent().unwrap()).unwrap();
    fs::write(&first, r#"{"plugins":["pkg@1.2.3"],"keep":1}"#).unwrap();
    fs::write(
        &second,
        r#"{"plugins":[{"package":"pkg@1.2.3","options":{}}],"keep":2}"#,
    )
    .unwrap();
    let documents = [first.clone(), second.clone()];
    let api = FakeHttp::new(move |request, stream| {
        assert_eq!(request.method, "GET");
        if request.path == "/api/info" {
            info(stream);
        } else if request.path.starts_with("/api/config?") {
            let entries: Vec<_> = documents
                .iter()
                .map(|path| json!({"type":"document","path":path,"info":{"plugins":["pkg@1.2.3"]}}))
                .collect();
            respond(stream, 200, &json!(entries));
        } else {
            respond(stream, 200, &envelope("/runtime", "pkg@1.2.3"));
        }
    });
    let connection = connect(&api, None);
    let runtime = list_with(&paths, &connection, deadline()).unwrap();
    assert!(!runtime.data[0].can_update);
    assert!(runtime.data[0].can_remove);
    let plan = removal::prepare_with(
        &paths,
        PluginRemovalTarget::Package {
            target: "pkg@1.2.3".into(),
        },
        &connection,
        deadline(),
    )
    .unwrap();
    assert_eq!(plan.sources.len(), 2);
    let mut omitted = plan.clone();
    omitted.sources.pop();
    assert!(removal::remove_with(&paths, omitted, &connection, deadline()).is_err());
    removal::remove_with(&paths, plan, &connection, deadline()).unwrap();
    for (path, keep) in [(first, 1), (second, 2)] {
        let text = fs::read_to_string(path).unwrap();
        let document = crate::document::JsoncDoc::parse(&text).unwrap();
        assert_eq!(document.raw()["plugins"], json!([]));
        assert_eq!(document.raw()["keep"], json!(keep));
    }
}

#[test]
fn plugin_native_api_uses_selected_target_auth_location_exact_bodies_and_204_without_json() {
    let temp = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(temp.path());
    let root = temp.path().join("project with space");
    fs::create_dir(&root).unwrap();
    let root = fs::canonicalize(root).unwrap();
    let expected_directory = root.to_str().unwrap().to_owned();
    let expected = expected_directory.clone();
    let posts = Arc::new(AtomicUsize::new(0));
    let count = posts.clone();
    let api = FakeHttp::global(&paths, json!(["pkg@latest"]), move |request, stream| {
        assert!(request.authenticated);
        if request.path == "/api/info" {
            assert_eq!(request.method, "GET");
            info(stream);
            return;
        }
        let url = reqwest::Url::parse(&format!("http://localhost{}", request.path)).unwrap();
        if request.method == "GET" {
            assert!(url.query().is_none());
        } else {
            assert_eq!(
                url.query_pairs().collect::<Vec<_>>(),
                [("location[directory]".into(), expected.clone().into())]
            );
        }
        match (request.method.as_str(), url.path()) {
            ("GET", "/api/plugin") => respond(stream, 200, &envelope(&expected, "pkg@latest")),
            ("POST", "/api/plugin/check") => {
                count.fetch_add(1, Ordering::SeqCst);
                assert_eq!(request.body, json!({"target":"pkg@latest"}));
                respond(stream, 200, &envelope(&expected, "pkg@latest"));
            }
            ("POST", "/api/plugin/update") => {
                count.fetch_add(1, Ordering::SeqCst);
                assert_eq!(request.body, json!({"targets":["pkg@latest"]}));
                respond(stream, 204, &Value::Null);
            }
            _ => panic!("unexpected plugin request"),
        }
    });
    let connection = connect(&api, Some("private"));
    let value = list_with(&paths, &connection, deadline()).unwrap();
    assert_eq!(value.directory, expected_directory);
    assert!(!serde_json::to_string(&value).unwrap().contains("PRIVATE"));
    assert!(value.data[0].can_update);
    assert!(!root.join(".opencode").exists());
    let checked = check_with(&paths, Some("pkg@latest"), &connection, deadline()).unwrap();
    assert_eq!(checked.directory, expected_directory);
    update_with(&paths, "pkg@latest", &connection, deadline()).unwrap();
    assert_eq!(posts.load(Ordering::SeqCst), 2);
}

#[test]
fn plugin_entrypoints_reject_project_scope_before_connecting_or_requesting() {
    let temp = tempfile::tempdir().unwrap();
    let global = ConfigPaths::for_home(temp.path());
    let root = temp.path().join("project");
    fs::create_dir(&root).unwrap();
    let project =
        crate::models::add_project(&global.config_file(), root.to_str().unwrap()).unwrap();
    let scoped = global.for_project(Some(project.id)).unwrap();
    for result in [
        list(&scoped).map(|_| ()),
        config_sources(&scoped).map(|_| ()),
        update(&scoped, "pkg"),
    ] {
        let error = result.unwrap_err();
        assert_eq!(error.code, crate::error::ErrorCode::ValidationFailed);
        assert!(error.message.contains("Global"));
    }
    let requests = Arc::new(AtomicUsize::new(0));
    let count = requests.clone();
    let api = FakeHttp::new(move |request, stream| {
        if request.path == "/api/info" {
            info(stream);
        } else {
            count.fetch_add(1, Ordering::SeqCst);
            respond(stream, 200, &envelope("/runtime", "pkg"));
        }
    });
    let connection = connect(&api, None);
    assert!(config_sources_with(&scoped, &connection, deadline()).is_err());
    assert!(check_with(&scoped, None, &connection, deadline()).is_err());
    assert!(update_with(&scoped, "pkg", &connection, deadline()).is_err());
    assert_eq!(requests.load(Ordering::SeqCst), 0);
}

#[test]
fn passwordless_global_runtime_retains_returned_location_and_checks_only_global_target() {
    let temp = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(temp.path());
    let api = FakeHttp::global(&paths, json!(["pkg"]), |request, stream| {
        assert!(!request.authenticated);
        match request.path.as_str() {
            "/api/info" => info(stream),
            "/api/plugin" => respond(stream, 200, &envelope("/returned/location", "pkg")),
            "/api/plugin/check?location%5Bdirectory%5D=%2Freturned%2Flocation" => {
                assert_eq!(request.body, json!({"target":"pkg"}));
                respond(stream, 200, &envelope("/returned/location", "pkg"));
            }
            _ => panic!("unexpected request"),
        }
    });
    let connection = connect(&api, None);
    assert_eq!(
        check_with(&paths, None, &connection, deadline())
            .unwrap()
            .directory,
        "/returned/location"
    );
}

#[test]
fn plugin_update_rejects_pins_local_directives_unknown_and_ambiguous_runtime_before_post() {
    for target in [
        "pkg@1.2.3",
        "@scope/pkg@=v1.2.3-beta.1+build",
        "github:owner/repo#v1.2.3",
        "github:owner/repo#v1.2.3-beta.1+build",
        "github:owner/repo#0123456789012345678901234567890123456789",
        "github:owner/repo#0123456789012345678901234567890123456789012345678901234567890123",
    ] {
        let runtime =
            project_runtime(serde_json::from_value(envelope("/runtime", target)).unwrap()).unwrap();
        assert!(allowed_target(target, &runtime, true).is_err(), "{target}");
        assert!(allowed_target(target, &runtime, false).is_ok());
        assert!(!runtime.data[0].can_update);
    }
    for data in [
        json!([]),
        json!([{"id":"pkg","source":{"type":"local","path":"/local"},"features":{},"state":{"status":"active"}}]),
        json!([{"id":"pkg","source":{"type":"builtin"},"features":{},"state":{"status":"active"}}]),
        json!([{"source":{"type":"package","target":"other"},"features":{},"state":{"status":"active"}}]),
        json!([{"source":{"type":"package","target":"pkg","updating":true},"features":{},"state":{"status":"active"}}]),
    ] {
        let value = json!({"location":{"directory":"/runtime"},"data":data});
        let runtime = project_runtime(serde_json::from_value(value).unwrap()).unwrap();
        assert!(allowed_target("pkg", &runtime, true).is_err());
    }
    let mut value = envelope("/runtime", "pkg");
    let duplicate = value["data"][0].clone();
    value["data"].as_array_mut().unwrap().push(duplicate);
    let runtime = project_runtime(serde_json::from_value(value).unwrap()).unwrap();
    assert!(allowed_target("pkg", &runtime, true).is_err());
    let runtime =
        project_runtime(serde_json::from_value(envelope("/runtime", "pkg")).unwrap()).unwrap();
    assert!(allowed_target("unknown", &runtime, true).is_err());
    assert!(allowed_target("pkg", &runtime, true).is_ok());
    for target in [
        "npm:@scope/pkg@latest",
        "pkg@^1.2.3",
        "pkg@~1.2.3",
        "owner/repo#main",
        "git+https://github.com/owner/repo#release",
    ] {
        let runtime =
            project_runtime(serde_json::from_value(envelope("/runtime", target)).unwrap()).unwrap();
        assert!(runtime.data[0].can_update, "{target}");
        assert!(allowed_target(target, &runtime, true).is_ok(), "{target}");
    }
}

#[test]
fn plugin_update_http_failure_disconnect_redirect_and_timeout_are_unknown_without_retry() {
    let temp = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(temp.path());
    for failure in ["http", "redirect", "disconnect", "timeout"] {
        let posts = Arc::new(AtomicUsize::new(0));
        let count = posts.clone();
        let api = FakeHttp::global(&paths, json!(["pkg"]), move |request, stream| match request
            .path
            .as_str()
        {
            "/api/info" => info(stream),
            "/api/plugin" => respond(stream, 200, &envelope("/runtime", "pkg")),
            "/api/plugin/update?location%5Bdirectory%5D=%2Fruntime" => {
                count.fetch_add(1, Ordering::SeqCst);
                match failure {
                    "http" => respond(stream, 503, &json!({"PRIVATE":"secret"})),
                    "redirect" => {
                        stream.write_all(b"HTTP/1.1 302 test\r\nLocation: http://127.0.0.1:1/PRIVATE\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
                    }
                    "timeout" => thread::sleep(Duration::from_millis(500)),
                    _ => {}
                }
            }
            _ => panic!("unexpected request"),
        });
        let connection = connect(&api, None);
        let expires = if failure == "timeout" {
            Instant::now() + Duration::from_millis(300)
        } else {
            deadline()
        };
        let error = update_with(&paths, "pkg", &connection, expires).unwrap_err();
        assert!(error.message.contains("outcome unknown"), "{error}");
        assert!(!error.message.contains("PRIVATE"));
        assert_eq!(posts.load(Ordering::SeqCst), 1);
        assert_eq!(
            runtime(&connection, None, deadline()).unwrap().data.len(),
            1
        );
        assert_eq!(posts.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn plugin_writes_recheck_pid_version_and_revision_and_hold_reservation() {
    let temp = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(temp.path());
    let changed = Arc::new(AtomicBool::new(false));
    let flag = changed.clone();
    let posts = Arc::new(AtomicUsize::new(0));
    let count = posts.clone();
    let api = FakeHttp::global(&paths, json!(["pkg"]), move |request, stream| match request
        .path
        .as_str()
    {
        "/api/info" => respond(
            stream,
            200,
            &json!({"pid":if flag.load(Ordering::SeqCst) {43} else {42},"version":"2.0.22"}),
        ),
        "/api/plugin" => respond(stream, 200, &envelope("/runtime", "pkg")),
        "/api/plugin/update" => {
            count.fetch_add(1, Ordering::SeqCst);
            respond(stream, 204, &Value::Null);
        }
        _ => panic!("unexpected request"),
    });
    let connection = connect(&api, None);
    changed.store(true, Ordering::SeqCst);
    assert!(update_with(&paths, "pkg", &connection, deadline())
        .unwrap_err()
        .message
        .contains("identity changed"));
    assert_eq!(posts.load(Ordering::SeqCst), 0);
    changed.store(false, Ordering::SeqCst);
    let target = TargetRuntime::new(
        temp.path().join("target.json"),
        temp.path().join("unused"),
        PathBuf::from("/runtime"),
    );
    let snapshot = target.set(Some(api.url()), None, || Ok(())).unwrap();
    let connection = transport::connect_target(snapshot, Path::new("unused"), deadline(), |_| {
        panic!("no bootstrap")
    })
    .unwrap();
    let (_response, reservation) = connection
        .plugin_post("update", None, &json!({"targets":["pkg"]}), deadline())
        .unwrap();
    assert_eq!(
        target
            .set(Some(api.url()), None, || Ok(()))
            .err()
            .unwrap()
            .code,
        crate::error::ErrorCode::Busy
    );
    drop(reservation);
    target.set(Some(api.url()), None, || Ok(())).unwrap();
    assert!(update_with(&paths, "pkg", &connection, deadline()).is_err());
    assert_eq!(posts.load(Ordering::SeqCst), 1);
}

#[test]
fn plugin_runtime_rejects_bad_shapes_and_redacts_native_failure_messages() {
    for value in [
        json!({"location":{"directory":"relative"},"data":[]}),
        json!({"location":{"directory":"/runtime"},"data":[{"source":{"type":"package","target":""},"features":{},"state":{"status":"active"}}]}),
        json!({"location":{"directory":"/runtime"},"data":[{"source":{"type":"builtin"},"features":{"server":false},"state":{"status":"active"}}]}),
    ] {
        assert!(project_runtime(serde_json::from_value(value).unwrap()).is_err());
    }
    let mut value = envelope("/runtime", "pkg");
    value["data"][0]["state"] = json!({"status":"failed","error":"PRIVATE token","ref":"err_1"});
    let runtime = project_runtime(serde_json::from_value(value).unwrap()).unwrap();
    assert!(!serde_json::to_string(&runtime).unwrap().contains("PRIVATE"));
    let mut invalid = envelope("/runtime", "pkg");
    invalid["data"][0]["features"] = json!([]);
    assert!(serde_json::from_value::<RuntimeEnvelope>(invalid).is_err());
}

#[test]
fn plugin_check_reservation_covers_body_and_failures_never_repeat_post() {
    let temp = tempfile::tempdir().unwrap();
    let target = TargetRuntime::new(
        temp.path().join("target.json"),
        temp.path().join("unused"),
        PathBuf::from("/runtime"),
    );
    let target_for_server = target.clone();
    let api = FakeHttp::new(move |request, stream| {
        if request.path == "/api/info" {
            info(stream);
        } else {
            assert_eq!(request.method, "POST");
            let body = serde_json::to_vec(&envelope("/runtime", "pkg")).unwrap();
            write!(
                stream,
                "HTTP/1.1 200 test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .unwrap();
            stream.flush().unwrap();
            assert_eq!(
                target_for_server
                    .set(None, None, || Ok(()))
                    .err()
                    .unwrap()
                    .code,
                crate::error::ErrorCode::Busy
            );
            stream.write_all(&body).unwrap();
        }
    });
    let snapshot = target.set(Some(api.url()), None, || Ok(())).unwrap();
    let connection = transport::connect_target(snapshot, Path::new("unused"), deadline(), |_| {
        panic!("no bootstrap")
    })
    .unwrap();
    let (response, reservation) = connection
        .plugin_post("check", Some(Path::new("/runtime")), &json!({}), deadline())
        .unwrap();
    let _: JsonObject<RuntimeEnvelope> =
        transport::response_json(response.unwrap(), deadline(), MAX_BYTES, "plugins", "2 MiB")
            .unwrap();
    assert_eq!(
        target.set(None, None, || Ok(())).err().unwrap().code,
        crate::error::ErrorCode::Busy
    );
    drop(reservation);
    target.set(None, None, || Ok(())).unwrap();

    let paths = ConfigPaths::for_home(temp.path());
    let posts = Arc::new(AtomicUsize::new(0));
    let count = posts.clone();
    let api = FakeHttp::global(&paths, json!(["pkg"]), move |request, stream| match request
        .path
        .as_str()
    {
        "/api/info" => info(stream),
        "/api/plugin" => respond(stream, 200, &envelope("/runtime", "pkg")),
        "/api/plugin/check?location%5Bdirectory%5D=%2Fruntime" => {
            count.fetch_add(1, Ordering::SeqCst);
            respond(stream, 503, &json!({"PRIVATE":"error"}));
        }
        _ => panic!("unexpected check request"),
    });
    let connection = connect(&api, None);
    let error = check_with(&paths, None, &connection, deadline()).unwrap_err();
    assert!(error.message.contains("outcome unknown"));
    assert!(!error.message.contains("PRIVATE"));
    assert_eq!(posts.load(Ordering::SeqCst), 1);
    runtime(&connection, None, deadline()).unwrap();
    assert_eq!(posts.load(Ordering::SeqCst), 1);
}

#[test]
fn plugin_get_and_check_validate_requested_location_and_never_retry_post() {
    let temp = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(temp.path());
    let posts = Arc::new(AtomicUsize::new(0));
    let count = posts.clone();
    let api = FakeHttp::global(&paths, json!(["pkg"]), move |request, stream| {
        let url = reqwest::Url::parse(&format!("http://localhost{}", request.path)).unwrap();
        match (request.method.as_str(), url.path()) {
            ("GET", "/api/info") => info(stream),
            ("GET", "/api/plugin") => respond(stream, 200, &envelope("/runtime", "pkg")),
            ("POST", "/api/plugin/check") => {
                assert_eq!(request.body, json!({"target":"pkg"}));
                assert_eq!(
                    url.query_pairs().collect::<Vec<_>>(),
                    [("location[directory]".into(), "/runtime".into())]
                );
                count.fetch_add(1, Ordering::SeqCst);
                respond(stream, 200, &envelope("/other", "pkg"));
            }
            _ => panic!("unexpected request"),
        }
    });
    let connection = connect(&api, None);
    assert!(runtime(&connection, Some(Path::new("/requested")), deadline()).is_err());
    let error = check_with(&paths, None, &connection, deadline()).unwrap_err();
    assert!(error.message.contains("outcome unknown"));
    assert_eq!(posts.load(Ordering::SeqCst), 1);
    assert_eq!(
        runtime(&connection, None, deadline()).unwrap().directory,
        "/runtime"
    );
    assert_eq!(posts.load(Ordering::SeqCst), 1);
}

#[test]
fn runtime_permissions_ignore_external_can_update_and_reject_ineligible_targets_without_post() {
    let temp = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(temp.path());
    let mut duplicate = envelope("/runtime", "pkg");
    let item = duplicate["data"][0].clone();
    duplicate["data"].as_array_mut().unwrap().push(item);
    let mut collision = envelope("/runtime", "pkg");
    collision["data"].as_array_mut().unwrap().push(json!({
        "id":"pkg","source":{"type":"builtin"},"features":{},"state":{"status":"active"}
    }));
    let mut current = envelope("/runtime", "pkg");
    current["data"][0]["source"]
        .as_object_mut()
        .unwrap()
        .remove("outdated");
    let mut updating = envelope("/runtime", "pkg");
    updating["data"][0]["source"]["updating"] = json!(true);
    for (target, mut value) in [
        ("pkg@1.2.3", envelope("/runtime", "pkg@1.2.3")),
        (
            "pkg@=v1.2.3-beta.1+build",
            envelope("/runtime", "pkg@=v1.2.3-beta.1+build"),
        ),
        (
            "github:owner/repo#0123456789012345678901234567890123456789",
            envelope(
                "/runtime",
                "github:owner/repo#0123456789012345678901234567890123456789",
            ),
        ),
        ("pkg", duplicate),
        ("pkg", collision),
        ("pkg", current),
        ("pkg", updating),
        (
            "pkg",
            json!({"location":{"directory":"/runtime"},"data":[{
                "id":"pkg","source":{"type":"local","path":"/local"},"features":{},"state":{"status":"active"}
            }]}),
        ),
        (
            "pkg",
            json!({"location":{"directory":"/runtime"},"data":[{
                "id":"pkg","source":{"type":"builtin"},"features":{},"state":{"status":"active"}
            }]}),
        ),
        ("./local", envelope("/runtime", "./local")),
        ("opencode.tools", envelope("/runtime", "opencode.tools")),
        (
            "future:registry/pkg@channel",
            envelope("/runtime", "future:registry/pkg@channel"),
        ),
    ] {
        for item in value["data"].as_array_mut().unwrap() {
            item["canUpdate"] = json!(true);
        }
        let posts = Arc::new(AtomicUsize::new(0));
        let count = posts.clone();
        let api = FakeHttp::new(move |request, stream| {
            if request.method == "POST" {
                count.fetch_add(1, Ordering::SeqCst);
                respond(stream, 204, &Value::Null);
            } else if request.path == "/api/info" {
                info(stream);
            } else {
                respond(stream, 200, &value);
            }
        });
        let connection = connect(&api, None);
        assert!(
            update_with(&paths, target, &connection, deadline()).is_err(),
            "{target}"
        );
        assert_eq!(posts.load(Ordering::SeqCst), 0, "{target}");
        if let Ok(runtime) = runtime(&connection, None, deadline()) {
            assert!(runtime.data.iter().all(|item| !item.can_update));
        }
    }
    let mut value = envelope("/runtime", "pkg@latest");
    value["data"][0]["canUpdate"] = json!(false);
    let runtime = project_runtime(serde_json::from_value(value).unwrap()).unwrap();
    assert!(runtime.data[0].can_update);
}

#[test]
fn unknown_native_package_formats_remain_visible_but_cannot_update() {
    let temp = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(temp.path());
    for target in [
        "future:registry/pkg@channel",
        "future://registry.example/plugin#main",
        "https://registry.example/plugin?version=latest",
    ] {
        let mut value = envelope("/runtime", target);
        value["data"][0]["canUpdate"] = json!(true);
        let posts = Arc::new(AtomicUsize::new(0));
        let count = posts.clone();
        let api = FakeHttp::new(move |request, stream| {
            if request.path == "/api/info" {
                info(stream);
            } else {
                count.fetch_add(usize::from(request.method == "POST"), Ordering::SeqCst);
                respond(stream, 200, &value);
            }
        });
        let connection = connect(&api, None);
        let result = runtime(&connection, None, deadline()).unwrap();
        assert!(!result.data[0].can_update, "{target}");
        assert_eq!(
            serde_json::to_value(result).unwrap()["data"][0]["source"]["target"],
            target
        );
        assert!(
            update_with(&paths, target, &connection, deadline()).is_err(),
            "{target}"
        );
        assert_eq!(posts.load(Ordering::SeqCst), 0, "{target}");
    }
}

#[test]
fn config_preview_pins_runtime_directory_and_projects_only_native_plugin_sources() {
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().join("global");
    fs::create_dir_all(&directory).unwrap();
    let document = directory.join("opencode.jsonc");
    fs::write(&document, r#"{"plugins":["pkg",{"package":"options-absent"},{"package":"options-empty","options":{}},{"package":"options-set","options":{"enabled":true}}]}"#).unwrap();
    let paths = ConfigPaths::with_parts(temp.path().into(), None, Some(directory.clone()));
    let source = document.clone();
    let source_directory = directory.clone();
    let api = FakeHttp::new(move |request, stream| {
        assert!(request.authenticated);
        assert_eq!(request.method, "GET");
        match request.path.as_str() {
            "/api/info" => info(stream),
            "/api/plugin" => respond(stream, 200, &envelope("/runtime with space", "pkg")),
            "/api/config?location%5Bdirectory%5D=%2Fruntime+with+space" => respond(
                stream,
                200,
                &json!([
                    {"type":"document","path":source,"info":{
                        "plugins":["pkg",{"package":"options-absent"},{"package":"options-empty","options":{}},{"package":"options-set","options":{"enabled":true}}],
                        "providers":{"secret":"PRIVATE"},"mcp":{"password":"PRIVATE"}
                    },"content":"PRIVATE"},
                    {"type":"directory","path":source_directory,"secret":"PRIVATE"},
                    {"type":"document","info":{"plugins":["*","-builtin","./local"],"credentials":"PRIVATE"}},
                    {"type":"document","path":"/project/opencode.jsonc","info":{"plugins":[]}},
                    {"type":"document","info":{"providers":{"token":"PRIVATE"}}}
                ]),
            ),
            _ => panic!("unexpected preview request: {}", request.path),
        }
    });
    let connection = connect(&api, Some("private"));
    let preview = config_sources_with(&paths, &connection, deadline()).unwrap();
    let value = serde_json::to_value(preview).unwrap();
    assert_eq!(value["directory"], "/runtime with space");
    assert_eq!(
        value["data"],
        json!([
            {"type":"document","path":document,"plugins":["pkg",{"package":"options-absent"},{"package":"options-empty","options":{}},{"package":"options-set","options":{"enabled":true}}]},
            {"type":"directory","path":directory},
        ])
    );
    assert!(!value.to_string().contains("PRIVATE"));
    assert_eq!(
        fs::read_to_string(&document).unwrap(),
        r#"{"plugins":["pkg",{"package":"options-absent"},{"package":"options-empty","options":{}},{"package":"options-set","options":{"enabled":true}}]}"#
    );
}

#[test]
fn config_preview_rejects_invalid_shapes_paths_and_oversized_bodies_without_leaking_values() {
    let temp = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(temp.path());
    for value in [
        json!({"data":[]}),
        json!([[]]),
        json!([{"type":"PRIVATE"}]),
        json!([{"type":"document","info":[]}]),
        json!([{"type":"document","info":{"plugins":null}}]),
        json!([{"type":"document","info":{"plugins":[["PRIVATE"]]}}]),
        json!([{"type":"document","info":{"plugins":[{"package":"pkg","options":[]}]}}]),
        json!([{"type":"document","info":{"plugins":[{"package":"pkg","options":null}]}}]),
        json!([{"type":"document","info":{"plugins":[{"package":"pkg","secret":"PRIVATE"}]}}]),
        json!([{"type":"document","path":null,"info":{}}]),
        json!([{"type":"document","path":"PRIVATE/relative","info":{}}]),
        json!([{"type":"directory","path":"/global/../PRIVATE"}]),
        json!([{"type":"directory","path":"/global/./PRIVATE"}]),
        json!([{"type":"directory","path":"/global/PRIVATE\u{0}"}]),
        json!([{"type":"document","info":{"plugins":[{"package":"pkg","options":{"PRIVATE":"x".repeat(MAX_BYTES as usize)}}]}}]),
    ] {
        let api = FakeHttp::new(move |request, stream| {
            if request.path == "/api/info" {
                info(stream);
            } else if request.path == "/api/plugin" {
                respond(stream, 200, &envelope("/runtime", "pkg"));
            } else {
                assert!(request.path.starts_with("/api/config?"));
                let body = serde_json::to_vec(&value).unwrap();
                write!(
                    stream,
                    "HTTP/1.1 200 test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .unwrap();
                let _ = stream.write_all(&body);
            }
        });
        let connection = connect(&api, None);
        let error = config_sources_with(&paths, &connection, deadline()).unwrap_err();
        assert!(!error.message.contains("PRIVATE"), "{error}");
    }
}

#[test]
fn config_preview_rejects_service_identity_changes_and_stale_target_replies() {
    let temp = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(temp.path());
    let flag = Arc::new(AtomicBool::new(false));
    let api = FakeHttp::new(move |request, stream| match request.path.as_str() {
        "/api/info" => respond(
            stream,
            200,
            &json!({"pid":if flag.load(Ordering::SeqCst) {43} else {42},"version":"2.0.22"}),
        ),
        "/api/plugin" => respond(stream, 200, &envelope("/runtime", "pkg")),
        _ => {
            assert!(request.path.starts_with("/api/config?"));
            flag.store(true, Ordering::SeqCst);
            respond(stream, 200, &json!([]));
        }
    });
    let connection = connect(&api, None);
    assert!(config_sources_with(&paths, &connection, deadline())
        .unwrap_err()
        .message
        .contains("identity changed"));

    for operation in ["plugin", "config"] {
        let target = TargetRuntime::new(
            temp.path().join("target.json"),
            temp.path().join("unused"),
            PathBuf::from("/runtime"),
        );
        let server_target = target.clone();
        let api = FakeHttp::new(move |request, stream| {
            if request.path == "/api/info" {
                info(stream);
            } else if request.path.starts_with("/api/plugin") {
                if operation == "plugin" {
                    server_target.set(None, None, || Ok(())).unwrap();
                }
                respond(stream, 200, &envelope("/runtime", "pkg"));
            } else {
                assert_eq!(operation, "config");
                server_target.set(None, None, || Ok(())).unwrap();
                respond(stream, 200, &json!([]));
            }
        });
        let snapshot = target.set(Some(api.url()), None, || Ok(())).unwrap();
        let connection =
            transport::connect_target(snapshot, Path::new("unused"), deadline(), |_| {
                panic!("no bootstrap")
            })
            .unwrap();
        assert!(config_sources_with(&paths, &connection, deadline()).is_err());
    }
}

#[test]
fn malformed_runtime_responses_are_redacted_and_never_allow_post() {
    let temp = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(temp.path());
    let mut bad_target = envelope("/runtime", "PRIVATE\ninvalid");
    bad_target["data"][0]["canUpdate"] = json!(true);
    let mut bad_features = envelope("/runtime", "pkg");
    bad_features["data"][0]["features"] = json!([]);
    let mut bad_source = envelope("/runtime", "pkg");
    bad_source["data"][0]["source"]["type"] = json!("PRIVATE");
    let mut bad_state = envelope("/runtime", "pkg");
    bad_state["data"][0]["state"] = json!({"status":"failed","error":{"PRIVATE":"secret"}});
    for value in [json!([]), bad_target, bad_features, bad_source, bad_state] {
        let posts = Arc::new(AtomicUsize::new(0));
        let count = posts.clone();
        let api = FakeHttp::new(move |request, stream| {
            if request.path == "/api/info" {
                info(stream);
            } else {
                count.fetch_add(usize::from(request.method == "POST"), Ordering::SeqCst);
                respond(stream, 200, &value);
            }
        });
        let connection = connect(&api, None);
        let error = update_with(&paths, "pkg", &connection, deadline()).unwrap_err();
        assert!(!error.message.contains("PRIVATE"), "{error}");
        assert_eq!(posts.load(Ordering::SeqCst), 0);
    }
}

#[test]
fn global_list_shows_pinned_slim_and_local_after_filtering_eighty_eight_builtins() {
    let temp = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(temp.path());
    let local = temp.path().join("ponytail");
    fs::create_dir(&local).unwrap();
    let local = fs::canonicalize(local).unwrap();
    let local_entry = local.join("index.ts").to_string_lossy().into_owned();
    let mut value = envelope("/runtime", "oh-my-opencode-slim@3.0.5");
    let slim = value["data"][0].clone();
    let data = value["data"].as_array_mut().unwrap();
    data.clear();
    for index in 0..71 {
        data.push(json!({"id":format!("builtin.{index}"),"source":{"type":"builtin"},"features":{},"state":{"status":"active"}}));
    }
    data.push(slim);
    data.push(json!({"source":{"type":"local","path":local_entry},"features":{},"state":{"status":"active"}}));
    for index in 71..88 {
        data.push(json!({"id":format!("builtin.{index}"),"source":{"type":"builtin"},"features":{},"state":{"status":"active"}}));
    }
    assert_eq!(data.len(), 90);
    let checks = Arc::new(AtomicUsize::new(0));
    let checked = checks.clone();
    let updates = Arc::new(AtomicUsize::new(0));
    let updated = updates.clone();
    let api = FakeHttp::global(
        &paths,
        json!(["oh-my-opencode-slim@3.0.5", local]),
        move |request, stream| {
            if request.path == "/api/info" {
                info(stream);
            } else if request.method == "POST" {
                if request.path.starts_with("/api/plugin/update") {
                    updated.fetch_add(1, Ordering::SeqCst);
                    respond(stream, 204, &Value::Null);
                } else {
                    assert!(request.path.starts_with("/api/plugin/check?"));
                    assert_eq!(request.body, json!({"target":"oh-my-opencode-slim@3.0.5"}));
                    checked.fetch_add(1, Ordering::SeqCst);
                    respond(stream, 200, &value);
                }
            } else {
                respond(stream, 200, &value);
            }
        },
    );
    let connection = connect(&api, None);
    let result = list_with(&paths, &connection, deadline()).unwrap();
    assert_eq!(result.data.len(), 2);
    assert!(
        matches!(&result.data[0].source, PluginSource::Package {target,..} if target == "oh-my-opencode-slim@3.0.5")
    );
    assert!(!result.data[0].can_update);
    assert!(matches!(&result.data[1].source, PluginSource::Local { .. }));
    assert_eq!(
        check_with(
            &paths,
            Some("oh-my-opencode-slim@3.0.5"),
            &connection,
            deadline()
        )
        .unwrap()
        .data
        .len(),
        2
    );
    assert!(update_with(&paths, "oh-my-opencode-slim@3.0.5", &connection, deadline()).is_err());
    assert_eq!(checks.load(Ordering::SeqCst), 1);
    assert_eq!(updates.load(Ordering::SeqCst), 0);
}

#[test]
fn global_whitelist_rejects_project_packages_and_pins_and_preserves_hidden_ambiguity() {
    let temp = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(temp.path());
    let mut value = envelope("/runtime", "global");
    value["data"].as_array_mut().unwrap().extend([
        envelope("/runtime", "project-only")["data"][0].clone(),
        envelope("/runtime", "pin@1.2.3")["data"][0].clone(),
        json!({"id":"global","source":{"type":"builtin"},"features":{},"state":{"status":"active"}}),
    ]);
    let posts = Arc::new(AtomicUsize::new(0));
    let count = posts.clone();
    let api = FakeHttp::global(
        &paths,
        json!(["global", "pin@1.2.3"]),
        move |request, stream| {
            if request.path == "/api/info" {
                info(stream);
            } else {
                count.fetch_add(usize::from(request.method == "POST"), Ordering::SeqCst);
                respond(stream, 200, &value);
            }
        },
    );
    let connection = connect(&api, None);
    let result = list_with(&paths, &connection, deadline()).unwrap();
    assert_eq!(result.data.len(), 2);
    assert!(result.data.iter().all(|item| !item.can_update));
    for target in ["project-only", "pin@1.2.3", "global"] {
        assert!(update_with(&paths, target, &connection, deadline()).is_err());
    }
    assert!(check_with(&paths, Some("project-only"), &connection, deadline()).is_err());
    assert_eq!(posts.load(Ordering::SeqCst), 0);
}

#[test]
fn global_local_paths_use_component_boundaries_and_canonical_paths() {
    let temp = tempfile::tempdir().unwrap();
    let config = temp.path().join("config");
    fs::create_dir_all(config.join("declared")).unwrap();
    fs::create_dir_all(config.join("plugins")).unwrap();
    fs::create_dir_all(config.join("declared-extra")).unwrap();
    fs::create_dir_all(config.join("plugins-extra")).unwrap();
    let config = fs::canonicalize(config).unwrap();
    let paths = ConfigPaths::with_parts(temp.path().into(), None, Some(config.clone()));
    fs::write(
        config.join("opencode.jsonc"),
        r#"{"plugins":["./declared"]}"#,
    )
    .unwrap();
    let preview = project_config_sources("/runtime".into(), serde_json::from_value(json!([
        {"type":"document","path":config.join("opencode.jsonc"),"info":{"plugins":["./declared"]}},
        {"type":"directory","path":config}
    ])).unwrap()).unwrap();
    let candidates = [
        config.join("declared/index.ts"),
        config.join("plugins/auto.ts"),
        config.join("declared-extra/no.ts"),
        config.join("plugins-extra/no.ts"),
        config.join("unrelated.ts"),
    ];
    let data: Vec<_> = candidates.iter().map(|path| json!({"source":{"type":"local","path":path},"features":{},"state":{"status":"active"}})).collect();
    let runtime = project_runtime(
        serde_json::from_value(json!({"location":{"directory":"/runtime"},"data":data})).unwrap(),
    )
    .unwrap();
    let scope = global::GlobalPaths::capture(&paths).unwrap();
    let result = global::filter_runtime(runtime, &scope.filter(preview).unwrap()).unwrap();
    assert_eq!(result.data.len(), 2);
    assert!(!result.data[0].can_remove);
    assert!(!result.data[1].can_remove);
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let outside = temp.path().join("outside");
        fs::create_dir(&outside).unwrap();
        symlink(&outside, config.join("declared/escape")).unwrap();
        let value = json!({"location":{"directory":"/runtime"},"data":[{"source":{"type":"local","path":config.join("declared/escape/no.ts")},"features":{},"state":{"status":"active"}}]});
        let runtime = project_runtime(serde_json::from_value(value).unwrap()).unwrap();
        let preview = project_config_sources("/runtime".into(), serde_json::from_value(json!([{"type":"document","path":config.join("opencode.jsonc"),"info":{"plugins":["./declared"]}}])).unwrap()).unwrap();
        assert!(global::filter_runtime(runtime, &preview)
            .unwrap()
            .data
            .is_empty());
    }
}

#[test]
fn global_config_failure_never_returns_unfiltered_runtime_or_posts() {
    let temp = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(temp.path());
    let posts = Arc::new(AtomicUsize::new(0));
    let count = posts.clone();
    let api = FakeHttp::new(move |request, stream| {
        count.fetch_add(usize::from(request.method == "POST"), Ordering::SeqCst);
        if request.path == "/api/info" {
            info(stream);
        } else if request.path.starts_with("/api/config?") {
            respond(stream, 503, &json!({"PRIVATE":"secret"}));
        } else {
            respond(stream, 200, &envelope("/runtime", "project-only"));
        }
    });
    let connection = connect(&api, None);
    assert!(list_with(&paths, &connection, deadline()).is_err());
    assert!(config_sources_with(&paths, &connection, deadline()).is_err());
    assert!(check_with(&paths, None, &connection, deadline()).is_err());
    assert!(update_with(&paths, "project-only", &connection, deadline()).is_err());
    assert_eq!(posts.load(Ordering::SeqCst), 0);
}

#[test]
fn check_all_sends_only_global_unique_targets_and_stops_on_first_failed_post() {
    let temp = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(temp.path());
    for fail in [false, true] {
        let mut value = envelope("/runtime", "global");
        value["data"].as_array_mut().unwrap().extend([
            envelope("/runtime", "pin@1.2.3")["data"][0].clone(),
            envelope("/runtime", "project-only")["data"][0].clone(),
        ]);
        let calls = Arc::new(std::sync::Mutex::new(Vec::new()));
        let recorded = calls.clone();
        let api = FakeHttp::global(
            &paths,
            json!(["global", "pin@1.2.3"]),
            move |request, stream| {
                if request.path == "/api/info" {
                    info(stream);
                } else if request.method == "POST" {
                    assert!(request.path.starts_with("/api/plugin/check?"));
                    let target = request.body["target"].as_str().unwrap().to_owned();
                    assert_ne!(target, "project-only");
                    recorded.lock().unwrap().push(target);
                    respond(stream, if fail { 503 } else { 200 }, &value);
                } else {
                    respond(stream, 200, &value);
                }
            },
        );
        let connection = connect(&api, None);
        let result = check_with(&paths, None, &connection, deadline());
        if fail {
            assert!(result.unwrap_err().message.contains("outcome unknown"));
        } else {
            assert_eq!(result.unwrap().data.len(), 2);
        }
        assert_eq!(
            *calls.lock().unwrap(),
            if fail {
                vec!["global"]
            } else {
                vec!["global", "pin@1.2.3"]
            }
        );
    }
}

#[cfg(unix)]
#[test]
fn config_preview_matches_global_canonical_sources_and_rejects_source_retargeting() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let first = temp.path().join("first.jsonc");
    let second = temp.path().join("second.jsonc");
    fs::write(&first, "{}").unwrap();
    fs::write(&second, "{}").unwrap();
    let logical = temp.path().join("global.jsonc");
    symlink(&first, &logical).unwrap();
    let paths = ConfigPaths::with_parts(temp.path().into(), Some(logical.clone()), None);
    let source = fs::canonicalize(&first).unwrap();
    let scope = global::GlobalPaths::capture(&paths).unwrap();
    let preview = project_config_sources("/runtime".into(), serde_json::from_value(json!([
        {"type":"document","path":source,"info":{"plugins":["global"]}},
        {"type":"document","path":temp.path().join("project.jsonc"),"info":{"plugins":["project"]}},
        {"type":"document","info":{"plugins":["inline"]}}
    ])).unwrap()).unwrap();
    assert_eq!(scope.filter(preview).unwrap().data.len(), 1);
    let api = FakeHttp::new(move |request, stream| {
        if request.path == "/api/info" {
            info(stream);
        } else if request.path == "/api/plugin" {
            respond(stream, 200, &envelope("/runtime", "global"));
        } else {
            fs::remove_file(&logical).unwrap();
            symlink(&second, &logical).unwrap();
            respond(
                stream,
                200,
                &json!([{ "type":"document","path":source,"info":{"plugins":["global"]}}]),
            );
        }
    });
    let connection = connect(&api, None);
    assert!(list_with(&paths, &connection, deadline())
        .unwrap_err()
        .message
        .contains("source changed"));
}

#[cfg(unix)]
#[test]
fn plugin_post_source_retargeting_reports_unknown_outcome_without_retry() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    for update in [false, true] {
        let first = temp.path().join(format!("first-{update}.jsonc"));
        let second = temp.path().join(format!("second-{update}.jsonc"));
        fs::write(&first, "{}").unwrap();
        fs::write(&second, "{}").unwrap();
        let logical = temp.path().join(format!("global-{update}.jsonc"));
        symlink(&first, &logical).unwrap();
        let paths = ConfigPaths::with_parts(temp.path().into(), Some(logical.clone()), None);
        let posts = Arc::new(AtomicUsize::new(0));
        let count = posts.clone();
        let api = FakeHttp::global(&paths, json!(["global"]), move |request, stream| {
            if request.path == "/api/info" {
                info(stream);
            } else if request.method == "POST" {
                count.fetch_add(1, Ordering::SeqCst);
                fs::remove_file(&logical).unwrap();
                symlink(&second, &logical).unwrap();
                respond(
                    stream,
                    if update { 204 } else { 200 },
                    &envelope("/runtime", "global"),
                );
            } else {
                respond(stream, 200, &envelope("/runtime", "global"));
            }
        });
        let connection = connect(&api, None);
        let error = if update {
            update_with(&paths, "global", &connection, deadline()).unwrap_err()
        } else {
            check_with(&paths, Some("global"), &connection, deadline()).unwrap_err()
        };
        assert!(error.message.contains("outcome unknown"), "{error}");
        assert_eq!(posts.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn public_list_checks_each_global_target_once_and_preview_never_posts() {
    let temp = tempfile::tempdir().unwrap();
    let paths = ConfigPaths::for_home(temp.path());
    crate::opencode::target::initialize(&paths).unwrap();
    for fail_on in [None, Some(0), Some(1)] {
        let checks = Arc::new(std::sync::Mutex::new(Vec::new()));
        let recorded = checks.clone();
        let api = FakeHttp::global(
            &paths,
            json!(["global-a", "global-b"]),
            move |request, stream| {
                if request.path == "/api/info" {
                    info(stream);
                    return;
                }
                let directory = std::env::current_dir()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                let mut value = envelope(&directory, "global-a");
                value["data"].as_array_mut().unwrap().extend([
                    envelope("/runtime", "global-b")["data"][0].clone(),
                    envelope("/runtime", "project-only")["data"][0].clone(),
                ]);
                for item in value["data"].as_array_mut().unwrap() {
                    item["source"].as_object_mut().unwrap().remove("outdated");
                }
                let url =
                    reqwest::Url::parse(&format!("http://localhost{}", request.path)).unwrap();
                let mut calls = recorded.lock().unwrap();
                if request.method == "POST" {
                    assert_eq!(url.path(), "/api/plugin/check");
                    assert_eq!(
                        url.query_pairs().collect::<Vec<_>>(),
                        [("location[directory]".into(), directory.into())]
                    );
                    let target = if calls.is_empty() {
                        "global-a"
                    } else {
                        "global-b"
                    };
                    assert_eq!(request.body, json!({"target":target}));
                    assert!(!calls.iter().any(|previous| previous == target));
                    let failure = fail_on == Some(calls.len());
                    calls.push(target.to_owned());
                    if failure {
                        respond(stream, 503, &json!({"PRIVATE":"secret"}));
                        return;
                    }
                } else {
                    assert_eq!(url.path(), "/api/plugin");
                }
                for (index, target) in ["global-a", "global-b"].into_iter().enumerate() {
                    if calls.iter().any(|checked| checked == target) {
                        value["data"][index]["source"]["outdated"] = json!(true);
                    }
                }
                respond(stream, 200, &value);
            },
        );
        crate::opencode::target::runtime()
            .unwrap()
            .set(Some(api.url()), None, || Ok(()))
            .unwrap();
        // The public preview must not initiate checks, even before the first list load.
        config_sources(&paths).unwrap();
        assert!(checks.lock().unwrap().is_empty());
        let result = list(&paths);
        let calls = checks.lock().unwrap().clone();
        if let Some(index) = fail_on {
            let error = result.unwrap_err();
            assert!(error.message.contains("outcome unknown"));
            assert!(!error.message.contains("PRIVATE"));
            assert_eq!(calls.len(), index + 1);
        } else {
            let runtime = result.unwrap();
            assert_eq!(runtime.data.len(), 2);
            assert!(runtime.data.iter().all(|item| item.can_update));
            assert_eq!(calls, ["global-a", "global-b"]);
        }
        config_sources(&paths).unwrap();
        assert_eq!(*checks.lock().unwrap(), calls);
    }
    crate::opencode::target::runtime()
        .unwrap()
        .set(None, None, || Ok(()))
        .unwrap();
}
