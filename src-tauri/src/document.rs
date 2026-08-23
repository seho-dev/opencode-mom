use std::fs;
use std::path::Path;

use serde_json::{Map, Value};

use crate::error::AppError;
use crate::jsonc::{parse_jsonc_object, patch};

pub const OPENCODE_SCHEMA: &str = "https://opencode.ai/config.json";
pub const SLIM_SCHEMA: &str =
    "https://unpkg.com/oh-my-opencode-slim@latest/oh-my-opencode-slim.schema.json";
pub const OMO_SCHEMA: &str =
    "https://raw.githubusercontent.com/code-yeongyu/oh-my-openagent/dev/assets/omo.schema.json";

/// Writes bytes to a file, creating parent directories as needed. Plain sequential write,
/// no atomicity, locking, or backups by design.
pub fn write_file(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|error| AppError::io("create", parent, error))?;
        }
    }
    fs::write(path, bytes).map_err(|error| AppError::io("write", path, error))
}

/// Reads a file as UTF-8 text; `None` when the file does not exist.
pub fn read_text(path: &Path) -> Result<Option<String>, AppError> {
    match fs::read_to_string(path) {
        Ok(content) => Ok(Some(content)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(AppError::io("read", path, error)),
    }
}

/// A parsed JSONC document that retains its original CST text. Mutations always use JSONC
/// paths, so comments and formatting outside changed nodes remain untouched.
#[derive(Debug, Clone, PartialEq)]
pub struct JsoncDoc {
    content: String,
    raw: Map<String, Value>,
}

impl JsoncDoc {
    pub fn parse(input: &str) -> Result<Self, AppError> {
        let raw = parse_jsonc_object(input)
            .map_err(|_| AppError::validation("malformed JSONC document"))?;
        Ok(Self {
            content: input.to_owned(),
            raw,
        })
    }

    /// An empty document carrying only the `$schema` marker for the given target file.
    pub fn bootstrap(schema: &str) -> Self {
        let content = format!("{{\n  \"$schema\": \"{schema}\"\n}}\n");
        Self::parse(&content).expect("constant JSONC bootstrap must parse")
    }

    /// Reads and parses a document from disk. A missing file yields an in-memory bootstrap
    /// document that is only persisted on the next write.
    pub fn read(path: &Path, schema: &str) -> Result<Self, AppError> {
        match read_text(path)? {
            Some(content) => JsoncDoc::parse(&content),
            None => Ok(Self::bootstrap(schema)),
        }
    }

    pub fn patch(&mut self, path: &[&str], value: Option<Value>) -> Result<(), AppError> {
        self.content = patch(&self.content, path, value)
            .map_err(|_| AppError::validation("malformed JSONC document"))?;
        self.raw = parse_jsonc_object(&self.content)
            .map_err(|_| AppError::validation("malformed JSONC document"))?;
        Ok(())
    }

    pub fn save(&self, path: &Path) -> Result<(), AppError> {
        write_file(path, self.content.as_bytes())
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn raw(&self) -> &Map<String, Value> {
        &self.raw
    }
}
