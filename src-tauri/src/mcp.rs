use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::document::JsoncDoc;
use crate::error::AppError;
use crate::paths::ConfigPaths;
use crate::resource_file::{self, Directory, Snapshot};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServer {
    pub name: String,
    pub r#type: String,
    pub disabled: bool,
    pub target: String,
    pub source_path: String,
    pub source_paths: Vec<String>,
    pub source_snapshot: String,
    pub config: Map<String, Value>,
}

#[derive(Debug, Default, Serialize)]
pub struct McpList {
    pub data: Vec<McpServer>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct McpDraft {
    pub name: String,
    pub config: Map<String, Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpUpdate {
    pub name: String,
    pub config: Map<String, Value>,
    pub expected_config: Map<String, Value>,
    pub expected_source_path: String,
    pub expected_source_snapshot: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct McpDisabledUpdate {
    pub name: String,
    pub disabled: bool,
    pub expected_disabled: bool,
    pub expected_source_path: String,
    pub expected_source_snapshot: String,
}

fn source_snapshot<'a>(
    paths: &ConfigPaths,
    sources: impl IntoIterator<Item = (&'a Path, Option<(&'a Path, &'a str)>)>,
) -> String {
    let mut hash = Sha256::new();
    let mut part = |bytes: &[u8]| {
        hash.update((bytes.len() as u64).to_le_bytes());
        hash.update(bytes);
    };
    part(b"opencode-mom-mcp-source-v1");
    part(if paths.is_project() {
        b"project"
    } else {
        b"global"
    });
    part(
        paths
            .project_directory()
            .map(|root| root.as_os_str().as_encoded_bytes())
            .unwrap_or_default(),
    );
    for (logical, source) in sources {
        part(logical.as_os_str().as_encoded_bytes());
        match source {
            Some((resolved, content)) => {
                part(b"present");
                part(resolved.as_os_str().as_encoded_bytes());
                part(content.as_bytes());
            }
            None => part(b"missing"),
        }
    }

    format!("{:x}", hash.finalize())
}

fn servers(document: &JsoncDoc) -> Result<Option<&Map<String, Value>>, AppError> {
    let Some(mcp) = document.raw().get("mcp") else {
        return Ok(None);
    };
    let mcp = mcp
        .as_object()
        .ok_or_else(|| AppError::validation("mcp must be an object"))?;
    mcp.get("servers")
        .map(|servers| {
            servers
                .as_object()
                .ok_or_else(|| AppError::validation("mcp.servers must be an object"))
        })
        .transpose()
}

fn validate_effective_server(
    document: &JsoncDoc,
    name: &str,
    expected: Option<&Map<String, Value>>,
) -> Result<(), AppError> {
    let effective = servers(document)?.and_then(|definitions| definitions.get(name));
    let mismatch = match expected {
        Some(config) => effective.and_then(Value::as_object) != Some(config),
        None => effective.is_some(),
    };
    if mismatch {
        return Err(AppError::validation(
            "MCP patch did not produce the requested effective configuration; check for duplicate keys",
        ));
    }

    Ok(())
}

pub fn list(paths: &ConfigPaths) -> Result<McpList, AppError> {
    paths.validate_resource_root()?;
    let mut result = McpList::default();
    let mut entries = BTreeMap::<String, (Value, Vec<String>)>::new();
    let mut sources = Vec::new();
    let mut complete = true;
    for path in paths.global_config_files() {
        let snapshot = match Snapshot::read(path) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                complete = false;
                result
                    .diagnostics
                    .push(format!("{}: {error}", path.display()));
                continue;
            }
        };
        sources.push((path, snapshot));
        let Some(snapshot) = sources.last().and_then(|(_, snapshot)| snapshot.as_ref()) else {
            continue;
        };
        let document = match JsoncDoc::parse(&snapshot.content) {
            Ok(document) => document,
            Err(error) => {
                result
                    .diagnostics
                    .push(format!("{}: {error}", path.display()));
                continue;
            }
        };
        let definitions = match servers(&document) {
            Ok(Some(definitions)) => definitions,
            Ok(None) => continue,
            Err(error) => {
                result
                    .diagnostics
                    .push(format!("{}: {error}", path.display()));
                continue;
            }
        };

        for (name, value) in definitions {
            if name.is_empty() {
                result
                    .diagnostics
                    .push(format!("{}: invalid MCP server '{name}'", path.display()));
                continue;
            }

            let (effective, sources) = entries.entry(name.clone()).or_default();
            *effective = value.clone();
            sources.push(path.display().to_string());
        }
    }

    for (_, snapshot) in &sources {
        if let Some(snapshot) = snapshot {
            if let Err(error) = snapshot.validate() {
                complete = false;
                result.diagnostics.push(error.to_string());
            }
        }
    }
    let source_snapshot = if complete {
        source_snapshot(
            paths,
            sources.iter().map(|(path, snapshot)| {
                (
                    path.as_path(),
                    snapshot
                        .as_ref()
                        .map(|snapshot| (snapshot.path.as_path(), snapshot.content.as_str())),
                )
            }),
        )
    } else {
        String::new()
    };

    for (name, (config, source_paths)) in entries {
        let Value::Object(config) = config else {
            result.diagnostics.push(format!(
                "MCP server '{name}': configuration must be an object"
            ));
            continue;
        };
        let server_type = match config.get("type").and_then(Value::as_str) {
            Some("local") => "local",
            Some("remote") => "remote",
            _ => {
                result
                    .diagnostics
                    .push(format!("MCP server '{name}': type must be local or remote"));
                continue;
            }
        };
        let disabled = match config.get("disabled") {
            None => false,
            Some(Value::Bool(value)) => *value,
            Some(_) => {
                result
                    .diagnostics
                    .push(format!("MCP server '{name}': disabled must be a boolean"));
                continue;
            }
        };
        let target = if server_type == "remote" {
            config.get("url").and_then(Value::as_str).map(str::to_owned)
        } else {
            config
                .get("command")
                .and_then(Value::as_array)
                .and_then(|command| {
                    command
                        .iter()
                        .map(Value::as_str)
                        .collect::<Option<Vec<_>>>()
                        .filter(|command| !command.is_empty())
                        .map(|command| command.join(" "))
                })
        };
        let Some(target) = target.filter(|value| !value.is_empty()) else {
            result
                .diagnostics
                .push(format!("MCP server '{name}': missing or invalid target"));
            continue;
        };
        result.data.push(McpServer {
            name,
            r#type: server_type.to_owned(),
            disabled,
            target,
            source_path: source_paths.last().cloned().unwrap_or_default(),
            source_paths,
            source_snapshot: source_snapshot.clone(),
            config,
        });
    }

    Ok(result)
}

pub fn get(paths: &ConfigPaths, name: &str) -> Result<McpServer, AppError> {
    list(paths)?
        .data
        .into_iter()
        .find(|server| server.name == name)
        .ok_or_else(|| AppError::not_found(format!("global MCP server '{name}' was not found")))
}

pub fn delete(paths: &ConfigPaths, name: &str) -> Result<(), AppError> {
    paths.ensure_global_write()?;
    delete_inner(paths, name, |_| {}, |_| {})
}

pub fn delete_project(
    paths: &ConfigPaths,
    name: &str,
    expected_source_path: &str,
    expected_source_snapshot: &str,
) -> Result<(), AppError> {
    delete_project_inner(
        paths,
        name,
        expected_source_path,
        expected_source_snapshot,
        || {},
    )
}

fn delete_project_inner(
    paths: &ConfigPaths,
    name: &str,
    expected_source_path: &str,
    expected_source_snapshot: &str,
    before_persist: impl FnOnce(),
) -> Result<(), AppError> {
    if !paths.is_project() {
        return Err(AppError::validation(
            "Project scope is required for project MCP deletion",
        ));
    }
    validate_name(name)?;
    let mut documents = managed_documents(paths)?;
    let index = update_source(
        paths,
        &documents,
        name,
        None,
        expected_source_path,
        expected_source_snapshot,
        true,
    )?;
    let (_, _, document) = documents
        .get_mut(index)
        .ok_or_else(|| AppError::not_found("MCP server no longer exists"))?;
    document.patch(&["mcp", "servers", name], None)?;
    validate_effective_server(document, name, None)?;
    before_persist();
    persist_document(paths, &documents, index)
}

fn delete_inner(
    paths: &ConfigPaths,
    name: &str,
    mut after_resolve: impl FnMut(&Path),
    mut before_replace: impl FnMut(&Path),
) -> Result<(), AppError> {
    if name.is_empty() {
        return Err(AppError::validation("MCP server name must not be empty"));
    }

    let mut updates = BTreeMap::<PathBuf, (Snapshot, String)>::new();
    // Validate every managed source before writing, including shadow definitions.
    for path in paths.global_config_files() {
        let Some(snapshot) = Snapshot::read_with(path, &mut after_resolve)? else {
            continue;
        };
        if let Some((existing, _)) = updates.get_mut(&snapshot.path) {
            if snapshot.content != existing.content {
                return Err(AppError::configuration(
                    "Config changed during deletion; retry",
                ));
            }
            existing.aliases.push(path.clone());
            continue;
        }

        let mut document = JsoncDoc::parse(&snapshot.content)
            .map_err(|error| AppError::validation(format!("{}: {error}", path.display())))?;
        if servers(&document)?.is_some_and(|servers| servers.contains_key(name)) {
            document.patch(&["mcp", "servers", name], None)?;
            validate_effective_server(&document, name, None)?;
        }
        updates.insert(
            snapshot.path.clone(),
            (snapshot, document.content().to_owned()),
        );
    }

    let updates = updates
        .into_values()
        .filter(|(snapshot, replacement)| snapshot.content != *replacement)
        .collect::<Vec<_>>();
    if updates.is_empty() {
        return Err(AppError::not_found(format!(
            "global MCP server '{name}' was not found"
        )));
    }
    resource_file::replace(&updates, |path| {
        before_replace(path);

        Ok(())
    })
}

fn validate_name(name: &str) -> Result<(), AppError> {
    if name.trim().is_empty() || name.len() > 256 || name.chars().any(char::is_control) {
        return Err(AppError::validation(
            "MCP name must be nonempty, at most 256 bytes, without control characters",
        ));
    }

    Ok(())
}

fn validate_draft(name: &str, config: &Map<String, Value>) -> Result<(), AppError> {
    validate_name(name)?;

    let invalid =
        || AppError::validation("Invalid MCP config: check type, target, and optional field types");

    match config.get("type").and_then(Value::as_str) {
        Some("local") => {
            let command = config
                .get("command")
                .and_then(Value::as_array)
                .ok_or_else(invalid)?;
            if command.iter().any(|value| !value.is_string())
                || command
                    .first()
                    .and_then(Value::as_str)
                    .is_none_or(|value| value.trim().is_empty())
            {
                return Err(invalid());
            }
        }
        Some("remote") => {
            let url = config
                .get("url")
                .and_then(Value::as_str)
                .ok_or_else(invalid)?;
            let url = reqwest::Url::parse(url).map_err(|_| invalid())?;
            if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
                return Err(invalid());
            }
        }
        _ => return Err(invalid()),
    }

