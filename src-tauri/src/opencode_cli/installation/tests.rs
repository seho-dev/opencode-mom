use std::cell::Cell;
use std::collections::VecDeque;
use std::ffi::OsStr;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use super::*;
use crate::opencode_cli::binary::resolve_path_command_in;

#[cfg(unix)]
thread_local! {
    static FAKE_EXECUTABLES: std::cell::RefCell<Vec<PathBuf>> = const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg(unix)]
pub(super) fn is_fake_executable(path: &Path) -> bool {
    let Ok(path) = path.canonicalize() else {
        return false;
    };
    FAKE_EXECUTABLES.with(|paths| paths.borrow().contains(&path))
}

fn latest(text: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "channel": "latest", "name": "cli", "distribution": "npm", "version": text,
        "metadata": { "package": "@opencode/cli" }, "active": true,
    }))
    .unwrap()
}

#[test]
fn versions_and_latest_metadata_require_bounded_stable_v2_identity() {
    assert_eq!(version(" v2.0.25\n").unwrap().text, "2.0.25");
    assert!(version("2.0.25-beta.1").unwrap().unstable);
    assert!(stable_v2("2.0.10").unwrap().numbers > stable_v2("2.0.9").unwrap().numbers);
    for invalid in [
        "",
        "2.0",
        "2.0.0.1",
        "02.0.0",
        "2.00.1",
        "OpenCode 2.0.25",
        "2.0.25\n3.0.0",
        "2.0.25+build",
        "2.0.25-beta..1",
        "2.0.25-beta.01",
        "2.0.25;echo token",
        "4294967296.0.0",
    ] {
        assert!(version(invalid).is_none(), "{invalid}");
    }
    assert!(version(&format!("2.0.0-{}", "x".repeat(96))).is_none());
    assert_eq!(
        latest_version(&latest("2.17.300")).unwrap().text,
        "2.17.300"
    );
    for invalid in ["1.9.9", "3.0.0", "2.0.25-beta.1", "2.0.25+build"] {
        assert!(stable_v2(invalid).is_err());
        assert!(latest_version(&latest(invalid)).is_err());
    }
    for (key, bad) in [
        ("channel", serde_json::json!("beta")),
        ("name", serde_json::json!("desktop")),
        ("distribution", serde_json::json!("docker")),
        ("active", serde_json::json!(false)),
        ("metadata", serde_json::json!({"package": "opencode-ai"})),
    ] {
        let mut value: serde_json::Value = serde_json::from_slice(&latest("2.0.25")).unwrap();
        value[key] = bad;
        assert!(
            latest_version(&serde_json::to_vec(&value).unwrap()).is_err(),
            "{key}"
        );
    }
    assert!(latest_version(b"{}").is_err());
    assert_eq!(LATEST_URL, "https://opencode.ai/update/api/latest/cli/npm");
}

#[test]
fn explicit_override_never_falls_back_and_dtos_keep_the_exact_wire_contract() {
    let fallback = PathBuf::from("/unrelated/opencode");
    for explicit in ["", "/missing/opencode"] {
        assert_eq!(
            selected_binary_with(Some(OsStr::new(explicit)), [fallback.clone()], |path| (path
                == fallback)
                .then(|| fallback.clone())),
            None
        );
    }
    assert_eq!(
        selected_binary_with(None, [fallback.clone()], |path| (path == fallback)
            .then(|| fallback.clone())),
        Some(fallback)
    );
    assert_eq!(
        serde_json::to_value(not_found().info).unwrap(),
        serde_json::json!({
            "binaryPath": null, "version": null, "method": "not-found", "canUpgrade": false,
            "upgradeReason": "not_found", "manualCommand": null,
        })
    );
    let check = serde_json::to_value(OpenCodeUpdateCheck {
        installation: not_found().info,
        latest_version: "2.0.25".into(),
        update_available: false,
    })
    .unwrap();
    assert_eq!(check["latestVersion"], "2.0.25");
    assert_eq!(check["updateAvailable"], false);
}

