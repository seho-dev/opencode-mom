use std::fs;
use std::path::{Path, PathBuf};

use crate::core::error::{PersistenceOperation, RepositoryError};
use crate::core::models::{AppSelectionState, ModelGroup, ModelGroupStore};

use super::replacement::replace_file;

#[derive(Debug, Clone)]
pub struct ModelGroupRepository {
    groups_file: PathBuf,
}

impl ModelGroupRepository {
    pub fn new(groups_file: PathBuf) -> Self {
        Self { groups_file }
    }

    pub fn load(&self) -> Result<Vec<ModelGroup>, RepositoryError> {
        if !self.groups_file.exists() {
            return Ok(Vec::new());
        }

        let data = fs::read_to_string(&self.groups_file).map_err(|source| {
            RepositoryError::ReadFailed {
                path: self.groups_file.clone(),
                source,
            }
        })?;
        let store: ModelGroupStore =
            serde_json::from_str(&data).map_err(|source| RepositoryError::MalformedJson {
                path: self.groups_file.clone(),
                operation: PersistenceOperation::Parse,
                source,
            })?;
        Ok(store.groups)
    }

    pub fn save(&self, groups: &[ModelGroup]) -> Result<(), RepositoryError> {
        let store = ModelGroupStore::new(groups.to_vec());
        let data = serde_json::to_string_pretty(&store).map_err(|source| {
            RepositoryError::MalformedJson {
                path: self.groups_file.clone(),
                operation: PersistenceOperation::Serialize,
                source,
            }
        })?;
        write_json(&self.groups_file, &data)
    }
}

#[derive(Debug, Clone)]
pub struct AppStateRepository {
    state_file: PathBuf,
}

impl AppStateRepository {
    pub fn new(state_file: PathBuf) -> Self {
        Self { state_file }
    }

    pub fn load(&self) -> Result<AppSelectionState, RepositoryError> {
        if !self.state_file.exists() {
            return Ok(AppSelectionState::default());
        }

        let data =
            fs::read_to_string(&self.state_file).map_err(|source| RepositoryError::ReadFailed {
                path: self.state_file.clone(),
                source,
            })?;
        serde_json::from_str(&data).map_err(|source| RepositoryError::MalformedJson {
            path: self.state_file.clone(),
            operation: PersistenceOperation::Parse,
            source,
        })
    }

    pub fn state_file(&self) -> &std::path::Path {
        &self.state_file
    }

    pub fn save(&self, state: &AppSelectionState) -> Result<(), RepositoryError> {
        let data = serde_json::to_string_pretty(state).map_err(|source| {
            RepositoryError::MalformedJson {
                path: self.state_file.clone(),
                operation: PersistenceOperation::Serialize,
                source,
            }
        })?;
        write_json(&self.state_file, &data)
    }
}

fn write_json(path: &Path, data: &str) -> Result<(), RepositoryError> {
    replace_file(path, data.as_bytes()).map_err(|error| {
        let (path, operation, source) = error.into_parts();
        RepositoryError::WriteFailed {
            path,
            operation,
            source,
        }
    })
}