    if config
        .get("disabled")
        .is_some_and(|value| !value.is_boolean())
        || config.get("cwd").is_some_and(|value| !value.is_string())
        || config
            .get("codemode")
            .is_some_and(|value| !value.is_boolean())
        || config
            .get("protocol")
            .is_some_and(|value| !matches!(value.as_str(), Some("legacy" | "auto" | "2026-07-28")))
    {
        return Err(invalid());
    }

    if let Some(timeout) = config.get("timeout") {
        let timeout = timeout.as_object().ok_or_else(invalid)?;
        for field in ["startup", "catalog", "execution"] {
            if timeout
                .get(field)
                .is_some_and(|value| value.as_u64().is_none_or(|value| value == 0))
            {
                return Err(invalid());
            }
        }
    }

    for field in ["environment", "headers"] {
        if let Some(value) = config.get(field) {
            if value
                .as_object()
                .is_none_or(|map| map.values().any(|value| !value.is_string()))
            {
                return Err(invalid());
            }
        }
    }

    if let Some(oauth) = config.get("oauth") {
        if oauth != &Value::Bool(false) {
            let oauth = oauth.as_object().ok_or_else(invalid)?;
            for field in [
                "client_id",
                "client_secret",
                "scope",
                "redirect_uri",
                "auth_server_metadata_url",
            ] {
                if oauth.get(field).is_some_and(|value| !value.is_string()) {
                    return Err(invalid());
                }
            }

            if oauth.get("callback_port").is_some_and(|value| {
                value
                    .as_u64()
                    .is_none_or(|value| !(1..=65535).contains(&value))
            }) {
                return Err(invalid());
            }
        }
    }

