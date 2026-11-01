use std::path::Path;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::AppError;
use crate::paths::ConfigPaths;
use crate::resource_file::{self, Snapshot};

use super::transport::ManagedConnection;
use super::{global, global_state, PluginConfigSource, PluginSource};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum PluginRemovalTarget {
    Package { target: String },
    Local { path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginRemovalSource {
    pub path: String,
    pub expected_path: String,
    pub expected_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginRemovalPlan {
    pub target: PluginRemovalTarget,
    pub sources: Vec<PluginRemovalSource>,
    pub service_key: String,
}

fn changed() -> AppError {
    AppError::configuration("Global plugin declaration changed; refresh before removing")
}

fn hash(content: &str) -> String {
    format!("{:x}", Sha256::digest(content.as_bytes()))
}

impl PluginRemovalTarget {
    pub(super) fn validate(&self) -> Result<(), AppError> {
        let valid = match self {
            Self::Package { target } => super::bounded_text(target, 4096),
            Self::Local { path } => super::absolute_path(path),
        };
        if valid {
            Ok(())
        } else {
            Err(AppError::validation("Invalid plugin removal target"))
        }
    }

    fn matches_source(&self, source: &PluginSource) -> bool {
        match (self, source) {
            (Self::Package { target }, PluginSource::Package { target: other, .. }) => {
                target == other
            }
            (Self::Local { path }, PluginSource::Local { path: other }) => path == other,
            _ => false,
        }
    }
}

impl PluginRemovalPlan {
    pub(super) fn validate(&self) -> Result<(), AppError> {
        self.target.validate()?;
        let digest =
            |value: &str| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit());
        if self.sources.is_empty()
            || self.sources.len() > 4096
            || !digest(&self.service_key)
            || self.sources.iter().any(|source| {
                !super::absolute_path(&source.path)
                    || !super::absolute_path(&source.expected_path)
                    || !digest(&source.expected_hash)
            })
        {
            return Err(AppError::validation("Invalid plugin removal snapshot"));
        }
        Ok(())
    }
}

fn matches_entry(
    target: &PluginRemovalTarget,
    entry: &str,
    document: &Path,
) -> Result<bool, AppError> {
    match target {
        PluginRemovalTarget::Package { target } => Ok(entry == target),
        PluginRemovalTarget::Local { path } => {
            global::local_declaration_matches(entry, document, path)
        }
    }
}

fn removal_text(
    content: &str,
    document: &Path,
    target: &PluginRemovalTarget,
) -> Result<String, AppError> {
    use jsonc_parser::cst::CstRootNode;
    let entries = global::read_declarations(content)?;
    let root = CstRootNode::parse(content, &jsonc_parser::ParseOptions::default())
        .map_err(|_| changed())?;
    let object = root.object_value().ok_or_else(changed)?;
    if object
        .properties()
        .iter()
        .filter(|property| {
            property
                .name()
                .is_some_and(|name| name.decoded_value().ok().as_deref() == Some("plugins"))
        })
        .count()
        > 1
    {
        return Err(changed());
    }
    let array = object.array_value("plugins").ok_or_else(changed)?;
    let elements = array.elements();
    if entries.len() != elements.len() {
        return Err(changed());
    }
    let mut removed = false;
    for (entry, element) in entries.iter().zip(elements).rev() {
        if matches_entry(target, global::package(entry), document)? {
            element.remove();
            removed = true;
        }
    }
    if !removed {
        return Err(changed());
    }
    let text = root.to_string();
    global::read_declarations(&text)?;
    Ok(text)
}

fn collect_sources(
    paths: &ConfigPaths,
    state: &super::GlobalState,
    target: &PluginRemovalTarget,
) -> Result<Vec<(PluginRemovalSource, Snapshot, String)>, AppError> {
    if !state
        .runtime
        .data
        .iter()
        .any(|item| target.matches_source(&item.source))
    {
        return Err(AppError::validation(
            "Plugin is absent from the selected runtime",
        ));
    }
    let mut sources = Vec::new();
    for source in &state.preview.data {
        let PluginConfigSource::Document {
            path: Some(path), ..
        } = source
        else {
            continue;
        };
        paths.validate_plugin_config_source(Path::new(path))?;
        let Some(snapshot) = Snapshot::read(Path::new(path))? else {
            continue;
        };
        let entries = global::read_declarations(&snapshot.content)?;
        let mut matched = false;
        for entry in &entries {
            matched |= matches_entry(target, global::package(entry), Path::new(path))?;
        }
        if !matched {
            continue;
        }
        let expected_path = snapshot
            .path
            .to_str()
            .filter(|path| super::absolute_path(path))
            .ok_or_else(changed)?
            .to_owned();
        let replacement = removal_text(&snapshot.content, Path::new(path), target)?;
        sources.push((
            PluginRemovalSource {
                path: path.clone(),
                expected_path,
                expected_hash: hash(&snapshot.content),
            },
            snapshot,
            replacement,
        ));
    }
    sources.sort_by(|left, right| left.0.path.cmp(&right.0.path));
    sources.dedup_by(|left, right| left.0.path == right.0.path);
    if sources.is_empty() {
        return Err(AppError::validation(
            "Plugin has no removable Global declaration",
        ));
    }
    for (_, snapshot, _) in &sources {
        snapshot.validate()?;
    }
    state.scope.ensure_current()?;
    Ok(sources)
}

pub(super) fn prepare_with(
    paths: &ConfigPaths,
    target: PluginRemovalTarget,
    connection: &ManagedConnection,
    deadline: Instant,
) -> Result<PluginRemovalPlan, AppError> {
    target.validate()?;
    let state = global_state(paths, connection, deadline)?;
    let sources = collect_sources(paths, &state, &target)?
        .into_iter()
        .map(|(source, _, _)| source)
        .collect();
    connection.verify_identity(deadline)?;
    Ok(PluginRemovalPlan {
        target,
        sources,
        service_key: connection.plugin_service_key(&state.runtime.directory)?,
    })
}

pub(super) fn remove_with(
    paths: &ConfigPaths,
    plan: PluginRemovalPlan,
    connection: &ManagedConnection,
    deadline: Instant,
) -> Result<(), AppError> {
    plan.validate()?;
    let state = global_state(paths, connection, deadline)?;
    if connection.plugin_service_key(&state.runtime.directory)? != plan.service_key {
        return Err(changed());
    }
    let sources = collect_sources(paths, &state, &plan.target)?;
    if sources
        .iter()
        .map(|(source, _, _)| source)
        .ne(plan.sources.iter())
    {
        return Err(changed());
    }
    let mut updates: Vec<(Snapshot, String)> = Vec::new();
    for (_, snapshot, replacement) in sources {
        if let Some((existing, text)) = updates
            .iter_mut()
            .find(|(existing, _)| existing.path == snapshot.path)
        {
            if text != &replacement || existing.content != snapshot.content {
                return Err(changed());
            }
            existing.aliases.extend(snapshot.aliases);
        } else {
            updates.push((snapshot, replacement));
        }
    }
    let _reservation = connection.reserve_plugin_removal(deadline)?;
    resource_file::replace_checked(&updates, |_| {
        for source in &plan.sources {
            paths.validate_plugin_config_source(Path::new(&source.path))?;
        }
        connection.verify_identity(deadline)?;
        state.scope.ensure_current()
    })?;
    Ok(())
}