fn detected(current: &str) -> Detected {
    let path = PathBuf::from("/home/test/.opencode/bin/opencode");
    Detected {
        info: OpenCodeInstallation {
            binary_path: Some(path.to_string_lossy().into_owned()),
            version: Some(current.into()),
            method: "curl".into(),
            can_upgrade: true,
            upgrade_reason: None,
            manual_command: manual_command("curl"),
        },
        logical: Some(path.clone()),
        canonical: Some(path),
        source: Some(Source::Curl {
            bash: PathBuf::from("/bin/bash"),
            curl: PathBuf::from("/usr/bin/curl"),
            home: PathBuf::from("/home/test"),
        }),
    }
}

#[test]
fn upgrade_rejects_stale_target_downgrade_v1_v3_prerelease_and_unverified_old_v2() {
    let before = detected("2.0.25");
    for (path, current, target, upstream) in [
        ("/different/opencode", "2.0.25", "2.0.26", "2.0.26"),
        (
            "/home/test/.opencode/bin/opencode",
            "2.0.26",
            "2.0.27",
            "2.0.27",
        ),
        (
            "/home/test/.opencode/bin/opencode",
            "2.0.25",
            "2.0.26",
            "2.0.27",
        ),
        (
            "/home/test/.opencode/bin/opencode",
            "2.0.25",
            "2.0.25",
            "2.0.25",
        ),
        (
            "/home/test/.opencode/bin/opencode",
            "2.0.25",
            "2.0.24",
            "2.0.24",
        ),
        (
            "/home/test/.opencode/bin/opencode",
            "2.0.25",
            "3.0.0",
            "3.0.0",
        ),
        (
            "/home/test/.opencode/bin/opencode",
            "1.9.9",
            "2.0.26",
            "2.0.26",
        ),
        (
            "/home/test/.opencode/bin/opencode",
            "2.0.24",
            "2.0.26",
            "2.0.26",
        ),
        (
            "/home/test/.opencode/bin/opencode",
            "2.0.25",
            "2.0.26-beta.1",
            "2.0.26-beta.1",
        ),
        (
            "/home/test/.opencode/bin/opencode",
            "2.0.25",
            "2.0.26",
            "3.0.0",
        ),
    ] {
        assert!(guarded_upgrade(
            path,
            current,
            target,
            |_| Ok(before.clone()),
            || Ok(version(upstream).unwrap()),
            |_, _| panic!("must not upgrade")
        )
        .is_err());
    }
}

#[test]
fn upgrade_revalidates_source_after_http_and_refuses_manual_or_missing_ownership() {
    let before = detected("2.0.25");
    for change in 0..5 {
        let mut changed = before.clone();
        match change {
            0 => changed.canonical = Some(PathBuf::from("/unrelated/opencode")),
            1 => {
                changed.source = Some(Source::Manager {
                    method: "npm",
                    manager: "/npm".into(),
                    root: "/global/node_modules".into(),
                    bin: "/global/bin".into(),
                })
            }
            2 => changed.source = None,
            3 => changed.info.can_upgrade = false,
            _ => changed.info.version = Some("2.0.26".into()),
        }
        let mut reads = VecDeque::from([before.clone(), changed]);
        assert!(guarded_upgrade(
            before.info.binary_path.as_deref().unwrap(),
            "2.0.25",
            "2.0.26",
            |_| Ok(reads.pop_front().unwrap()),
            || stable_v2("2.0.26"),
            |_, _| panic!("must not upgrade")
        )
        .is_err());
    }
}

