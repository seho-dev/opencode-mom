use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::error::AppError;
use crate::paths::ConfigPaths;

use super::active::JsonObject;
use super::transport::{self, ManagedConnection};

const MAX_BYTES: u64 = 2 * 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(20);

mod global;
mod removal;
mod spec;

pub use removal::{PluginRemovalPlan, PluginRemovalTarget};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRuntimeItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub source: PluginSource,
    #[serde(deserialize_with = "features_object")]
    pub features: PluginFeatures,
    pub state: PluginState,
    #[serde(rename = "canUpdate", skip_deserializing)]
    pub can_update: bool,
    #[serde(rename = "canRemove", skip_deserializing)]
    pub can_remove: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PluginSource {
    Builtin,
    Sdk,
    Local {
        path: String,
    },
    Package {
        target: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        version: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        outdated: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        updating: Option<bool>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginFeatures {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tui: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rpc: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum PluginState {
    Active,
    Failed {
        error: String,
        #[serde(rename = "ref", skip_serializing_if = "Option::is_none")]
        reference: Option<String>,
    },
}

#[derive(Debug, Serialize)]
pub struct PluginRuntime {
    pub directory: String,
    pub data: Vec<PluginRuntimeItem>,
}

#[derive(Debug, Serialize)]
pub struct PluginConfigPreview {
    pub directory: String,
    pub data: Vec<PluginConfigSource>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum PluginConfigSource {
    Document {
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<String>,
        plugins: Vec<PluginConfigEntry>,
    },
    Directory {
        path: String,
    },
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum PluginConfigEntry {
    Native(String),
    Package {
        package: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        options: Option<Map<String, Value>>,
    },
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ConfigSource {
    Document {
        #[serde(default, deserialize_with = "present_string")]
        path: Option<String>,
        info: JsonObject<ConfigInfo>,
    },
    Directory {
        path: String,
    },
}

#[derive(Deserialize)]
struct ConfigInfo {
    #[serde(default)]
    plugins: Vec<ConfigPluginEntry>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ConfigPluginEntry {
    Native(String),
    Package(JsonObject<ConfigPackage>),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigPackage {
    package: String,
    #[serde(default, deserialize_with = "present_options")]
    options: Option<JsonObject<Map<String, Value>>>,
}

fn present_string<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    String::deserialize(deserializer).map(Some)
}

fn present_options<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<JsonObject<Map<String, Value>>>, D::Error> {
    JsonObject::deserialize(deserializer).map(Some)
}

fn features_object<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<PluginFeatures, D::Error> {
    JsonObject::deserialize(deserializer).map(|value| value.0)
}

#[derive(Deserialize)]
struct RuntimeEnvelope {
    location: JsonObject<RuntimeLocation>,
    data: Vec<JsonObject<PluginRuntimeItem>>,
}

#[derive(Deserialize)]
struct RuntimeLocation {
    directory: String,
}

// Loading the list checks each eligible Global target; config preview remains GET-only.
pub fn list(paths: &ConfigPaths) -> Result<PluginRuntime, AppError> {
    validate_global_scope(paths)?;
    let deadline = Instant::now() + TIMEOUT;
    let connection = transport::connect(deadline)?;
    check_with(paths, None, &connection, deadline)
}

pub fn update(paths: &ConfigPaths, target: &str) -> Result<(), AppError> {
    validate_global_scope(paths)?;
    let deadline = Instant::now() + TIMEOUT;
    let connection = transport::connect(deadline)?;
    update_with(paths, target, &connection, deadline)
}

pub fn config_sources(paths: &ConfigPaths) -> Result<PluginConfigPreview, AppError> {
    validate_global_scope(paths)?;
    let deadline = Instant::now() + TIMEOUT;
    let connection = transport::connect(deadline)?;
    config_sources_with(paths, &connection, deadline)
}

pub fn prepare_removal(
    paths: &ConfigPaths,
    target: PluginRemovalTarget,
) -> Result<PluginRemovalPlan, AppError> {
    validate_global_scope(paths)?;
    target.validate()?;
    let deadline = Instant::now() + TIMEOUT;
    let connection = transport::connect(deadline)?;
    removal::prepare_with(paths, target, &connection, deadline)
}

pub fn remove_declaration(paths: &ConfigPaths, plan: PluginRemovalPlan) -> Result<(), AppError> {
    validate_global_scope(paths)?;
    plan.validate()?;
    let deadline = Instant::now() + TIMEOUT;
    let connection = transport::connect(deadline)?;
    removal::remove_with(paths, plan, &connection, deadline)
}

fn config_sources_with(
    paths: &ConfigPaths,
    connection: &ManagedConnection,
    deadline: Instant,
) -> Result<PluginConfigPreview, AppError> {
    Ok(global_state(paths, connection, deadline)?.preview)
}

fn read_config_sources(
    connection: &ManagedConnection,
    directory: &str,
    deadline: Instant,
) -> Result<PluginConfigPreview, AppError> {
    connection.verify_identity(deadline)?;
    let sources: Vec<JsonObject<ConfigSource>> = connection.get_json(
        &["config"],
        &[("location[directory]", directory)],
        deadline,
        MAX_BYTES,
        "plugin config sources",
        "2 MiB",
    )?;
    let result = project_config_sources(directory.to_owned(), sources)?;
    connection.verify_identity(deadline)?;
    Ok(result)
}

struct GlobalState {
    runtime: PluginRuntime,
    preview: PluginConfigPreview,
    scope: global::GlobalPaths,
}

fn global_state(
    paths: &ConfigPaths,
    connection: &ManagedConnection,
    deadline: Instant,
) -> Result<GlobalState, AppError> {
    validate_global_scope(paths)?;
    let scope = global::GlobalPaths::capture(paths)?;
    let runtime = runtime(connection, connection.directory(), deadline)?;
    let preview = scope.filter(read_config_sources(
        connection,
        &runtime.directory,
        deadline,
    )?)?;
    scope.ensure_current()?;
    connection.ensure_current()?;
    Ok(GlobalState {
        runtime,
        preview,
        scope,
    })
}

#[cfg(test)]
fn list_with(
    paths: &ConfigPaths,
    connection: &ManagedConnection,
    deadline: Instant,
) -> Result<PluginRuntime, AppError> {
    let state = global_state(paths, connection, deadline)?;
    let result = global::filter_runtime(state.runtime, &state.preview)?;
    state.scope.ensure_current()?;
    connection.ensure_current()?;
    Ok(result)
}

fn project_config_sources(
    directory: String,
    sources: Vec<JsonObject<ConfigSource>>,
) -> Result<PluginConfigPreview, AppError> {
    let invalid = || AppError::configuration("Invalid OpenCode plugin config sources response");
    if sources.len() > 4096 {
        return Err(invalid());
    }
    let mut data = Vec::with_capacity(sources.len());
    for source in sources {
        data.push(match source.0 {
            ConfigSource::Document { path, info } => {
                if path.as_ref().is_some_and(|path| !absolute_path(path))
                    || info.0.plugins.len() > 4096
                {
                    return Err(invalid());
                }
                let plugins = info
                    .0
                    .plugins
                    .into_iter()
                    .map(|entry| {
                        let entry = match entry {
                            ConfigPluginEntry::Native(value) => PluginConfigEntry::Native(value),
                            ConfigPluginEntry::Package(value) => PluginConfigEntry::Package {
                                package: value.0.package,
                                options: value.0.options.map(|options| options.0),
                            },
                        };
                        let value = match &entry {
                            PluginConfigEntry::Native(value) => value,
                            PluginConfigEntry::Package { package, .. } => package,
                        };
                        if !bounded_text(value, 4096) {
                            return Err(invalid());
                        }
                        Ok(entry)
                    })
                    .collect::<Result<_, AppError>>()?;
                PluginConfigSource::Document { path, plugins }
            }
            ConfigSource::Directory { path } => {
                if !absolute_path(&path) {
                    return Err(invalid());
                }
                PluginConfigSource::Directory { path }
            }
        });
    }
    Ok(PluginConfigPreview { directory, data })
}

fn check_with(
    paths: &ConfigPaths,
    target: Option<&str>,
    connection: &ManagedConnection,
    deadline: Instant,
) -> Result<PluginRuntime, AppError> {
    let mut state = global_state(paths, connection, deadline)?;
    let targets = match target {
        Some(target) => {
            allowed_global_target(target, &state, false)?;
            vec![target.to_owned()]
        }
        None => state
            .runtime
            .data
            .iter()
            .filter_map(|item| match &item.source {
                PluginSource::Package { target, .. }
                    if allowed_global_target(target, &state, false).is_ok() =>
                {
                    Some(target.clone())
                }
                _ => None,
            })
            .collect(),
    };
    for target in targets {
        allowed_global_target(&target, &state, false)?;
        state.scope.ensure_current()?;
        state.runtime = check_one(
            connection,
            &state.runtime.directory,
            &target,
            &state.scope,
            deadline,
        )?;
    }
    let result = global::filter_runtime(state.runtime, &state.preview)?;
    state.scope.ensure_current()?;
    connection.ensure_current()?;
    Ok(result)
}

fn check_one(
    connection: &ManagedConnection,
    directory: &str,
    target: &str,
    scope: &global::GlobalPaths,
    deadline: Instant,
) -> Result<PluginRuntime, AppError> {
    let (response, _reservation) = connection.plugin_post(
        "check",
        Some(Path::new(directory)),
        &json!({"target": target}),
        deadline,
    )?;
    let response = response.ok_or_else(invalid_runtime)?;
    let envelope: JsonObject<RuntimeEnvelope> =
        transport::response_json(response, deadline, MAX_BYTES, "plugin check", "2 MiB").map_err(
            |_| {
                AppError::configuration(
        "Plugin check outcome unknown; refresh with GET before deciding whether to retry",
    )
            },
        )?;
    (|| {
        connection.verify_identity(deadline)?;
        scope.ensure_current()?;
        let result = project_runtime(envelope.0)?;
        validate_requested_directory(Some(Path::new(directory)), &result)?;
        Ok(result)
    })()
    .map_err(|_: AppError| {
        AppError::configuration(
            "Plugin check outcome unknown; refresh with GET before deciding whether to retry",
        )
    })
}

fn update_with(
    paths: &ConfigPaths,
    target: &str,
    connection: &ManagedConnection,
    deadline: Instant,
) -> Result<(), AppError> {
    let state = global_state(paths, connection, deadline)?;
    allowed_global_target(target, &state, true)?;
    state.scope.ensure_current()?;
    let (_response, _reservation) = connection.plugin_post(
        "update",
        Some(Path::new(&state.runtime.directory)),
        &json!({"targets": [target]}),
        deadline,
    )?;
    (|| {
        connection.verify_identity(deadline)?;
        state.scope.ensure_current()
    })()
    .map_err(|_: AppError| {
        AppError::configuration(
            "Plugin update outcome unknown; refresh with GET before deciding whether to retry",
        )
    })?;
    Ok(())
}

fn allowed_global_target(
    target: &str,
    state: &GlobalState,
    updating: bool,
) -> Result<(), AppError> {
    if !global::declares_package(&state.preview, target) {
        return Err(AppError::validation(
            "Plugin target is not declared by Global configuration",
        ));
    }
    allowed_target(target, &state.runtime, updating)
}

fn runtime(
    connection: &ManagedConnection,
    directory: Option<&Path>,
    deadline: Instant,
) -> Result<PluginRuntime, AppError> {
    let requested_directory = directory;
    let directory = directory
        .map(|path| {
            path.to_str()
                .ok_or_else(|| AppError::validation("Plugin runtime directory must be UTF-8"))
        })
        .transpose()?;
    let query: Vec<_> = directory
        .into_iter()
        .map(|directory| ("location[directory]", directory))
        .collect();
    let envelope: JsonObject<RuntimeEnvelope> = connection.get_json(
        &["plugin"],
        &query,
        deadline,
        MAX_BYTES,
        "plugin runtime",
        "2 MiB",
    )?;
    let result = project_runtime(envelope.0)?;
    validate_requested_directory(requested_directory, &result)?;
    Ok(result)
}

fn project_runtime(envelope: RuntimeEnvelope) -> Result<PluginRuntime, AppError> {
    if !absolute_path(&envelope.location.0.directory) || envelope.data.len() > 4096 {
        return Err(invalid_runtime());
    }
    let mut data = Vec::with_capacity(envelope.data.len());
    for item in envelope.data {
        let mut item = item.0;
        if item.id.as_ref().is_some_and(|id| !bounded_text(id, 4096))
            || [item.features.server, item.features.tui, item.features.rpc].contains(&Some(false))
        {
            return Err(invalid_runtime());
        }
        match &item.source {
            PluginSource::Local { path } if !bounded_text(path, 4096) => {
                return Err(invalid_runtime())
            }
            PluginSource::Package {
                target,
                version,
                outdated,
                updating,
            } => {
                if !bounded_text(target, 4096)
                    || version
                        .as_ref()
                        .is_some_and(|version| !bounded_text(version, 4096))
                    || *outdated == Some(false)
                    || *updating == Some(false)
                {
                    return Err(invalid_runtime());
                }
            }
            _ => {}
        }
        if let PluginState::Failed { reference, .. } = &item.state {
            if reference
                .as_ref()
                .is_some_and(|reference| !bounded_text(reference, 4096))
            {
                return Err(invalid_runtime());
            }
            // Native failure messages can contain credentials and plugin-owned output.
            item.state = PluginState::Failed {
                error: "Plugin failed; inspect OpenCode diagnostics".to_owned(),
                reference: reference.clone(),
            };
        }
        data.push(item);
    }
    let counts = target_counts(&data);
    let can_update: Vec<_> = data
        .iter()
        .map(|item| match &item.source {
            PluginSource::Package {
                target,
                outdated: Some(true),
                updating,
                ..
            } => {
                *updating != Some(true)
                    && counts.get(target.as_str()) == Some(&1)
                    && spec::package_mutability(target) == Some(true)
            }
            _ => false,
        })
        .collect();
    for (item, can_update) in data.iter_mut().zip(can_update) {
        item.can_update = can_update;
    }
    Ok(PluginRuntime {
        directory: envelope.location.0.directory,
        data,
    })
}

fn allowed_target(target: &str, runtime: &PluginRuntime, updating: bool) -> Result<(), AppError> {
    let package = runtime.data.iter().find(|item| {
        matches!(
            &item.source, PluginSource::Package { target: candidate, .. } if candidate == target
        )
    });
    if target_counts(&runtime.data).get(target) != Some(&1)
        || package.is_none_or(|item| {
            (updating && !item.can_update)
                || matches!(
                    &item.source,
                    PluginSource::Package {
                        updating: Some(true),
                        ..
                    }
                )
        })
    {
        return Err(AppError::validation("Plugin target is absent, ambiguous, local, built-in, pinned, current or already updating in the selected runtime"));
    }
    Ok(())
}

fn target_counts(data: &[PluginRuntimeItem]) -> HashMap<&str, usize> {
    let mut counts = HashMap::new();
    for item in data {
        if let Some(id) = &item.id {
            *counts.entry(id.as_str()).or_default() += 1;
        }
        if let PluginSource::Package { target, .. } = &item.source {
            if item.id.as_deref() != Some(target) {
                *counts.entry(target.as_str()).or_default() += 1;
            }
        }
    }
    counts
}

fn absolute_path(value: &str) -> bool {
    bounded_text(value, 4096)
        && Path::new(value).is_absolute()
        && !Path::new(value)
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
        && !value
            .split(['/', '\\'])
            .any(|part| matches!(part, "." | ".."))
}

fn validate_global_scope(paths: &ConfigPaths) -> Result<(), AppError> {
    if paths.is_project() {
        return Err(AppError::validation(
            "Plugins only support Global configuration",
        ));
    }
    paths.validate_resource_root()
}

fn validate_requested_directory(
    directory: Option<&Path>,
    runtime: &PluginRuntime,
) -> Result<(), AppError> {
    if directory.is_some_and(|directory| PathBuf::from(&runtime.directory) != directory) {
        return Err(AppError::configuration(
            "Plugin runtime location differs from the requested location",
        ));
    }
    Ok(())
}

fn bounded_text(value: &str, max: usize) -> bool {
    !value.is_empty() && value.len() <= max && !value.chars().any(char::is_control)
}

fn invalid_runtime() -> AppError {
    AppError::configuration("Invalid OpenCode plugin runtime response")
}

#[cfg(test)]
mod tests;
