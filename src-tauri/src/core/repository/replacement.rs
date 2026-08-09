use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::core::error::PersistenceOperation;

const TEMPORARY_FILE_ATTEMPTS: usize = 16;

static NEXT_TEMPORARY_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub(super) struct ReplacementError {
    path: PathBuf,
    operation: PersistenceOperation,
    source: io::Error,
}

impl ReplacementError {
    pub(super) fn into_parts(self) -> (PathBuf, PersistenceOperation, io::Error) {
        (self.path, self.operation, self.source)
    }
}

/// Replaces an ordinary repository file with a fully written adjacent temporary file.
///
/// This is a conservative replacement path, not a universal power-loss or parent-directory
/// durability guarantee.
pub(super) fn replace_file(path: &Path, bytes: &[u8]) -> Result<(), ReplacementError> {
    let directory = parent_directory(path);
    if let Some(parent) = directory {
        fs::create_dir_all(parent).map_err(|source| ReplacementError {
            path: parent.to_path_buf(),
            operation: PersistenceOperation::CreateDirectory,
            source,
        })?;
    }

    let directory = directory.unwrap_or_else(|| Path::new("."));
    let (temporary_path, mut temporary_file) =
        create_temporary_file(directory, path).map_err(|source| {
            replacement_failure(path, PersistenceOperation::CreateTemporaryFile, source)
        })?;

    let write_result = write_and_sync(&mut temporary_file, bytes);
    // File::drop does not expose close errors; sync_all above captures the observable durability
    // failure before the handle is closed for the Windows rename path.
    drop(temporary_file);
    if let Err((operation, source)) = write_result {
        cleanup_temporary_file(&temporary_path);
        return Err(replacement_failure(path, operation, source));
    }

    #[cfg(test)]
    let rename_result =
        fail_if_injected(ReplacementStep::Rename).and_then(|()| fs::rename(&temporary_path, path));
    #[cfg(not(test))]
    let rename_result = fs::rename(&temporary_path, path);
    if let Err(source) = rename_result {
        cleanup_temporary_file(&temporary_path);
        return Err(replacement_failure(
            path,
            PersistenceOperation::Rename,
            source,
        ));
    }

    Ok(())
}

fn parent_directory(path: &Path) -> Option<&Path> {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
}

fn create_temporary_file(directory: &Path, destination: &Path) -> io::Result<(PathBuf, File)> {
    for _ in 0..TEMPORARY_FILE_ATTEMPTS {
        let temporary_path = temporary_path(directory, destination);
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
        {
            Ok(file) => return Ok((temporary_path, file)),
            Err(source) if source.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(source) => return Err(source),
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate a unique adjacent temporary file",
    ))
}

fn temporary_path(directory: &Path, destination: &Path) -> PathBuf {
    let file_name = destination
        .file_name()
        .unwrap_or_else(|| OsStr::new("repository"));
    let temporary_id = NEXT_TEMPORARY_ID.fetch_add(1, Ordering::Relaxed);
    let mut temporary_name = OsString::from(".");
    temporary_name.push(file_name);
    temporary_name.push(format!(
        ".omo-switch-{}-{temporary_id}.tmp",
        std::process::id()
    ));
    directory.join(temporary_name)
}

fn write_and_sync(
    temporary_file: &mut File,
    bytes: &[u8],
) -> Result<(), (PersistenceOperation, io::Error)> {
    #[cfg(test)]
    fail_if_injected(ReplacementStep::Write)
        .map_err(|source| (PersistenceOperation::Write, source))?;
    temporary_file
        .write_all(bytes)
        .map_err(|source| (PersistenceOperation::Write, source))?;

    #[cfg(test)]
    fail_if_injected(ReplacementStep::Flush)
        .map_err(|source| (PersistenceOperation::Flush, source))?;
    temporary_file
        .flush()
        .map_err(|source| (PersistenceOperation::Flush, source))?;

    #[cfg(test)]
    fail_if_injected(ReplacementStep::Sync)
        .map_err(|source| (PersistenceOperation::Sync, source))?;
    temporary_file
        .sync_all()
        .map_err(|source| (PersistenceOperation::Sync, source))
}

fn cleanup_temporary_file(path: &Path) {
    let _ = fs::remove_file(path);
}

