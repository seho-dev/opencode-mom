use std::fs;
use std::io::Write;
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

use serde_json::{json, Value};

use super::*;
use crate::opencode_cli::token_usage::tests::{
    connection, deadline, healthy_response, page, respond, FakeApi,
};
use crate::opencode_cli::token_usage::{Message, Session};

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("opencode-mom-http-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn registration(&self) -> PathBuf {
        self.0.join("service.json")
    }

    fn write(&self, value: &Value) {
        fs::write(self.registration(), serde_json::to_vec(value).unwrap()).unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn registration(url: &str) -> Value {
    json!({
        "id": uuid::Uuid::new_v4().to_string(),
        "version": "2.0.22",
        "url": url,
        "pid": 42,
        "password": "private",
    })
}

#[test]
fn invalid_registration_never_connects_or_bootstraps_and_errors_are_redacted() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let fixture = Fixture::new();
    for (field, value) in [
        ("url", json!("http://localhost:1234")),
        ("url", json!("http://example.com:1234")),
        ("url", json!("http://192.0.2.1:1234")),
        ("url", json!("http://127.0.0.2:1234")),
        ("url", json!("http://127.1:1234")),
        ("url", json!("http://2130706433:1234")),
        ("url", json!("http://127.0.0.1")),
        ("url", json!("http://127.0.0.1:0")),
        ("url", json!("https://127.0.0.1:1234")),
        ("url", json!("http://[::2]:1234")),
        ("url", json!(format!("{url}/api"))),
        ("url", json!(format!("{url}/?PRIVATE_QUERY"))),
        ("url", json!(format!("{url}/#PRIVATE_FRAGMENT"))),
        (
            "url",
            json!(format!(
                "http://PRIVATE_USER@{}",
                listener.local_addr().unwrap()
            )),
        ),
        ("id", json!("PRIVATE_INVALID_ID")),
        ("version", json!("1.0.0")),
        ("version", json!("2.garbage")),
        ("version", json!("")),
        ("pid", json!(0)),
        ("pid", json!(-1)),
        ("password", json!("")),
        ("password", json!("PRIVATE\nPASSWORD")),
        ("password", json!("x".repeat(4097))),
    ] {
        let mut value_with_error = registration(&url);
        value_with_error[field] = value;
        fixture.write(&value_with_error);
        let error = connect_at(&fixture.registration(), deadline(), |_| {
            panic!("invalid registration must not bootstrap")
        })
        .err()
        .unwrap();
        assert!(!error.message.contains("PRIVATE"));
        assert!(error.detail.is_none());
        assert!(
            matches!(listener.accept(), Err(error) if error.kind() == io::ErrorKind::WouldBlock)
        );
    }
    for bytes in [b"PRIVATE_INVALID_JSON".to_vec(), vec![b'x'; 8193]] {
        fs::write(fixture.registration(), bytes).unwrap();
        let error = connect_at(&fixture.registration(), deadline(), |_| {
            panic!("invalid registration must not bootstrap")
        })
        .err()
        .unwrap();
        assert!(!error.message.contains("PRIVATE"));
    }
    let ipv6 = serde_json::from_value(registration("http://[::1]:80/")).unwrap();
    assert!(ManagedConnection::new(client().unwrap(), ipv6).is_ok());
    let explicit_http_port = serde_json::from_value(registration("http://127.0.0.1:80")).unwrap();
    assert!(ManagedConnection::new(client().unwrap(), explicit_http_port).is_ok());
}

