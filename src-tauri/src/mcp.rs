use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::document::{read_text, JsoncDoc};
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
    let mut result = McpList::default();
    let mut entries = BTreeMap::<String, (Value, Vec<String>)>::new();
    for path in paths.global_config_files() {
        let document = match read_text(path)
            .and_then(|content| content.map(|content| JsoncDoc::parse(&content)).transpose())
        {
            Ok(Some(document)) => document,
            Ok(None) => continue,
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
    delete_inner(paths, name, |_| {}, |_| {})
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

fn validate_draft(name: &str, config: &Map<String, Value>) -> Result<(), AppError> {
    if name.trim().is_empty() || name.len() > 256 || name.chars().any(char::is_control) {
        return Err(AppError::validation(
            "MCP name must be nonempty, at most 256 bytes, without control characters",
        ));
    }
    let invalid =
        || AppError::validation("Invalid MCP config: check type, target, and optional field types");
    match config.get("type").and_then(Value::as_str) {
        Some("local") => {
            let command = config
                .get("command")
                .and_then(Value::as_array)
                .ok_or_else(invalid)?;
            if command.is_empty()
                || command.iter().any(|value| !value.is_string())
                || command[0]
                    .as_str()
                    .map_or(true, |value| value.trim().is_empty())
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
                .is_some_and(|value| value.as_u64().map_or(true, |value| value == 0))
            {
                return Err(invalid());
            }
        }
    }
    for field in ["environment", "headers"] {
        if let Some(value) = config.get(field) {
            if value
                .as_object()
                .map_or(true, |map| map.values().any(|value| !value.is_string()))
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
                    .map_or(true, |value| !(1..=65535).contains(&value))
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
            let snapshot = Snapshot::read(path)?;
            let document = match &snapshot {
                Some(snapshot) => JsoncDoc::parse(&snapshot.content).map_err(|_| {
                    AppError::validation(format!("{}: malformed JSONC document", path.display()))
                })?,
                None => JsoncDoc::bootstrap(crate::document::OPENCODE_SCHEMA),
            };
            servers(&document)?;
            Ok((path.clone(), snapshot, document))
        })
        .collect()
}

fn saved_server(name: String, config: Map<String, Value>, source_paths: Vec<String>) -> McpServer {
    let server_type = config["type"].as_str().unwrap().to_owned();
    let target = if server_type == "local" {
        config["command"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap())
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        config["url"].as_str().unwrap().to_owned()
    };
    McpServer {
        name,
        r#type: server_type,
        disabled: config
            .get("disabled")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        target,
        source_path: source_paths.last().cloned().unwrap_or_default(),
        source_paths,
        config,
    }
}

pub fn create(paths: &ConfigPaths, draft: McpDraft) -> Result<McpServer, AppError> {
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
        .unwrap_or(documents.len() - 1);
    documents[index].2.patch(
        &["mcp", "servers", &draft.name],
        Some(Value::Object(draft.config.clone())),
    )?;
    validate_effective_server(&documents[index].2, &draft.name, Some(&draft.config))?;
    persist_document(&documents, index)?;
    let source_paths = match &documents[index].1 {
        Some(target) => documents
            .iter()
            .filter(|(_, snapshot, _)| {
                snapshot
                    .as_ref()
                    .is_some_and(|snapshot| snapshot.path == target.path)
            })
            .map(|(path, _, _)| path.display().to_string())
            .collect(),
        None => vec![documents[index].0.display().to_string()],
    };
    Ok(saved_server(draft.name, draft.config, source_paths))
}

pub fn update(paths: &ConfigPaths, draft: McpUpdate) -> Result<McpServer, AppError> {
    validate_draft(&draft.name, &draft.config)?;
    let mut documents = managed_documents(paths)?;
    let index = documents
        .iter()
        .rposition(|(_, _, document)| {
            servers(document)
                .ok()
                .flatten()
                .is_some_and(|definitions| definitions.contains_key(&draft.name))
        })
        .ok_or_else(|| AppError::not_found("MCP server no longer exists"))?;
    let current = servers(&documents[index].2)?.unwrap()[&draft.name].as_object();
    if current != Some(&draft.expected_config)
        || documents[index].0.display().to_string() != draft.expected_source_path
    {
        return Err(AppError::configuration(
            "MCP server changed; reload before saving",
        ));
    }
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
    documents[index].2.patch(
        &["mcp", "servers", &draft.name],
        Some(Value::Object(draft.config.clone())),
    )?;
    validate_effective_server(&documents[index].2, &draft.name, Some(&draft.config))?;
    persist_document(&documents, index)?;
    Ok(saved_server(draft.name, draft.config, source_paths))
}

fn persist_document(
    documents: &[(PathBuf, Option<Snapshot>, JsoncDoc)],
    index: usize,
) -> Result<(), AppError> {
    for (path, snapshot, _) in documents {
        match snapshot {
            Some(snapshot) => snapshot.validate()?,
            None => resource_file::absent(path)?,
        }
    }
    let (path, snapshot, document) = &documents[index];
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
        resource_file::replace(&[(snapshot, document.content().to_owned())], |_| {
            for (path, snapshot, _) in documents {
                match snapshot {
                    Some(snapshot) => snapshot.validate()?,
                    None => resource_file::absent(path)?,
                }
            }
            Ok(())
        })
    } else {
        let parent = Directory::ensure(path.parent().unwrap())?;
        parent.create_file_checked(
            path.file_name().unwrap().to_str().unwrap(),
            document.content(),
            || {
                for (path, snapshot, _) in documents {
                    match snapshot {
                        Some(snapshot) => snapshot.validate()?,
                        None => resource_file::absent(path)?,
                    }
                }
                Ok(())
            },
        )
    }
}

#[cfg(all(test, unix))]
#[path = "mcp/tests.rs"]
mod safety_tests;