#[test]
fn upgrade_reads_back_only_the_original_logical_binary_and_exact_target() {
    let before = detected("2.0.25");
    for (current, changed_path) in [
        ("2.0.26", false),
        ("2.0.25", false),
        ("2.0.27", false),
        ("2.0.26", true),
    ] {
        let mut after = detected(current);
        after.canonical = Some(PathBuf::from("/new/version/opencode"));
        if changed_path {
            after.logical = Some("/unrelated/opencode".into());
        }
        let mut reads = VecDeque::from([before.clone(), before.clone(), after]);
        let calls = Cell::new(0);
        let result = guarded_upgrade(
            before.info.binary_path.as_deref().unwrap(),
            "2.0.25",
            "2.0.26",
            |path| {
                assert_eq!(
                    path,
                    if reads.len() == 1 {
                        before.logical.as_deref()
                    } else {
                        None
                    }
                );
                Ok(reads.pop_front().unwrap())
            },
            || stable_v2("2.0.26"),
            |_, target| {
                assert_eq!(target, "2.0.26");
                calls.set(calls.get() + 1);
                Ok(())
            },
        );
        assert_eq!(result.is_ok(), current == "2.0.26" && !changed_path);
        assert_eq!(calls.get(), 1);
    }
    let calls = Cell::new(0);
    let error = guarded_upgrade(
        before.info.binary_path.as_deref().unwrap(),
        "2.0.25",
        "2.0.26",
        |_| {
            calls.set(calls.get() + 1);
            Ok(before.clone())
        },
        || stable_v2("2.0.26"),
        |_, _| Err(AppError::configuration("fake upgrade failed")),
    )
    .unwrap_err();
    assert_eq!(error.message, "fake upgrade failed");
    assert_eq!(calls.get(), 2);
}

#[test]
fn concurrent_upgrade_returns_busy_before_discovery_or_network() {
    let _guard = UPGRADE_LOCK.lock().unwrap();
    assert_eq!(
        upgrade("/never/executed", "2.0.25", "2.0.26")
            .unwrap_err()
            .code,
        ErrorCode::Busy
    );
}

