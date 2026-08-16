use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigPathError {
    MissingHome,
}

impl ConfigPathError {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::MissingHome => "missingHome",
        }
    }

    pub const fn detail(&self) -> &'static str {
        match self {
            Self::MissingHome => "HOME or USERPROFILE must be set",
        }
    }
}

impl fmt::Display for ConfigPathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.detail())
    }
}

impl std::error::Error for ConfigPathError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentError {
    MalformedJson,
}

impl fmt::Display for DocumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedJson => formatter.write_str("malformed JSON document"),
        }
    }
}

impl std::error::Error for DocumentError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersistenceOperation {
    Read,
    Parse,
    Serialize,
    CreateDirectory,
    CreateTemporaryFile,
    Write,
    Flush,
    Sync,
    Rename,
}

impl PersistenceOperation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Parse => "parse",
            Self::Serialize => "serialize",
            Self::CreateDirectory => "create directory",
            Self::CreateTemporaryFile => "create temporary file",
            Self::Write => "write",
            Self::Flush => "flush",
            Self::Sync => "sync",
            Self::Rename => "rename",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepositoryErrorCode {
    ReadFailed,
    MalformedJson,
    WriteFailed,
}

impl RepositoryErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ReadFailed => "readFailed",
            Self::MalformedJson => "malformedJson",
            Self::WriteFailed => "writeFailed",
        }
    }
}

#[derive(Debug)]
pub enum RepositoryError {
    ReadFailed {
        path: PathBuf,
        source: io::Error,
    },
    MalformedJson {
        path: PathBuf,
        operation: PersistenceOperation,
        source: serde_json::Error,
    },
    WriteFailed {
        path: PathBuf,
        operation: PersistenceOperation,
        source: io::Error,
    },
}

impl RepositoryError {
    pub const fn code(&self) -> RepositoryErrorCode {
        match self {
            Self::ReadFailed { .. } => RepositoryErrorCode::ReadFailed,
            Self::MalformedJson { .. } => RepositoryErrorCode::MalformedJson,
            Self::WriteFailed { .. } => RepositoryErrorCode::WriteFailed,
        }
    }

    pub const fn operation(&self) -> PersistenceOperation {
        match self {
            Self::ReadFailed { .. } => PersistenceOperation::Read,
            Self::MalformedJson { operation, .. } | Self::WriteFailed { operation, .. } => {
                *operation
            }
        }
    }

    pub fn path(&self) -> &Path {
        match self {
            Self::ReadFailed { path, .. }
            | Self::MalformedJson { path, .. }
            | Self::WriteFailed { path, .. } => path,
        }
    }

    pub fn detail(&self) -> String {
        match self {
            Self::ReadFailed { path, source } => format!("read {}: {source}", path.display()),
            Self::MalformedJson {
                path,
                operation,
                source,
            } => {
                format!("{} {}: {source}", operation.as_str(), path.display())
            }
            Self::WriteFailed {
                path,
                operation,
                source,
            } => {
                format!("{} {}: {source}", operation.as_str(), path.display())
            }
        }
    }
}

impl fmt::Display for RepositoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code().as_str(), self.detail())
    }
}

impl std::error::Error for RepositoryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ReadFailed { source, .. } | Self::WriteFailed { source, .. } => Some(source),
            Self::MalformedJson { source, .. } => Some(source),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetConfigErrorCode {
    FileNotFound,
    ReadFailed,
    MalformedConfig,
    WriteFailed,
}

impl TargetConfigErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FileNotFound => "fileNotFound",
            Self::ReadFailed => "readFailed",
            Self::MalformedConfig => "malformedConfig",
            Self::WriteFailed => "writeFailed",
        }
    }
}

#[derive(Debug)]
pub enum TargetConfigError {
    FileNotFound {
        path: PathBuf,
    },
    ReadFailed {
        path: PathBuf,
        source: io::Error,
    },
    MalformedConfig {
        path: PathBuf,
    },
    WriteFailed {
        path: PathBuf,
        operation: PersistenceOperation,
        source: io::Error,
    },
}

impl TargetConfigError {
    pub const fn code(&self) -> TargetConfigErrorCode {
        match self {
            Self::FileNotFound { .. } => TargetConfigErrorCode::FileNotFound,
            Self::ReadFailed { .. } => TargetConfigErrorCode::ReadFailed,
            Self::MalformedConfig { .. } => TargetConfigErrorCode::MalformedConfig,
            Self::WriteFailed { .. } => TargetConfigErrorCode::WriteFailed,
        }
    }

    pub const fn operation(&self) -> PersistenceOperation {
        match self {
            Self::FileNotFound { .. } | Self::ReadFailed { .. } => PersistenceOperation::Read,
            Self::MalformedConfig { .. } => PersistenceOperation::Parse,
            Self::WriteFailed { operation, .. } => *operation,
        }
    }

    pub fn path(&self) -> &Path {
        match self {
            Self::FileNotFound { path }
            | Self::ReadFailed { path, .. }
            | Self::MalformedConfig { path }
            | Self::WriteFailed { path, .. } => path,
        }
    }

    pub fn detail(&self) -> String {
        match self {
            Self::FileNotFound { path } => format!("read {}: file not found", path.display()),
            Self::ReadFailed { path, source } => format!("read {}: {source}", path.display()),
            Self::MalformedConfig { path } => format!("parse {}: malformed config", path.display()),
            Self::WriteFailed {
                path,
                operation,
                source,
            } => {
                format!("{} {}: {source}", operation.as_str(), path.display())
            }
        }
    }
}

impl PartialEq for TargetConfigError {
    fn eq(&self, other: &Self) -> bool {
        self.code() == other.code()
            && self.operation() == other.operation()
            && self.path() == other.path()
            && io_kind(self) == io_kind(other)
    }
}

impl Eq for TargetConfigError {}

impl fmt::Display for TargetConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail())
    }
}

impl std::error::Error for TargetConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::FileNotFound { .. } | Self::MalformedConfig { .. } => None,
            Self::ReadFailed { source, .. } | Self::WriteFailed { source, .. } => Some(source),
        }
    }
}

fn io_kind(error: &TargetConfigError) -> Option<io::ErrorKind> {
    match error {
        TargetConfigError::FileNotFound { .. } | TargetConfigError::MalformedConfig { .. } => None,
        TargetConfigError::ReadFailed { source, .. }
        | TargetConfigError::WriteFailed { source, .. } => Some(source.kind()),
    }
}
