use std::io;

use time::OffsetDateTime;
use uuid::Uuid;

use crate::core::backup::BackupRepository;
use crate::core::document::{OhMyOpenAgentDocument, OpenCodeDocument};
use crate::core::models::{AppSelectionState, LastSuccessfulWriteMetadata, ModelGroup};
use crate::core::projection::{
    OhMyOpenAgentProjectionService, OpenCodeProjectionService, ProjectionResult,
};
use crate::core::switching::error::SwitchError;
use crate::core::switching::fault::{
    interruption_stage, is_interruption, SwitchTestControl, TransactionFault,
};
use crate::core::switching::journal::TransactionFiles;
use crate::core::switching::recover_pending_transaction;
use crate::core::switching::support::{
    backup_summary, create_backup_if_source_exists, has_effective_opencode_overrides,
    merge_warnings, warning_summary, SwitchBackups,
};
use crate::core::switching::types::{SwitchGroupRepositories, SwitchOutcome};

const BACKUP_RETENTION_LIMIT: usize = 5;

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
