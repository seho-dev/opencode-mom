use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::agents_md::MarkdownAgentDocument;
use crate::document::{read_text, write_file, JsoncDoc, OPENCODE_SCHEMA};
use crate::error::{AppError, ErrorCode};
use crate::providers::ModelRef;

const BUILT_IN_AGENT_IDS: &[&str] = &["build", "plan", "general", "explore"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentStorage {
    Inline,
    GlobalMarkdown,
    ProjectMarkdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentSource {
    Inline,
    Markdown,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentReferenceKind {
    DefaultAgent,
    Command,
    PermissionTask,
    Group,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentReference {
    pub kind: AgentReferenceKind,
    pub owner: String,
}

pub type AgentReferenceIndex = BTreeMap<String, Vec<AgentReference>>;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InlineAgentSource {
    pub raw: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownAgentSource {
    pub storage: AgentStorage,
    pub path: PathBuf,
    pub raw: String,
    pub frontmatter: Map<String, Value>,
    pub prompt: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentFieldOverride {
    pub field: String,
    pub inline: Value,
    pub markdown: Value,
    pub effective: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDefinition {
    pub id: String,
    pub source: AgentSource,
    pub inline: Option<InlineAgentSource>,
    pub markdown: Vec<MarkdownAgentSource>,
    pub effective: Value,
    pub overrides: Vec<AgentFieldOverride>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMutation {
    #[serde(default)]
    pub fields: Map<String, Value>,
    pub prompt: Option<String>,
    #[serde(default)]
    pub clear_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCreate {
    pub id: String,
    pub storage: AgentStorage,
    #[serde(default)]
    pub fields: Map<String, Value>,
    pub prompt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDeleteResult {
    pub id: String,
    pub deleted_storage: AgentStorage,
    pub replacement: Option<AgentDefinition>,
    pub another_source_takes_over: bool,
}

fn invalid(detail: impl Into<String>) -> AppError {
    AppError::validation(detail)
}

fn not_found(id: &str) -> AppError {
    AppError::not_found(format!("agent '{id}' was not found"))
}

/// Lists every agent definition merged from inline (OpenCode config) and Markdown sources.
pub fn list(paths: &crate::paths::ConfigPaths) -> Result<Vec<AgentDefinition>, AppError> {
    let document = JsoncDoc::read(&paths.opencode_file(), OPENCODE_SCHEMA)?;
    let inline = inline_agents(document.raw())?;
    let markdown = scan_markdown(paths)?;
    let ids = inline
        .keys()
        .chain(markdown.keys())
        .cloned()
        .collect::<BTreeSet<_>>();

    ids.into_iter()
        .map(|id| definition_from_sources(&id, inline.get(&id), markdown.get(&id)))
        .collect()
}

pub fn get(paths: &crate::paths::ConfigPaths, id: &str) -> Result<AgentDefinition, AppError> {
    validate_agent_id(id)?;
    list(paths)?
        .into_iter()
        .find(|definition| definition.id == id)
        .ok_or_else(|| not_found(id))
}

/// Scans all agent sources for `model` references used by deletion protection.
pub fn model_references(
    paths: &crate::paths::ConfigPaths,
) -> Result<Vec<crate::refs::ModelReference>, AppError> {
    let document = JsoncDoc::read(&paths.opencode_file(), OPENCODE_SCHEMA)?;
    let inline = inline_agents(document.raw())?;
    let markdown = scan_markdown(paths)?;
    let mut references = Vec::new();
    for (id, raw) in inline {
        if let Some(model) = raw
            .get("model")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
        {
            references.push(crate::refs::ModelReference {
                source: crate::refs::ReferenceSource::OpenCodeInlineAgent,
                location: format!("agent.{id}.model"),
                model_ref: ModelRef::parse(model).map_err(|_| invalid_model_ref(model))?,
            });
        }
    }
    for (id, sources) in markdown {
        for source in sources {
            if let Some(model) = source
                .frontmatter
                .get("model")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
            {
                references.push(crate::refs::ModelReference {
                    source: crate::refs::ReferenceSource::MarkdownAgent,
                    location: format!("{}:{id}.model", source.path.display()),
                    model_ref: ModelRef::parse(model).map_err(|_| invalid_model_ref(model))?,
                });
            }
        }
    }
    references.sort();
    Ok(references)
}

pub fn create(
    paths: &crate::paths::ConfigPaths,
    request: AgentCreate,
) -> Result<AgentDefinition, AppError> {
    validate_agent_id(&request.id)?;
    validate_model(&request.fields)?;
    let (fields, prompt, _) = normalize_mutation(&request.fields, request.prompt, Vec::new())?;

    match request.storage {
        AgentStorage::Inline => create_inline(paths, &request.id, fields, prompt)?,
        AgentStorage::GlobalMarkdown | AgentStorage::ProjectMarkdown => {
            let path = markdown_path(paths, request.storage, &request.id)?;
            if path.exists() {
                return Err(invalid(format!(
                    "agent '{}' already exists at {}",
                    request.id,
                    path.display()
                )));
            }
            let mut document = MarkdownAgentDocument::empty();
            document.apply_fields(&fields, prompt, false)?;
            write_file(&path, document.serialize().as_bytes())?;
        }
    }
    get(paths, &request.id)
}

pub fn update(
    paths: &crate::paths::ConfigPaths,
    id: &str,
    storage: AgentStorage,
    mutation: AgentMutation,
) -> Result<AgentDefinition, AppError> {
    validate_agent_id(id)?;
    validate_model(&mutation.fields)?;
    let (fields, prompt, clear_fields) =
        normalize_mutation(&mutation.fields, mutation.prompt, mutation.clear_fields)?;

    match storage {
        AgentStorage::Inline => update_inline(paths, id, fields, prompt, clear_fields)?,
        AgentStorage::GlobalMarkdown | AgentStorage::ProjectMarkdown => {
            let path = markdown_path(paths, storage, id)?;
            if !path.exists() {
                return Err(not_found(id));
            }
            let content =
                read_text(&path)?.ok_or_else(|| AppError::io("read", &path, fs_not_found()))?;
            let mut document = MarkdownAgentDocument::parse(&content)?;
            document.apply_fields(&fields, prompt, false)?;
            document.remove_fields(&clear_fields);
            if clear_fields.iter().any(|field| field == "prompt") {
                document.prompt.clear();
            }
            write_file(&path, document.serialize().as_bytes())?;
        }
    }
    get(paths, id)
}

fn fs_not_found() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "file disappeared while editing",
    )
}

pub fn delete(
    paths: &crate::paths::ConfigPaths,
    id: &str,
    storage: AgentStorage,
    references: &AgentReferenceIndex,
) -> Result<AgentDeleteResult, AppError> {
    validate_agent_id(id)?;
    if BUILT_IN_AGENT_IDS.contains(&id) {
        return Err(AppError::references(
            format!(
                "built-in agent '{id}' cannot be physically deleted; use an overlay or disable it"
            ),
            "built-in agents are protected",
        ));
    }
    if let Some(agent_references) = references.get(id) {
        if !agent_references.is_empty() {
            let locations = agent_references
                .iter()
                .map(|reference| format!("{:?} at {}", reference.kind, reference.owner))
                .collect::<Vec<_>>()
                .join(", ");
            return Err(AppError::references(
                format!("agent '{id}' is still referenced"),
                locations,
            ));
        }
    }

    match storage {
        AgentStorage::Inline => delete_inline(paths, id)?,
        AgentStorage::GlobalMarkdown | AgentStorage::ProjectMarkdown => {
            let path = markdown_path(paths, storage, id)?;
            if !path.exists() {
                return Err(not_found(id));
            }
            fs::remove_file(&path).map_err(|error| AppError::io("remove", &path, error))?;
        }
    }

    let replacement = match get(paths, id) {
        Ok(definition) => Some(definition),
        Err(error) if error.code == ErrorCode::NotFound => None,
        Err(error) => return Err(error),
    };
    Ok(AgentDeleteResult {
        id: id.to_owned(),
        deleted_storage: storage,
        another_source_takes_over: replacement.is_some(),
        replacement,
    })
}

fn create_inline(
    paths: &crate::paths::ConfigPaths,
    id: &str,
    fields: Map<String, Value>,
    prompt: Option<String>,
) -> Result<(), AppError> {
    let opencode_file = paths.opencode_file();
    let mut document = JsoncDoc::read(&opencode_file, OPENCODE_SCHEMA)?;
    let agents = inline_agents(document.raw())?;
    if agents.contains_key(id) {
        return Err(invalid(format!("inline agent '{id}' already exists")));
    }
    let mut value = fields;
    value.remove("tools");
    if let Some(prompt) = prompt {
        value.insert("prompt".to_owned(), Value::String(prompt));
    }
    document.patch(&["agent", id], Some(Value::Object(value)))?;
    document.save(&opencode_file)
}

fn update_inline(
    paths: &crate::paths::ConfigPaths,
    id: &str,
    fields: Map<String, Value>,
    prompt: Option<String>,
    clear_fields: Vec<String>,
) -> Result<(), AppError> {
    let opencode_file = paths.opencode_file();
    let mut document = JsoncDoc::read(&opencode_file, OPENCODE_SCHEMA)?;
    let agents = inline_agents(document.raw())?;
    let existing = agents.get(id).ok_or_else(|| not_found(id))?;
    if !existing.is_object() {
        return Err(invalid(format!("inline agent '{id}' must be an object")));
    }
    let mut values = fields;
    let prompt_is_set = prompt.is_some();
    if let Some(prompt) = prompt {
        values.insert("prompt".to_owned(), Value::String(prompt));
    }
    values.remove("tools");
    for (key, value) in values {
        document.patch(&["agent", id, &key], Some(value))?;
    }
    for key in clear_fields {
        if key == "prompt" && prompt_is_set {
            continue;
        }
        document.patch(&["agent", id, &key], None)?;
    }
    document.save(&opencode_file)
}

fn delete_inline(paths: &crate::paths::ConfigPaths, id: &str) -> Result<(), AppError> {
    let opencode_file = paths.opencode_file();
    let mut document = JsoncDoc::read(&opencode_file, OPENCODE_SCHEMA)?;
    if !inline_agents(document.raw())?.contains_key(id) {
        return Err(not_found(id));
    }
    document.patch(&["agent", id], None)?;
    document.save(&opencode_file)
}

fn inline_agents(document: &Map<String, Value>) -> Result<&Map<String, Value>, AppError> {
    match document.get("agent") {
        None => Ok(empty_object()),
        Some(Value::Object(agents)) => Ok(agents),
        Some(_) => Err(invalid("top-level 'agent' must be an object")),
    }
}

fn empty_object() -> &'static Map<String, Value> {
    static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
    EMPTY.get_or_init(Map::new)
}

fn scan_markdown(
    paths: &crate::paths::ConfigPaths,
) -> Result<BTreeMap<String, Vec<MarkdownAgentSource>>, AppError> {
    let mut results = BTreeMap::<String, Vec<MarkdownAgentSource>>::new();
    scan_markdown_root(
        AgentStorage::GlobalMarkdown,
        &paths.global_agents_dir(),
        &mut results,
    )?;
    if let Some(project_agents_dir) = paths.project_agents_dir() {
        scan_markdown_root(
            AgentStorage::ProjectMarkdown,
            &project_agents_dir,
            &mut results,
        )?;
    }
    Ok(results)
}

fn scan_markdown_root(
    storage: AgentStorage,
    root: &Path,
    results: &mut BTreeMap<String, Vec<MarkdownAgentSource>>,
) -> Result<(), AppError> {
    if !root.exists() {
        return Ok(());
    }
    let mut files = Vec::new();
    collect_markdown_files(root, &mut files)?;
    files.sort();
    for path in files {
        let relative = path.strip_prefix(root).map_err(|error| {
            AppError::configuration(format!("resolve {}: {error}", path.display()))
        })?;
        let id = markdown_id(relative)?;
        let raw = read_text(&path)?.unwrap_or_default();
        let document = MarkdownAgentDocument::parse(&raw)?;
        results.entry(id).or_default().push(MarkdownAgentSource {
            storage,
            path,
            raw,
            frontmatter: document.frontmatter,
            prompt: document.prompt,
        });
    }
    Ok(())
}

fn markdown_path(
    paths: &crate::paths::ConfigPaths,
    storage: AgentStorage,
    id: &str,
) -> Result<PathBuf, AppError> {
    let root = match storage {
        AgentStorage::Inline => {
            return Err(invalid("inline storage does not have a Markdown path"))
        }
        AgentStorage::GlobalMarkdown => paths.global_agents_dir(),
        AgentStorage::ProjectMarkdown => paths
            .project_agents_dir()
            .ok_or_else(|| invalid("project Markdown agents directory was not configured"))?,
    };
    Ok(root.join(id).with_extension("md"))
}

/// Lists every `.md` file under the given agents roots. Used by model replacement.
pub fn collect_markdown_files_in_roots(
    paths: &crate::paths::ConfigPaths,
) -> Result<Vec<PathBuf>, AppError> {
    let mut files = Vec::new();
    collect_markdown_files(&paths.global_agents_dir(), &mut files)?;
    if let Some(project_agents_dir) = paths.project_agents_dir() {
        collect_markdown_files(&project_agents_dir, &mut files)?;
    }
    files.sort();
    Ok(files)
}

fn definition_from_sources(
    id: &str,
    inline: Option<&Value>,
    markdown: Option<&Vec<MarkdownAgentSource>>,
) -> Result<AgentDefinition, AppError> {
    let inline = inline.cloned().map(|raw| InlineAgentSource { raw });
    let markdown = markdown.cloned().unwrap_or_default();
    let source = match (inline.is_some(), markdown.is_empty()) {
        (true, true) => AgentSource::Inline,
        (false, false) => AgentSource::Markdown,
        (true, false) => AgentSource::Both,
        (false, true) => return Err(not_found(id)),
    };
    let mut effective = inline
        .as_ref()
        .map(|source| source.raw.clone())
        .unwrap_or_else(|| Value::Object(Map::new()));
    if !effective.is_object() {
        return Err(invalid(format!("inline agent '{id}' must be an object")));
    }
    let mut overrides = Vec::new();
    for markdown_source in &markdown {
        let fields = markdown_effective_fields(markdown_source);
        if let (Some(inline), Value::Object(markdown)) = (inline.as_ref(), &fields) {
            let Value::Object(inline) = &inline.raw else {
                unreachable!("checked above")
            };
            for (field, markdown_value) in markdown {
                if let Some(inline_value) = inline.get(field) {
                    overrides.push(AgentFieldOverride {
                        field: field.clone(),
                        inline: inline_value.clone(),
                        markdown: markdown_value.clone(),
                        effective: markdown_value.clone(),
                    });
                }
            }
        }
        deep_merge(&mut effective, fields);
    }
    Ok(AgentDefinition {
        id: id.to_owned(),
        source,
        inline,
        markdown,
        effective,
        overrides,
    })
}

fn markdown_effective_fields(source: &MarkdownAgentSource) -> Value {
    let mut fields = source.frontmatter.clone();
    fields.insert("prompt".to_owned(), Value::String(source.prompt.clone()));
    Value::Object(fields)
}

fn deep_merge(destination: &mut Value, overlay: Value) {
    match (destination, overlay) {
        (Value::Object(destination), Value::Object(overlay)) => {
            for (key, value) in overlay {
                match destination.get_mut(&key) {
                    Some(existing) => deep_merge(existing, value),
                    None => {
                        destination.insert(key, value);
                    }
                }
            }
        }
        (destination, overlay) => *destination = overlay,
    }
}

fn collect_markdown_files(root: &Path, files: &mut Vec<PathBuf>) -> Result<(), AppError> {
    if !root.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(root).map_err(|error| AppError::io("read", root, error))? {
        let entry = entry.map_err(|error| AppError::io("read", root, error))?;
        let path = entry.path();
        if path.is_dir() {
            collect_markdown_files(&path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "md") {
            files.push(path);
        }
    }
    Ok(())
}

fn markdown_id(relative: &Path) -> Result<String, AppError> {
    if relative.extension().is_none()
        || relative
            .extension()
            .is_some_and(|extension| extension != "md")
    {
        return Err(invalid(format!(
            "Markdown agent path '{}' does not end with .md",
            relative.display()
        )));
    }
    let without_extension = relative.with_extension("");
    let components = without_extension
        .components()
        .map(|component| match component {
            Component::Normal(value) => value.to_string_lossy().into_owned(),
            _ => String::new(),
        })
        .collect::<Vec<_>>();
    if components.is_empty() || components.iter().any(String::is_empty) {
        return Err(invalid(format!(
            "invalid Markdown agent path '{}'",
            relative.display()
        )));
    }
    Ok(components.join("/"))
}

fn validate_agent_id(id: &str) -> Result<(), AppError> {
    if id.is_empty()
        || id.starts_with('/')
        || id.ends_with('/')
        || id.contains('\\')
        || id.split('/').any(|segment| {
            segment.is_empty()
                || matches!(segment, "." | "..")
                || !segment.chars().all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
                })
        })
    {
        return Err(invalid(format!("invalid agent id '{id}'")));
    }
    Ok(())
}

fn validate_model(fields: &Map<String, Value>) -> Result<(), AppError> {
    let Some(model) = fields.get("model") else {
        return Ok(());
    };
    let valid = model.as_str().is_some_and(|model| {
        model.is_empty()
            || (model.matches('/').count() == 1
                && model.split_once('/').is_some_and(|(provider, name)| {
                    !provider.is_empty()
                        && !name.is_empty()
                        && !provider.contains(char::is_whitespace)
                        && !name.contains(char::is_whitespace)
                }))
    });
    if valid {
        Ok(())
    } else {
        Err(invalid(
            "agent model must be empty or a complete provider/model reference",
        ))
    }
}

fn invalid_model_ref(model: &str) -> AppError {
    invalid(format!(
        "agent model must be a complete provider/model reference: {model}"
    ))
}

fn normalize_mutation(
    fields: &Map<String, Value>,
    prompt: Option<String>,
    clear_fields: Vec<String>,
) -> Result<(Map<String, Value>, Option<String>, Vec<String>), AppError> {
    let mut fields = fields.clone();
    let frontmatter_prompt = fields.remove("prompt");
    fields.remove("tools");
    let prompt = match (prompt, frontmatter_prompt) {
        (Some(prompt), _) => Some(prompt),
        (None, Some(Value::String(prompt))) => Some(prompt),
        (None, Some(_)) => {
            return Err(invalid("agent prompt must be a string"));
        }
        (None, None) => None,
    };
    let clear_fields = clear_fields
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if let Some(field) = clear_fields.iter().find(|field| field.trim().is_empty()) {
        return Err(invalid(format!(
            "agent field to clear must not be empty: '{field}'"
        )));
    }
    if let Some(field) = clear_fields
        .iter()
        .find(|field| fields.contains_key(*field))
    {
        return Err(invalid(format!(
            "agent field '{field}' cannot be set and cleared together"
        )));
    }
    Ok((fields, prompt, clear_fields))
}
