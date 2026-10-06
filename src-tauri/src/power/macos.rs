use super::PowerController;
use crate::error::AppError;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[path = "../../native/macos-helper/src/protocol.rs"]
mod protocol;

const HELPER: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/opencode-lid-helper"));
const DIGEST: &str = env!("OPENCODE_LID_HELPER_SHA256");
const BOOTSTRAP: &str = include_str!("../../native/macos_bootstrap.pl");
const SOCKET_DIR: &str = "/private/var/run/opencode-mom-lid";
const SOCKET: &str = "/private/var/run/opencode-mom-lid/control.sock";
const IPC_TIMEOUT: Duration = Duration::from_secs(9);
const AUTH_TIMEOUT: Duration = Duration::from_secs(120);

fn error(message: &'static str) -> AppError {
    AppError::configuration(message)
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn applescript_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn random_token() -> Result<[u8; 32], AppError> {
    let mut token = [0; 32];
    File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut token))
        .map_err(|_| error("Could not generate the helper authentication token"))?;
    Ok(token)
}

fn hex(token: &[u8; 32]) -> String {
    token.iter().map(|byte| format!("{byte:02x}")).collect()
}

struct StagedSource {
    directory: PathBuf,
    helper: PathBuf,
}

impl StagedSource {
    fn create(token: &[u8; 32]) -> Result<Self, AppError> {
        let directory = std::env::temp_dir().join(format!("opencode-lid-{}", hex(token)));
        std::os::unix::fs::DirBuilderExt::mode(&mut fs::DirBuilder::new(), 0o700)
            .create(&directory)
            .map_err(|_| error("Could not create exclusive helper source staging"))?;
        let source = Self {
            helper: directory.join("helper"),
            directory,
        };
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&source.helper)
            .map_err(|_| error("Could not stage the embedded helper bytes"))?;
        file.write_all(HELPER)
            .and_then(|_| file.sync_all())
            .map_err(|_| error("Could not persist the embedded helper bytes"))?;
        Ok(source)
    }
}

impl Drop for StagedSource {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.helper);
        let _ = fs::remove_dir(&self.directory);
    }
}

fn authorize(source: &Path, token: &[u8; 32], uid: u32, pid: u32) -> Result<(), AppError> {
    let source = source
        .to_str()
        .filter(|value| {
            value.starts_with('/') && value.len() <= 2048 && !value.chars().any(|c| c.is_control())
        })
        .ok_or_else(|| error("The helper source path cannot be represented safely"))?;
    let command = format!(
        "/usr/bin/env -i PATH=/usr/bin:/bin:/usr/sbin:/sbin LANG=C LC_ALL=C /usr/bin/perl -T -e {} -- {} {} {} {} {}",
        shell_quote(BOOTSTRAP),
        shell_quote(source),
        uid,
        pid,
        hex(token),
        DIGEST,
    );
    let script = format!(
        "do shell script {} with administrator privileges",
        applescript_quote(&command)
    );
    let mut child = Command::new("/usr/bin/osascript")
        .arg("-e")
        .arg(script)
        .env_clear()
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
        .env("LANG", "C")
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| error("Could not request administrator authorization"))?;
    let result = (|| {
        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| error("Missing authorization result"))?;
        let fd = stdout.as_raw_fd();
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
            return Err(error("Could not bound the authorization result"));
        }
        let deadline = Instant::now() + AUTH_TIMEOUT;
        let mut output = Vec::new();
        let mut exited = None;
        let mut eof = false;
        loop {
            let mut buffer = [0; 128];
            match stdout.read(&mut buffer) {
                Ok(0) => eof = true,
                Ok(count) => {
                    if output.len() + count > 128 {
                        return Err(error("The authorization result exceeded its limit"));
                    }
                    output.extend_from_slice(&buffer[..count]);
                }
                Err(io) if io.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(io) if io.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => return Err(error("Could not read the authorization result")),
            }
            if exited.is_none() {
                exited = child
                    .try_wait()
                    .map_err(|_| error("Could not collect administrator authorization"))?;
            }
            if let Some(status) = exited {
                if !status.success() {
                    return Err(error(
                        "Administrator authorization or safe helper launch failed",
                    ));
                }
                if eof {
                    return if output == b"READY\n" || output == b"READY\r\n" {
                        Ok(())
                    } else {
                        Err(error("The authorized helper did not confirm readiness"))
                    };
                }
            }
            if Instant::now() >= deadline {
                return Err(error("Administrator authorization timed out"));
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    })();
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    result
}

