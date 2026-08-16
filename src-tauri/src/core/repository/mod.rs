mod model;
mod replacement;
mod target;

pub use crate::core::error::{RepositoryError, RepositoryErrorCode, TargetConfigError};
pub use model::ConfigRepository;
pub use target::{OhMyOpenAgentConfigRepository, OpenCodeConfigRepository};

pub type OhMyOpenAgentConfigError = TargetConfigError;
pub type OpenCodeConfigError = TargetConfigError;
