use serde::Serialize;

/// Error codes exposed to the frontend. `busy` and `conflict` no longer occur because the
/// backend performs plain sequential file writes, but the codes stay reserved for the
/// frontend type contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    NotFound,
    ReferencesBlocked,
    ValidationFailed,
    ConfigurationFailed,
}

/// The single error type for every backend operation. It serializes directly into the
/// frontend `CommandError` shape (`{ code, message, detail? }`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            detail: None,
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotFound, message)
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::ValidationFailed, message)
    }

    pub fn configuration(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::ConfigurationFailed, message)
    }

    pub fn references(message: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::ReferencesBlocked,
            message: message.into(),
            detail: Some(detail.into()),
        }
    }

    /// Wraps an IO error with a short operation context and the involved path.
    pub fn io(context: &str, path: &std::path::Path, error: std::io::Error) -> Self {
        Self::new(
            ErrorCode::ConfigurationFailed,
            format!("{context} {}: {error}", path.display()),
        )
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.detail {
            Some(detail) => write!(formatter, "{} ({detail})", self.message),
            None => formatter.write_str(&self.message),
        }
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        Self::new(ErrorCode::ConfigurationFailed, error.to_string())
    }
}
