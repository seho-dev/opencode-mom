use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::agents_md::MarkdownAgentDocument;
use crate::document::{JsoncDoc, OPENCODE_SCHEMA};
use crate::error::{AppError, ErrorCode};
use crate::providers::ModelRef;
use crate::resource_file::{self, Directory, Snapshot};

const AGENT_CATALOG_JSON: &str = include_str!("../../shared/agents.catalog.json");

#[derive(Deserialize)]
struct AgentCatalogEntry {
    id: String,
}

#[derive(Deserialize)]
struct AgentCatalog {
    agents: Vec<AgentCatalogEntry>,
}

fn built_in_agent_ids() -> &'static BTreeSet<String> {
    static IDS: std::sync::OnceLock<BTreeSet<String>> = std::sync::OnceLock::new();
    IDS.get_or_init(|| {
        let catalog: AgentCatalog = serde_json::from_str(AGENT_CATALOG_JSON)
            .expect("shared agents.catalog.json must be valid JSON");
        catalog.agents.into_iter().map(|entry| entry.id).collect()
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentStorage {
    Inline,
    GlobalMarkdown,
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
    pub path: PathBuf,
    pub resolved_path: PathBuf,
    pub raw: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownAgentSource {
    pub storage: AgentStorage,
    pub path: PathBuf,
    pub resolved_path: PathBuf,
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
    pub inline_sources: Vec<InlineAgentSource>,
    pub markdown: Vec<MarkdownAgentSource>,
    pub effective: Value,
    pub overrides: Vec<AgentFieldOverride>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMutation {
    #[serde(default)]
    pub fields: Map<String, Value>,
    pub prompt: Option<String>,
    #[serde(default)]
    pub clear_fields: Vec<String>,
    // Bind updates to the resolved target shown with the draft, not a retargetable alias.
    pub expected_path: Option<PathBuf>,
    pub expected_content: Option<String>,
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
    paths.validate_resource_root()?;
    let inline = scan_inline(paths)?;
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

fn scan_inline(
    paths: &crate::paths::ConfigPaths,
) -> Result<BTreeMap<String, Vec<InlineAgentSource>>, AppError> {
    let mut sources = BTreeMap::<String, Vec<InlineAgentSource>>::new();
    for path in paths.agent_config_files() {
        let Some(snapshot) = Snapshot::read(path)? else {
            continue;
        };
        let document = JsoncDoc::parse(&snapshot.content)?;
        for (id, raw) in inline_agents(document.raw())? {
            sources
                .entry(id.clone())
                .or_default()
                .push(InlineAgentSource {
                    path: path.clone(),
                    resolved_path: snapshot.path.clone(),
                    raw: raw.clone(),
                });
        }
    }
    Ok(sources)
}

pub fn get(paths: &crate::paths::ConfigPaths, id: &str) -> Result<AgentDefinition, AppError> {
    validate_agent_id(id)?;
    list(paths)?
        .into_iter()
        .find(|definition| definition.id == id)
        .ok_or_else(|| not_found(id))
}

fn scan_inline_for_id(
    paths: &crate::paths::ConfigPaths,
    id: &str,
) -> Result<Vec<InlineAgentSource>, AppError> {
    let mut sources = Vec::new();
    for path in paths.agent_config_files() {
        let Some(snapshot) = Snapshot::read(path)? else {
            continue;
        };
        let document = JsoncDoc::parse(&snapshot.content)?;
        if let Some(raw) = inline_agents(document.raw())?.get(id) {
            sources.push(InlineAgentSource {
                path: path.clone(),
                resolved_path: snapshot.path.clone(),
                raw: raw.clone(),
            });
        }
    }
    Ok(sources)
}

fn scan_markdown_for_id(
    paths: &crate::paths::ConfigPaths,
    id: &str,
) -> Result<Vec<MarkdownAgentSource>, AppError> {
    let mut sources = Vec::new();
    for root in paths.agent_dirs() {
        if !root.exists() {
            continue;
        }
        let path = root.join(id).with_extension("md");
        match fs::symlink_metadata(&path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(AppError::io("read", &path, error)),
            Ok(_) => {}
        }
        let snapshot = Snapshot::read(&path)?.ok_or_else(agent_changed)?;
        let raw = snapshot.content;
        let document = MarkdownAgentDocument::parse(&raw)?;
        sources.push(MarkdownAgentSource {
            storage: AgentStorage::GlobalMarkdown,
            path,
            resolved_path: snapshot.path,
            raw,
            frontmatter: document.frontmatter,
            prompt: document.prompt,
        });
    }
    Ok(sources)
}

fn read_single(paths: &crate::paths::ConfigPaths, id: &str) -> Result<AgentDefinition, AppError> {
    paths.validate_resource_root()?;
    let inline = scan_inline_for_id(paths, id)?;
    let markdown = scan_markdown_for_id(paths, id)?;
    if inline.is_empty() && markdown.is_empty() {
        return Err(not_found(id));
    }
    definition_from_sources(
        id,
        if inline.is_empty() {
            None
        } else {
            Some(&inline)
        },
        if markdown.is_empty() {
            None
        } else {
            Some(&markdown)
        },
    )
}

/// Scans all agent sources for `model` references used by deletion protection.
pub fn model_references(
    paths: &crate::paths::ConfigPaths,
) -> Result<Vec<crate::refs::ModelReference>, AppError> {
    paths.validate_resource_root()?;
    let inline = scan_inline(paths)?;
    let markdown = scan_markdown(paths)?;
    let mut references = Vec::new();
    for (id, sources) in inline {
        for source in sources {
            if let Some(model) = source
                .raw
                .get("model")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
            {
                references.push(crate::refs::ModelReference {
                    source: crate::refs::ReferenceSource::OpenCodeInlineAgent,
                    location: format!("agents.{id}.model"),
                    model_ref: ModelRef::parse(model).map_err(|_| invalid_model_ref(model))?,
                });
            }
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
    paths.ensure_global_write()?;
    create_inner(paths, request, |_| {})
}

fn create_inner(
    paths: &crate::paths::ConfigPaths,
    request: AgentCreate,
    mut before_publish: impl FnMut(&Path),
) -> Result<AgentDefinition, AppError> {
    validate_agent_id(&request.id)?;
    validate_model(&request.fields)?;
    let (fields, prompt, _) = normalize_mutation(request.fields, request.prompt, Vec::new())?;

    match request.storage {
        AgentStorage::Inline => {
            create_inline(paths, &request.id, fields, prompt, &mut before_publish)?
        }
        AgentStorage::GlobalMarkdown => {
            let path = markdown_path(paths, request.storage, &request.id, false)?;
            if path.exists() {
                return Err(invalid(format!(
                    "agent '{}' already exists at {}",
                    request.id,
                    path.display()
                )));
            }
            let mut document = MarkdownAgentDocument::empty();
            document.apply_fields(&fields, prompt)?;
            let parent = Directory::ensure(path.parent().unwrap())?;
            parent.create_file_checked(
                path.file_name().unwrap().to_str().unwrap(),
                &document.serialize(),
                || {
                    before_publish(&path);
                    if markdown_path(paths, request.storage, &request.id, false)? != path {
                        return Err(agent_changed());
                    }
                    resource_file::absent(&path)
                },
            )?;
        }
    }
    read_single(paths, &request.id)
}

pub fn update(
    paths: &crate::paths::ConfigPaths,
    id: &str,
    storage: AgentStorage,
    mutation: AgentMutation,
) -> Result<AgentDefinition, AppError> {
    paths.ensure_global_write()?;
    update_inner(paths, id, storage, mutation, |_| {})
}

fn update_inner(
    paths: &crate::paths::ConfigPaths,
    id: &str,
    storage: AgentStorage,
    mutation: AgentMutation,
    mut before_replace: impl FnMut(&Path),
) -> Result<AgentDefinition, AppError> {
    validate_agent_id(id)?;
    validate_model(&mutation.fields)?;
    let expected_path = mutation.expected_path.as_deref().ok_or_else(|| {
        invalid("agent update requires an expected source path and content; reload before saving")
    })?;
    let expected_content = mutation.expected_content.as_deref().ok_or_else(|| {
        invalid("agent update requires an expected source path and content; reload before saving")
    })?;
    let (fields, prompt, clear_fields) =
        normalize_mutation(mutation.fields, mutation.prompt, mutation.clear_fields)?;

    match storage {
        AgentStorage::Inline => {
            let path = paths.opencode_file();
            let (snapshot, mut document) = read_inline(&path)?;
            let existing = inline_agents(document.raw())?
                .get(id)
                .ok_or_else(|| not_found(id))?;
            validate_expected(
                snapshot.as_ref().unwrap(),
                &existing.to_string(),
                expected_path,
                expected_content,
            )?;
            update_inline(&mut document, id, fields, prompt, clear_fields)?;
            persist_inline(&path, snapshot, &document, &mut before_replace)?;
        }
        AgentStorage::GlobalMarkdown => {
            let path = markdown_path(paths, storage, id, false)?;
            let snapshot = Snapshot::read(&path)?.ok_or_else(|| not_found(id))?;
            validate_expected(
                &snapshot,
                &snapshot.content,
                expected_path,
                expected_content,
            )?;
            let mut document = MarkdownAgentDocument::parse(&snapshot.content)?;
            document.apply_fields(&fields, prompt)?;
            document.remove_fields(&clear_fields);
            if clear_fields.iter().any(|field| field == "prompt") {
                document.prompt.clear();
            }
            resource_file::replace(&[(snapshot, document.serialize())], |resolved| {
                before_replace(resolved);
                if markdown_path(paths, storage, id, false)? != path {
                    return Err(agent_changed());
                }
                Ok(())
            })?;
        }
    }
    read_single(paths, id)
}

fn agent_changed() -> AppError {
    AppError::configuration("Agent source changed; reload before saving")
}

fn validate_expected(
    snapshot: &Snapshot,
    content: &str,
    expected_path: &Path,
    expected_content: &str,
) -> Result<(), AppError> {
    if snapshot.path != expected_path || content != expected_content {
        return Err(agent_changed());
    }
    Ok(())
}

pub fn delete(
    paths: &crate::paths::ConfigPaths,
    id: &str,
    storage: AgentStorage,
    references: &AgentReferenceIndex,
) -> Result<AgentDeleteResult, AppError> {
    paths.ensure_global_write()?;
    validate_agent_id(id)?;
    if built_in_agent_ids().contains(id) {
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
        AgentStorage::GlobalMarkdown => {
            let path = markdown_path(paths, storage, id, true)?;
            let snapshot = Snapshot::read(&path)?.ok_or_else(|| not_found(id))?;
            let parent = Directory::ensure(path.parent().unwrap())?;
            snapshot.validate()?;
            parent.validate()?;
            let resolved = parent.resolved.join(path.file_name().unwrap());
            fs::remove_file(&resolved).map_err(|error| AppError::io("remove", &path, error))?;
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
    before_publish: impl FnMut(&Path),
) -> Result<(), AppError> {
    let opencode_file = paths.opencode_file();
    let (snapshot, mut document) = read_inline(&opencode_file)?;
    let agents = inline_agents(document.raw())?;
    if agents.contains_key(id) {
        return Err(invalid(format!("inline agent '{id}' already exists")));
    }
    let mut value = fields;
    if let Some(prompt) = prompt {
        value.insert("system".to_owned(), Value::String(prompt));
    }
    document.patch(&["agents", id], Some(Value::Object(value)))?;
    persist_inline(&opencode_file, snapshot, &document, before_publish)
}

fn update_inline(
    document: &mut JsoncDoc,
    id: &str,
    fields: Map<String, Value>,
    prompt: Option<String>,
    clear_fields: Vec<String>,
) -> Result<(), AppError> {
    let agents = inline_agents(document.raw())?;
    let existing = agents.get(id).ok_or_else(|| not_found(id))?;
    if !existing.is_object() {
        return Err(invalid(format!("inline agent '{id}' must be an object")));
    }
    let mut values = fields;
    let prompt_is_set = prompt.is_some();
    if let Some(prompt) = prompt {
        values.insert("system".to_owned(), Value::String(prompt));
    }
    for (key, value) in values {
        document.patch(&["agents", id, &key], Some(value))?;
    }
    for key in clear_fields {
        // The V2 inline prompt field is `system`; `prompt` is the DTO-level name.
        let key = if key == "prompt" {
            "system"
        } else {
            key.as_str()
        };
        if key == "system" && prompt_is_set {
            continue;
        }
        document.patch(&["agents", id, key], None)?;
    }
    Ok(())
}

fn delete_inline(paths: &crate::paths::ConfigPaths, id: &str) -> Result<(), AppError> {
    let opencode_file = paths.opencode_file();
    let (snapshot, mut document) = read_inline(&opencode_file)?;
    if !inline_agents(document.raw())?.contains_key(id) {
        return Err(not_found(id));
    }
    document.patch(&["agents", id], None)?;
    persist_inline(&opencode_file, snapshot, &document, |_| {})
}

fn read_inline(path: &Path) -> Result<(Option<Snapshot>, JsoncDoc), AppError> {
    let snapshot = Snapshot::read(path)?;
    let document = match &snapshot {
        Some(snapshot) => JsoncDoc::parse(&snapshot.content)?,
        None => JsoncDoc::bootstrap(OPENCODE_SCHEMA),
    };
    Ok((snapshot, document))
}

fn persist_inline(
    path: &Path,
    snapshot: Option<Snapshot>,
    document: &JsoncDoc,
    mut before_publish: impl FnMut(&Path),
) -> Result<(), AppError> {
    if let Some(snapshot) = snapshot {
        resource_file::replace(&[(snapshot, document.content().to_owned())], |resolved| {
            before_publish(resolved);
            Ok(())
        })
    } else {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| invalid("invalid config filename"))?;
        let parent = Directory::ensure(path.parent().unwrap())?;
        parent.create_file_checked(name, document.content(), || {
            before_publish(path);
            resource_file::absent(path)
        })
    }
}

fn inline_agents(document: &Map<String, Value>) -> Result<&Map<String, Value>, AppError> {
    match document.get("agents") {
        None => Ok(empty_object()),
        Some(Value::Object(agents)) => Ok(agents),
        Some(_) => Err(invalid("top-level 'agents' must be an object")),
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
    for root in paths.agent_dirs() {
        scan_markdown_root(AgentStorage::GlobalMarkdown, root, &mut results)?;
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
    let mut visited = BTreeSet::new();
    collect_markdown_files(root, &mut files, &mut visited, 0)?;
    files.sort();
    for path in files {
        let relative = path.strip_prefix(root).map_err(|error| {
            AppError::configuration(format!("resolve {}: {error}", path.display()))
        })?;
        let id = markdown_id(relative)?;
        let snapshot = Snapshot::read(&path)?.ok_or_else(agent_changed)?;
        let raw = snapshot.content;
        let document = MarkdownAgentDocument::parse(&raw)?;
        results.entry(id).or_default().push(MarkdownAgentSource {
            storage,
            path,
            resolved_path: snapshot.path,
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
    for_deletion: bool,
) -> Result<PathBuf, AppError> {
    let root = match storage {
        AgentStorage::Inline => {
            return Err(invalid("inline storage does not have a Markdown path"))
        }
        AgentStorage::GlobalMarkdown => paths.global_agents_dir(),
    };
    let mut targets = BTreeMap::new();
    let mut logical_paths = BTreeSet::new();
    for root in paths.agent_dirs() {
        let path = root.join(id).with_extension("md");
        match fs::symlink_metadata(&path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(AppError::io("read", &path, error)),
            Ok(_) => {}
        }
        let canonical =
            fs::canonicalize(&path).map_err(|error| AppError::io("resolve", &path, error))?;
        logical_paths.insert(path.clone());
        targets.insert(canonical, path);
    }
    if targets.len() > 1 || (for_deletion && logical_paths.len() > 1) {
        return Err(invalid(format!(
            "agent '{id}' has multiple Markdown paths; resolve ambiguous aliases before editing"
        )));
    }
    Ok(targets
        .into_values()
        .next()
        .unwrap_or_else(|| root.join(id).with_extension("md")))
}

/// Lists every `.md` file under the given agents roots. Used by model replacement.
pub fn collect_markdown_files_in_roots(
    paths: &crate::paths::ConfigPaths,
) -> Result<Vec<PathBuf>, AppError> {
    let mut files = Vec::new();
    let mut visited = BTreeSet::new();
    paths.validate_resource_root()?;
    for root in paths.agent_dirs() {
        collect_markdown_files(root, &mut files, &mut visited, 0)?;
    }
    files.sort();
    Ok(files)
}

fn definition_from_sources(
    id: &str,
    inline: Option<&Vec<InlineAgentSource>>,
    markdown: Option<&Vec<MarkdownAgentSource>>,
) -> Result<AgentDefinition, AppError> {
    let inline_sources = inline.cloned().unwrap_or_default();
    let mut inline = None;
    for source in &inline_sources {
        if !source.raw.is_object() {
            return Err(invalid(format!("inline agent '{id}' must be an object")));
        }
        let aggregate = inline.get_or_insert_with(|| InlineAgentSource {
            path: source.path.clone(),
            resolved_path: source.resolved_path.clone(),
            raw: Value::Object(Map::new()),
        });
        deep_merge(&mut aggregate.raw, source.raw.clone());
        aggregate.path = source.path.clone();
        aggregate.resolved_path = source.resolved_path.clone();
    }
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
        inline_sources,
        markdown,
        effective,
        overrides,
    })
}

fn markdown_effective_fields(source: &MarkdownAgentSource) -> Value {
    let mut fields = source.frontmatter.clone();
    // The V2 system prompt of a Markdown agent lives in the body.
    fields.insert("system".to_owned(), Value::String(source.prompt.clone()));
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

fn collect_markdown_files(
    root: &Path,
    files: &mut Vec<PathBuf>,
    visited: &mut BTreeSet<PathBuf>,
    depth: usize,
) -> Result<(), AppError> {
    if depth >= 128 || !root.exists() {
        return Ok(());
    }
    let Ok(canonical) = fs::canonicalize(root) else {
        return Ok(());
    };
    if !visited.insert(canonical) {
        return Ok(());
    }
    for entry in fs::read_dir(root).map_err(|error| AppError::io("read", root, error))? {
        let entry = entry.map_err(|error| AppError::io("read", root, error))?;
        let path = entry.path();
        if path.is_dir() {
            collect_markdown_files(&path, files, visited, depth + 1)?;
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
        // V2 selectors: provider/model with an optional #variant suffix.
        let base = model.split('#').next().unwrap_or("");
        let variant = model.split('#').nth(1);
        let variant_ok = variant.is_none() || variant.is_some_and(|value| !value.is_empty());
        variant_ok && model.matches('#').count() <= 1 && ModelRef::parse(base).is_ok()
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

type NormalizedMutation = (Map<String, Value>, Option<String>, Vec<String>);

fn normalize_mutation(
    mut fields: Map<String, Value>,
    prompt: Option<String>,
    clear_fields: Vec<String>,
) -> Result<NormalizedMutation, AppError> {
    let frontmatter_prompt = fields.remove("prompt");
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

#[cfg(all(test, unix))]
#[path = "agents/safety_tests.rs"]
mod safety_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_model_accepts_valid_selectors() {
        let cases = [
            "openrouter/anthropic/claude",
            "openrouter/anthropic/claude#high",
            "openai/gpt-4o",
            "openai/gpt-4o#fast",
        ];
        for model in cases {
            let mut fields = Map::new();
            fields.insert("model".to_owned(), Value::String(model.to_owned()));
            assert!(
                validate_model(&fields).is_ok(),
                "expected '{model}' to be valid"
            );
        }
    }

    #[test]
    fn validate_model_rejects_invalid_selectors() {
        let cases = [
            "openrouter/anthropic/claude#",
            "openrouter/anthropic/claude#high#low",
            "no_provider",
            "provider with space/model",
            "provider/model with space",
            "",
        ];
        for model in cases {
            let mut fields = Map::new();
            fields.insert("model".to_owned(), Value::String(model.to_owned()));
            assert!(
                validate_model(&fields).is_err(),
                "expected '{model}' to be invalid"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn collect_markdown_files_handles_directory_cycles() {
        let temp_dir =
            std::env::temp_dir().join(format!("opencode-mom-dir-cycle-{}", uuid::Uuid::new_v4()));
        let sub = temp_dir.join("sub");
        fs::create_dir_all(&sub).unwrap();
        std::os::unix::fs::symlink(&temp_dir, sub.join("loop")).unwrap();
        fs::write(sub.join("test.md"), "---\nmodel: openai/gpt-4o\n---\nbody").unwrap();

        let mut files = Vec::new();
        let mut visited = BTreeSet::new();
        let result = collect_markdown_files(&temp_dir, &mut files, &mut visited, 0);
        let _ = fs::remove_dir_all(&temp_dir);

        assert!(result.is_ok());
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn create_and_read_single_agent_avoids_cascading_failure_from_broken_markdown() {
        let temp_home =
            std::env::temp_dir().join(format!("opencode-mom-single-read-{}", uuid::Uuid::new_v4()));
        let paths = crate::paths::ConfigPaths::for_home(&temp_home);
        fs::create_dir_all(paths.global_agents_dir()).unwrap();
        fs::create_dir_all(paths.opencode_file().parent().unwrap()).unwrap();

        // Write a broken markdown agent in the agents dir
        fs::write(
            paths.global_agents_dir().join("broken.md"),
            "---\n: invalid yaml frontmatter\n---\nbody",
        )
        .unwrap();

        // list should fail due to broken.md
        assert!(
            list(&paths).is_err(),
            "list should fail due to broken markdown"
        );

        // create a new inline agent should succeed and not cascade into broken.md
        let request = AgentCreate {
            id: "good-agent".to_owned(),
            storage: AgentStorage::Inline,
            fields: serde_json::from_value(serde_json::json!({"model": "openai/gpt-4o"})).unwrap(),
            prompt: Some("hello".to_owned()),
        };
        let created = create(&paths, request);
        let _ = fs::remove_dir_all(&temp_home);

        assert!(
            created.is_ok(),
            "create should succeed despite unrelated broken markdown: {:?}",
            created.err()
        );
        assert_eq!(created.unwrap().id, "good-agent");
    }
}