    Ok(())
}

fn managed_documents(
    paths: &ConfigPaths,
) -> Result<Vec<(PathBuf, Option<Snapshot>, JsoncDoc)>, AppError> {
    paths
        .global_config_files()
        .iter()
        .map(|path| {
            let target = paths.mcp_config_target(path)?;
            let snapshot = Snapshot::read(path)?;
            if snapshot
                .as_ref()
                .is_some_and(|snapshot| snapshot.path != target)
            {
                return Err(AppError::configuration("MCP source changed; reload"));
            }
            let document = match &snapshot {
                Some(snapshot) => JsoncDoc::parse(&snapshot.content).map_err(|error| {
                    AppError::validation(format!("{}: {error}", path.display()))
                })?,
                None => JsoncDoc::bootstrap(crate::document::OPENCODE_SCHEMA),
            };
            servers(&document)?;

            Ok((path.clone(), snapshot, document))
        })
        .collect()
}

fn saved_server(
    name: String,
    config: Map<String, Value>,
    source_paths: Vec<String>,
) -> Result<McpServer, AppError> {
    let invalid = || AppError::validation("Invalid MCP config: missing or invalid type or target");
    let server_type = config
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(invalid)?
        .to_owned();
    let target = match server_type.as_str() {
        "local" => config
            .get("command")
            .and_then(Value::as_array)
            .filter(|command| !command.is_empty())
            .and_then(|command| {
                command
                    .iter()
                    .map(Value::as_str)
                    .collect::<Option<Vec<_>>>()
            })
            .ok_or_else(invalid)?
            .join(" "),
        "remote" => config
            .get("url")
            .and_then(Value::as_str)
            .ok_or_else(invalid)?
            .to_owned(),
        _ => return Err(invalid()),
    };
    Ok(McpServer {
        name,
        r#type: server_type,
        disabled: config
            .get("disabled")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        target,
        source_path: source_paths.last().cloned().unwrap_or_default(),
        source_paths,
        source_snapshot: String::new(),
        config,
    })
}