fn verify_socket_path(uid: u32) -> Result<(), AppError> {
    let parent = fs::symlink_metadata(SOCKET_DIR)
        .map_err(|_| error("The authorized helper control directory is missing"))?;
    let socket = fs::symlink_metadata(SOCKET)
        .map_err(|_| error("The authorized helper control socket is missing"))?;
    if !parent.is_dir()
        || parent.uid() != 0
        || parent.mode() & 0o7777 != 0o711
        || !socket.file_type().is_socket()
        || socket.uid() != uid
        || socket.mode() & 0o7777 != 0o600
    {
        return Err(error(
            "The helper control socket has unsafe ownership or permissions",
        ));
    }
    Ok(())
}

fn verify_root_peer(stream: &UnixStream) -> Result<i32, AppError> {
    let mut uid = u32::MAX;
    let mut gid = u32::MAX;
    if unsafe { libc::getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) } != 0 || uid != 0 {
        return Err(error("The control peer is not the authorized root helper"));
    }
    let mut pid: libc::pid_t = 0;
    let mut length = std::mem::size_of_val(&pid) as libc::socklen_t;
    if unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_LOCAL,
            libc::LOCAL_PEERPID,
            (&mut pid as *mut libc::pid_t).cast(),
            &mut length,
        )
    } != 0
        || length as usize != std::mem::size_of_val(&pid)
        || pid <= 1
    {
        return Err(error("Could not verify the root helper process"));
    }
    Ok(pid)
}

fn connect_bounded() -> Result<UnixStream, AppError> {
    let fd = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM, 0) };
    if fd < 0 {
        return Err(error("Could not create the helper control channel"));
    }
    let stream = unsafe { UnixStream::from_raw_fd(fd) };
    stream
        .set_nonblocking(true)
        .map_err(|_| error("Could not bound the helper connection"))?;
    if unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } < 0 {
        return Err(error("Could not protect the helper connection"));
    }
    let mut address: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    address.sun_family = libc::AF_UNIX as libc::sa_family_t;
    address.sun_len = std::mem::size_of_val(&address) as u8;
    for (destination, source) in address.sun_path.iter_mut().zip(SOCKET.as_bytes()) {
        *destination = *source as libc::c_char;
    }
    let connected = unsafe {
        libc::connect(
            fd,
            (&address as *const libc::sockaddr_un).cast(),
            std::mem::size_of_val(&address) as libc::socklen_t,
        )
    };
    if connected != 0 {
        if std::io::Error::last_os_error().raw_os_error() != Some(libc::EINPROGRESS) {
            return Err(error(
                "Could not connect to the administrator-authorized helper",
            ));
        }
        let mut descriptor = libc::pollfd {
            fd,
            events: libc::POLLOUT,
            revents: 0,
        };
        if unsafe { libc::poll(&mut descriptor, 1, 2000) } <= 0 {
            return Err(error("The helper connection timed out"));
        }
        let mut status: libc::c_int = 0;
        let mut length = std::mem::size_of_val(&status) as libc::socklen_t;
        if unsafe {
            libc::getsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_ERROR,
                (&mut status as *mut libc::c_int).cast(),
                &mut length,
            )
        } != 0
            || length as usize != std::mem::size_of_val(&status)
            || status != 0
        {
            return Err(error("The helper connection failed"));
        }
    }
    Ok(stream)
}

