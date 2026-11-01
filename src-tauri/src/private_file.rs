use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

use crate::protected_data::SecretBytes;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ReadError {
    Unreadable,
    NotRegular,
    MetadataTooLarge,
    TooLarge,
    UnsafePermissions,
    #[cfg(windows)]
    ReparsePoint,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PublishOutcome {
    Committed,
    CommittedWithWarning,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PublishError {
    DirectoryUnavailable,
    TemporaryCreate,
    TemporaryWrite,
    Publication,
}

struct PendingFile {
    path: std::path::PathBuf,
    owned: bool,
}

impl Drop for PendingFile {
    fn drop(&mut self) {
        if self.owned {
            let _ = fs::remove_file(&self.path);
        }
    }
}

pub(crate) fn read(path: &Path, max_bytes: u64) -> Result<Option<Vec<u8>>, ReadError> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x00200000);
    }
    let file = match options.open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(ReadError::Unreadable),
    };
    let metadata = file.metadata().map_err(|_| ReadError::Unreadable)?;
    if !metadata.is_file() {
        return Err(ReadError::NotRegular);
    }
    if metadata.len() > max_bytes {
        return Err(ReadError::MetadataTooLarge);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(ReadError::ReparsePoint);
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        // SAFETY: geteuid takes no pointers and has no caller preconditions.
        if metadata.uid() != unsafe { libc::geteuid() } || metadata.mode() & 0o777 != 0o600 {
            return Err(ReadError::UnsafePermissions);
        }
    }
    read_bounded(file, max_bytes).map(Some)
}

fn read_bounded(reader: impl Read, max_bytes: u64) -> Result<Vec<u8>, ReadError> {
    let mut bytes = SecretBytes(Vec::new());
    reader
        .take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes.0)
        .map_err(|_| ReadError::Unreadable)?;
    if bytes.0.len() as u64 > max_bytes {
        return Err(ReadError::TooLarge);
    }
    Ok(bytes.into_inner())
}

pub(crate) fn publish(path: &Path, bytes: &[u8]) -> Result<PublishOutcome, PublishError> {
    let directory = path.parent().ok_or(PublishError::DirectoryUnavailable)?;
    #[cfg(unix)]
    let directory_handle = File::open(directory).map_err(|_| PublishError::DirectoryUnavailable)?;
    publish_inner(path, bytes, atomic_replace, move || {
        #[cfg(unix)]
        return directory_handle.sync_all();
        #[cfg(not(unix))]
        Ok(())
    })
}

#[cfg(test)]
pub(crate) fn publish_with(
    path: &Path,
    bytes: &[u8],
    replace: impl FnOnce(&Path, &Path) -> std::io::Result<()>,
    sync_directory: impl FnOnce() -> std::io::Result<()>,
) -> Result<PublishOutcome, PublishError> {
    publish_inner(path, bytes, replace, sync_directory)
}

fn publish_inner(
    path: &Path,
    bytes: &[u8],
    replace: impl FnOnce(&Path, &Path) -> std::io::Result<()>,
    sync_directory: impl FnOnce() -> std::io::Result<()>,
) -> Result<PublishOutcome, PublishError> {
    let directory = path.parent().ok_or(PublishError::DirectoryUnavailable)?;
    let mut temporary = PendingFile {
        path: directory.join(format!(".private-{}.pending", uuid::Uuid::new_v4())),
        owned: false,
    };
    let mut file = create_private(&temporary.path)?;
    temporary.owned = true;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| PublishError::TemporaryWrite)?;
    drop(file);
    replace(&temporary.path, path).map_err(|_| PublishError::Publication)?;
    temporary.owned = false;
    if sync_directory().is_err() {
        return Ok(PublishOutcome::CommittedWithWarning);
    }
    Ok(PublishOutcome::Committed)
}

fn create_private(path: &Path) -> Result<File, PublishError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options
        .open(path)
        .map_err(|_| PublishError::TemporaryCreate)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let metadata = match file.metadata() {
            Ok(metadata) => metadata,
            Err(_) => {
                let _ = fs::remove_file(path);
                return Err(PublishError::TemporaryWrite);
            }
        };
        // SAFETY: geteuid takes no pointers and has no caller preconditions.
        if metadata.uid() != unsafe { libc::geteuid() } {
            let _ = fs::remove_file(path);
            return Err(PublishError::TemporaryWrite);
        }
        if metadata.mode() & 0o777 != 0o600
            && file
                .set_permissions(fs::Permissions::from_mode(0o600))
                .is_err()
        {
            let _ = fs::remove_file(path);
            return Err(PublishError::TemporaryWrite);
        }
    }
    Ok(file)
}

#[cfg(not(windows))]
fn atomic_replace(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::rename(source, destination)
}

#[cfg(windows)]
fn atomic_replace(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    // SAFETY: both Vecs hold NUL-terminated UTF-16 paths and remain alive throughout
    // this synchronous call; MoveFileExW does not retain either pointer.
    if unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    } == 0
    {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publication_unwind_removes_only_the_owned_pending_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("private.json");
        fs::write(&path, b"original").unwrap();
        let result = std::panic::catch_unwind(|| {
            publish_with(
                &path,
                b"secret",
                |_, _| panic!("injected publication unwind"),
                || Ok(()),
            )
        });
        assert!(result.is_err());
        assert_eq!(fs::read(&path).unwrap(), b"original");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[test]
    fn bounded_read_transfers_success_and_rejects_partial_errors_and_overflow() {
        struct PartialError(bool);
        impl Read for PartialError {
            fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
                if std::mem::replace(&mut self.0, false) {
                    buffer[0] = b's';
                    Ok(1)
                } else {
                    Err(std::io::Error::other("injected read failure"))
                }
            }
        }

        let before = crate::protected_data::WIPED_BYTES.with(|count| count.get());
        assert_eq!(read_bounded(&b"secret"[..], 6), Ok(b"secret".to_vec()));
        assert_eq!(
            crate::protected_data::WIPED_BYTES.with(|count| count.get()),
            before
        );
        assert_eq!(read_bounded(&b"secret"[..], 5), Err(ReadError::TooLarge));
        assert_eq!(
            crate::protected_data::WIPED_BYTES.with(|count| count.get()) - before,
            6
        );
        assert_eq!(
            read_bounded(PartialError(true), 6),
            Err(ReadError::Unreadable)
        );
        assert_eq!(
            crate::protected_data::WIPED_BYTES.with(|count| count.get()) - before,
            7
        );

        struct PartialPanic(bool);
        impl Read for PartialPanic {
            fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
                if std::mem::replace(&mut self.0, false) {
                    buffer[0] = b's';
                    Ok(1)
                } else {
                    panic!("injected read unwind");
                }
            }
        }
        assert!(std::panic::catch_unwind(|| read_bounded(PartialPanic(true), 6)).is_err());
        assert_eq!(
            crate::protected_data::WIPED_BYTES.with(|count| count.get()) - before,
            8
        );
    }
}
