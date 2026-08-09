use std::fmt;

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
