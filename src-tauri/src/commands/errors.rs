use crate::application::ApplicationError;
use crate::commands::types::{CommandError, CommandErrorCode};
use crate::core::paths::ConfigPathError;
use crate::core::repository::RepositoryError;
use crate::core::switching::SwitchError;

pub(crate) fn command_error_from_application(error: ApplicationError) -> CommandError {
    match error {
        ApplicationError::ConfigPath(error) => command_error_from_config_path(error),
        ApplicationError::LoadGroups(error) => load_groups_error(error),
        ApplicationError::SaveGroups(error) => save_groups_error(error),
        ApplicationError::LoadAppState(error) => load_app_state_error(error),
        ApplicationError::SaveAppState(error) => save_app_state_error(error),
        ApplicationError::GroupNotFound => group_not_found_error(),
        ApplicationError::DuplicateGroupName => duplicate_group_name_error(),
        ApplicationError::Switch(error) => command_error_from_switch(error),
    }
}

pub(crate) fn command_error_from_config_path(error: ConfigPathError) -> CommandError {
    match error {
        ConfigPathError::MissingHome => {
            CommandError::new(CommandErrorCode::MissingHome, error.to_string())
        }
    }
}

pub(crate) fn command_error_from_switch(error: SwitchError) -> CommandError {
    match error {
        SwitchError::LoadGroupsFailed { source } => load_groups_error(source),
        SwitchError::GroupNotFound => group_not_found_error(),
        SwitchError::GroupDisabled => {
            CommandError::new(CommandErrorCode::GroupDisabled, "Group is disabled.")
        }
        SwitchError::LoadAppStateFailed { source } => load_app_state_error(source),
        SwitchError::LoadOpenCodeConfigFailed => CommandError::new(
            CommandErrorCode::MissingOpenCodeConfig,
            "OpenCode config not found or malformed.",
        ),
        SwitchError::LoadOhMyConfigFailed => CommandError::new(
            CommandErrorCode::LoadOhMyConfigFailed,
            "Failed to load Oh My OpenAgent config.",
        ),
        SwitchError::BackupFailed { source } => CommandError::with_detail(
            CommandErrorCode::BackupFailed,
            "Failed to create backup.",
            source.to_string(),
        ),
        SwitchError::WriteFailed { target, source } => CommandError::with_detail(
            CommandErrorCode::WriteFailed,
            format!("Failed to write {target}."),
            source,
        ),
        SwitchError::RollbackFailed { source } => CommandError::with_detail(
            CommandErrorCode::RollbackFailed,
            "Rollback failed.",
            source.to_string(),
        ),
        SwitchError::SaveAppStateFailed { source } => save_app_state_error(source),
        SwitchError::TransactionFailed { stage, source } => CommandError::with_detail(
            CommandErrorCode::WriteFailed,
            "Transactional config write failed.",
            format!("{stage}: {source}"),
        ),
        SwitchError::CompensationFailed { primary, source } => CommandError::with_detail(
            CommandErrorCode::RollbackFailed,
            "Transactional rollback failed; recovery evidence was retained.",
            format!("primary={primary}; compensation={source}"),
        ),
        SwitchError::Interrupted { stage } => CommandError::with_detail(
            CommandErrorCode::WriteFailed,
            "Transactional config write was interrupted.",
            stage,
        ),
    }
}

pub(crate) fn load_groups_error(source: RepositoryError) -> CommandError {
    CommandError::with_detail(
        CommandErrorCode::LoadGroupsFailed,
        "Failed to load groups.",
        source.to_string(),
    )
}

pub(crate) fn save_groups_error(source: RepositoryError) -> CommandError {
    CommandError::with_detail(
        CommandErrorCode::SaveGroupsFailed,
        "Failed to save groups.",
        source.to_string(),
    )
}

pub(crate) fn load_app_state_error(source: RepositoryError) -> CommandError {
    CommandError::with_detail(
        CommandErrorCode::LoadAppStateFailed,
        "Failed to load app state.",
        source.to_string(),
    )
}

pub(crate) fn save_app_state_error(source: RepositoryError) -> CommandError {
    CommandError::with_detail(
        CommandErrorCode::SaveAppStateFailed,
        "Failed to save app state.",
        source.to_string(),
    )
}

pub(crate) fn group_not_found_error() -> CommandError {
    CommandError::new(CommandErrorCode::GroupNotFound, "Group not found.")
}