fn replacement_failure(
    path: &Path,
    operation: PersistenceOperation,
    source: io::Error,
) -> ReplacementError {
    ReplacementError {
        path: path.to_path_buf(),
        operation,
        source,
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReplacementStep {
    Write,
    Flush,
    Sync,
    Rename,
}

#[cfg(test)]
fn fail_if_injected(_step: ReplacementStep) -> io::Result<()> {
    let should_fail = TEST_FAILURE.with(|failure| failure.get() == Some(_step));
    if should_fail {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("injected {_step:?} failure"),
        ));
    }

    Ok(())
}

#[cfg(test)]
thread_local! {
    static TEST_FAILURE: std::cell::Cell<Option<ReplacementStep>> = std::cell::Cell::new(None);
}

#[cfg(test)]
struct InjectedFailureGuard {
    previous: Option<ReplacementStep>,
}

#[cfg(test)]
impl Drop for InjectedFailureGuard {
    fn drop(&mut self) {
        TEST_FAILURE.with(|failure| failure.set(self.previous));
    }
}

#[cfg(test)]
fn inject_failure(step: ReplacementStep) -> InjectedFailureGuard {
    let previous = TEST_FAILURE.with(|failure| {
        let previous = failure.get();
        failure.set(Some(step));
        previous
    });
    InjectedFailureGuard { previous }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static NEXT_TEST_DIRECTORY_ID: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn replacement_when_destination_exists_overwrites_it_on_the_same_directory() {
        let directory = temporary_directory("existing-overwrite");
        let destination = directory.join("state.json");
        fs::write(&destination, b"old bytes").expect("Given: original bytes persist");

        replace_file(&destination, b"new bytes").expect("When: replacement succeeds");

        assert_eq!(
            fs::read(&destination).expect("Then: replaced bytes read"),
            b"new bytes"
        );
        assert!(temporary_files(&directory).is_empty());
        remove_temporary_directory(&directory);
    }

    #[test]
    fn replacement_when_parent_creation_fails_preserves_current_error_context_without_temp_file() {
        let directory = temporary_directory("parent-failure");
        let blocked_parent = directory.join("blocked");
        fs::write(&blocked_parent, b"not a directory").expect("Given: blocking file persists");
        let destination = blocked_parent.join("state.json");

        let error = replace_file(&destination, b"new bytes")
            .expect_err("When: parent directory cannot be created");
        let (path, operation, _) = error.into_parts();

        assert_eq!(path, blocked_parent);
        assert_eq!(operation, PersistenceOperation::CreateDirectory);
        assert!(temporary_files(&directory).is_empty());
        remove_temporary_directory(&directory);
    }

    #[test]
    fn replacement_when_write_flush_sync_or_rename_fails_keeps_original_bytes_and_cleans_temp() {
        for (step, expected_operation, label) in [
            (ReplacementStep::Write, PersistenceOperation::Write, "write"),
            (ReplacementStep::Flush, PersistenceOperation::Flush, "flush"),
            (ReplacementStep::Sync, PersistenceOperation::Sync, "sync"),
            (
                ReplacementStep::Rename,
                PersistenceOperation::Rename,
                "rename",
            ),
        ] {
            let directory = temporary_directory(label);
            let destination = directory.join("groups.json");
            fs::write(&destination, b"original bytes")
                .expect("Given: original destination bytes persist");
            let _failure = inject_failure(step);

            let error = replace_file(&destination, b"replacement bytes")
                .expect_err("When: a replacement step fails");
            let (path, operation, _) = error.into_parts();

            assert_eq!(path, destination, "{label}");
            assert_eq!(operation, expected_operation, "{label}");
            assert_eq!(
                fs::read(&destination).expect("Then: original bytes read"),
                b"original bytes",
                "{label}"
            );
            assert!(temporary_files(&directory).is_empty(), "{label}");
            drop(_failure);
            remove_temporary_directory(&directory);
        }
    }

    fn temporary_directory(label: &str) -> PathBuf {
        let id = NEXT_TEST_DIRECTORY_ID.fetch_add(1, Ordering::Relaxed);
        let directory = std::env::temp_dir().join(format!(
            "omo-switch-replacement-{label}-{}-{id}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).expect("Given: temporary directory is created");
        directory
    }

    fn temporary_files(directory: &Path) -> Vec<PathBuf> {
        fs::read_dir(directory)
            .expect("Then: temporary directory reads")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.contains(".omo-switch-"))
            })
            .collect()
    }

    fn remove_temporary_directory(directory: &Path) {
        let _ = fs::remove_dir_all(directory);
    }
}
