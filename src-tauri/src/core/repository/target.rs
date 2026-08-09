use std::fs;
use std::path::{Path, PathBuf};

use crate::core::document::{OhMyOpenAgentDocument, OpenCodeDocument};
use crate::core::error::{DocumentError, TargetConfigError};

use super::replacement::replace_file;

#[derive(Debug, Clone)]
pub struct OhMyOpenAgentConfigRepository {
    config_file: PathBuf,
}

impl OhMyOpenAgentConfigRepository {
    pub fn new(config_file: PathBuf) -> Self {
        Self { config_file }
    }

    pub fn config_file(&self) -> &Path {
        &self.config_file
    }

    pub fn load(&self) -> Result<OhMyOpenAgentDocument, TargetConfigError> {
        if !self.config_file.exists() {
            return Ok(OhMyOpenAgentDocument::bootstrap());
        }

        let data = fs::read_to_string(&self.config_file).map_err(|source| {
            TargetConfigError::ReadFailed {
                path: self.config_file.clone(),
                source,
            }
        })?;
        OhMyOpenAgentDocument::parse_jsonc(&data)
            .map_err(|error| malformed_document(&self.config_file, error))
    }

    pub fn save(&self, document: &OhMyOpenAgentDocument) -> Result<(), TargetConfigError> {
        let data = document
            .serialize()
            .map_err(|error| malformed_document(&self.config_file, error))?;
        write_document(&self.config_file, &data)
    }
}

#[derive(Debug, Clone)]
pub struct OpenCodeConfigRepository {
    config_file: PathBuf,
}

impl OpenCodeConfigRepository {
    pub fn new(config_file: PathBuf) -> Self {
        Self { config_file }
    }

    pub fn config_file(&self) -> &Path {
        &self.config_file
    }

    pub fn load(&self) -> Result<OpenCodeDocument, TargetConfigError> {
        if !self.config_file.exists() {
            return Err(TargetConfigError::FileNotFound {
                path: self.config_file.clone(),
            });
        }

        let data = fs::read_to_string(&self.config_file).map_err(|source| {
            TargetConfigError::ReadFailed {
                path: self.config_file.clone(),
                source,
            }
        })?;
        OpenCodeDocument::parse_jsonc(&data)
            .map_err(|error| malformed_document(&self.config_file, error))
    }

    pub fn save(&self, document: &OpenCodeDocument) -> Result<(), TargetConfigError> {
        let data = document
            .serialize()
            .map_err(|error| malformed_document(&self.config_file, error))?;
        write_document(&self.config_file, &data)
    }
}

fn write_document(path: &Path, data: &str) -> Result<(), TargetConfigError> {
    replace_file(path, data.as_bytes()).map_err(|error| {
        let (path, operation, source) = error.into_parts();
        TargetConfigError::WriteFailed {
            path,
            operation,
            source,
        }
    })
}

fn malformed_document(path: &Path, error: DocumentError) -> TargetConfigError {
    match error {
        DocumentError::MalformedJson => TargetConfigError::MalformedConfig {
            path: path.to_path_buf(),
        },
    }
}