#[cfg(unix)]
#[test]
fn fifo_and_symlink_fifo_registrations_fail_without_waiting_or_bootstrap() {
    use std::ffi::CString;
    use std::os::unix::{ffi::OsStrExt, fs::symlink};

    const PROBE_ENV: &str = "OPENCODE_MOM_REGISTRATION_FIFO_PROBE";
    if let Some(directory) = std::env::var_os(PROBE_ENV) {
        let directory = PathBuf::from(directory);
        for path in [
            directory.join("PRIVATE_FIFO_CONTENT_PASSWORD"),
            directory.join("service.json"),
        ] {
            let started = Instant::now();
            let error = connect_at(&path, started + Duration::from_millis(500), |_| {
                panic!("nonregular registration must not bootstrap")
            })
            .err()
            .unwrap();
            assert!(started.elapsed() < Duration::from_millis(500));
            assert_eq!(
                error.message,
                "opencode token usage: invalid managed service registration file type."
            );
            assert!(error.detail.is_none());
        }
        return;
    }

    let fixture = Fixture::new();
    let fifo = fixture.0.join("PRIVATE_FIFO_CONTENT_PASSWORD");
    let path = CString::new(fifo.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
    symlink(&fifo, fixture.registration()).unwrap();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "opencode_cli::token_usage::transport::tests::fifo_and_symlink_fifo_registrations_fail_without_waiting_or_bootstrap",
            "--test-threads=1",
        ])
        .env(PROBE_ENV, &fixture.0)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let started = Instant::now();
    while child.try_wait().unwrap().is_none() {
        if started.elapsed() >= Duration::from_secs(2) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("isolated FIFO registration check timed out");
        }
        thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "isolated FIFO registration check failed: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[cfg(unix)]
#[test]
fn symlink_to_regular_registration_remains_readable() {
    let fixture = Fixture::new();
    fixture.write(&registration("http://127.0.0.1:1234"));
    let target = fixture.0.join("regular.json");
    fs::rename(fixture.registration(), &target).unwrap();
    std::os::unix::fs::symlink(target, fixture.registration()).unwrap();
    let registration = read_registration(&fixture.registration(), deadline())
        .unwrap()
        .unwrap();
    assert_eq!(registration.url, "http://127.0.0.1:1234");
    assert_eq!(registration.password.as_deref(), Some("private"));
}

#[test]
fn bootstrap_occurs_once_for_missing_unreachable_or_mismatched_registration() {
    let api = FakeApi::new(|target, stream| {
        assert!(target == "/api/info");
        healthy_response(stream);
    });
    for cause in ["missing", "unreachable", "pid", "version"] {
        let fixture = Fixture::new();
        match cause {
            "unreachable" => {
                let listener = TcpListener::bind("127.0.0.1:0").unwrap();
                fixture.write(&registration(&format!(
                    "http://{}",
                    listener.local_addr().unwrap()
                )));
                drop(listener);
            }
            "pid" | "version" => {
                let mut value = registration(&api.url());
                value[cause] = if cause == "pid" {
                    json!(43)
                } else {
                    json!("2.0.23")
                };
                fixture.write(&value);
            }
            _ => {}
        }
        let calls = AtomicUsize::new(0);
        let connection = connect_at(&fixture.registration(), deadline(), |_| {
            calls.fetch_add(1, Ordering::Relaxed);
            fixture.write(&registration(&api.url()));
            Ok(())
        })
        .unwrap();
        assert_eq!(connection.registration.pid, 42);
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        let _ = connect_at(&fixture.registration(), deadline(), |_| {
            panic!("healthy registration must not bootstrap")
        })
        .unwrap();
    }
}

#[test]
fn identity_mismatch_after_bootstrap_never_fetches_any_sessions() {
    for mismatch in ["pid", "version"] {
        let pages = Arc::new(AtomicUsize::new(0));
        let requested = pages.clone();
        let api = FakeApi::new(move |target, stream| {
            if target != "/api/info" {
                requested.fetch_add(1, Ordering::Relaxed);
            }
            let mut info = json!({ "pid": 42, "version": "2.0.22" });
            info[mismatch] = if mismatch == "pid" {
                json!(43)
            } else {
                json!("2.0.23")
            };
            respond(stream, 200, "", &serde_json::to_vec(&info).unwrap());
        });
        let fixture = Fixture::new();
        fixture.write(&registration(&api.url()));
        let calls = AtomicUsize::new(0);
        let error = connect_at(&fixture.registration(), deadline(), |_| {
            calls.fetch_add(1, Ordering::Relaxed);
            Ok(())
        })
        .err()
        .unwrap();
        assert!(error.message.contains("identity mismatch"));
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        assert_eq!(pages.load(Ordering::Relaxed), 0);
    }
}

