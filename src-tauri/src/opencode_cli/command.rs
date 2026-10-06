use std::io::{self, Read};
use std::process::{Child, ChildStdout, Output};
use std::thread;
use std::time::{Duration, Instant};

use crate::error::AppError;

pub(super) fn collect_output(
    mut child: Child,
    timeout: Duration,
    max_bytes: u64,
    operation: &str,
    limit: &str,
) -> Result<Output, AppError> {
    let started = Instant::now();
    let mut stdout = child.stdout.take();
    let result = (|| {
        let stdout = stdout.as_mut().ok_or_else(|| {
            AppError::configuration(format!("failed to read {operation}: stdout is not piped"))
        })?;
        let read_failure = |error: io::Error| {
            AppError::configuration(format!("failed to read {operation}: {:?}", error.kind()))
        };
        prepare_stdout(stdout).map_err(read_failure)?;
        let mut bytes = Vec::new();
        let mut buffer = [0; 8192];
        let mut status = None;
        let mut eof = false;
        let mut read_error = None;
        loop {
            if status.is_none() {
                status = child.try_wait().map_err(|error| {
                    AppError::configuration(format!(
                        "failed to wait for {operation}: {:?}",
                        error.kind()
                    ))
                })?;
            }
            if let Some(status) = status {
                if !status.success() {
                    return Ok(Output {
                        status,
                        stdout: Vec::new(),
                        stderr: Vec::new(),
                    });
                }
                if let Some(error) = read_error.take() {
                    return Err(read_failure(error));
                }
                if eof {
                    return Ok(Output {
                        status,
                        stdout: bytes,
                        stderr: Vec::new(),
                    });
                }
            }
            if started.elapsed() >= timeout {
                return Err(AppError::configuration(if status.is_some() {
                    format!("{operation} output timed out.")
                } else {
                    format!("{operation} timed out. Check the OpenCode CLI and retry.")
                }));
            }
            if !eof && read_error.is_none() {
                let remaining = max_bytes
                    .saturating_sub(bytes.len() as u64)
                    .saturating_add(1)
                    .min(buffer.len() as u64) as usize;
                match read_available(stdout, &mut buffer[..remaining]) {
                    Ok(0) => eof = true,
                    Ok(count) => {
                        bytes.extend_from_slice(&buffer[..count]);
                        if bytes.len() as u64 > max_bytes {
                            return Err(AppError::configuration(format!(
                                "{operation} response exceeds the {limit} limit."
                            )));
                        }
                        continue;
                    }
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                    Err(error) => read_error = Some(error),
                }
            }
            thread::sleep(Duration::from_millis(20).min(timeout.saturating_sub(started.elapsed())));
        }
    })();
    drop(stdout);
    if result.is_err() && child.try_wait().ok().flatten().is_none() {
        // Only reap this CLI child; a descendant may be the persistent OpenCode service.
        let _ = child.kill();
        let _ = child.wait();
    }
    result
}

#[cfg(unix)]
fn prepare_stdout(stdout: &ChildStdout) -> io::Result<()> {
    use std::os::fd::AsRawFd;

    let fd = stdout.as_raw_fd();
    // The owned stdout keeps this descriptor valid throughout both fcntl calls.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags == -1 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(unix)]
fn read_available(stdout: &mut ChildStdout, buffer: &mut [u8]) -> io::Result<usize> {
    stdout.read(buffer)
}

#[cfg(windows)]
fn prepare_stdout(stdout: &ChildStdout) -> io::Result<()> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Foundation::{ERROR_INVALID_HANDLE, INVALID_HANDLE_VALUE};

    let handle = stdout.as_raw_handle();
    if handle.is_null() || handle == INVALID_HANDLE_VALUE {
        return Err(io::Error::from_raw_os_error(ERROR_INVALID_HANDLE as i32));
    }
    Ok(())
}

#[cfg(windows)]
fn read_available(stdout: &mut ChildStdout, buffer: &mut [u8]) -> io::Result<usize> {
    use std::os::windows::io::AsRawHandle;
    use std::ptr::null_mut;
    use windows_sys::Win32::Foundation::ERROR_BROKEN_PIPE;
    use windows_sys::Win32::System::Pipes::PeekNamedPipe;

    let mut available = 0;
    // This collector owns the sole reader; only read bytes already buffered in the pipe.
    let success = unsafe {
        PeekNamedPipe(
            stdout.as_raw_handle(),
            null_mut(),
            0,
            null_mut(),
            &mut available,
            null_mut(),
        )
    };
    if success == 0 {
        let error = io::Error::last_os_error();
        return if error.raw_os_error() == Some(ERROR_BROKEN_PIPE as i32) {
            Ok(0)
        } else {
            Err(error)
        };
    }
    if available == 0 {
        return Err(io::ErrorKind::WouldBlock.into());
    }
    let count = buffer.len().min(available as usize);
    match stdout.read(&mut buffer[..count]) {
        Err(error) if error.raw_os_error() == Some(ERROR_BROKEN_PIPE as i32) => Ok(0),
        result => result,
    }
}

#[cfg(not(any(unix, windows)))]
fn prepare_stdout(_: &ChildStdout) -> io::Result<()> {
    Err(io::ErrorKind::Unsupported.into())
}

#[cfg(not(any(unix, windows)))]
fn read_available(_: &mut ChildStdout, _: &mut [u8]) -> io::Result<usize> {
    Err(io::ErrorKind::Unsupported.into())
}

#[cfg(all(test, unix))]
mod tests;