#[test]
fn native_command_pins_selected_binary_target_and_method_with_sanitized_environment() {
    for method in ["curl", "npm", "bun", "pnpm", "yarn"] {
        let mut value = detected("2.0.25");
        if method != "curl" {
            value.source = Some(Source::Manager {
                method,
                manager: "/never-execute-manager".into(),
                root: "/global/node_modules".into(),
                bin: "/global/bin".into(),
            });
        }
        let command = upgrade_command(&value, "v2.0.26").unwrap();
        assert_eq!(command.get_program(), value.logical.as_ref().unwrap());
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            ["upgrade", "2.0.26", "--method", method]
        );
        for (key, _) in command.get_envs() {
            let key = key.to_string_lossy().to_ascii_uppercase();
            assert!(
                !key.contains("TOKEN")
                    && !key.contains("PASSWORD")
                    && !key.contains("API_KEY")
                    && key != "BASH_ENV"
                    && key != "NODE_OPTIONS"
            );
        }
        assert!(command
            .get_envs()
            .any(|(key, value)| key == "OPENCODE_DISABLE_AUTOUPDATE"
                && value == Some(OsStr::new("1"))));
        assert!(upgrade_command(&value, "2.0.26;echo secret").is_err());
    }
    let mut unknown = detected("2.0.25");
    unknown.source = None;
    assert!(upgrade_command(&unknown, "2.0.26").is_err());
    assert_eq!(
        manual_command("bun").unwrap(),
        "bun install -g --trust @opencode/cli@2"
    );
    assert_eq!(
        manual_command("pnpm").unwrap(),
        "pnpm add -g --allow-build=@opencode/cli @opencode/cli@2"
    );
}

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "opencode-mom-installation-test-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn file(&self, relative: &str, bytes: impl AsRef<[u8]>) -> PathBuf {
        let path = self.0.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, bytes).unwrap();
        path
    }
    #[cfg(unix)]
    fn executable(&self, relative: &str, script: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = self.file(relative, script);
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        // Fresh macOS shebang executables stall before script entry under parallel load; read them with /bin/sh.
        FAKE_EXECUTABLES.with(|paths| paths.borrow_mut().push(path.canonicalize().unwrap()));
        path
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Ok(root) = self.0.canonicalize() {
            FAKE_EXECUTABLES
                .with(|paths| paths.borrow_mut().retain(|path| !path.starts_with(&root)));
        }
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn modern_native_package_bin_requires_exact_metadata_not_substrings_or_bundle_ancestry() {
    let fixture = Fixture::new();
    let root = fixture.0.join("node_modules");
    let binary = fixture
        .file("node_modules/@opencode/cli/bin/opencode.exe", b"native")
        .canonicalize()
        .unwrap();
    let metadata = fixture.file("node_modules/@opencode/cli/package.json",
        br#"{"name":"@opencode/cli","bin":{"opencode":"./bin/opencode.exe"},"optionalDependencies":{"@opencode/cli-native":"2.0.25"}}"#);
    assert!(package_owns(&root, &binary));
    assert!(!bundled(&binary));
    let unrelated = fixture
        .file("node_modules/@opencode/cli-native/bin/opencode", b"native")
        .canonicalize()
        .unwrap();
    assert!(!package_owns(&root, &unrelated));
    for bad in [
        br#"{"name":"opencode-ai","bin":"bin/opencode.exe"}"#.as_slice(),
        br#"{"name":"@opencode/cli","bin":"../../../outside"}"#,
    ] {
        fs::write(&metadata, bad).unwrap();
        assert!(!package_owns(&root, &binary));
    }
    let shim = fixture.file(
        "global/opencode.cmd",
        br#"@ECHO off
"%dp0%\node_modules\@opencode\cli\bin\opencode.exe" %*
"#,
    );
    assert!(npm_native_shim(&shim));
    fs::write(
        &shim,
        br#""%dp0%\node_modules\opencode-ai\bin\opencode.exe" %*"#,
    )
    .unwrap();
    assert!(!npm_native_shim(&shim));
}

#[cfg(unix)]
#[test]
fn metadata_cannot_block_on_fifo_or_follow_symlink() {
    use std::os::unix::{ffi::OsStrExt, fs::symlink};
    let fixture = Fixture::new();
    let fifo = fixture.0.join("fifo");
    let name = std::ffi::CString::new(fifo.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    let started = Instant::now();
    assert!(bounded_file(&fifo, 1024).is_none());
    assert!(started.elapsed() < Duration::from_secs(1));
    let real = fixture.file("real", b"{}");
    let link = fixture.0.join("link");
    symlink(real, &link).unwrap();
    assert!(bounded_file(&link, 1024).is_none());
}

#[cfg(unix)]
#[test]
fn curl_detection_is_exact_and_only_verified_stable_native_versions_can_upgrade() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    for (text, expected, reason) in [
        ("1.9.9", Some("1.9.9"), Some("not_v2")),
        ("2.0.24", Some("2.0.24"), Some("unsupported")),
        ("2.0.25-beta.1", Some("2.0.25-beta.1"), Some("unstable")),
        ("OpenCode 2.0.25", None, Some("unsupported")),
        ("2.0.25", Some("2.0.25"), None),
    ] {
        let binary = fixture.executable(".opencode/bin/opencode", &format!(
            "#!/bin/sh\n[ \"$1\" = --version ] || exit 99\n[ \"$OPENCODE_DISABLE_AUTOUPDATE\" = 1 ] || exit 98\nprintf '%s' '{text}'\n"));
        let info = inspect_path(binary, Some(&fixture.0)).unwrap().info;
        assert_eq!(info.method, "curl");
        assert_eq!(info.version.as_deref(), expected);
        assert_eq!(info.upgrade_reason.as_deref(), reason);
        assert_eq!(info.can_upgrade, reason.is_none());
    }
    let binary = fixture.0.join(".opencode/bin/opencode");
    let result = identify_source_with(
        &binary,
        &binary.canonicalize().unwrap(),
        Some(&fixture.0),
        Instant::now() + LOCAL_TIMEOUT,
        |_| None,
    );
    assert_eq!(
        (result.0.as_str(), result.2),
        ("curl", "manager_unavailable")
    );
    fs::remove_file(&binary).unwrap();
    let outside = fixture.executable("outside/opencode", "#!/bin/sh\nexit 0\n");
    symlink(&outside, &binary).unwrap();
    assert!(!official_curl_path(
        &binary,
        &binary.canonicalize().unwrap(),
        &fixture.0
    ));
    fs::remove_file(&binary).unwrap();
    fs::remove_dir(fixture.0.join(".opencode/bin")).unwrap();
    symlink(fixture.0.join("outside"), fixture.0.join(".opencode/bin")).unwrap();
    assert!(!official_curl_path(
        &binary,
        &binary.canonicalize().unwrap(),
        &fixture.0
    ));
}

#[test]
fn bundled_and_manual_sources_fail_closed_without_manager_queries() {
    let fixture = Fixture::new();
    for (relative, method, reason) in [
        (
            "OpenCode.app/Contents/Resources/opencode",
            "bundled",
            "bundled",
        ),
        (
            "Programs/@openchamberelectron/resources/opencode-cli/opencode.exe",
            "bundled",
            "bundled",
        ),
        (
            "opencode-npm-brew/.opencode/bin/opencode",
            "manual",
            "unsupported",
        ),
    ] {
        let binary = fixture.file(relative, b"fake").canonicalize().unwrap();
        let result = identify_source_with(
            &binary,
            &binary,
            None,
            Instant::now() + LOCAL_TIMEOUT,
            |_| panic!("must not query manager"),
        );
        assert_eq!((result.0.as_str(), result.2), (method, reason));
        assert!(result.1.is_none());
    }
    let logical = fixture.0.join("OpenCode.app/Contents/Resources/opencode");
    let canonical = fixture
        .0
        .join("global/node_modules/@opencode/cli/bin/opencode.exe");
    let result = identify_source_with(
        &logical,
        &canonical,
        None,
        Instant::now() + LOCAL_TIMEOUT,
        |_| panic!("bundled links must not query manager"),
    );
    assert_eq!((result.0.as_str(), result.2), ("bundled", "bundled"));
}

#[cfg(unix)]
#[test]
fn each_manager_requires_queried_global_root_bin_and_native_package_metadata() {
    use std::os::unix::fs::symlink;
    for method in ["npm", "pnpm", "yarn", "bun"] {
        let fixture = Fixture::new();
        let (root_rel, bin_rel, root_query, bin_query) = match method {
            "npm" => (
                "global/lib/node_modules",
                "global/bin",
                "root -g",
                "prefix -g",
            ),
            "pnpm" => ("global/node_modules", "global/bin", "root -g", "bin -g"),
            "yarn" => (
                "global/node_modules",
                "global/bin",
                "global dir",
                "global bin",
            ),
            _ => (
                "global/install/global/node_modules",
                "global/bin",
                "unused",
                "pm bin -g",
            ),
        };
        let root = fixture.0.join(root_rel);
        let bin = fixture.0.join(bin_rel);
        let binary = fixture
            .executable(
                &format!("{root_rel}/@opencode/cli/bin/opencode.exe"),
                "#!/bin/sh\nprintf '2.0.25\\n'\n",
            )
            .canonicalize()
            .unwrap();
        let metadata = fixture.file(
            &format!("{root_rel}/@opencode/cli/package.json"),
            br#"{"name":"@opencode/cli","bin":{"opencode":"./bin/opencode.exe"}}"#,
        );
        fs::create_dir_all(&bin).unwrap();
        let logical = bin.join("opencode");
        symlink(&binary, &logical).unwrap();
        let root_reply = if method == "yarn" {
            fixture.0.join("global")
        } else {
            root.clone()
        };
        let bin_reply = if method == "npm" {
            fixture.0.join("global")
        } else {
            bin.clone()
        };
        let manager = fixture.executable("manager.sh", &format!(
            "#!/bin/sh\n[ \"$OPENCODE_DISABLE_AUTOUPDATE\" = 1 ] || exit 98\ncase \"$*\" in\n'{root_query}') printf '%s\\n' '{}' ;;\n'{bin_query}') printf '%s\\n' '{}' ;;\n*) exit 99 ;;\nesac\n",
            root_reply.display(), bin_reply.display()));
        let identify = |binary: &Path| {
            identify_source_with(
                &logical,
                binary,
                None,
                Instant::now() + LOCAL_TIMEOUT,
                |path| (path == Path::new(method)).then(|| manager.clone()),
            )
        };
        let result = identify(&binary);
        assert_eq!(result.0, method);
        assert_eq!(result.2, "");
        assert!(matches!(result.1, Some(Source::Manager { method: found, .. }) if found == method));
        let unrelated = fixture
            .file("other/node_modules/@opencode/cli/bin/opencode", b"fake")
            .canonicalize()
            .unwrap();
        assert!(identify(&unrelated).1.is_none());
        fs::write(
            metadata,
            br#"{"name":"opencode-ai","bin":"bin/opencode.exe"}"#,
        )
        .unwrap();
        assert!(identify(&binary).1.is_none());
    }
}

#[cfg(unix)]
#[test]
fn brew_requires_formula_identity_but_never_offers_unpinned_automatic_upgrade() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    let binary = fixture
        .executable(
            "homebrew/Cellar/opencode-v2/2.0.25/bin/opencode",
            "#!/bin/sh\nprintf '2.0.25\\n'\n",
        )
        .canonicalize()
        .unwrap();
    let prefix = fixture.0.join("homebrew/Cellar/opencode-v2/2.0.25");
    let manager = fixture.executable("homebrew/bin/brew", &format!(
        "#!/bin/sh\ncase \"$1\" in\n--prefix) printf '%s\\n' '{}' ;;\ninfo) printf '%s' '{{\"formulae\":[{{\"name\":\"opencode-v2\",\"full_name\":\"anomalyco/tap/opencode-v2\",\"installed\":[{{}}]}}]}}' ;;\n*) exit 99 ;;\nesac\n", prefix.display()));
    let logical = fixture.0.join("homebrew/bin/opencode");
    symlink(&binary, &logical).unwrap();
    assert_eq!(
        brew_formula(&binary, Instant::now() + LOCAL_TIMEOUT),
        Some(true)
    );
    let info = inspect_path(logical, None).unwrap().info;
    assert_eq!(info.method, "brew");
    assert!(!info.can_upgrade);
    assert_eq!(info.upgrade_reason.as_deref(), Some("unsupported"));
    fs::write(manager, format!("#!/bin/sh\n[ \"$1\" = --prefix ] && printf '%s\\n' '{}' && exit 0\nprintf '%s' '{{\"formulae\":[{{\"name\":\"opencode-v2\",\"full_name\":\"other/tap/opencode-v2\",\"installed\":[{{}}]}}]}}'\n", prefix.display())).unwrap();
    assert_eq!(
        brew_formula(&binary, Instant::now() + LOCAL_TIMEOUT),
        Some(false)
    );
}

#[cfg(unix)]
#[test]
fn local_queries_are_bounded_and_private_and_resolution_preserves_logical_symlinks() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    let bad = fixture.executable(
        "nonzero",
        "#!/bin/sh\nprintf 'private api-key' >&2\nexit 7\n",
    );
    let error = read_version(&bad, Instant::now() + PROCESS_TIMEOUT).unwrap_err();
    assert!(error.message.contains('7'));
    assert!(!error.message.contains("private"));
    let timeout = fixture.executable("timeout", "#!/bin/sh\nexec sleep 60\n");
    let started = Instant::now();
    assert!(read_version(&timeout, started + Duration::from_millis(100))
        .unwrap_err()
        .message
        .contains("timed out"));
    assert!(started.elapsed() < Duration::from_secs(2));
    let oversized = fixture.executable(
        "oversized",
        "#!/bin/sh\nwhile :; do printf 1234567890123456789012345678901234567890; done\n",
    );
    assert!(read_version(&oversized, Instant::now() + PROCESS_TIMEOUT)
        .unwrap_err()
        .message
        .contains("limit"));
    fixture.file("first/opencode", b"not executable");
    let executable = fixture.executable("second/opencode", "#!/bin/sh\nexit 0\n");
    let path = std::env::join_paths([fixture.0.join("first"), fixture.0.join("second")]).unwrap();
    assert_eq!(
        resolve_path_command_in(Path::new("opencode"), Some(&path), &fixture.0),
        Some(executable.clone())
    );
    let link = fixture.0.join("first/alias");
    symlink(&executable, &link).unwrap();
    assert_eq!(resolve_path_command_in(&link, None, &fixture.0), Some(link));
    assert_eq!(
        resolve_path_command_in(Path::new("second/opencode"), None, &fixture.0),
        Some(executable)
    );
}