#[test]
fn non_200_and_cross_host_redirects_never_follow_or_expose_error_bodies() {
    let target = TcpListener::bind("127.0.0.1:0").unwrap();
    target.set_nonblocking(true).unwrap();
    let destination = format!("http://{}/PRIVATE_REDIRECT", target.local_addr().unwrap());
    for status in [302, 401, 403, 500] {
        let location = destination.clone();
        let api = FakeApi::new(move |path, stream| {
            if path == "/api/info" {
                healthy_response(stream);
            } else {
                respond(
                    stream,
                    status,
                    &format!("Location: {location}\r\n"),
                    b"PRIVATE_ERROR_BODY",
                );
            }
        });
        let error = connection(&api)
            .fetch_page::<Session>(None, None, deadline())
            .err()
            .unwrap();
        assert_eq!(
            error.message,
            format!("opencode token usage: API HTTP {status}.")
        );
        assert!(error.detail.is_none());
        assert!(matches!(target.accept(), Err(error) if error.kind() == io::ErrorKind::WouldBlock));
    }
}

#[test]
fn health_unauthorized_fails_closed_without_bootstrap() {
    let api = FakeApi::new(|_, stream| {
        respond(stream, 401, "", b"PRIVATE_AUTH_ERROR");
    });
    let fixture = Fixture::new();
    fixture.write(&registration(&api.url()));
    let error = connect_at(&fixture.registration(), deadline(), |_| {
        panic!("unauthorized service must not bootstrap another target")
    })
    .err()
    .unwrap();
    assert_eq!(error.message, "opencode token usage: API HTTP 401.");
}