pub fn create(paths: &ConfigPaths, draft: McpDraft) -> Result<McpServer, AppError> {
    paths.ensure_global_write()?;
    validate_draft(&draft.name, &draft.config)?;
    let mut documents = managed_documents(paths)?;
    for (_, _, document) in &documents {
        if servers(document)?.is_some_and(|definitions| definitions.contains_key(&draft.name)) {
            return Err(AppError::validation(
                "An MCP server with this name already exists",
            ));
        }
    }

    let index = documents
        .iter()
        .rposition(|(_, snapshot, _)| snapshot.is_some())
        .or_else(|| documents.len().checked_sub(1))
        .ok_or_else(|| AppError::configuration("No managed MCP configuration files"))?;
    let (_, _, document) = documents
        .get_mut(index)
        .ok_or_else(|| AppError::configuration("MCP configuration document is unavailable"))?;
    document.patch(
        &["mcp", "servers", &draft.name],
        Some(Value::Object(draft.config.clone())),
    )?;
    validate_effective_server(document, &draft.name, Some(&draft.config))?;
    let (path, snapshot, _) = documents
        .get(index)
        .ok_or_else(|| AppError::configuration("MCP configuration document is unavailable"))?;
    let source_paths = match snapshot {
        Some(target) => documents
            .iter()
            .filter(|(_, snapshot, _)| {
                snapshot
                    .as_ref()
                    .is_some_and(|snapshot| snapshot.path == target.path)
            })
            .map(|(path, _, _)| path.display().to_string())
            .collect(),
        None => vec![path.display().to_string()],
    };
    let mut server = saved_server(draft.name, draft.config, source_paths)?;
    server.source_snapshot = saved_source_snapshot(paths, &documents, index)?;
    persist_document(paths, &documents, index)?;

    Ok(server)
}