#[cfg(unix)]
#[test]
fn fake_native_upgrade_observes_exact_args_null_stdin_and_same_path_readback() {
    let fixture = Fixture::new();
    let state = fixture.file("version", b"2.0.25");
    let marker = fixture.0.join("upgraded");
    let binary = fixture.executable(".opencode/bin/opencode", &format!(
        "#!/bin/sh\n[ \"$OPENCODE_DISABLE_AUTOUPDATE\" = 1 ] || exit 98\nif [ \"$1\" = --version ]; then exec /bin/cat '{}'; fi\n[ \"$*\" = 'upgrade 2.0.26 --method curl' ] || exit 97\nif read input; then exit 96; fi\nprintf 'private stdout'\nprintf 'private stderr' >&2\nprintf 2.0.26 > '{}'\nprintf ok > '{}'\n",
        state.display(), state.display(), marker.display()));
    let before = inspect_path(binary.clone(), Some(&fixture.0)).unwrap();
    let info = guarded_upgrade(
        before.info.binary_path.as_deref().unwrap(),
        "2.0.25",
        "2.0.26",
        |path| inspect_path(path.unwrap_or(&binary).to_path_buf(), Some(&fixture.0)),
        || stable_v2("2.0.26"),
        |value, target| run_upgrade(&mut upgrade_command(value, target)?, PROCESS_TIMEOUT),
    )
    .unwrap();
    assert_eq!(info.version.as_deref(), Some("2.0.26"));
    assert_eq!(fs::read(marker).unwrap(), b"ok");
}

