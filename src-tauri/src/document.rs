use std::fs;
use std::io::Write;
use std::path::Path;

use serde_json::{Map, Value};

use crate::error::AppError;
use crate::jsonc::{parse_jsonc_object, patch_with_effective};

pub const OPENCODE_SCHEMA: &str = "https://opencode.ai/config.json";
pub const SLIM_SCHEMA: &str =
    "https://unpkg.com/oh-my-opencode-slim@latest/oh-my-opencode-slim.schema.json";
pub const OMO_SCHEMA: &str =
    "https://raw.githubusercontent.com/code-yeongyu/oh-my-openagent/dev/assets/omo.schema.json";

struct TempFileGuard<'a> {
    path: &'a Path,
    active: bool,
}

impl<'a> TempFileGuard<'a> {
    fn new(path: &'a Path) -> Self {
        Self { path, active: true }
    }

    fn defuse(&mut self) {
        self.active = false;
    }
}

impl<'a> Drop for TempFileGuard<'a> {
    fn drop(&mut self) {
        if self.active {
            let _ = fs::remove_file(self.path);
        }
    }
}

/// Writes bytes to a file atomically via a temporary file in the same directory,
/// creating parent directories as needed and preserving file permissions.
pub fn write_file(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    let parent = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => {
            fs::create_dir_all(parent).map_err(|error| AppError::io("create", parent, error))?;
            parent
        }
        _ => Path::new("."),
    };

    let original_permissions = match fs::metadata(path) {
        Ok(metadata) => Some(metadata.permissions()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(AppError::io("inspect", path, error)),
    };

    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("document");
    let temporary = parent.join(format!(".{filename}.{}.tmp", uuid::Uuid::new_v4()));
    let mut guard = TempFileGuard::new(&temporary);

    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mode = match &original_permissions {
            Some(perms) => {
                use std::os::unix::fs::PermissionsExt;
                perms.mode()
            }
            None => 0o644,
        };
        options.mode(mode);
    }

    let mut file = options
        .open(&temporary)
        .map_err(|error| AppError::io("create", &temporary, error))?;

    file.write_all(bytes)
        .map_err(|error| AppError::io("write", &temporary, error))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = match original_permissions {
            Some(perms) => perms,
            None => fs::Permissions::from_mode(0o644),
        };
        file.set_permissions(perms)
            .map_err(|error| AppError::io("set permissions", &temporary, error))?;
    }
    #[cfg(not(unix))]
    {
        if let Some(perms) = original_permissions {
            file.set_permissions(perms)
                .map_err(|error| AppError::io("set permissions", &temporary, error))?;
        }
    }

    file.sync_all()
        .map_err(|error| AppError::io("sync", &temporary, error))?;
    drop(file);

    fs::rename(&temporary, path).map_err(|error| AppError::io("rename", path, error))?;
    guard.defuse();

    Ok(())
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
        let (content, raw) = patch_with_effective(&self.content, path, value)
            .map_err(|_| AppError::validation("malformed JSONC document"))?;
        self.content = content;
        self.raw = raw;
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn write_file_atomically_creates_and_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("nested").join("test.txt");

        write_file(&file_path, b"initial content").unwrap();
        assert_eq!(
            read_text(&file_path).unwrap(),
            Some("initial content".to_owned())
        );

        write_file(&file_path, b"updated content").unwrap();
        assert_eq!(
            read_text(&file_path).unwrap(),
            Some("updated content".to_owned())
        );
    }

    #[cfg(unix)]
    #[test]
    fn write_file_preserves_existing_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("perm.txt");

        write_file(&file_path, b"test").unwrap();
        fs::set_permissions(&file_path, fs::Permissions::from_mode(0o600)).unwrap();

        write_file(&file_path, b"new content").unwrap();
        let metadata = fs::metadata(&file_path).unwrap();
        assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
    }

    #[test]
    fn jsonc_doc_patch_updates_both_content_and_raw() {
        let input = "{\n  // schema\n  \"name\": \"initial\"\n}";
        let mut doc = JsoncDoc::parse(input).unwrap();
        doc.patch(&["name"], Some(json!("updated"))).unwrap();

        assert_eq!(doc.raw().get("name"), Some(&json!("updated")));
        assert!(doc.content().contains("\"name\": \"updated\""));
        assert!(doc.content().contains("// schema"));
    }

    #[test]
    fn jsonc_doc_patch_rejects_shadowed_duplicate_key() {
        let input = r#"{"name": "first", "name": "second"}"#;
        let mut doc = JsoncDoc::parse(input).unwrap();
        let result = doc.patch(&["name"], Some(json!("third")));
        assert!(result.is_err());
    }
}
