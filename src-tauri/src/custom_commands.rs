use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::agents_md::MarkdownAgentDocument;
use crate::error::AppError;
use crate::paths::ConfigPaths;
use crate::resource_file::{self, Directory, Snapshot};

const MAX_CONTENT_BYTES: usize = 2 * 1024 * 1024;
const MAX_SCAN_DEPTH: usize = 64;
// Reserve .md and resource_file's .<filename>.<32 hex>.tmp/.bak overhead within NAME_MAX.
const MAX_BASENAME_BYTES: usize = 255 - 3 - 38;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlashCommandEntry {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subagent: Option<bool>,
    pub source_path: PathBuf,
    pub resolved_path: PathBuf,
    pub content: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SlashCommandList {
    pub data: Vec<SlashCommandEntry>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlashCommandDraft {
    pub name: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlashCommandUpdate {
    pub id: String,
    pub content: String,
    pub expected_path: PathBuf,
    pub expected_content: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlashCommandDelete {
    pub id: String,
    pub expected_path: PathBuf,
    pub expected_content: String,
}

pub fn list(paths: &ConfigPaths) -> Result<SlashCommandList, AppError> {
    discover(paths, None)
}

fn discover(paths: &ConfigPaths, only: Option<&str>) -> Result<SlashCommandList, AppError> {
    paths.validate_resource_root()?;
    let mut result = SlashCommandList::default();
    for root in paths.command_dirs() {
        let mut files = Vec::new();
        let mut visited = BTreeSet::new();
        let mut remaining = 10_000;
        collect(
            root,
            &mut files,
            &mut visited,
            0,
            &mut remaining,
            &mut result.diagnostics,
            paths.project_directory(),
        );
        files.sort();
        for path in files {
            if only.is_some_and(|id| source_id(&path) != id) {
                continue;
            }
            match read_entry(paths, &path) {
                Ok(entry) => result.data.push(entry),
                Err(error) => result
                    .diagnostics
                    .push(format!("{}: {error}", path.display())),
            }
        }
    }
    Ok(result)
}

pub fn get(paths: &ConfigPaths, id: &str) -> Result<SlashCommandEntry, AppError> {
    discover(paths, Some(id))?
        .data
        .into_iter()
        .find(|entry| entry.id == id)
        .ok_or_else(|| not_found(id))
}

pub fn create(
    paths: &ConfigPaths,
    draft: SlashCommandDraft,
) -> Result<SlashCommandEntry, AppError> {
    create_inner(paths, draft, |_| Ok(()))
}

fn create_inner(
    paths: &ConfigPaths,
    draft: SlashCommandDraft,
    before_publish: impl FnOnce(&Path) -> Result<(), AppError>,
) -> Result<SlashCommandEntry, AppError> {
    validate_name(&draft.name)?;
    validate_content(&draft.content)?;
    let path = paths
        .commands_write_dir()
        .join(format!("{}.md", draft.name));
    paths.validate_managed_write(&path)?;
    resource_file::absent(&path)?;
    let parent = Directory::ensure(path.parent().ok_or_else(changed)?)?;
    paths.validate_managed_write(&path)?;
    parent.create_file_checked(filename(&path)?, &draft.content, || {
        before_publish(&path)?;
        paths.validate_managed_write(&path)?;
        resource_file::absent(&path)
    })?;
    read_entry(paths, &path)
}

pub fn update(
    paths: &ConfigPaths,
    draft: SlashCommandUpdate,
) -> Result<SlashCommandEntry, AppError> {
    update_inner(paths, draft, |_| Ok(()))
}

fn update_inner(
    paths: &ConfigPaths,
    draft: SlashCommandUpdate,
    mut before_replace: impl FnMut(&Path) -> Result<(), AppError>,
) -> Result<SlashCommandEntry, AppError> {
    let path = get(paths, &draft.id)?.source_path;
    paths.validate_managed_write(&path)?;
    validate_content(&draft.content)?;
    let snapshot = Snapshot::read(&path)?.ok_or_else(|| not_found(&draft.id))?;
    validate_expected(&snapshot, &draft.expected_path, &draft.expected_content)?;
    resource_file::replace_checked(&[(snapshot, draft.content)], |_| {
        before_replace(&path)?;
        paths.validate_managed_write(&path)
    })?;
    read_entry(paths, &path)
}

pub fn delete(paths: &ConfigPaths, draft: SlashCommandDelete) -> Result<(), AppError> {
    let path = get(paths, &draft.id)?.source_path;
    paths.validate_managed_write(&path)?;
    let snapshot = Snapshot::read(&path)?.ok_or_else(|| not_found(&draft.id))?;
    validate_expected(&snapshot, &draft.expected_path, &draft.expected_content)?;
    resource_file::delete_checked(&path, &snapshot, |_| paths.validate_managed_write(&path))
}

fn read_entry(paths: &ConfigPaths, path: &Path) -> Result<SlashCommandEntry, AppError> {
    let name = command_name(paths, path)?;
    if paths.is_project() {
        paths.validate_managed_write(path)?;
    } else {
        paths.validate_resource_root()?;
    }
    if fs::metadata(path).is_ok_and(|metadata| metadata.len() > MAX_CONTENT_BYTES as u64) {
        return Err(AppError::validation("Slash-command content exceeds 2 MiB"));
    }
    let snapshot = Snapshot::read(path)?.ok_or_else(|| not_found(&path.display().to_string()))?;
    if path.to_str().is_none() || snapshot.path.to_str().is_none() {
        return Err(AppError::validation("Command paths must be valid UTF-8"));
    }
    if paths.is_project() {
        paths.validate_managed_write(path)?;
        if !paths
            .project_directory()
            .is_some_and(|root| snapshot.path.starts_with(root))
        {
            return Err(changed());
        }
    }
    let document = validate_content(&snapshot.content)?;
    Ok(SlashCommandEntry {
        id: source_id(path),
        name,
        source_path: path.to_owned(),
        resolved_path: snapshot.path,
        content: snapshot.content,
        description: document
            .frontmatter
            .get("description")
            .and_then(|value| value.as_str())
            .map(str::to_owned),
        agent: document
            .frontmatter
            .get("agent")
            .and_then(|value| value.as_str())
            .map(str::to_owned),
        model: document
            .frontmatter
            .get("model")
            .and_then(|value| value.as_str())
            .map(str::to_owned),
        subagent: document
            .frontmatter
            .get("subagent")
            .and_then(|value| value.as_bool()),
    })
}

fn command_name(paths: &ConfigPaths, path: &Path) -> Result<String, AppError> {
    let relative = paths
        .command_dirs()
        .iter()
        .find_map(|root| path.strip_prefix(root).ok())
        .ok_or_else(|| AppError::validation("Command path is outside the selected scope"))?;
    let name = relative
        .to_str()
        .and_then(|name| name.strip_suffix(".md"))
        .ok_or_else(|| AppError::validation("Command source must be a UTF-8 .md file"))?
        .replace(std::path::MAIN_SEPARATOR, "/");
    validate_name(&name)?;
    Ok(name)
}

fn source_id(path: &Path) -> String {
    format!("{:x}", Sha256::digest(path.as_os_str().as_encoded_bytes()))
}

fn validate_name(name: &str) -> Result<(), AppError> {
    if name.is_empty()
        || name.len() > 1024
        || name.contains('\\')
        || name.split('/').count() > MAX_SCAN_DEPTH
        || name
            .rsplit('/')
            .next()
            .is_some_and(|segment| segment.len() > MAX_BASENAME_BYTES)
        || name.split('/').any(|segment| {
            segment.is_empty()
                || segment == "."
                || segment == ".."
                || segment.len() > 240
                || segment.ends_with(['.', ' '])
                || segment.chars().any(|character| {
                    character.is_control()
                        || matches!(character, ':' | '<' | '>' | '"' | '|' | '?' | '*')
                })
        })
    {
        return Err(AppError::validation("Invalid slash-command name"));
    }
    Ok(())
}

fn validate_content(content: &str) -> Result<MarkdownAgentDocument, AppError> {
    if content.len() > MAX_CONTENT_BYTES || content.contains('\0') {
        return Err(AppError::validation(
            "Slash-command content must be at most 2 MiB without NUL",
        ));
    }
    let input = content.strip_prefix('\u{feff}').unwrap_or(content);
    if let Some(frontmatter) = input
        .strip_prefix("---\n")
        .or_else(|| input.strip_prefix("---\r\n"))
    {
        if !frontmatter
            .lines()
            .any(|line| matches!(line, "---" | "..."))
        {
            return Err(AppError::validation("Unclosed slash-command frontmatter"));
        }
    }
    let document = MarkdownAgentDocument::parse(content)?;
    if document.prompt.trim().is_empty() {
        return Err(AppError::validation(
            "Slash-command template must not be empty",
        ));
    }
    for key in ["description", "agent", "model"] {
        if document
            .frontmatter
            .get(key)
            .is_some_and(|value| !value.is_string())
        {
            return Err(AppError::validation(format!(
                "Slash-command {key} must be a string"
            )));
        }
    }
    if document
        .frontmatter
        .get("subagent")
        .is_some_and(|value| !value.is_boolean())
    {
        return Err(AppError::validation(
            "Slash-command subagent must be a boolean",
        ));
    }
    Ok(document)
}

fn filename(path: &Path) -> Result<&str, AppError> {
    path.file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| AppError::validation("Invalid command filename"))
}

fn validate_expected(snapshot: &Snapshot, path: &Path, content: &str) -> Result<(), AppError> {
    if snapshot.path != path || snapshot.content != content {
        return Err(changed());
    }
    Ok(())
}

fn changed() -> AppError {
    AppError::configuration("Command source changed; reload before saving or deleting")
}

fn not_found(id: &str) -> AppError {
    AppError::not_found(format!("Slash command not found: {id}"))
}

fn collect(
    root: &Path,
    files: &mut Vec<PathBuf>,
    visited: &mut BTreeSet<PathBuf>,
    depth: usize,
    remaining: &mut usize,
    diagnostics: &mut Vec<String>,
    boundary: Option<&Path>,
) {
    // ponytail: depth/entry caps bound local scans; use paged traversal if catalogs exceed these limits.
    if depth >= MAX_SCAN_DEPTH || *remaining == 0 {
        diagnostics.push(format!("{}: command scan limit reached", root.display()));
        return;
    }
    let canonical = match fs::canonicalize(root) {
        Ok(path) => path,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(error) => {
            diagnostics.push(format!("{}: {error}", root.display()));
            return;
        }
    };
    if boundary.is_some_and(|boundary| !canonical.starts_with(boundary)) {
        diagnostics.push(format!(
            "{}: command directory escapes the project",
            root.display()
        ));
        return;
    }
    if !visited.insert(canonical) {
        return;
    }
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) => {
            diagnostics.push(format!("{}: {error}", root.display()));
            return;
        }
    };
    for entry in entries {
        if *remaining == 0 {
            diagnostics.push(format!("{}: command scan limit reached", root.display()));
            break;
        }
        *remaining -= 1;
        match entry {
            Ok(entry) => {
                let path = entry.path();
                if path.is_dir() {
                    collect(
                        &path,
                        files,
                        visited,
                        depth + 1,
                        remaining,
                        diagnostics,
                        boundary,
                    );
                } else if path.extension().is_some_and(|extension| extension == "md") {
                    files.push(path);
                }
            }
            Err(error) => diagnostics.push(format!("{}: {error}", root.display())),
        }
    }
}

#[cfg(test)]
mod tests;