#[cfg(unix)]
#[test]
fn native_upgrade_failures_discard_diagnostics_and_success_does_not_signal_reaped_group() {
    let fixture = Fixture::new();
    let binary = fixture.executable(
        "fake-cli",
        "#!/bin/sh\nprintf 'private stdout'\nprintf 'private stderr' >&2\nexit 7\n",
    );
    let mut value = detected("2.0.25");
    value.logical = Some(binary.clone());
    let command = || upgrade_command(&value, "2.0.26").unwrap();
    let error = run_upgrade(&mut command(), PROCESS_TIMEOUT).unwrap_err();
    assert!(error.message.contains('7'));
    assert!(!error.message.contains("private"));
    let pidfile = fixture.0.join("owned.pid");
    fs::write(
        &binary,
        format!(
            "#!/bin/sh\nsleep 60 >/dev/null 2>&1 &\nprintf '%s' \"$!\" > '{}'\nexit 0\n",
            pidfile.display()
        ),
    )
    .unwrap();
    run_upgrade(&mut command(), PROCESS_TIMEOUT).unwrap();
    let pid: i32 = fs::read_to_string(pidfile).unwrap().parse().unwrap();
    let alive = unsafe { libc::kill(pid, 0) };
    let _ = unsafe { libc::kill(pid, libc::SIGKILL) };
    assert_eq!(
        alive, 0,
        "normal success must not signal a reaped process group"
    );
}

