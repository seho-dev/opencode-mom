use std::fs;

use time::OffsetDateTime;

use crate::core::backup::{BackupArtifact, BackupError, BackupRepository};
use crate::core::document::{OhMyOpenAgentDocument, OpenCodeDocument};
use crate::core::models::{ModelGroup, ProjectionIssueSummary};
use crate::core::projection::ProjectionResult;
use crate::core::switching::error::SwitchError;

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
