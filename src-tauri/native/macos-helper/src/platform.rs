use crate::policy::{Backend, Policy, Result};
use crate::protocol::{self, Operation};
use crate::watchdog::Watchdog;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const ROOT: &str = "/private/var/db/opencode-mom-lid";
pub const SOCKET: &str = "/private/var/run/opencode-mom-lid/control.sock";
const SOCKET_DIR: &str = "/private/var/run/opencode-mom-lid";
const LOCK: &str = "/private/var/db/opencode-mom-lid/lock";
const JOURNAL: &str = "/private/var/db/opencode-mom-lid/journal";
const PMSET_TIMEOUT: Duration = Duration::from_secs(2);

fn trusted_file(file: &File, mode: u32) -> Result<()> {
    let metadata = file.metadata().map_err(|_| "Could not inspect root file")?;
    if !metadata.is_file()
        || metadata.uid() != 0
        || metadata.mode() & 0o7777 != mode
        || metadata.nlink() != 1
    {
        return Err("Unsafe root file");
    }
    Ok(())
}

fn open_root_file(path: &str, create: bool) -> Result<File> {
    let mut options = OpenOptions::new();
    options
        .read(true)
        .write(create)
        .create_new(create)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    let file = options
        .open(path)
        .map_err(|_| "Could not open root journal")?;
    trusted_file(&file, 0o600)?;
    Ok(file)
}

fn sync_root() -> Result<()> {
    File::open(ROOT)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| "Could not persist root directory")
}

pub fn parse_pmset(bytes: &[u8]) -> Result<bool> {
    let text = std::str::from_utf8(bytes).map_err(|_| "Invalid power policy output")?;
    let mut flag = None;
    for line in text.lines() {
        let mut fields = line.split_whitespace();
        if fields.next() == Some("SleepDisabled") {
            let value = match fields.next() {
                Some("0") => false,
                Some("1") => true,
                _ => return Err("Unsupported global power policy value"),
            };
            if fields.next().is_some() || flag.replace(value).is_some() {
                return Err("Ambiguous global power policy value");
            }
        }
    }
    flag.ok_or("Global SleepDisabled is unavailable on this Mac")
}

fn pmset(arguments: &[&str]) -> Result<Vec<u8>> {
    let mut child = Command::new("/usr/bin/pmset")
        .args(arguments)
        .env_clear()
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
        .env("LANG", "C")
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "Could not start the power policy tool")?;
    let result = (|| {
        let mut stdout = child.stdout.take().ok_or("Missing power tool output")?;
        let fd = stdout.as_raw_fd();
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
            return Err("Could not bound power tool output");
        }
        let deadline = Instant::now() + PMSET_TIMEOUT;
        let mut output = Vec::new();
        let mut exited = None;
        let mut eof = false;
        loop {
            let mut buffer = [0; 1024];
            match stdout.read(&mut buffer) {
                Ok(0) => eof = true,
                Ok(count) => {
                    if output.len() + count > 16_384 {
                        return Err("Power tool output exceeded its limit");
                    }
                    output.extend_from_slice(&buffer[..count]);
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => return Err("Could not read power tool output"),
            }
            if exited.is_none() {
                exited = child
                    .try_wait()
                    .map_err(|_| "Could not collect power tool")?;
            }
            if let Some(status) = exited {
                if !status.success() {
                    return Err("Power policy operation failed");
                }
                if eof {
                    return Ok(output);
                }
            }
            if Instant::now() >= deadline {
                return Err("Power policy operation timed out");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    })();
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    result
}

struct System;

impl Backend for System {
    fn read(&mut self) -> Result<bool> {
        parse_pmset(&pmset(&["-g"])?)
    }

    fn write(&mut self, value: bool) -> Result<()> {
        pmset(&["-a", "disablesleep", if value { "1" } else { "0" }]).map(|_| ())
    }

    fn journal(&mut self) -> Result<Option<bool>> {
        match fs::symlink_metadata(JOURNAL) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(_) => Err("Could not inspect recovery journal"),
            Ok(_) => {
                let mut file = open_root_file(JOURNAL, false)?;
                let mut bytes = Vec::new();
                Read::by_ref(&mut file)
                    .take(33)
                    .read_to_end(&mut bytes)
                    .map_err(|_| "Could not read recovery journal")?;
                match bytes.as_slice() {
                    b"OMLID1 original=0 applied=1\n" => Ok(Some(false)),
                    b"OMLID1 original=1 applied=1\n" => Ok(Some(true)),
                    _ => Err("Recovery journal is corrupt; no policy change was made"),
                }
            }
        }
    }

    fn save(&mut self, original: bool) -> Result<()> {
        let mut file = open_root_file(JOURNAL, true)?;
        file.write_all(if original {
            b"OMLID1 original=1 applied=1\n"
        } else {
            b"OMLID1 original=0 applied=1\n"
        })
        .and_then(|_| file.sync_all())
        .map_err(|_| "Could not persist the original power policy")?;
        sync_root()
    }

    fn clear(&mut self) -> Result<()> {
        let file = open_root_file(JOURNAL, false)?;
        trusted_file(&file, 0o600)?;
        fs::remove_file(JOURNAL).map_err(|_| "Could not clear recovery journal")?;
        sync_root()
    }
}

