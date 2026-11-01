use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::paths::ConfigPaths;
use crate::resource_file::{self, Directory, Snapshot};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BehaviorDocument {
    pub path: PathBuf,
    pub resolved_path: Option<PathBuf>,
    pub content: String,
    pub exists: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BehaviorUpdate {
    pub content: String,
    pub expected_path: Option<PathBuf>,
    pub expected_content: Option<String>,
}

pub fn get(paths: &ConfigPaths) -> Result<BehaviorDocument, AppError> {
    let path = paths.global_instructions_file();
    let snapshot = Snapshot::read(&path)?;
    Ok(BehaviorDocument {
        path,
        resolved_path: snapshot.as_ref().map(|snapshot| snapshot.path.clone()),
        content: snapshot
            .as_ref()
            .map(|snapshot| snapshot.content.clone())
            .unwrap_or_default(),
        exists: snapshot.is_some(),
    })
}

pub fn save(paths: &ConfigPaths, draft: BehaviorUpdate) -> Result<BehaviorDocument, AppError> {
    save_inner(paths, draft, || Ok(()))
}

fn save_inner(
    paths: &ConfigPaths,
    draft: BehaviorUpdate,
    mut before_publish: impl FnMut() -> Result<(), AppError>,
) -> Result<BehaviorDocument, AppError> {
    let path = paths.global_instructions_file();
    paths.validate_managed_write(&path)?;
    match Snapshot::read(&path)? {
        Some(snapshot) => {
            let (Some(expected_path), Some(expected_content)) =
                (draft.expected_path, draft.expected_content)
            else {
                return Err(AppError::validation(
                    "Existing instructions require a source snapshot; reload before saving",
                ));
            };
            if snapshot.path != expected_path || snapshot.content != expected_content {
                return Err(changed());
            }
            resource_file::replace_checked(&[(snapshot, draft.content)], |_| {
                before_publish()?;
                paths.validate_managed_write(&path)
            })?;
        }
        None => {
            if draft.expected_path.is_some() || draft.expected_content.is_some() {
                return Err(changed());
            }
            let parent = Directory::ensure(path.parent().ok_or_else(changed)?)?;
            paths.validate_managed_write(&path)?;
            parent.create_file_checked("AGENTS.md", &draft.content, || {
                before_publish()?;
                paths.validate_managed_write(&path)?;
                resource_file::absent(&path)
            })?;
        }
    }
    get(paths)
}

fn changed() -> AppError {
    AppError::configuration("Global instructions changed; reload before saving")
}

#[cfg(test)]
mod tests;