#[cfg(unix)]
#[test]
fn upgrade_timeout_cleans_its_group_without_touching_an_existing_service() {
    use std::process::Child;
    struct Sleep(Child);
    impl Drop for Sleep {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let outside = Sleep(Command::new("/bin/sleep").arg("60").spawn().unwrap());
    let fixture = Fixture::new();
    let pidfile = fixture.0.join("owned.pid");
    let binary = fixture.executable(
        "fake-cli",
        &format!(
            "#!/bin/sh\nsleep 60 &\nprintf '%s' \"$!\" > '{}'\nwait\n",
            pidfile.display()
        ),
    );
    let mut value = detected("2.0.25");
    value.logical = Some(binary);
    let mut command = upgrade_command(&value, "2.0.26").unwrap();
    let started = Instant::now();
    let error = run_upgrade(&mut command, Duration::from_secs(1)).unwrap_err();
    assert!(error.message.contains("timed out"));
    assert!(started.elapsed() < Duration::from_secs(3));
    assert_eq!(unsafe { libc::kill(outside.0.id() as libc::pid_t, 0) }, 0);
    let pid: i32 = fs::read_to_string(pidfile).unwrap().parse().unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while unsafe { libc::kill(pid, 0) } == 0 && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
}

fn response(status: &str, headers: &str, body: &[u8]) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\n{headers}Connection: close\r\n\r\n{}",
        body.len(),
        String::from_utf8_lossy(body)
    )
}