pub fn update(paths: &ConfigPaths, draft: McpUpdate) -> Result<McpServer, AppError> {
    validate_draft(&draft.name, &draft.config)?;
    let mut documents = managed_documents(paths)?;
    let index = update_source(
        paths,
        &documents,
        &draft.name,
        Some(&draft.expected_config),
        &draft.expected_source_path,
        &draft.expected_source_snapshot,
        paths.is_project(),
    )?;
    let (_, _, document) = documents
        .get_mut(index)
        .ok_or_else(|| AppError::not_found("MCP server no longer exists"))?;
    document.patch(
        &["mcp", "servers", &draft.name],
        Some(Value::Object(draft.config.clone())),
    )?;
    validate_effective_server(document, &draft.name, Some(&draft.config))?;
    let source_paths = documents
        .iter()
        .filter(|(_, _, document)| {
            servers(document)
                .ok()
                .flatten()
                .is_some_and(|definitions| definitions.contains_key(&draft.name))
        })
        .map(|(path, _, _)| path.display().to_string())
        .collect();
    let mut server = saved_server(draft.name, draft.config, source_paths)?;
    server.source_snapshot = saved_source_snapshot(paths, &documents, index)?;
    persist_document(paths, &documents, index)?;

    Ok(server)
}

fn update_source(
    paths: &ConfigPaths,
    documents: &[(PathBuf, Option<Snapshot>, JsoncDoc)],
    name: &str,
    expected_config: Option<&Map<String, Value>>,
    expected_source_path: &str,
    expected_source_snapshot: &str,
    reject_shadows: bool,
) -> Result<usize, AppError> {
    let definitions = documents
        .iter()
        .enumerate()
        .filter(|(_, (_, _, document))| {
            servers(document)
                .ok()
                .flatten()
                .is_some_and(|definitions| definitions.contains_key(name))
        })
        .collect::<Vec<_>>();
    let &(index, (path, snapshot, document)) = definitions
        .last()
        .ok_or_else(|| AppError::not_found("MCP server no longer exists"))?;
    let current = servers(document)?
        .and_then(|definitions| definitions.get(name))
        .and_then(Value::as_object);
    let token = source_snapshot(
        paths,
        documents.iter().map(|(path, snapshot, _)| {
            (
                path.as_path(),
                snapshot
                    .as_ref()
                    .map(|snapshot| (snapshot.path.as_path(), snapshot.content.as_str())),
            )
        }),
    );
    if expected_config.is_some_and(|expected| current != Some(expected))
        || path.display().to_string() != expected_source_path
        || token != expected_source_snapshot
    {
        return Err(AppError::configuration(
            "MCP server changed; reload before saving",
        ));
    }
    if reject_shadows
        && definitions.iter().any(|(_, (_, other, _))| {
            other.as_ref().map(|source| &source.path)
                != snapshot.as_ref().map(|source| &source.path)
        })
    {
        return Err(AppError::validation(
            "MCP server has multiple sources; resolve duplicate definitions before editing",
        ));
    }

    Ok(index)
}

