use std::fs;
use std::io;
use std::path::PathBuf;

use time::OffsetDateTime;
use uuid::Uuid;

use crate::core::backup::{BackupArtifact, BackupError, BackupRepository};
use crate::core::document::{OhMyOpenAgentDocument, OpenCodeDocument};
use crate::core::models::{
    AppSelectionState, LastSuccessfulWriteMetadata, ModelGroup, ProjectionIssueSummary,
};
use crate::core::projection::{
    OhMyOpenAgentProjectionService, OpenCodeProjectionService, ProjectionResult,
};
use crate::core::repository::{
    AppStateRepository, ModelGroupRepository, OhMyOpenAgentConfigRepository,
    OpenCodeConfigRepository,
};

use super::error::SwitchError;
use super::fault::{interruption_stage, is_interruption, SwitchTestControl, TransactionFault};
use super::journal::{recover_pending_transaction, TransactionFiles};

const BACKUP_RETENTION_LIMIT: usize = 5;

#[derive(Debug, Clone)]
pub struct SwitchGroupRepositories {
    pub model_groups: ModelGroupRepository,
    pub app_state: AppStateRepository,
    pub backups_root: PathBuf,
    pub opencode: OpenCodeConfigRepository,
    pub oh_my_openagent: OhMyOpenAgentConfigRepository,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SwitchOutcome {
    Success {
        oh_my_document: OhMyOpenAgentDocument,
        warnings: Vec<String>,
    },
    NoOp,
}

pub struct SwitchBackups {
    pub opencode: Option<BackupArtifact>,
    pub oh_my: Option<BackupArtifact>,
}

pub fn has_effective_opencode_overrides(group: &ModelGroup) -> bool {
    group
        .open_code_agent_overrides
        .iter()
        .any(|override_row| !override_row.model_ref.trim().is_empty())
}

pub fn create_backup_if_source_exists<C>(
    backup_repository: &BackupRepository<C>,
    target: &str,
    source_file: &std::path::Path,
) -> Result<Option<BackupArtifact>, SwitchError>
where
    C: Fn() -> OffsetDateTime + Clone,
{
    if !source_file.exists() {
        return Ok(None);
    }
    let contents = fs::read(source_file).map_err(|source| SwitchError::BackupFailed {
        source: BackupError::Io(source),
    })?;
    backup_repository
        .create_backup(target, source_file, &contents)
        .map(Some)
        .map_err(|source| SwitchError::BackupFailed { source })
}

pub fn merge_warnings(
    opencode_projection: Option<&ProjectionResult<OpenCodeDocument>>,
    oh_my_projection: &ProjectionResult<OhMyOpenAgentDocument>,
) -> Vec<String> {
    opencode_projection
        .into_iter()
        .flat_map(|projection| projection.warnings.iter().cloned())
        .chain(oh_my_projection.warnings.iter().cloned())
        .collect()
}

pub fn warning_summary(warnings: &[String]) -> Option<ProjectionIssueSummary> {
    if warnings.is_empty() {
        return None;
    }
    Some(ProjectionIssueSummary {
        message: warnings.join("; "),
        count: warnings.len().try_into().unwrap_or(u32::MAX),
    })
}

pub fn backup_summary(
    opencode_backup: Option<&BackupArtifact>,
    oh_my_backup: Option<&BackupArtifact>,
) -> String {
    format!(
        "opencode:{};oh-my-openagent:{}",
        opencode_backup
            .map(|backup| backup.file_path.display().to_string())
            .unwrap_or_else(|| "none".to_owned()),
        oh_my_backup
            .map(|backup| backup.file_path.display().to_string())
            .unwrap_or_else(|| "none".to_owned())
    )
}

#[derive(Clone)]
pub struct SwitchGroupUseCase<C>
where
    C: Fn() -> OffsetDateTime + Clone,
{
    repositories: SwitchGroupRepositories,
    now: C,
    test_control: SwitchTestControl,
}

impl<C> SwitchGroupUseCase<C>
where
    C: Fn() -> OffsetDateTime + Clone,
{
    pub fn new(repositories: SwitchGroupRepositories, now: C) -> Self {
        Self {
            repositories,
            now,
            test_control: SwitchTestControl::default(),
        }
    }

    pub fn with_test_control(mut self, test_control: SwitchTestControl) -> Self {
        self.test_control = test_control;
        self
    }

    pub fn fail_next_oh_my_save_for_test(&mut self) {
        self.test_control = SwitchTestControl::fail_once(TransactionFault::SecondTargetRename);
    }

    pub fn switch_to(&self, group_id: Uuid) -> Result<SwitchOutcome, SwitchError> {
        self.recover_pending()?;
        let (group, current_state) = self.load_context(group_id)?;
        if current_state.selected_group_id == Some(group_id) {
            return Ok(SwitchOutcome::NoOp);
        }
        self.persist_projection(&group, current_state)
    }

    pub fn save_active_group_projection(
        &self,
        group_id: Uuid,
    ) -> Result<SwitchOutcome, SwitchError> {
        self.recover_pending()?;
        let (group, current_state) = self.load_context(group_id)?;
        if current_state.selected_group_id != Some(group_id) {
            return Ok(SwitchOutcome::NoOp);
        }
        self.persist_projection(&group, current_state)
    }

    fn load_context(&self, group_id: Uuid) -> Result<(ModelGroup, AppSelectionState), SwitchError> {
        let groups = self
            .repositories
            .model_groups
            .load()
            .map_err(|source| SwitchError::LoadGroupsFailed { source })?;
        let group = groups
            .into_iter()
            .find(|group| group.id == group_id)
            .ok_or(SwitchError::GroupNotFound)?;
        if !group.is_enabled {
            return Err(SwitchError::GroupDisabled);
        }
        let state = self
            .repositories
            .app_state
            .load()
            .map_err(|source| SwitchError::LoadAppStateFailed { source })?;
        Ok((group, state))
    }

    fn persist_projection(
        &self,
        group: &ModelGroup,
        current_state: AppSelectionState,
    ) -> Result<SwitchOutcome, SwitchError> {
        let should_write_opencode = has_effective_opencode_overrides(group);
        let opencode_document = self.load_opencode_if_needed(should_write_opencode)?;
        let oh_my_document = self
            .repositories
            .oh_my_openagent
            .load()
            .map_err(|_| SwitchError::LoadOhMyConfigFailed)?;
        let backups = self.create_backups(should_write_opencode)?;
        let opencode_projection = opencode_document
            .as_ref()
            .map(|document| OpenCodeProjectionService::project(group, document));
        let oh_my_projection = OhMyOpenAgentProjectionService::project(group, &oh_my_document);

        let warnings = merge_warnings(opencode_projection.as_ref(), &oh_my_projection);
        let next_state = self.success_state(current_state, group, &backups, &warnings);
        self.save_transaction(opencode_projection.as_ref(), &oh_my_projection, &next_state)?;
        self.cleanup_backups(should_write_opencode);

        Ok(SwitchOutcome::Success {
            oh_my_document: oh_my_projection.document,
            warnings,
        })
    }

    fn load_opencode_if_needed(
        &self,
        should_write_opencode: bool,
    ) -> Result<Option<OpenCodeDocument>, SwitchError> {
        if !should_write_opencode {
            return Ok(None);
        }
        self.repositories
            .opencode
            .load()
            .map(Some)
            .map_err(|_| SwitchError::LoadOpenCodeConfigFailed)
    }

    fn create_backups(&self, should_write_opencode: bool) -> Result<SwitchBackups, SwitchError> {
        let backup_repository =
            BackupRepository::new(self.repositories.backups_root.clone(), self.now.clone());
        let opencode = if should_write_opencode {
            create_backup_if_source_exists(
                &backup_repository,
                "opencode",
                self.repositories.opencode.config_file(),
            )?
        } else {
            None
        };
        let oh_my = create_backup_if_source_exists(
            &backup_repository,
            "oh-my-openagent",
            self.repositories.oh_my_openagent.config_file(),
        )?;
        Ok(SwitchBackups { opencode, oh_my })
    }

    fn save_transaction(
        &self,
        opencode_projection: Option<&ProjectionResult<OpenCodeDocument>>,
        oh_my_projection: &ProjectionResult<OhMyOpenAgentDocument>,
        state: &AppSelectionState,
    ) -> Result<(), SwitchError> {
        let opencode_bytes = opencode_projection
            .map(|projection| projection.document.serialize())
            .transpose()
            .map_err(|_| SwitchError::LoadOpenCodeConfigFailed)?;
        let oh_my_bytes = oh_my_projection
            .document
            .serialize()
            .map_err(|_| SwitchError::LoadOhMyConfigFailed)?;
        let state_bytes =
            serde_json::to_vec_pretty(state).map_err(|source| SwitchError::TransactionFailed {
                stage: "state-serialize",
                source: io::Error::other(source),
            })?;
        let mut writes = Vec::with_capacity(if opencode_bytes.is_some() { 3 } else { 2 });
        if let Some(bytes) = opencode_bytes.as_deref() {
            writes.push((
                "opencode",
                self.repositories.opencode.config_file(),
                bytes.as_bytes(),
            ));
        }
        writes.push((
            "oh-my-openagent",
            self.repositories.oh_my_openagent.config_file(),
            oh_my_bytes.as_bytes(),
        ));
        writes.push((
            "state",
            self.repositories.app_state.state_file(),
            state_bytes.as_slice(),
        ));
        let mut transaction =
            TransactionFiles::prepare(&self.repositories.backups_root, &writes, |stage| {
                self.test_control.inject(stage)
            })
            .map_err(|source| self.transaction_error(source))?;
        match transaction.commit(|stage| self.test_control.inject(stage)) {
            Ok(()) => Ok(()),
            Err(source) if is_interruption(&source) => Err(SwitchError::Interrupted {
                stage: interruption_stage(&source),
            }),
            Err(source) => {
                let primary = source.to_string();
                transaction
                    .compensate(|stage| self.test_control.inject(stage))
                    .map_err(|source| SwitchError::CompensationFailed { primary, source })?;
                Err(SwitchError::TransactionFailed {
                    stage: "commit",
                    source,
                })
            }
        }
    }

    fn success_state(
        &self,
        mut state: AppSelectionState,
        group: &ModelGroup,
        backups: &SwitchBackups,
        warnings: &[String],
    ) -> AppSelectionState {
        state.selected_group_id = Some(group.id);
        state.selected_group_name = Some(group.name.clone());
        state.last_successful_write = Some(LastSuccessfulWriteMetadata {
            target: "switch-group".to_owned(),
            wrote_at: (self.now)(),
            backup_path: Some(backup_summary(
                backups.opencode.as_ref(),
                backups.oh_my.as_ref(),
            )),
        });
        state.last_error_summary = None;
        state.last_warning_summary = warning_summary(warnings);
        state
    }

    fn transaction_error(&self, source: io::Error) -> SwitchError {
        SwitchError::TransactionFailed {
            stage: "prepare",
            source,
        }
    }

    fn recover_pending(&self) -> Result<(), SwitchError> {
        recover_pending_transaction(&self.repositories.backups_root).map_err(|source| {
            SwitchError::TransactionFailed {
                stage: "startup-recovery",
                source,
            }
        })
    }

    fn cleanup_backups(&self, should_write_opencode: bool) {
        let backup_repository =
            BackupRepository::new(self.repositories.backups_root.clone(), self.now.clone());
        if should_write_opencode {
            let _ = backup_repository.cleanup("opencode", BACKUP_RETENTION_LIMIT);
        }
        let _ = backup_repository.cleanup("oh-my-openagent", BACKUP_RETENTION_LIMIT);
    }
}
