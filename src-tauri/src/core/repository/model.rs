use std::fs;
use std::path::PathBuf;

use crate::core::error::{PersistenceOperation, RepositoryError};
use crate::core::models::OmoSwitchConfig;

use super::replacement::replace_file;

#[derive(Debug, Clone)]
pub struct ConfigRepository {
    config_file: PathBuf,
}

impl ConfigRepository {
    pub fn new(config_file: PathBuf) -> Self {
        Self { config_file }
    }

    pub fn load(&self) -> Result<OmoSwitchConfig, RepositoryError> {
        if !self.config_file.exists() {
            return Ok(OmoSwitchConfig::default());
        }

        let data = fs::read_to_string(&self.config_file).map_err(|source| {
            RepositoryError::ReadFailed {
                path: self.config_file.clone(),
                source,
            }
        })?;
        serde_json::from_str(&data).map_err(|source| RepositoryError::MalformedJson {
            path: self.config_file.clone(),
            operation: PersistenceOperation::Parse,
            source,
        })
    }

    pub fn save(&self, config: &OmoSwitchConfig) -> Result<(), RepositoryError> {
        let data = serde_json::to_string_pretty(config).map_err(|source| {
            RepositoryError::MalformedJson {
                path: self.config_file.clone(),
                operation: PersistenceOperation::Serialize,
                source,
            }
        })?;
        replace_file(&self.config_file, data.as_bytes()).map_err(|error| {
            let (path, operation, source) = error.into_parts();
            RepositoryError::WriteFailed {
                path,
                operation,
                source,
            }
        })
    }

    pub fn config_file(&self) -> &std::path::Path {
        &self.config_file
    }
}
