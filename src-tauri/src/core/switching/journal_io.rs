#[cfg(not(windows))]
use std::fs::File;
use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CommitPhase {
    Prepared,
    FirstTargetCommitted,
    TargetsCommitted,
    StateCommitted,
    Compensated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JournalEntry {
    pub(crate) name: String,
    pub(crate) target_path: PathBuf,
    pub(crate) staged_path: PathBuf,
    pub(crate) original_path: PathBuf,
    pub(crate) original_existed: bool,
    pub(crate) expected_bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TransactionJournal {
    pub(crate) transaction_id: Uuid,
    pub(crate) phase: CommitPhase,
    pub(crate) ordered_targets: Vec<String>,
    pub(crate) entries: Vec<JournalEntry>,
}

pub(crate) fn restore(entry: &JournalEntry) -> io::Result<()> {
    if entry.original_existed {
        fs::copy(&entry.original_path, &entry.target_path)?;
        sync_path(&entry.target_path)?;
        sync_parent(&entry.target_path)
    } else {
        remove_if_exists(&entry.target_path)
    }
}

pub(crate) fn sibling(target: &Path, id: &Uuid, suffix: &str) -> io::Result<PathBuf> {
    let name = target
        .file_name()
        .ok_or_else(|| io::Error::other("transaction target has no file name"))?
        .to_string_lossy();
    Ok(target.with_file_name(format!(".{name}.omo-txn-{id}.{suffix}")))
}

pub(crate) fn replace(source: &Path, target: &Path) -> io::Result<()> {
    remove_if_exists(target)?;
    fs::rename(source, target)
}

pub(crate) fn sync_path(path: &Path) -> io::Result<()> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)?
        .sync_all()
}

pub(crate) fn sync_parent(_path: &Path) -> io::Result<()> {
    #[cfg(not(windows))]
    if let Some(parent) = _path.parent() {
        File::open(parent)?.sync_all()?;
    }
    Ok(())
}

pub(crate) fn remove_if_exists(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

pub(crate) fn io_context(operation: &str, path: &Path, error: io::Error) -> io::Error {
    io::Error::new(
        error.kind(),
        format!("{operation} {}: {error}", path.display()),
    )
}