fn peer(stream: &UnixStream, uid: u32, pid: i32) -> Result<()> {
    let mut peer_uid = 0;
    let mut peer_gid = 0;
    if unsafe { libc::getpeereid(stream.as_raw_fd(), &mut peer_uid, &mut peer_gid) } != 0 {
        return Err("Could not verify the peer user");
    }
    let mut peer_pid: libc::pid_t = 0;
    let mut length = std::mem::size_of_val(&peer_pid) as libc::socklen_t;
    if unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_LOCAL,
            libc::LOCAL_PEERPID,
            (&mut peer_pid as *mut libc::pid_t).cast(),
            &mut length,
        )
    } != 0
        || length as usize != std::mem::size_of_val(&peer_pid)
        || peer_uid != uid
        || peer_pid != pid
    {
        return Err("The control peer is not the authorized app");
    }
    Ok(())
}

struct Parent(OwnedFd);

impl Parent {
    fn new(pid: i32) -> Result<Self> {
        let fd = unsafe { libc::kqueue() };
        if fd < 0 {
            return Err("Could not monitor the app lifetime");
        }
        let owned = unsafe { OwnedFd::from_raw_fd(fd) };
        if unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } < 0 {
            return Err("Could not protect the lifetime monitor");
        }
        let event = libc::kevent {
            ident: pid as libc::uintptr_t,
            filter: libc::EVFILT_PROC,
            flags: libc::EV_ADD | libc::EV_ENABLE | libc::EV_ONESHOT,
            fflags: libc::NOTE_EXIT,
            data: 0,
            udata: std::ptr::null_mut(),
        };
        if unsafe { libc::kevent(fd, &event, 1, std::ptr::null_mut(), 0, std::ptr::null()) } < 0 {
            return Err("The authorized app has already exited");
        }
        Ok(Self(owned))
    }

    fn alive(&self) -> bool {
        let mut event: libc::kevent = unsafe { std::mem::zeroed() };
        let timeout = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        unsafe {
            libc::kevent(
                self.0.as_raw_fd(),
                std::ptr::null(),
                0,
                &mut event,
                1,
                &timeout,
            ) == 0
        }
    }
}

fn set_timeouts(stream: &UnixStream, timeout: Duration) -> Result<()> {
    stream
        .set_read_timeout(Some(timeout))
        .and_then(|_| stream.set_write_timeout(Some(timeout)))
        .map_err(|_| "Could not bound the control socket")
}

fn has_extra_bytes(stream: &UnixStream) -> bool {
    let mut byte = 0u8;
    unsafe {
        libc::recv(
            stream.as_raw_fd(),
            (&mut byte as *mut u8).cast(),
            1,
            libc::MSG_PEEK | libc::MSG_DONTWAIT,
        ) > 0
    }
}