fn exchange<const N: usize>(stream: &mut UnixStream, request: &[u8]) -> Result<[u8; N], AppError> {
    let deadline = Instant::now() + IPC_TIMEOUT;
    let mut sent = 0;
    let mut received = 0;
    let mut response = [0; N];
    while received < N {
        if Instant::now() >= deadline {
            return Err(error(
                "The helper acknowledgment timed out; recovery may be required",
            ));
        }
        if sent < request.len() {
            match stream.write(&request[sent..]) {
                Ok(0) => return Err(error("The authorized helper disconnected")),
                Ok(count) => sent += count,
                Err(io) if io.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(io) if io.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => return Err(error("Could not send the bounded helper request")),
            }
        } else {
            match stream.read(&mut response[received..]) {
                Ok(0) => {
                    return Err(error(
                        "The helper exited without a verified restoration acknowledgment",
                    ))
                }
                Ok(count) => received += count,
                Err(io) if io.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(io) if io.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => return Err(error("Could not read the bounded helper acknowledgment")),
            }
        }
        if received < N {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    Ok(response)
}

pub struct NativeController {
    stream: Option<UnixStream>,
    sequence: u64,
    authorization_attempted: bool,
    restoration_unconfirmed: bool,
}

impl NativeController {
    pub fn new(_: PathBuf) -> Self {
        Self {
            stream: None,
            sequence: 1,
            authorization_attempted: false,
            restoration_unconfirmed: false,
        }
    }

    fn send(&mut self, operation: protocol::Operation) -> Result<(), AppError> {
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| error("The administrator-authorized helper is not available"))?;
        let response =
            exchange::<{ protocol::ACK_LEN }>(stream, &protocol::request(self.sequence, operation));
        let acknowledged = response
            .as_ref()
            .is_ok_and(|bytes| protocol::valid_ack(bytes, self.sequence));
        let result = response.and_then(|bytes| {
            let flag = protocol::parse_ack(&bytes, self.sequence).map_err(error)?;
            if matches!(operation, protocol::Operation::Busy(_)) && !flag {
                return Err(error("The helper did not confirm lid protection"));
            }
            Ok(())
        });
        if acknowledged {
            self.sequence += 1;
        } else {
            self.stream.take();
        }
        if result.is_err() {
            self.restoration_unconfirmed = true;
        } else if !matches!(operation, protocol::Operation::Busy(_)) {
            self.restoration_unconfirmed = false;
        }
        result
    }
}

impl PowerController for NativeController {
    fn prepare(&mut self) -> Result<(), AppError> {
        if self.stream.is_some() {
            return Ok(());
        }
        if self.authorization_attempted {
            return Err(error(
                "Restart the app to authorize or recover the macOS helper again",
            ));
        }
        self.authorization_attempted = true;
        let uid = unsafe { libc::geteuid() };
        if uid == 0 {
            return Err(error("Run the desktop app as a regular user, not as root"));
        }
        let token = random_token()?;
        let source = StagedSource::create(&token)?;
        self.restoration_unconfirmed = true;
        authorize(&source.helper, &token, uid, std::process::id())?;
        verify_socket_path(uid)?;
        let mut stream = connect_bounded()?;
        verify_root_peer(&stream)?;
        let ack = exchange::<{ protocol::ACK_LEN }>(&mut stream, &protocol::hello(&token))?;
        protocol::parse_ack(&ack, 0).map_err(error)?;
        self.stream = Some(stream);
        self.sequence = 1;
        self.restoration_unconfirmed = false;
        Ok(())
    }

    fn busy(&mut self, ttl: Duration) -> Result<(), AppError> {
        if ttl.is_zero() || ttl > protocol::MAX_TTL || ttl.as_millis() == 0 {
            return Err(AppError::validation(
                "The macOS helper lease must be between 1ms and 10s",
            ));
        }
        self.send(protocol::Operation::Busy(ttl.as_millis() as u64))
    }

    fn idle(&mut self) -> Result<(), AppError> {
        if self.stream.is_none() {
            return if self.restoration_unconfirmed {
                Err(error(
                    "Restoration is unconfirmed; restart and authorize recovery",
                ))
            } else {
                Ok(())
            };
        }
        self.send(protocol::Operation::Idle)
    }

    fn stop(&mut self) -> Result<(), AppError> {
        if self.stream.is_none() {
            return self.idle();
        }
        let result = self.send(protocol::Operation::Stop);
        if result.is_ok() {
            self.stream.take();
        }
        result
    }
}

impl Drop for NativeController {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_and_applescript_quotes_cannot_end_the_literal() {
        assert_eq!(shell_quote("a'b"), "'a'\\''b'");
        assert_eq!(applescript_quote("a\\\"b"), "\"a\\\\\\\"b\"");
    }

    #[test]
    fn lease_validation_never_starts_a_helper() {
        let mut controller = NativeController::new(PathBuf::new());
        for ttl in [
            Duration::ZERO,
            Duration::from_nanos(1),
            Duration::from_secs(11),
        ] {
            assert!(controller.busy(ttl).is_err());
        }
        assert!(!controller.authorization_attempted);
        assert!(controller.stop().is_ok());
    }

    #[test]
    fn ipc_only_claims_success_after_exact_readback_ack() {
        let (client, mut server) = UnixStream::pair().unwrap();
        client.set_nonblocking(true).unwrap();
        let worker = std::thread::spawn(move || {
            let mut frame = [0; protocol::REQUEST_LEN];
            server.read_exact(&mut frame).unwrap();
            assert_eq!(
                protocol::parse_request(&frame, 1),
                Ok(protocol::Operation::Busy(1000))
            );
            server
                .write_all(&protocol::ack(1, Some(false), true))
                .unwrap();
        });
        let mut controller = NativeController::new(PathBuf::new());
        controller.stream = Some(client);
        assert!(controller.busy(Duration::from_secs(1)).is_err());
        assert!(controller.stop().is_err());
        worker.join().unwrap();
    }

    #[test]
    fn no_second_authorization_after_stop_or_failure() {
        let mut controller = NativeController::new(PathBuf::new());
        controller.authorization_attempted = true;
        assert!(controller
            .prepare()
            .unwrap_err()
            .message
            .contains("Restart"));
    }
}