fn fake_http(reply: String, delay: Duration) -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/fixture", listener.local_addr().unwrap());
    let worker = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            assert_eq!(stream.read(&mut byte).unwrap(), 1);
            request.push(byte[0]);
            assert!(request.len() < 8192);
        }
        thread::sleep(delay);
        let _ = stream.write_all(reply.as_bytes());
        String::from_utf8(request).unwrap()
    });
    (url, worker)
}

#[test]
fn metadata_http_is_bounded_private_nonredirecting_and_timeout_errors_hide_bodies() {
    let client = http_client().unwrap();
    let (url, worker) = fake_http(response("200 OK", "", &latest("2.0.26")), Duration::ZERO);
    assert_eq!(
        latest_version(&fetch_bytes(&client, &url, 16 * 1024).unwrap())
            .unwrap()
            .text,
        "2.0.26"
    );
    let request = worker.join().unwrap().to_ascii_lowercase();
    assert!(!request.contains("authorization:") && !request.contains("cookie:"));
    for reply in [
        response(
            "302 Found",
            "Location: http://127.0.0.1:1/untrusted\r\n",
            b"private",
        ),
        response("200 OK", "", &vec![b'x'; 1025]),
        "HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n".to_owned() + &"x".repeat(1025),
    ] {
        let (url, worker) = fake_http(reply, Duration::ZERO);
        let error = fetch_bytes(&client, &url, 1024).unwrap_err();
        assert!(!error.message.contains("private"));
        worker.join().unwrap();
    }
    let client = Client::builder()
        .timeout(Duration::from_millis(100))
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let (url, worker) = fake_http(
        response("200 OK", "", b"private"),
        Duration::from_millis(250),
    );
    let started = Instant::now();
    let error = fetch_bytes(&client, &url, 1024).unwrap_err();
    assert!(!error.message.contains("private"));
    assert!(started.elapsed() < Duration::from_secs(2));
    worker.join().unwrap();
}
