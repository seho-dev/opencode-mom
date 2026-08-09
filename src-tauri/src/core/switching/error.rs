use std::io;

use crate::core::backup::BackupError;
use crate::core::repository::RepositoryError;

#[derive(Debug)]
pub enum SwitchError {
    LoadGroupsFailed {
        source: RepositoryError,
    },
    GroupNotFound,
    GroupDisabled,
    LoadAppStateFailed {
        source: RepositoryError,
    },
    LoadOpenCodeConfigFailed,
    LoadOhMyConfigFailed,
    BackupFailed {
        source: BackupError,
    },
    WriteFailed {
        target: &'static str,
        source: String,
    },
    RollbackFailed {
        source: io::Error,
    },
    SaveAppStateFailed {
        source: RepositoryError,
    },
    TransactionFailed {
        stage: &'static str,
        source: io::Error,
    },
    CompensationFailed {
        primary: String,
        source: io::Error,
    },
    Interrupted {
        stage: &'static str,
    },
}

impl PartialEq for SwitchError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::LoadGroupsFailed { .. }, Self::LoadGroupsFailed { .. })
            | (Self::GroupNotFound, Self::GroupNotFound)
            | (Self::GroupDisabled, Self::GroupDisabled)
            | (Self::LoadAppStateFailed { .. }, Self::LoadAppStateFailed { .. })
            | (Self::LoadOpenCodeConfigFailed, Self::LoadOpenCodeConfigFailed)
            | (Self::LoadOhMyConfigFailed, Self::LoadOhMyConfigFailed)
            | (Self::BackupFailed { .. }, Self::BackupFailed { .. })
            | (Self::RollbackFailed { .. }, Self::RollbackFailed { .. })
            | (Self::SaveAppStateFailed { .. }, Self::SaveAppStateFailed { .. })
            | (Self::TransactionFailed { .. }, Self::TransactionFailed { .. })
            | (Self::CompensationFailed { .. }, Self::CompensationFailed { .. })
            | (Self::Interrupted { .. }, Self::Interrupted { .. }) => true,
            (
                Self::WriteFailed {
                    target: left_target,
                    ..
                },
                Self::WriteFailed {
                    target: right_target,
                    ..
                },
            ) => left_target == right_target,
            _ => false,
        }
    }
}

impl Eq for SwitchError {}

impl std::fmt::Display for SwitchError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LoadGroupsFailed { source } => {
                write!(formatter, "failed to load groups: {source}")
            }
            Self::GroupNotFound => formatter.write_str("group not found"),
            Self::GroupDisabled => formatter.write_str("group is disabled"),
            Self::LoadAppStateFailed { source } => {
                write!(formatter, "failed to load app state: {source}")
            }
            Self::LoadOpenCodeConfigFailed => formatter.write_str("failed to load opencode config"),
            Self::LoadOhMyConfigFailed => {
                formatter.write_str("failed to load oh-my-openagent config")
            }
            Self::BackupFailed { source } => write!(formatter, "failed to create backup: {source}"),
            Self::WriteFailed { target, source } => {
                write!(formatter, "failed to write {target}: {source}")
            }
            Self::RollbackFailed { source } => write!(formatter, "rollback failed: {source}"),
            Self::SaveAppStateFailed { source } => {
                write!(formatter, "config written but state save failed: {source}")
            }
            Self::TransactionFailed { stage, source } => {
                write!(formatter, "transaction failed at {stage}: {source}")
            }
            Self::CompensationFailed { primary, source } => {
                write!(
                    formatter,
                    "transaction failed ({primary}); compensation failed: {source}"
                )
            }
            Self::Interrupted { stage } => write!(formatter, "transaction interrupted at {stage}"),
        }
    }
}

impl std::error::Error for SwitchError {}