pub(crate) fn duplicate_group_name_error() -> CommandError {
    CommandError::new(
        CommandErrorCode::DuplicateGroupName,
        "A group with this name already exists.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::path::PathBuf;

    use crate::core::backup::BackupError;
    use crate::core::error::PersistenceOperation;

    #[test]
    fn application_errors_map_every_variant_to_its_stable_wire_code() {
        let cases = vec![
            (
                "config path",
                ApplicationError::ConfigPath(ConfigPathError::MissingHome),
                CommandErrorCode::MissingHome,
            ),
            (
                "load groups",
                ApplicationError::LoadGroups(repository_error("groups")),
                CommandErrorCode::LoadGroupsFailed,
            ),
            (
                "save groups",
                ApplicationError::SaveGroups(repository_error("groups")),
                CommandErrorCode::SaveGroupsFailed,
            ),
            (
                "load app state",
                ApplicationError::LoadAppState(repository_error("state")),
                CommandErrorCode::LoadAppStateFailed,
            ),
            (
                "save app state",
                ApplicationError::SaveAppState(repository_error("state")),
                CommandErrorCode::SaveAppStateFailed,
            ),
            (
                "group not found",
                ApplicationError::GroupNotFound,
                CommandErrorCode::GroupNotFound,
            ),
            (
                "duplicate group name",
                ApplicationError::DuplicateGroupName,
                CommandErrorCode::DuplicateGroupName,
            ),
            (
                "switch",
                ApplicationError::Switch(SwitchError::GroupDisabled),
                CommandErrorCode::GroupDisabled,
            ),
        ];

        for (name, error, expected_code) in cases {
            assert_eq!(
                command_error_from_application(error).code,
                expected_code,
                "{name}"
            );
        }
    }

    #[test]
    fn switch_errors_map_every_variant_to_its_stable_wire_code_and_diagnostics() {
        let cases = vec![
            (
                "load groups",
                SwitchError::LoadGroupsFailed {
                    source: repository_error("groups"),
                },
                CommandErrorCode::LoadGroupsFailed,
                None,
            ),
            (
                "group not found",
                SwitchError::GroupNotFound,
                CommandErrorCode::GroupNotFound,
                None,
            ),
            (
                "group disabled",
                SwitchError::GroupDisabled,
                CommandErrorCode::GroupDisabled,
                None,
            ),
            (
                "load app state",
                SwitchError::LoadAppStateFailed {
                    source: repository_error("state"),
                },
                CommandErrorCode::LoadAppStateFailed,
                None,
            ),
            (
                "load opencode config",
                SwitchError::LoadOpenCodeConfigFailed,
                CommandErrorCode::MissingOpenCodeConfig,
                None,
            ),
            (
                "load oh-my-openagent config",
                SwitchError::LoadOhMyConfigFailed,
                CommandErrorCode::LoadOhMyConfigFailed,
                None,
            ),
            (
                "backup",
                SwitchError::BackupFailed {
                    source: BackupError::Io(io::Error::other("backup filesystem unavailable")),
                },
                CommandErrorCode::BackupFailed,
                Some((
                    "Failed to create backup.",
                    "backup filesystem error: backup filesystem unavailable",
                )),
            ),
            (
                "write",
                SwitchError::WriteFailed {
                    target: "opencode",
                    source: "disk full".to_owned(),
                },
                CommandErrorCode::WriteFailed,
                Some(("Failed to write opencode.", "disk full")),
            ),
            (
                "rollback",
                SwitchError::RollbackFailed {
                    source: io::Error::other("restore failed"),
                },
                CommandErrorCode::RollbackFailed,
                Some(("Rollback failed.", "restore failed")),
            ),
            (
                "save app state",
                SwitchError::SaveAppStateFailed {
                    source: repository_error("state"),
                },
                CommandErrorCode::SaveAppStateFailed,
                None,
            ),
            (
                "transaction",
                SwitchError::TransactionFailed {
                    stage: "commit",
                    source: io::Error::other("rename failed"),
                },
                CommandErrorCode::WriteFailed,
                Some((
                    "Transactional config write failed.",
                    "commit: rename failed",
                )),
            ),
            (
                "compensation",
                SwitchError::CompensationFailed {
                    primary: "opencode write failed".to_owned(),
                    source: io::Error::other("compensation rename failed"),
                },
                CommandErrorCode::RollbackFailed,
                Some((
                    "Transactional rollback failed; recovery evidence was retained.",
                    "primary=opencode write failed; compensation=compensation rename failed",
                )),
            ),
            (
                "interrupted",
                SwitchError::Interrupted {
                    stage: "after-state-rename",
                },
                CommandErrorCode::WriteFailed,
                Some((
                    "Transactional config write was interrupted.",
                    "after-state-rename",
                )),
            ),
        ];

        for (name, error, expected_code, diagnostic) in cases {
            let mapped = command_error_from_switch(error);

            assert_eq!(mapped.code, expected_code, "{name}");
            if let Some((message, detail)) = diagnostic {
                assert_eq!(mapped.message, message, "{name}");
                assert_eq!(mapped.detail.as_deref(), Some(detail), "{name}");
            }
        }
    }

    fn repository_error(name: &str) -> RepositoryError {
        RepositoryError::WriteFailed {
            path: PathBuf::from(format!("{name}.json")),
            operation: PersistenceOperation::Write,
            source: io::Error::other(format!("{name} write failed")),
        }
    }
}
