use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use time::format_description::FormatItem;
use time::macros::format_description;
use time::OffsetDateTime;

const BACKUP_TIMESTAMP_FORMAT: &[FormatItem<'_>] =
    format_description!("[year][month][day]T[hour][minute][second]Z");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupArtifact {
    pub target: String,
    pub file_path: PathBuf,
    pub created_at: Option<OffsetDateTime>,
}

#[derive(Debug)]
pub enum BackupError {
    Io(io::Error),
    Timestamp(time::error::Format),
}

impl PartialEq for BackupError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Io(left), Self::Io(right)) => left.kind() == right.kind(),
            (Self::Timestamp(_), Self::Timestamp(_)) => true,
            (Self::Io(_), Self::Timestamp(_)) | (Self::Timestamp(_), Self::Io(_)) => false,
        }
    }
}

impl Eq for BackupError {}

impl std::fmt::Display for BackupError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "backup filesystem error: {error}"),
            Self::Timestamp(error) => write!(formatter, "backup timestamp error: {error}"),
        }
    }
}

impl std::error::Error for BackupError {}

impl From<io::Error> for BackupError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<time::error::Format> for BackupError {
    fn from(error: time::error::Format) -> Self {
        Self::Timestamp(error)
    }
}

#[derive(Clone)]
pub struct BackupRepository<C>
where
    C: Fn() -> OffsetDateTime + Clone,
{
    backups_root: PathBuf,
    now: C,
}

impl<C> BackupRepository<C>
where
    C: Fn() -> OffsetDateTime + Clone,
{
    pub fn new(config_root: PathBuf, now: C) -> Self {
        Self {
            backups_root: config_root.join("backups"),
            now,
        }
    }

    pub fn create_backup(
        &self,
        target: &str,
        source_file: &Path,
        contents: &[u8],
    ) -> Result<BackupArtifact, BackupError> {
        let sanitized_target = sanitized(target);
        let directory = self.backups_root.join(&sanitized_target);
        fs::create_dir_all(&directory)?;
        let timestamp = (self.now)().format(BACKUP_TIMESTAMP_FORMAT)?;
        let base_name = source_file
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| sanitized_target.clone());
        let file_path = directory.join(format!("{timestamp}-{base_name}"));
        fs::write(&file_path, contents)?;

        Ok(BackupArtifact {
            target: sanitized_target,
            file_path,
            created_at: Some((self.now)()),
        })
    }

    pub fn list_backups(&self, target: Option<&str>) -> Result<Vec<BackupArtifact>, BackupError> {
        if !self.backups_root.exists() {
            return Ok(Vec::new());
        }
        match target {
            Some(target) => {
                self.list_backups_in(&self.backups_root.join(sanitized(target)), target)
            }
            None => {
                let mut artifacts = Vec::new();
                for entry in fs::read_dir(&self.backups_root)? {
                    let entry = entry?;
                    if entry.file_type()?.is_dir() {
                        let target = entry.file_name().to_string_lossy().into_owned();
                        artifacts.extend(self.list_backups_in(&entry.path(), &target)?);
                    }
                }
                artifacts.sort_by(|left, right| right.file_path.cmp(&left.file_path));
                Ok(artifacts)
            }
        }
    }

    pub fn cleanup(&self, target: &str, keeping_latest: usize) -> Result<(), BackupError> {
        let backups = self.list_backups(Some(target))?;
        for backup in backups.iter().skip(keeping_latest) {
            fs::remove_file(&backup.file_path)?;
        }
        Ok(())
    }

    fn list_backups_in(
        &self,
        directory: &Path,
        target: &str,
    ) -> Result<Vec<BackupArtifact>, BackupError> {
        if !directory.exists() {
            return Ok(Vec::new());
        }
        let mut artifacts = Vec::new();
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                artifacts.push(BackupArtifact {
                    target: sanitized(target),
                    file_path: entry.path(),
                    created_at: None,
                });
            }
        }
        artifacts.sort_by(|left, right| right.file_path.cmp(&left.file_path));
        Ok(artifacts)
    }
}

fn sanitized(value: &str) -> String {
    let mut output = String::new();
    for character in value.trim().chars() {
        match character {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '.' | '_' | '-' => output.push(character),
            _ => output.push('-'),
        }
    }
    if output.is_empty() {
        "unknown-target".to_owned()
    } else {
        output
    }
}
