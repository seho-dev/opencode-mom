use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use super::collect_output;

const TIMEOUT: Duration = Duration::from_millis(150);

struct FakeCli {
    directory: PathBuf,
    child: Option<Child>,
    direct_pid: libc::pid_t,
    descendant_pid: Option<libc::pid_t>,
}

impl FakeCli {
    fn new(ending: &str) -> Self {
        let directory = std::env::temp_dir().join(format!(
            "opencode-mom-collector-test-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir(&directory).unwrap();
        let script = directory.join("opencode");
        fs::write(
            &script,
            format!(
                "#!/bin/sh\n[ \"$1\" = api ] && [ \"$2\" = session.active ] || exit 99\nsleep 60 &\nprintf '%s' \"$!\" > \"$3\"\nprintf '%s' '{{\"data\":{{}}}}'\n{ending}\n"
            ),
        )
        .unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
        let child = Command::new(&script)
            .args(["api", "session.active"])
            .arg(directory.join("descendant.pid"))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut fixture = Self {
            direct_pid: child.id() as libc::pid_t,
            child: Some(child),
            directory,
            descendant_pid: None,
        };
        let started = Instant::now();
        loop {
            if let Ok(pid) = fs::read_to_string(fixture.directory.join("descendant.pid")) {
                if let Ok(pid) = pid.parse::<libc::pid_t>() {
                    assert!(pid > 0);
                    fixture.descendant_pid = Some(pid);
                    break;
                }
            }
            assert!(started.elapsed() < Duration::from_secs(5));
            thread::sleep(Duration::from_millis(10));
        }
        fixture
    }

    fn collect(&mut self, max_bytes: u64) -> Result<Output, crate::error::AppError> {
        collect_output(
            self.child.take().unwrap(),
            TIMEOUT,
            max_bytes,
            "opencode session stats",
            "16 MiB",
        )
    }

    fn assert_direct_reaped_and_descendant_alive(&self) {
        let mut status = 0;
        assert_eq!(
            unsafe { libc::waitpid(self.direct_pid, &mut status, libc::WNOHANG) },
            -1
        );
        assert_eq!(
            io::Error::last_os_error().raw_os_error(),
            Some(libc::ECHILD)
        );
        assert_eq!(unsafe { libc::kill(self.descendant_pid.unwrap(), 0) }, 0);
    }
}

impl Drop for FakeCli {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(pid) = self.descendant_pid {
            // This fixture's sleep stays alive until cleanup; never signal a process group.
            let _ = unsafe { libc::kill(pid, libc::SIGKILL) };
        }
        let _ = fs::remove_dir_all(&self.directory);
    }
}

#[test]
fn collector_drains_large_stdout_after_direct_child_has_exited() {
    let mut child = Command::new("/bin/sh")
        .args([
            "-c",
            r#"(printf '{"padding":"'; dd if=/dev/zero bs=65536 count=2 2>/dev/null | tr '\000' x; printf '"}') & exit 0"#,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    // The direct child exits before collection; its writer must still be drained to EOF.
    assert!(child.wait().unwrap().success());
    let output = collect_output(
        child,
        Duration::from_secs(5),
        1024 * 1024,
        "opencode test page",
        "1 MiB",
    )
    .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout.len(), 2 * 65536 + 14);
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["padding"].as_str().unwrap(), "x".repeat(2 * 65536));
}

#[test]
fn collector_successful_exit_requires_eof_from_inherited_stdout() {
    let mut cli = FakeCli::new("exit 0");
    let started = Instant::now();
    let error = cli.collect(16 * 1024 * 1024).unwrap_err();
    assert_eq!(error.message, "opencode session stats output timed out.");
    assert!(started.elapsed() < Duration::from_secs(2));
    cli.assert_direct_reaped_and_descendant_alive();
}

#[test]
fn collector_nonzero_exit_drops_stdout_without_waiting_for_descendant() {
    let mut cli = FakeCli::new("exit 7");
    let started = Instant::now();
    let output = cli.collect(16 * 1024 * 1024).unwrap();
    assert_eq!(output.status.code(), Some(7));
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
    assert!(started.elapsed() < Duration::from_secs(2));
    cli.assert_direct_reaped_and_descendant_alive();
}

#[test]
fn collector_timeout_reaps_only_direct_child_with_inherited_stdout() {
    let mut cli = FakeCli::new("exec sleep 60");
    let started = Instant::now();
    let error = cli.collect(16 * 1024 * 1024).unwrap_err();
    assert_eq!(
        error.message,
        "opencode session stats timed out. Check the OpenCode CLI and retry."
    );
    assert!(started.elapsed() < Duration::from_secs(2));
    cli.assert_direct_reaped_and_descendant_alive();
}

#[test]
fn collector_oversize_reaps_direct_child_without_waiting_for_exit() {
    let mut cli = FakeCli::new("exec sleep 60");
    let started = Instant::now();
    let error = cli.collect(1).unwrap_err();
    assert_eq!(
        error.message,
        "opencode session stats response exceeds the 16 MiB limit."
    );
    assert!(started.elapsed() < TIMEOUT);
    cli.assert_direct_reaped_and_descendant_alive();
}

fn open_fd_count() -> usize {
    fs::read_dir("/dev/fd").unwrap().count()
}

#[test]
fn collector_repeated_inherited_stdout_closes_all_reader_fds() {
    const PROBE_ENV: &str = "OPENCODE_MOM_COLLECTOR_FD_PROBE";
    if std::env::var_os(PROBE_ENV).is_none() {
        // Isolate the FD baseline from other concurrently running test fixtures.
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "opencode_cli::command::tests::collector_repeated_inherited_stdout_closes_all_reader_fds",
                "--test-threads=1",
            ])
            .env(PROBE_ENV, "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let baseline = open_fd_count();
    for _ in 0..3 {
        for (ending, max_bytes) in [
            ("exit 0", 16 * 1024 * 1024),
            ("exit 7", 16 * 1024 * 1024),
            ("exec sleep 60", 16 * 1024 * 1024),
            ("exec sleep 60", 1),
        ] {
            let mut cli = FakeCli::new(ending);
            let result = cli.collect(max_bytes);
            if ending == "exit 7" {
                assert_eq!(result.unwrap().status.code(), Some(7));
            } else {
                assert!(result.is_err());
            }
            cli.assert_direct_reaped_and_descendant_alive();
            // Check while the descendant still holds stdout, before fixture cleanup.
            assert_eq!(open_fd_count(), baseline);
        }
    }
}