pub fn set_disabled(paths: &ConfigPaths, draft: McpDisabledUpdate) -> Result<McpServer, AppError> {
    validate_name(&draft.name)?;
    let mut documents = managed_documents(paths)?;
    let index = update_source(
        paths,
        &documents,
        &draft.name,
        None,
        &draft.expected_source_path,
        &draft.expected_source_snapshot,
        true,
    )?;
    let mut config = documents
        .get(index)
        .and_then(|(_, _, document)| servers(document).ok().flatten())
        .and_then(|definitions| definitions.get(&draft.name))
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| AppError::validation("MCP configuration must be an object"))?;
    let current_disabled = match config.get("disabled") {
        None => false,
        Some(Value::Bool(disabled)) => *disabled,
        Some(_) => return Err(AppError::validation("MCP disabled must be a boolean")),
    };
    if current_disabled != draft.expected_disabled {
        return Err(AppError::configuration(
            "MCP server changed; reload before saving",
        ));
    }
    let sources = documents
        .iter()
        .filter(|(_, _, document)| {
            servers(document)
                .ok()
                .flatten()
                .is_some_and(|servers| servers.contains_key(&draft.name))
        })
        .map(|(path, _, _)| path.display().to_string())
        .collect();
    if current_disabled == draft.disabled {
        let mut server = saved_server(draft.name, config, sources)?;
        server.source_snapshot = draft.expected_source_snapshot;
        validate_documents(paths, &documents)?;
        return Ok(server);
    }
    config.insert("disabled".to_owned(), Value::Bool(draft.disabled));
    let (_, _, document) = documents
        .get_mut(index)
        .ok_or_else(|| AppError::not_found("MCP server no longer exists"))?;
    document.patch(
        &["mcp", "servers", &draft.name, "disabled"],
        Some(Value::Bool(draft.disabled)),
    )?;
    validate_effective_server(document, &draft.name, Some(&config))?;
    let mut server = saved_server(draft.name, config, sources)?;
    server.source_snapshot = saved_source_snapshot(paths, &documents, index)?;
    persist_document(paths, &documents, index)?;

    Ok(server)
}

fn saved_source_snapshot(
    paths: &ConfigPaths,
    documents: &[(PathBuf, Option<Snapshot>, JsoncDoc)],
    index: usize,
) -> Result<String, AppError> {
    let (path, snapshot, document) = documents
        .get(index)
        .ok_or_else(|| AppError::configuration("MCP configuration document is unavailable"))?;
    let target = match snapshot {
        Some(snapshot) => snapshot.path.clone(),
        None => paths.mcp_config_target(path)?,
    };
    Ok(source_snapshot(
        paths,
        documents.iter().map(|(path, snapshot, _)| {
            let source = match snapshot {
                Some(snapshot) if snapshot.path == target => {
                    Some((target.as_path(), document.content()))
                }
                Some(snapshot) => Some((snapshot.path.as_path(), snapshot.content.as_str())),
                None if path == &documents[index].0 => Some((target.as_path(), document.content())),
                None => None,
            };
            (path.as_path(), source)
        }),
    ))
}

fn validate_documents(
    paths: &ConfigPaths,
    documents: &[(PathBuf, Option<Snapshot>, JsoncDoc)],
) -> Result<(), AppError> {
    for (path, snapshot, _) in documents {
        let target = paths.mcp_config_target(path)?;
        match snapshot {
            Some(snapshot) if target == snapshot.path => snapshot.validate()?,
            Some(_) => return Err(AppError::configuration("MCP source changed; reload")),
            None => resource_file::absent(path)?,
        }
    }

    Ok(())
}

fn persist_document(
    paths: &ConfigPaths,
    documents: &[(PathBuf, Option<Snapshot>, JsoncDoc)],
    index: usize,
) -> Result<(), AppError> {
    let (path, snapshot, document) = documents
        .get(index)
        .ok_or_else(|| AppError::configuration("MCP configuration document is unavailable"))?;
    let validate_sources = || validate_documents(paths, documents);
    validate_sources()?;
    if let Some(snapshot) = snapshot {
        let mut snapshot = snapshot.clone();
        snapshot.aliases = documents
            .iter()
            .filter(|(_, other, _)| {
                other
                    .as_ref()
                    .is_some_and(|other| other.path == snapshot.path)
            })
            .map(|(path, _, _)| path.clone())
            .collect();
        resource_file::replace_checked(&[(snapshot, document.content().to_owned())], |_| {
            validate_sources()
        })
    } else {
        let filename = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| AppError::validation("MCP config filename must be valid UTF-8"))?;
        let parent = Directory::ensure(
            path.parent()
                .ok_or_else(|| AppError::validation("MCP config path must have a parent"))?,
        )?;
        parent.create_file_checked(filename, document.content(), validate_sources)
    }
}

#[cfg(all(test, unix))]
#[path = "mcp/tests.rs"]
mod safety_tests;

#[cfg(all(test, unix))]
#[path = "mcp/scoped_tests.rs"]
mod scoped_tests;