fn serve<B: Backend>(
    listener: &UnixListener,
    mut parent_alive: impl FnMut() -> bool,
    uid: u32,
    pid: i32,
    token: &[u8; 32],
    policy: &mut Policy<B>,
) -> Result<()> {
    let accept_deadline = Instant::now() + Duration::from_secs(15);
    let mut stream = loop {
        if !parent_alive() || Instant::now() >= accept_deadline {
            return Err("The authorized app did not connect");
        }
        match listener.accept() {
            Ok((mut stream, _)) => {
                if peer(&stream, uid, pid).is_err() {
                    continue;
                }
                set_timeouts(&stream, Duration::from_secs(1))?;
                let mut hello = [0; protocol::HELLO_LEN];
                if stream.read_exact(&mut hello).is_err()
                    || !protocol::authenticate(&hello, token)
                    || has_extra_bytes(&stream)
                {
                    continue;
                }
                let flag = policy.backend.read()?;
                stream
                    .write_all(&protocol::ack(0, Some(flag), true))
                    .map_err(|_| "Could not acknowledge preparation")?;
                break stream;
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(_) => return Err("The control listener failed"),
        }
    };
    stream
        .set_nonblocking(true)
        .map_err(|_| "Could not bound control reads")?;
    let mut watchdog = Watchdog::default();
    let mut seq = 1;
    let mut frame = [0; protocol::REQUEST_LEN];
    let mut used = 0;
    let mut frame_deadline = None;
    loop {
        let now = Instant::now();
        let alive = parent_alive();
        if watchdog.must_restore(now, true, alive) {
            policy.idle()?;
            watchdog.idle();
            if !alive {
                return Ok(());
            }
        }
        if frame_deadline.is_some_and(|deadline| now >= deadline) {
            return Err("The control frame timed out");
        }
        match stream.read(&mut frame[used..]) {
            Ok(0) => return policy.idle().map(|_| ()),
            Ok(count) => {
                if used == 0 {
                    frame_deadline = Some(now + Duration::from_secs(1));
                }
                used += count;
                if used < frame.len() {
                    continue;
                }
                if has_extra_bytes(&stream) {
                    return Err("Oversized or pipelined control frame");
                }
                let operation = protocol::parse_request(&frame, seq)?;
                let result = match operation {
                    Operation::Busy(ttl) => {
                        // Start the lease before slow pmset work, not after its acknowledgment.
                        watchdog.renew(now, ttl);
                        let result = policy.busy();
                        if result.is_ok() && watchdog.must_restore(Instant::now(), true, true) {
                            policy.idle()?;
                            watchdog.idle();
                            Err("The requested lease expired before acknowledgment")
                        } else {
                            result
                        }
                    }
                    Operation::Idle | Operation::Stop => {
                        let result = policy.idle();
                        if result.is_ok() {
                            watchdog.idle();
                        }
                        result
                    }
                };
                stream
                    .set_nonblocking(false)
                    .map_err(|_| "Could not bound acknowledgment")?;
                set_timeouts(&stream, Duration::from_secs(1))?;
                let response = protocol::ack(seq, result.as_ref().ok().copied(), result.is_ok());
                stream
                    .write_all(&response)
                    .map_err(|_| "Could not acknowledge the power policy")?;
                stream
                    .set_nonblocking(true)
                    .map_err(|_| "Could not resume bounded control reads")?;
                if operation == Operation::Stop && result.is_ok() {
                    return Ok(());
                }
                seq += 1;
                used = 0;
                frame_deadline = None;
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => return Err("The app disconnected"),
        }
    }
}

fn run_inner(ready: &mut File) -> Result<()> {
    if unsafe { libc::geteuid() } != 0 {
        return Err("The helper requires administrator authorization");
    }
    unsafe { libc::umask(0o077) };
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        return Err("Invalid helper arguments");
    }
    let uid: u32 = args[0].parse().map_err(|_| "Invalid app user")?;
    let pid: i32 = args[1].parse().map_err(|_| "Invalid app process")?;
    if uid == 0 || pid <= 1 {
        return Err("Invalid app identity");
    }
    let token = protocol::decode_token(&args[2])?;
    let metadata = fs::symlink_metadata(ROOT).map_err(|_| "Missing root directory")?;
    if !metadata.is_dir() || metadata.uid() != 0 || metadata.mode() & 0o7777 != 0o700 {
        return Err("Unsafe root directory");
    }
    let lock = unsafe { File::from_raw_fd(3) };
    trusted_file(&lock, 0o600)?;
    let lock_path = fs::symlink_metadata(LOCK).map_err(|_| "Missing root lock")?;
    let lock_fd = lock.metadata().map_err(|_| "Could not inspect root lock")?;
    if !lock_path.is_file()
        || lock_path.dev() != lock_fd.dev()
        || lock_path.ino() != lock_fd.ino()
        || unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0
        || unsafe { libc::fcntl(lock.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC) } < 0
    {
        return Err("Another authorized helper is running");
    }
    let parent = Parent::new(pid)?;
    let socket_dir = fs::symlink_metadata(SOCKET_DIR).map_err(|_| "Missing control directory")?;
    if !socket_dir.is_dir() || socket_dir.uid() != 0 || socket_dir.mode() & 0o7777 != 0o711 {
        return Err("Unsafe control directory");
    }
    let mut policy = Policy::new(System);
    policy.recover()?;
    match fs::symlink_metadata(SOCKET) {
        Ok(metadata) if metadata.file_type().is_socket() && metadata.mode() & 0o7777 == 0o600 => {
            fs::remove_file(SOCKET).map_err(|_| "Could not replace stale root socket")?;
        }
        Ok(_) => return Err("Unsafe control socket"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err("Could not inspect control socket"),
    }
    let listener = UnixListener::bind(SOCKET).map_err(|_| "Could not create root socket")?;
    fs::set_permissions(SOCKET, fs::Permissions::from_mode(0o600))
        .map_err(|_| "Could not protect root socket")?;
    // Only the authorized uid can connect; the root-owned parent is never user-writable.
    let socket_c = std::ffi::CString::new(SOCKET).unwrap();
    if unsafe { libc::chown(socket_c.as_ptr(), uid, u32::MAX) } != 0 {
        return Err("Could not grant bounded app socket access");
    }
    listener
        .set_nonblocking(true)
        .map_err(|_| "Could not bound socket acceptance")?;
    ready
        .write_all(b"READY\n")
        .map_err(|_| "Could not acknowledge helper launch")?;
    let result = serve(&listener, || parent.alive(), uid, pid, &token, &mut policy);
    let restore = if result.is_err() {
        policy.idle().map(|_| ())
    } else {
        Ok(())
    };
    let _ = fs::remove_file(SOCKET);
    restore?;
    result
}

pub fn run() -> Result<()> {
    let mut ready = unsafe { File::from_raw_fd(4) };
    if unsafe { libc::fcntl(ready.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC) } < 0 {
        return Err("Could not protect the launch acknowledgment");
    }
    let result = run_inner(&mut ready);
    if result.is_err() {
        let _ = ready.write_all(b"ERROR\n");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct MockState {
        flag: bool,
        original: Option<bool>,
    }

    struct Mock(Arc<Mutex<MockState>>);

    impl Backend for Mock {
        fn read(&mut self) -> Result<bool> {
            Ok(self.0.lock().unwrap().flag)
        }
        fn write(&mut self, value: bool) -> Result<()> {
            let mut state = self.0.lock().unwrap();
            assert!(state.original.is_some());
            state.flag = value;
            Ok(())
        }
        fn journal(&mut self) -> Result<Option<bool>> {
            Ok(self.0.lock().unwrap().original)
        }
        fn save(&mut self, original: bool) -> Result<()> {
            self.0.lock().unwrap().original = Some(original);
            Ok(())
        }
        fn clear(&mut self) -> Result<()> {
            self.0.lock().unwrap().original = None;
            Ok(())
        }
    }

    struct Harness {
        path: std::path::PathBuf,
        state: Arc<Mutex<MockState>>,
        alive: Arc<AtomicBool>,
        worker: Option<std::thread::JoinHandle<Result<()>>>,
    }

    impl Harness {
        fn new() -> Self {
            static ID: AtomicU64 = AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "lid-test-{}-{}.sock",
                std::process::id(),
                ID.fetch_add(1, Ordering::Relaxed)
            ));
            let listener = UnixListener::bind(&path).unwrap();
            listener.set_nonblocking(true).unwrap();
            let state = Arc::new(Mutex::new(MockState::default()));
            let alive = Arc::new(AtomicBool::new(true));
            let mock = Mock(Arc::clone(&state));
            let lifetime = Arc::clone(&alive);
            let worker = std::thread::spawn(move || {
                let mut policy = Policy::new(mock);
                policy.recover()?;
                let result = serve(
                    &listener,
                    || lifetime.load(Ordering::SeqCst),
                    unsafe { libc::geteuid() },
                    std::process::id() as i32,
                    &[7; 32],
                    &mut policy,
                );
                policy.idle()?;
                result
            });
            Self {
                path,
                state,
                alive,
                worker: Some(worker),
            }
        }

        fn connect(&self, token: &[u8; 32]) -> UnixStream {
            let mut stream = UnixStream::connect(&self.path).unwrap();
            stream.write_all(&protocol::hello(token)).unwrap();
            stream
        }

        fn authenticated(&self) -> UnixStream {
            let mut stream = self.connect(&[7; 32]);
            let mut ack = [0; protocol::ACK_LEN];
            stream.read_exact(&mut ack).unwrap();
            assert_eq!(protocol::parse_ack(&ack, 0), Ok(false));
            stream
        }

        fn finish(mut self) -> Result<()> {
            self.alive.store(false, Ordering::SeqCst);
            let result = self.worker.take().unwrap().join().unwrap();
            assert!(!self.state.lock().unwrap().flag);
            assert!(self.state.lock().unwrap().original.is_none());
            result
        }
    }

    impl Drop for Harness {
        fn drop(&mut self) {
            self.alive.store(false, Ordering::SeqCst);
            if let Some(worker) = self.worker.take() {
                let _ = worker.join();
            }
            let _ = fs::remove_file(&self.path);
        }
    }

    fn send(stream: &mut UnixStream, seq: u64, operation: Operation) -> Result<bool> {
        stream
            .write_all(&protocol::request(seq, operation))
            .unwrap();
        let mut ack = [0; protocol::ACK_LEN];
        stream.read_exact(&mut ack).unwrap();
        protocol::parse_ack(&ack, seq)
    }

    #[test]
    fn only_exact_global_flag_is_accepted() {
        assert_eq!(
            parse_pmset(b"System-wide power settings:\n SleepDisabled 0\n"),
            Ok(false)
        );
        assert_eq!(parse_pmset(b" SleepDisabled 1\n sleep 15\n"), Ok(true));
        for text in [
            "sleep 0",
            "SleepDisabled 2",
            "SleepDisabled 0 extra",
            "SleepDisabled 0\nSleepDisabled 1",
        ] {
            assert!(parse_pmset(text.as_bytes()).is_err());
        }
    }

    #[test]
    fn kernel_peer_auth_checks_uid_pid_and_return_length() {
        let harness = Harness::new();
        let stream = UnixStream::connect(&harness.path).unwrap();
        let uid = unsafe { libc::geteuid() };
        let pid = std::process::id() as i32;
        assert!(peer(&stream, uid, pid).is_ok());
        assert!(peer(&stream, uid.wrapping_add(1), pid).is_err());
        assert!(peer(&stream, uid, pid + 1).is_err());
        drop(stream);
        let _ = harness.finish();
    }

    #[test]
    fn bad_token_is_rejected_then_the_authorized_app_can_connect() {
        let harness = Harness::new();
        let mut bad = harness.connect(&[8; 32]);
        let mut response = [0; protocol::ACK_LEN];
        assert!(bad.read_exact(&mut response).is_err());
        let mut good = harness.authenticated();
        assert_eq!(send(&mut good, 1, Operation::Stop), Ok(false));
        assert!(harness.finish().is_ok());
    }

    #[test]
    fn invalid_sequence_ttl_and_oversize_restore_and_close() {
        for malformed in [
            protocol::request(2, Operation::Idle).to_vec(),
            protocol::request(1, Operation::Busy(10_001)).to_vec(),
            [protocol::request(1, Operation::Busy(1000)).as_slice(), &[0]].concat(),
        ] {
            let harness = Harness::new();
            let mut stream = harness.authenticated();
            stream.write_all(&malformed).unwrap();
            let mut response = [0; protocol::ACK_LEN];
            assert!(stream.read_exact(&mut response).is_err());
            assert!(harness.finish().is_err());
        }
    }

    #[test]
    fn real_socket_mock_policy_expires_disconnects_and_observes_parent_exit() {
        for reason in ["expiry", "disconnect", "parent"] {
            let harness = Harness::new();
            let mut stream = harness.authenticated();
            assert_eq!(send(&mut stream, 1, Operation::Busy(100)), Ok(true));
            assert!(harness.state.lock().unwrap().flag);
            if reason == "disconnect" {
                drop(stream);
            } else if reason == "parent" {
                harness.alive.store(false, Ordering::SeqCst);
                drop(stream);
            } else {
                let deadline = Instant::now() + Duration::from_secs(2);
                while harness.state.lock().unwrap().flag {
                    assert!(Instant::now() < deadline);
                    std::thread::sleep(Duration::from_millis(10));
                }
                // Expiry restores but keeps the same authorized helper ready for another session.
                assert_eq!(send(&mut stream, 2, Operation::Busy(1000)), Ok(true));
                assert_eq!(send(&mut stream, 3, Operation::Idle), Ok(false));
                assert_eq!(send(&mut stream, 4, Operation::Stop), Ok(false));
            }
            assert!(harness.finish().is_ok());
        }
    }
}
