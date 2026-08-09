use std::fmt;

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
