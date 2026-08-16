use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::fault::TransactionFault;

const JOURNAL_FILE: &str = "transaction-journal.json";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CommitPhase {
    Prepared,
    FirstTargetCommitted,
    TargetsCommitted,
    ConfigCommitted,
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

pub(crate) struct TransactionFiles {
    root: PathBuf,
    journal_path: PathBuf,
    journal: TransactionJournal,
}

impl TransactionFiles {
    pub(crate) fn prepare(
        root: &Path,
        writes: &[(&str, &Path, &[u8])],
        mut inject: impl FnMut(TransactionFault) -> io::Result<()>,
    ) -> io::Result<Self> {
        fs::create_dir_all(root)?;
        let transaction_id = Uuid::new_v4();
        let mut entries = Vec::with_capacity(writes.len());
        for (name, target, bytes) in writes {
            let staged_path = sibling(target, &transaction_id, "staged")?;
            let original_path = sibling(target, &transaction_id, "original")?;
            let original_existed = target.exists();
            if original_existed {
                fs::copy(target, &original_path)
                    .map_err(|error| io_context("copy original", &original_path, error))?;
                sync_path(&original_path)
                    .map_err(|error| io_context("sync original", &original_path, error))?;
            }
            inject(TransactionFault::StageCreate)?;
            let mut staged = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&staged_path)
                .map_err(|error| io_context("create staged", &staged_path, error))?;
            inject(TransactionFault::StageWrite)?;
            staged
                .write_all(bytes)
                .map_err(|error| io_context("write staged", &staged_path, error))?;
            inject(TransactionFault::StageFlush)?;
            staged
                .sync_all()
                .map_err(|error| io_context("sync staged", &staged_path, error))?;
            entries.push(JournalEntry {
                name: (*name).to_owned(),
                target_path: (*target).to_path_buf(),
                staged_path,
                original_path,
                original_existed,
                expected_bytes: bytes.to_vec(),
            });
        }
        let journal = TransactionJournal {
            transaction_id,
            phase: CommitPhase::Prepared,
            ordered_targets: entries.iter().map(|entry| entry.name.clone()).collect(),
            entries,
        };
        let mut files = Self {
            root: root.to_path_buf(),
            journal_path: root.join(JOURNAL_FILE),
            journal,
        };
        inject(TransactionFault::JournalCreate)?;
        files.sync_journal(&mut inject)?;
        Ok(files)
    }

    pub(crate) fn commit(
        &mut self,
        mut inject: impl FnMut(TransactionFault) -> io::Result<()>,
    ) -> io::Result<()> {
        inject(TransactionFault::AfterJournalSync)?;
        for index in 0..self.journal.entries.len() {
            let rename_fault = match index {
                0 => TransactionFault::FirstTargetRename,
                1 => TransactionFault::SecondTargetRename,
                _ => TransactionFault::ConfigRename,
            };
            inject(rename_fault)?;
            replace(
                &self.journal.entries[index].staged_path,
                &self.journal.entries[index].target_path,
            )?;
            sync_parent(&self.journal.entries[index].target_path)?;
            self.journal.phase = match index {
                0 => CommitPhase::FirstTargetCommitted,
                1 if self.journal.entries.len() == 3 => CommitPhase::TargetsCommitted,
                _ => CommitPhase::ConfigCommitted,
            };
            self.sync_journal(&mut inject)?;
            let interruption = match index {
                0 => TransactionFault::AfterFirstTargetRename,
                1 if self.journal.entries.len() == 3 => TransactionFault::AfterSecondTargetRename,
                _ => TransactionFault::AfterConfigRename,
            };
            inject(interruption)?;
        }
        self.verify_new()?;
        self.cleanup()
    }

    pub(crate) fn compensate(
        &mut self,
        mut inject: impl FnMut(TransactionFault) -> io::Result<()>,
    ) -> io::Result<()> {
        for entry in self.journal.entries.iter().rev() {
            inject(TransactionFault::CompensationRename)?;
            restore(entry)?;
        }
        self.journal.phase = CommitPhase::Compensated;
        self.sync_journal(&mut inject)?;
        self.verify_original()?;
        self.cleanup()
    }

    fn sync_journal(
        &mut self,
        inject: &mut impl FnMut(TransactionFault) -> io::Result<()>,
    ) -> io::Result<()> {
        let temporary = self.root.join(format!(".{JOURNAL_FILE}.tmp"));
        inject(TransactionFault::JournalWrite)?;
        let bytes = serde_json::to_vec_pretty(&self.journal).map_err(io::Error::other)?;
        let mut file = File::create(&temporary)?;
        file.write_all(&bytes)?;
        inject(TransactionFault::JournalFlush)?;
        file.sync_all()?;
        drop(file);
        replace(&temporary, &self.journal_path)
            .map_err(|error| io_context("replace journal", &self.journal_path, error))?;
        sync_parent(&self.journal_path)
    }

    fn verify_new(&self) -> io::Result<()> {
        for entry in &self.journal.entries {
            if fs::read(&entry.target_path)? != entry.expected_bytes {
                return Err(io::Error::other("committed bytes differ from staged bytes"));
            }
        }
        Ok(())
    }

    fn verify_original(&self) -> io::Result<()> {
        for entry in &self.journal.entries {
            if entry.original_existed
                && fs::read(&entry.target_path)? != fs::read(&entry.original_path)?
            {
                return Err(io::Error::other(
                    "compensated bytes differ from original bytes",
                ));
            }
            if !entry.original_existed && entry.target_path.exists() {
                return Err(io::Error::other(
                    "compensation retained newly created target",
                ));
            }
        }
        Ok(())
    }

    fn cleanup(&self) -> io::Result<()> {
        for entry in &self.journal.entries {
            remove_if_exists(&entry.staged_path)?;
            remove_if_exists(&entry.original_path)?;
        }
        remove_if_exists(&self.journal_path)?;
        remove_if_exists(&self.root.join(format!(".{JOURNAL_FILE}.tmp")))
    }
}

pub fn recover_pending_transaction(root: &Path) -> io::Result<()> {
    let journal_path = root.join(JOURNAL_FILE);
    if !journal_path.exists() {
        cleanup_stale_artifacts(root)?;
        return Ok(());
    }
    let journal: TransactionJournal =
        serde_json::from_slice(&fs::read(&journal_path)?).map_err(io::Error::other)?;
    let mut files = TransactionFiles {
        root: root.to_path_buf(),
        journal_path,
        journal,
    };
    match files.journal.phase {
        CommitPhase::ConfigCommitted => {
            files.verify_new()?;
            files.cleanup()
        }
        CommitPhase::Prepared
        | CommitPhase::FirstTargetCommitted
        | CommitPhase::TargetsCommitted
        | CommitPhase::Compensated => files.compensate(|_| Ok(())),
    }
}

fn cleanup_stale_artifacts(root: &Path) -> io::Result<()> {
    if !root.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if name.contains(".omo-txn-") || name == format!(".{JOURNAL_FILE}.tmp") {
            remove_if_exists(&path)?;
        }
    }
    Ok(())
}