#[test]
fn fixed_target_selects_b_for_active_catalog_reload_usage_and_async_island_never_registered_a() {
    use crate::opencode_cli::target::{Selection, Snapshot};
    use crate::opencode_cli::{active, models, token_usage};

    let a_calls = Arc::new(AtomicUsize::new(0));
    let calls = a_calls.clone();
    let a = FakeApi::new(move |_, stream| {
        calls.fetch_add(1, Ordering::Relaxed);
        healthy_response(stream);
    });
    let fixture = Fixture::new();
    fixture.write(&registration(&a.url()));
    let b_calls = Arc::new(AtomicUsize::new(0));
    let calls = b_calls.clone();
    let b = FakeApi::new(move |path, stream| {
        calls.fetch_add(1, Ordering::Relaxed);
        match path {
            "/api/info" => healthy_response(stream),
            "/api/session/active" => respond(stream, 200, "", br#"{"data":{"ses_b":{"type":"running"}}}"#),
            "/api/model" => respond(stream, 200, "", br#"{"location":{"directory":"/b"},"data":[{"id":"provider/model","providerID":"provider","modelID":"model","enabled":true}]}"#),
            "/api/location/reload" => respond(stream, 204, "", b""),
            "/api/session?limit=1000&order=asc" => respond(stream, 200, "", br#"{"data":[],"cursor":{"next":null}}"#),
            _ => panic!("unexpected fixture route"),
        }
    });
    let snapshot = Snapshot::detached(Selection {
        url: Some(b.url()),
        password: Some("private".into()),
    });
    let connection = connect_target(
        snapshot.clone(),
        &fixture.registration(),
        deadline(),
        |_| panic!("explicit never bootstraps"),
    )
    .unwrap();
    assert_eq!(active::active_sessions(&connection, deadline()).unwrap(), 1);
    // Test the real catalog parser independently of local configuration metadata.
    assert!(models::model_refs(&connection, deadline())
        .unwrap()
        .contains("provider/model"));
    connection.reload(deadline()).unwrap();
    assert!(token_usage::scan(&connection, deadline())
        .unwrap()
        .is_empty());
    let async_connection = connection.into_async().unwrap();
    let result: Value = tauri::async_runtime::block_on(async {
        async_connection
            .request(reqwest::Method::GET, &["session", "active"])
            .unwrap()
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap()
    });
    assert_eq!(result["data"]["ses_b"]["type"], "running");
    let status = service_status(snapshot, deadline()).unwrap();
    assert!(status.connected);
    assert_eq!(status.url.as_deref(), Some(b.url().as_str()));
    assert_eq!(a_calls.load(Ordering::Relaxed), 0);
    assert!(b_calls.load(Ordering::Relaxed) >= 7);
}

#[test]
fn explicit_unreachable_unauthorized_redirect_and_invalid_info_never_bootstrap_or_fallback() {
    use crate::opencode_cli::target::{Selection, Snapshot};
    let fixture = Fixture::new();
    let a_calls = Arc::new(AtomicUsize::new(0));
    let count = a_calls.clone();
    let a = FakeApi::new(move |_, stream| {
        count.fetch_add(1, Ordering::Relaxed);
        healthy_response(stream);
    });
    fixture.write(&registration(&a.url()));
    for (status, body) in [
        (401, "PRIVATE_AUTH"),
        (403, "PRIVATE_AUTH"),
        (302, "PRIVATE_REDIRECT"),
        (200, "{\"pid\":0,\"version\":\"2.0.22\"}"),
        (200, "{\"pid\":42,\"version\":\"1.0.0\"}"),
        (200, "{\"pid\":42,\"version\":\"2.garbage\"}"),
        (200, "[42,\"2.0.22\"]"),
        (200, "PRIVATE_INVALID_JSON"),
    ] {
        let b = FakeApi::new(move |_, stream| {
            respond(
                stream,
                status,
                "Location: http://127.0.0.1:1234\r\n",
                body.as_bytes(),
            )
        });
        let snapshot = Snapshot::detached(Selection {
            url: Some(b.url()),
            password: Some("private".into()),
        });
        let error = connect_target(
            snapshot.clone(),
            &fixture.registration(),
            deadline(),
            |_| panic!("explicit never bootstraps"),
        )
        .err()
        .unwrap();
        assert!(!error.message.contains("PRIVATE"));
        let status = service_status(snapshot, deadline()).unwrap();
        assert!(!status.connected);
        assert!(status.url.is_none() && status.pid.is_none() && status.version.is_none());
        assert!(!serde_json::to_string(&status).unwrap().contains("private"));
    }
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let snapshot = Snapshot::detached(Selection {
        url: Some(url),
        password: Some("private".into()),
    });
    assert!(
        connect_target(snapshot, &fixture.registration(), deadline(), |_| panic!(
            "down fixed endpoint never bootstraps"
        ))
        .is_err()
    );
    assert_eq!(a_calls.load(Ordering::Relaxed), 0);
}

#[test]
fn v2_identity_requires_bounded_semantic_version_not_only_major_prefix() {
    for version in ["2.0.0", "2.12.34-beta.1", "2.0.22+build.01"] {
        assert!(valid_v2_version(version), "{version}");
    }
    for version in [
        "2.garbage",
        "2.0",
        "2.0.0.0",
        "2.01.0",
        "2.0.00",
        "2.0.-1",
        "2.0.0-",
        "2.0.0-alpha..1",
        "2.0.0-01",
        "2.0.0+",
        "2.0.0+bad+build",
        "2.4294967296.0",
        "2.0.0\n",
        "v2.0.0",
        "1.0.0",
    ] {
        assert!(!valid_v2_version(version), "{version}");
        assert!(validate_info(&Info {
            pid: 42,
            version: version.into()
        })
        .is_err());
    }
    assert!(!valid_v2_version(&format!("2.0.0+{}", "x".repeat(128))));
}

#[test]
fn explicit_without_password_sends_no_authorization_for_health_reads_reload_and_async() {
    use std::io::Read;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = thread::spawn(move || {
        // Connect, read, reload health+POST, async read, write health+POST, status.
        for _ in 0..8 {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut bytes = Vec::new();
            while !bytes.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                stream.read_exact(&mut byte).unwrap();
                bytes.push(byte[0]);
                assert!(bytes.len() <= 8192);
            }
            let headers = String::from_utf8(bytes).unwrap();
            assert!(!headers.to_ascii_lowercase().contains("authorization:"));
            let length = headers
                .lines()
                .find_map(|line| {
                    let (key, value) = line.split_once(':')?;
                    key.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            let mut body = vec![0; length];
            stream.read_exact(&mut body).unwrap();
            match headers.split_whitespace().nth(1).unwrap() {
                "/api/info" => healthy_response(&mut stream),
                "/api/location/reload" => respond(&mut stream, 204, "", b""),
                "/api/session/ses_a/prompt" => respond(&mut stream, 200, "", b"null"),
                _ => respond(&mut stream, 200, "", br#"{"data":{}}"#),
            }
        }
    });
    let snapshot = Snapshot::detached(Selection {
        url: Some(url),
        password: None,
    });
    let connection = connect_target(snapshot.clone(), Path::new("unused"), deadline(), |_| {
        panic!("no bootstrap")
    })
    .unwrap();
    let _: Value = connection
        .get_json(
            &["session", "active"],
            &[],
            deadline(),
            1024,
            "read",
            "1 KiB",
        )
        .unwrap();
    connection.reload(deadline()).unwrap();
    let connection = connection.into_async().unwrap();
    tauri::async_runtime::block_on(async {
        assert!(connection
            .request(reqwest::Method::GET, &["session", "active"])
            .unwrap()
            .send()
            .await
            .unwrap()
            .status()
            .is_success());
        let _reservation = connection.prepare_write(deadline()).await.unwrap();
        assert!(connection
            .request(reqwest::Method::POST, &["session", "ses_a", "prompt"])
            .unwrap()
            .json(&json!({"text":"hello"}))
            .send()
            .await
            .unwrap()
            .status()
            .is_success());
    });
    assert!(service_status(snapshot, deadline()).unwrap().connected);
    server.join().unwrap();
}

#[test]
fn restarted_service_rejects_reload_before_post_without_rebootstrap() {
    let changed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = changed.clone();
    let posts = Arc::new(AtomicUsize::new(0));
    let count = posts.clone();
    let api = FakeApi::new(move |target, stream| {
        if target == "/api/info" {
            respond(stream, 200, "", &serde_json::to_vec(&json!({"pid":if flag.load(Ordering::SeqCst) {43} else {42}, "version":"2.0.22"})).unwrap());
        } else {
            count.fetch_add(1, Ordering::SeqCst);
            respond(stream, 204, "", b"");
        }
    });
    let connection = connection(&api);
    changed.store(true, Ordering::SeqCst);
    assert!(connection
        .reload(deadline())
        .unwrap_err()
        .message
        .contains("identity changed"));
    assert_eq!(posts.load(Ordering::SeqCst), 0);
}

#[test]
fn public_info_does_not_claim_protected_read_ready_or_retry_with_invented_auth() {
    use std::io::Read;
    for status in [401, 403] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let server = thread::spawn(move || {
            for _ in 0..3 {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut bytes = Vec::new();
                while !bytes.ends_with(b"\r\n\r\n") {
                    let mut byte = [0];
                    stream.read_exact(&mut byte).unwrap();
                    bytes.push(byte[0]);
                }
                let headers = String::from_utf8(bytes).unwrap();
                assert!(!headers.to_ascii_lowercase().contains("authorization:"));
                if headers.split_whitespace().nth(1) == Some("/api/info") {
                    healthy_response(&mut stream);
                } else {
                    respond(&mut stream, status, "", b"PRIVATE_AUTH_ERROR");
                }
            }
            listener.set_nonblocking(true).unwrap();
            assert!(
                matches!(listener.accept(), Err(error) if error.kind() == io::ErrorKind::WouldBlock)
            );
        });
        let snapshot = Snapshot::detached(Selection {
            url: Some(url),
            password: None,
        });
        assert!(
            service_status(snapshot.clone(), deadline())
                .unwrap()
                .connected
        );
        let connection = connect_target(snapshot, Path::new("unused"), deadline(), |_| {
            panic!("no fallback")
        })
        .unwrap();
        let error = connection
            .get_json::<Value>(
                &["session", "active"],
                &[],
                deadline(),
                1024,
                "read",
                "1 KiB",
            )
            .unwrap_err();
        assert!(error.message.contains(&format!("HTTP {status}")));
        assert!(!error.message.contains("PRIVATE"));
        server.join().unwrap();
    }
}

#[test]
fn managed_api_does_not_send_requests_to_environment_proxies() {
    const PROBE_ENV: &str = "OPENCODE_MOM_HTTP_PROXY_PROBE";
    if std::env::var_os(PROBE_ENV).is_none() {
        let proxy = TcpListener::bind("127.0.0.1:0").unwrap();
        proxy.set_nonblocking(true).unwrap();
        let url = format!("http://{}", proxy.local_addr().unwrap());
        let mut command = Command::new(std::env::current_exe().unwrap());
        command.args([
            "--exact",
            "opencode_cli::token_usage::transport::tests::managed_api_does_not_send_requests_to_environment_proxies",
            "--test-threads=1",
        ]).env(PROBE_ENV, "1").env("NO_PROXY", "").env("no_proxy", "");
        for name in [
            "HTTP_PROXY",
            "http_proxy",
            "HTTPS_PROXY",
            "https_proxy",
            "ALL_PROXY",
            "all_proxy",
        ] {
            command.env(name, &url);
        }
        let output = command.output().unwrap();
        assert!(output.status.success(), "isolated no-proxy check failed");
        assert!(matches!(proxy.accept(), Err(error) if error.kind() == io::ErrorKind::WouldBlock));
        return;
    }
    let api = FakeApi::new(|target, stream| {
        if target == "/api/info" {
            healthy_response(stream);
        } else {
            respond(
                stream,
                200,
                "",
                &serde_json::to_vec(&page(vec![], None)).unwrap(),
            );
        }
    });
    assert!(connection(&api)
        .fetch_page::<Session>(None, None, deadline())
        .unwrap()
        .data
        .is_empty());
}

#[test]
fn fixed_length_and_chunked_pages_enforce_exact_32_mib_bound() {
    for (chunked, extra) in [(false, false), (false, true), (true, false), (true, true)] {
        let api = FakeApi::new(move |path, stream| {
            if path == "/api/info" {
                healthy_response(stream);
                return;
            }
            let body = serde_json::to_vec(&page(vec![], None)).unwrap();
            let size = PAGE_MAX_BYTES as usize + usize::from(extra);
            if chunked {
                let _ = write!(stream, "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n{size:x}\r\n");
            } else {
                let _ = write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {size}\r\nConnection: close\r\n\r\n"
                );
            }
            if stream.write_all(&body).is_err() {
                return;
            }
            let mut left = size - body.len();
            let padding = [b' '; 8192];
            while left > 0 {
                let count = left.min(padding.len());
                if stream.write_all(&padding[..count]).is_err() {
                    return;
                }
                left -= count;
            }
            if chunked {
                let _ = stream.write_all(b"\r\n0\r\n\r\n");
            }
        });
        let result = connection(&api).fetch_page::<Message>(Some("s"), None, deadline());
        if extra {
            assert!(result.err().unwrap().message.contains("32 MiB"));
        } else {
            assert!(result.unwrap().data.is_empty());
        }
    }
}

#[test]
fn trickling_body_cannot_reset_the_scan_wide_timeout() {
    let api = FakeApi::new(|target, stream| {
        if target == "/api/info" {
            healthy_response(stream);
            return;
        }
        let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 1000\r\n\r\n{");
        for _ in 0..10 {
            thread::sleep(Duration::from_millis(30));
            if stream.write_all(b" ").is_err() {
                return;
            }
        }
    });
    let connection = connection(&api);
    let started = Instant::now();
    assert!(connection
        .fetch_page::<Session>(None, None, started + Duration::from_millis(100))
        .is_err());
    assert!(started.elapsed() < Duration::from_millis(250));
}

#[cfg(unix)]
mod bootstrap {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    fn cli(fixture: &Fixture, script: &str) -> PathBuf {
        let binary = fixture.0.join("opencode");
        fs::write(
            &binary,
            format!("#!/bin/sh\n[ \"$1:$2:$3\" = api:get:/api/info ] || exit 99\n{script}\n"),
        )
        .unwrap();
        fs::set_permissions(&binary, fs::Permissions::from_mode(0o700)).unwrap();
        binary
    }

    #[test]
    fn bootstrap_cli_is_small_bounded_private_and_only_called_when_needed() {
        let api = FakeApi::new(|_, stream| healthy_response(stream));
        let fixture = Fixture::new();
        let value = serde_json::to_string(&registration(&api.url())).unwrap();
        let binary = cli(
            &fixture,
            &format!(
                "printf '%s' '{value}' > '{}'; printf '%s' '{{\"pid\":42,\"version\":\"2.0.22\"}}'",
                fixture.registration().display()
            ),
        );
        let calls = AtomicUsize::new(0);
        let _ = connect_at(&fixture.registration(), deadline(), |deadline| {
            calls.fetch_add(1, Ordering::Relaxed);
            bootstrap_cli(&[binary], deadline)
        })
        .unwrap();
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        for script in [
            "printf PRIVATE_STDOUT; printf PRIVATE_STDERR >&2; exit 7",
            "printf PRIVATE_INVALID_JSON",
            "exec /usr/bin/yes PRIVATE_OUTPUT",
        ] {
            let binary = cli(&fixture, script);
            let error = bootstrap_cli(&[binary], deadline()).unwrap_err();
            assert!(!error.message.contains("PRIVATE"));
            assert!(error.detail.is_none());
        }
        let args = bootstrap_command(Path::new("opencode"))
            .get_args()
            .map(|arg| arg.to_owned())
            .collect::<Vec<_>>();
        assert!(args == ["api", "get", "/api/info"]);
    }

    #[test]
    fn bootstrap_deadline_kills_and_reaps_only_the_cli_child() {
        struct ReapOnDrop(Option<std::process::Child>);

        impl Drop for ReapOnDrop {
            fn drop(&mut self) {
                if let Some(mut child) = self.0.take() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
            }
        }

        let fixture = Fixture::new();
        let ready_file = fixture.0.join("ready");
        let binary = cli(
            &fixture,
            &format!(": > '{}'; exec sleep 60", ready_file.display()),
        );
        let mut child = ReapOnDrop(Some(bootstrap_command(&binary).spawn().unwrap()));
        let pid = child.0.as_ref().unwrap().id() as libc::pid_t;
        // Separate fixture startup from the collector deadline; retain ownership on setup failure.
        let startup = Instant::now();
        while !ready_file.try_exists().unwrap() {
            assert!(
                child.0.as_mut().unwrap().try_wait().unwrap().is_none(),
                "bootstrap fixture exited before readiness"
            );
            assert!(
                startup.elapsed() < Duration::from_secs(5),
                "bootstrap fixture did not become ready"
            );
            thread::sleep(Duration::from_millis(10));
        }
        let started = Instant::now();
        let deadline = started + Duration::from_millis(100);
        let error = collect_output(
            child.0.take().unwrap(),
            deadline
                .saturating_duration_since(Instant::now())
                .min(BOOTSTRAP_TIMEOUT),
            INFO_MAX_BYTES,
            "opencode managed API bootstrap",
            "32 KiB",
        )
        .unwrap_err();
        assert_eq!(
            error.message,
            "opencode managed API bootstrap timed out. Check the OpenCode CLI and retry."
        );
        assert!(started.elapsed() < Duration::from_secs(2));
        let mut status = 0;
        assert_eq!(
            unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) },
            -1
        );
        assert_eq!(
            io::Error::last_os_error().raw_os_error(),
            Some(libc::ECHILD)
        );
    }
}
