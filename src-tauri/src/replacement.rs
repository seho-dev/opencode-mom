use std::collections::BTreeMap;
use std::path::PathBuf;

use serde_json::Value;
use time::OffsetDateTime;

use crate::agents_md::MarkdownAgentDocument;
use crate::document::JsoncDoc;
use crate::error::AppError;
use crate::models::AppConfig;
use crate::paths::ConfigPaths;
use crate::providers::ModelRef;

// OMO stores OpenCode-specific mappings under the literal `opencode` object.
const OMO_OPENCODE_BLOCK: &str = "opencode";

#[derive(Debug, Clone)]
enum OpenCodeKeyRename {
    Model { from: ModelRef, to: ModelRef },
    Provider { old_id: String, new_id: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JsoncRole {
    OpenCode,
    Slim,
    Omo,
}

/// Scans Slim and OMO files for model references used by deletion protection.
pub fn scan_plugin_references(
    paths: &ConfigPaths,
) -> Result<Vec<crate::refs::ModelReference>, AppError> {
    let mut references = Vec::new();
    if let Some(content) = crate::document::read_text(&paths.slim_file())? {
        let document = JsoncDoc::parse(&content)?;
        scan_slim(&document, &mut references)?;
    }
    if let Some(content) = crate::document::read_text(&paths.omo_file())? {
        let document = JsoncDoc::parse(&content)?;
        scan_omo(&document, &mut references)?;
    }
    references.sort();
    Ok(references)
}

/// Replaces every `from` reference with `to` across OpenCode/Slim/OMO files, Markdown
/// agents, and application groups. Returns the number of replacements.
pub fn replace(paths: &ConfigPaths, from: &ModelRef, to: &ModelRef) -> Result<usize, AppError> {
    let (opencode_bytes, opencode) =
        required_document(&paths.opencode_file(), "OpenCode configuration")?;
    ensure_models_exist(&opencode, from, to)?;
    if from == to {
        return Ok(0);
    }

    let mut replacements = 0;
    let mut targets: BTreeMap<PathBuf, JsoncTarget> = BTreeMap::new();
    add_jsonc_target(
        &mut targets,
        paths.opencode_file(),
        opencode_bytes.clone(),
        JsoncRole::OpenCode,
    );
    if let Some((original, document)) = optional_document(&paths.slim_file(), "Slim configuration")?
    {
        add_jsonc_target(&mut targets, paths.slim_file(), original, JsoncRole::Slim);
        drop(document);
    }
    if let Some((original, document)) = optional_document(&paths.omo_file(), "OMO configuration")? {
        add_jsonc_target(&mut targets, paths.omo_file(), original, JsoncRole::Omo);
        drop(document);
    }

    for target in targets.into_values() {
        let content = String::from_utf8(target.original).map_err(|error| {
            AppError::configuration(format!("read {}: {error}", target.path.display()))
        })?;
        let mapping = (from.clone(), to.clone());
        let mut updated = content.clone();
        let mut replacements_for_target = 0;
        for role in &target.roles {
            let (next, changed) =
                patch_jsonc_role(*role, &updated, std::slice::from_ref(&mapping), None)?;
            updated = next;
            replacements_for_target += changed;
        }
        replacements += replacements_for_target;
        if updated != content {
            JsoncDoc::parse(&updated)?.save(&target.path)?;
        }
    }

    for path in agents_markdown_files(paths)? {
        let raw = crate::document::read_text(&path)?.unwrap_or_default();
        let mut document = MarkdownAgentDocument::parse(&raw).map_err(|error| {
            AppError::validation(format!("parse Markdown agent {}: {error}", path.display()))
        })?;
        let Some(model) = document.frontmatter.get("model").and_then(Value::as_str) else {
            continue;
        };
        if model != from.as_str() {
            continue;
        }
        document.apply_fields(
            &serde_json::Map::from_iter([("model".to_owned(), Value::String(to.as_str()))]),
            None,
            true,
        )?;
        crate::document::write_file(&path, document.serialize().as_bytes())?;
        replacements += 1;
    }

    let config_path = paths.config_file();
    if config_path.exists() {
        let original = crate::document::read_text(&config_path)?.unwrap_or_default();
        let mut config: AppConfig = serde_json::from_str(&original).map_err(|error| {
            AppError::validation(format!("application config: invalid JSON: {error}"))
        })?;
        let changed = patch_groups(&mut config, from, to);
        if changed > 0 {
            crate::models::save_config(&config_path, &config)?;
            replacements += changed;
        }
    }
    Ok(replacements)
}

pub fn rename_model(
    paths: &ConfigPaths,
    from: &ModelRef,
    new_model_id: &str,
) -> Result<(), AppError> {
    let to = ModelRef::new(&from.provider_id, new_model_id)?;
    rename_with_mappings(
        paths,
        vec![(from.clone(), to.clone())],
        OpenCodeKeyRename::Model {
            from: from.clone(),
            to,
        },
    )
}

pub fn rename_provider(paths: &ConfigPaths, old_id: &str, new_id: &str) -> Result<(), AppError> {
    validate_rename_id(old_id, "provider")?;
    validate_rename_id(new_id, "provider")?;
    rename_with_mappings(
        paths,
        Vec::new(),
        OpenCodeKeyRename::Provider {
            old_id: old_id.to_owned(),
            new_id: new_id.to_owned(),
        },
    )
}

fn rename_with_mappings(
    paths: &ConfigPaths,
    mappings: Vec<(ModelRef, ModelRef)>,
    key_rename: OpenCodeKeyRename,
) -> Result<(), AppError> {
    let (opencode_bytes, opencode) =
        required_document(&paths.opencode_file(), "OpenCode configuration")?;
    validate_key_rename(&opencode, &key_rename)?;
    let mappings = match &key_rename {
        OpenCodeKeyRename::Provider { old_id, new_id } => {
            provider_model_mappings(&opencode, old_id, new_id)?
        }
        OpenCodeKeyRename::Model { .. } => mappings,
    };

    let mut targets: BTreeMap<PathBuf, JsoncTarget> = BTreeMap::new();
    add_jsonc_target(
        &mut targets,
        paths.opencode_file(),
        opencode_bytes,
        JsoncRole::OpenCode,
    );
    if let Some((original, _)) = optional_document(&paths.slim_file(), "Slim configuration")? {
        add_jsonc_target(&mut targets, paths.slim_file(), original, JsoncRole::Slim);
    }
    if let Some((original, _)) = optional_document(&paths.omo_file(), "OMO configuration")? {
        add_jsonc_target(&mut targets, paths.omo_file(), original, JsoncRole::Omo);
    }

    for target in targets.into_values() {
        let content = String::from_utf8(target.original).map_err(|error| {
            AppError::configuration(format!("read {}: {error}", target.path.display()))
        })?;
        let mut updated = content.clone();
        for role in &target.roles {
            let (next, _) = patch_jsonc_role(*role, &updated, &mappings, Some(&key_rename))?;
            updated = next;
        }
        if updated != content {
            JsoncDoc::parse(&updated)?.save(&target.path)?;
        }
    }

    for path in agents_markdown_files(paths)? {
        let raw = crate::document::read_text(&path)?.unwrap_or_default();
        let mut document = MarkdownAgentDocument::parse(&raw).map_err(|error| {
            AppError::validation(format!("parse Markdown agent {}: {error}", path.display()))
        })?;
        let Some(model) = document.frontmatter.get("model").and_then(Value::as_str) else {
            continue;
        };
        let Some(to) = replacement_for(model, &mappings) else {
            continue;
        };
        document.apply_fields(
            &serde_json::Map::from_iter([("model".to_owned(), Value::String(to.as_str()))]),
            None,
            true,
        )?;
        crate::document::write_file(&path, document.serialize().as_bytes())?;
    }

    let config_path = paths.config_file();
    if config_path.exists() {
        let original = crate::document::read_text(&config_path)?.unwrap_or_default();
        let mut config: AppConfig = serde_json::from_str(&original).map_err(|error| {
            AppError::validation(format!("application config: invalid JSON: {error}"))
        })?;
        let changed = patch_groups_mappings(&mut config, &mappings);
        if changed > 0 {
            crate::models::save_config(&config_path, &config)?;
        }
    }
    Ok(())
}

struct JsoncTarget {
    path: PathBuf,
    original: Vec<u8>,
    roles: Vec<JsoncRole>,
}

fn add_jsonc_target(
    targets: &mut BTreeMap<PathBuf, JsoncTarget>,
    path: PathBuf,
    original: Vec<u8>,
    role: JsoncRole,
) {
    match targets.get_mut(&path) {
        Some(target) => target.roles.push(role),
        None => {
            targets.insert(
                path.clone(),
                JsoncTarget {
                    path,
                    original,
                    roles: vec![role],
                },
            );
        }
    }
}

/// Applies model mappings (and optionally an OpenCode key rename) to one document role.
fn patch_jsonc_role(
    role: JsoncRole,
    source: &str,
    mappings: &[(ModelRef, ModelRef)],
    key_rename: Option<&OpenCodeKeyRename>,
) -> Result<(String, usize), AppError> {
    let mut document = parse_role(role, source)?;
    let mut changed = 0;
    for (from, to) in mappings {
        changed += match role {
            JsoncRole::OpenCode => patch_document_opencode(&mut document, from, to)?,
            JsoncRole::Slim => patch_document_slim(&mut document, from, to)?,
            JsoncRole::Omo => patch_document_omo(&mut document, from, to)?,
        };
    }
    if let (JsoncRole::OpenCode, Some(rename)) = (role, key_rename) {
        patch_opencode_key(&mut document, rename)?;
    }
    Ok((document.content().to_owned(), changed))
}

fn parse_role(role: JsoncRole, source: &str) -> Result<JsoncDoc, AppError> {
    let label = match role {
        JsoncRole::OpenCode => "OpenCode",
        JsoncRole::Slim => "Slim",
        JsoncRole::Omo => "OMO",
    };
    JsoncDoc::parse(source)
        .map_err(|error| AppError::validation(format!("parse {label} configuration: {error}")))
}

fn patch_opencode_key(document: &mut JsoncDoc, rename: &OpenCodeKeyRename) -> Result<(), AppError> {
    let (create_path, remove_path, value): (Vec<&str>, Vec<&str>, Option<Value>) = match rename {
        OpenCodeKeyRename::Model { from, to } => (
            vec![
                "provider",
                from.provider_id.as_str(),
                "models",
                to.model_id.as_str(),
            ],
            vec![
                "provider",
                from.provider_id.as_str(),
                "models",
                from.model_id.as_str(),
            ],
            document
                .raw()
                .get("provider")
                .and_then(Value::as_object)
                .and_then(|providers| providers.get(&from.provider_id))
                .and_then(Value::as_object)
                .and_then(|provider| provider.get("models"))
                .and_then(Value::as_object)
                .and_then(|models| models.get(&from.model_id))
                .cloned(),
        ),
        OpenCodeKeyRename::Provider { old_id, new_id } => (
            vec!["provider", new_id.as_str()],
            vec!["provider", old_id.as_str()],
            document
                .raw()
                .get("provider")
                .and_then(Value::as_object)
                .and_then(|providers| providers.get(old_id))
                .cloned(),
        ),
    };
    let value = value
        .ok_or_else(|| AppError::validation("OpenCode rename source disappeared while planning"))?;
    document.patch(&create_path, Some(value))?;
    document.patch(&remove_path, None)
}

fn validate_key_rename(document: &JsoncDoc, rename: &OpenCodeKeyRename) -> Result<(), AppError> {
    let providers = document
        .raw()
        .get("provider")
        .and_then(Value::as_object)
        .ok_or_else(|| AppError::validation("OpenCode provider must be an object"))?;
    match rename {
        OpenCodeKeyRename::Model { from, to } => {
            let models = providers
                .get(&from.provider_id)
                .and_then(Value::as_object)
                .and_then(|provider| provider.get("models"))
                .and_then(Value::as_object)
                .ok_or_else(|| {
                    AppError::validation(format!("provider '{}' does not exist", from.provider_id))
                })?;
            if !models.contains_key(&from.model_id) {
                return Err(AppError::validation(format!(
                    "model '{}' does not exist",
                    from.as_str()
                )));
            }
            if models.contains_key(&to.model_id) {
                return Err(AppError::validation(format!(
                    "model '{}' already exists",
                    to.as_str()
                )));
            }
        }
        OpenCodeKeyRename::Provider { old_id, new_id } => {
            if !providers.contains_key(old_id) {
                return Err(AppError::validation(format!(
                    "provider '{old_id}' does not exist"
                )));
            }
            if providers.contains_key(new_id) {
                return Err(AppError::validation(format!(
                    "provider '{new_id}' already exists"
                )));
            }
        }
    }
    Ok(())
}

fn provider_model_mappings(
    document: &JsoncDoc,
    old_id: &str,
    new_id: &str,
) -> Result<Vec<(ModelRef, ModelRef)>, AppError> {
    let models = document
        .raw()
        .get("provider")
        .and_then(Value::as_object)
        .and_then(|providers| providers.get(old_id))
        .and_then(Value::as_object)
        .and_then(|provider| provider.get("models"))
        .and_then(Value::as_object)
        .ok_or_else(|| {
            AppError::validation(format!("provider '{old_id}'.models must be an object"))
        })?
        .clone();
    models
        .keys()
        .map(|model_id| {
            Ok((
                ModelRef::new(old_id, model_id)?,
                ModelRef::new(new_id, model_id)?,
            ))
        })
        .collect()
}

fn validate_rename_id(value: &str, kind: &str) -> Result<(), AppError> {
    ModelRef::new(value, "placeholder")
        .map(|_| ())
        .map_err(|error| AppError::validation(format!("invalid {kind} ID '{value}': {error}")))
}

fn required_document(path: &std::path::Path, label: &str) -> Result<(Vec<u8>, JsoncDoc), AppError> {
    let bytes = std::fs::read(path).map_err(|error| AppError::io("read", path, error))?;
    let content = String::from_utf8(bytes.clone())
        .map_err(|error| AppError::validation(format!("read {label}: {error}")))?;
    let document = JsoncDoc::parse(&content)
        .map_err(|error| AppError::validation(format!("parse {label}: {error}")))?;
    Ok((bytes, document))
}

fn optional_document(
    path: &std::path::Path,
    label: &str,
) -> Result<Option<(Vec<u8>, JsoncDoc)>, AppError> {
    if !path.exists() {
        return Ok(None);
    }
    required_document(path, label).map(Some)
}

fn ensure_models_exist(
    document: &JsoncDoc,
    from: &ModelRef,
    to: &ModelRef,
) -> Result<(), AppError> {
    let providers = document
        .raw()
        .get("provider")
        .and_then(Value::as_object)
        .ok_or_else(|| AppError::validation("OpenCode provider must be an object"))?;
    for model_ref in [from, to] {
        let models = providers
            .get(&model_ref.provider_id)
            .and_then(Value::as_object)
            .and_then(|provider| provider.get("models"))
            .and_then(Value::as_object)
            .ok_or_else(|| {
                AppError::validation(format!(
                    "provider '{}'.models must be an object",
                    model_ref.provider_id
                ))
            })?;
        if !models.contains_key(&model_ref.model_id) {
            return Err(AppError::not_found(format!(
                "model '{}' does not exist",
                model_ref.as_str()
            )));
        }
    }
    Ok(())
}

fn patch_document_opencode(
    document: &mut JsoncDoc,
    from: &ModelRef,
    to: &ModelRef,
) -> Result<usize, AppError> {
    let Some(agents) = document.raw().get("agent") else {
        return Ok(0);
    };
    let agents = agents
        .as_object()
        .ok_or_else(|| AppError::validation("OpenCode agent must be an object"))?;
    let mut ids = Vec::new();
    for (id, agent) in agents {
        let agent = agent.as_object().ok_or_else(|| {
            AppError::validation(format!("OpenCode agent.{id} must be an object"))
        })?;
        if let Some(model) = agent.get("model") {
            let model = model.as_str().ok_or_else(|| {
                AppError::validation(format!("OpenCode agent.{id}.model must be a string"))
            })?;
            if model == from.as_str() {
                ids.push(id.clone());
            }
        }
    }
    for id in &ids {
        document.patch(&["agent", id, "model"], Some(Value::String(to.as_str())))?;
    }
    Ok(ids.len())
}

fn patch_groups(config: &mut AppConfig, from: &ModelRef, to: &ModelRef) -> usize {
    let mut replacements = 0;
    for group in &mut config.groups {
        let mut changed = false;
        for binding in &mut group.open_code_agent_overrides {
            changed |= replace_binding(&mut binding.model_ref, from, to, &mut replacements);
        }
        for binding in group.slim_agent_overrides.iter_mut().flatten() {
            changed |= replace_binding(&mut binding.model_ref, from, to, &mut replacements);
        }
        for binding in group.omo_agent_overrides.iter_mut().flatten() {
            changed |= replace_binding(&mut binding.model_ref, from, to, &mut replacements);
        }
        for mapping in group.omo_category_mappings.iter_mut().flatten() {
            changed |= replace_binding(&mut mapping.model_ref, from, to, &mut replacements);
        }
        if changed {
            group.updated_at = OffsetDateTime::now_utc();
        }
    }
    replacements
}

fn patch_groups_mappings(config: &mut AppConfig, mappings: &[(ModelRef, ModelRef)]) -> usize {
    let mut replacements = 0;
    for group in &mut config.groups {
        let mut changed = false;
        for binding in &mut group.open_code_agent_overrides {
            changed |= replace_mapped_binding(&mut binding.model_ref, mappings, &mut replacements);
        }
        for binding in group.slim_agent_overrides.iter_mut().flatten() {
            changed |= replace_mapped_binding(&mut binding.model_ref, mappings, &mut replacements);
        }
        for binding in group.omo_agent_overrides.iter_mut().flatten() {
            changed |= replace_mapped_binding(&mut binding.model_ref, mappings, &mut replacements);
        }
        for mapping in group.omo_category_mappings.iter_mut().flatten() {
            changed |= replace_mapped_binding(&mut mapping.model_ref, mappings, &mut replacements);
        }
        if changed {
            group.updated_at = OffsetDateTime::now_utc();
        }
    }
    replacements
}

fn replace_mapped_binding(
    value: &mut String,
    mappings: &[(ModelRef, ModelRef)],
    count: &mut usize,
) -> bool {
    let Some(to) = replacement_for(value, mappings) else {
        return false;
    };
    *value = to.as_str();
    *count += 1;
    true
}

fn replacement_for<'a>(value: &str, mappings: &'a [(ModelRef, ModelRef)]) -> Option<&'a ModelRef> {
    mappings
        .iter()
        .find(|(from, _)| value == from.as_str())
        .map(|(_, to)| to)
}

fn replace_binding(value: &mut String, from: &ModelRef, to: &ModelRef, count: &mut usize) -> bool {
    if *value == from.as_str() {
        *value = to.as_str();
        *count += 1;
        true
    } else {
        false
    }
}

fn patch_document_slim(
    document: &mut JsoncDoc,
    from: &ModelRef,
    to: &ModelRef,
) -> Result<usize, AppError> {
    let Some(presets) = document.raw().get("presets") else {
        return Ok(0);
    };
    let presets = presets
        .as_object()
        .ok_or_else(|| AppError::validation("Slim presets must be an object"))?;
    let mut paths = Vec::new();
    for (preset_name, preset) in presets {
        let preset = preset.as_object().ok_or_else(|| {
            AppError::validation(format!("Slim presets.{preset_name} must be an object"))
        })?;
        for (agent_name, agent) in preset {
            let agent = agent.as_object().ok_or_else(|| {
                AppError::validation(format!(
                    "Slim presets.{preset_name}.{agent_name} must be an object"
                ))
            })?;
            if let Some(model) = agent.get("model") {
                let model = model.as_str().ok_or_else(|| {
                    AppError::validation(format!(
                        "Slim presets.{preset_name}.{agent_name}.model must be a string"
                    ))
                })?;
                if model == from.as_str() {
                    paths.push((preset_name.clone(), agent_name.clone()));
                }
            }
        }
    }
    for (preset, agent) in &paths {
        document.patch(
            &["presets", preset, agent, "model"],
            Some(Value::String(to.as_str())),
        )?;
    }
    Ok(paths.len())
}

fn patch_document_omo(
    document: &mut JsoncDoc,
    from: &ModelRef,
    to: &ModelRef,
) -> Result<usize, AppError> {
    let Some(opencode) = document.raw().get(OMO_OPENCODE_BLOCK) else {
        return Ok(0);
    };
    let opencode = opencode
        .as_object()
        .ok_or_else(|| AppError::validation("OMO opencode must be an object"))?;
    let mut paths = Vec::new();
    for kind in ["agents", "categories"] {
        let Some(entries) = opencode.get(kind) else {
            continue;
        };
        let entries = entries.as_object().ok_or_else(|| {
            AppError::validation(format!("OMO opencode.{kind} must be an object"))
        })?;
        for (name, entry) in entries {
            let entry = entry.as_object().ok_or_else(|| {
                AppError::validation(format!("OMO opencode.{kind}.{name} must be an object"))
            })?;
            if let Some(model) = entry.get("model") {
                let model = model.as_str().ok_or_else(|| {
                    AppError::validation(format!(
                        "OMO opencode.{kind}.{name}.model must be a string"
                    ))
                })?;
                if model == from.as_str() {
                    paths.push((kind.to_owned(), name.clone()));
                }
            }
        }
    }
    for (kind, name) in &paths {
        document.patch(
            &[OMO_OPENCODE_BLOCK, kind, name, "model"],
            Some(Value::String(to.as_str())),
        )?;
    }
    Ok(paths.len())
}

fn scan_slim(
    document: &JsoncDoc,
    references: &mut Vec<crate::refs::ModelReference>,
) -> Result<(), AppError> {
    let Some(presets) = document.raw().get("presets") else {
        return Ok(());
    };
    let presets = presets
        .as_object()
        .ok_or_else(|| AppError::validation("Slim presets must be an object"))?;
    for (preset, entries) in presets {
        let entries = entries.as_object().ok_or_else(|| {
            AppError::validation(format!("Slim presets.{preset} must be an object"))
        })?;
        for (agent, entry) in entries {
            let entry = entry.as_object().ok_or_else(|| {
                AppError::validation(format!("Slim presets.{preset}.{agent} must be an object"))
            })?;
            if let Some(model) = entry.get("model").and_then(Value::as_str) {
                if !model.is_empty() {
                    references.push(crate::refs::ModelReference {
                        source: crate::refs::ReferenceSource::SlimPreset,
                        location: format!("presets.{preset}.{agent}.model"),
                        model_ref: ModelRef::parse(model)?,
                    });
                }
            }
        }
    }
    Ok(())
}

fn scan_omo(
    document: &JsoncDoc,
    references: &mut Vec<crate::refs::ModelReference>,
) -> Result<(), AppError> {
    let Some(opencode) = document.raw().get(OMO_OPENCODE_BLOCK) else {
        return Ok(());
    };
    let opencode = opencode
        .as_object()
        .ok_or_else(|| AppError::validation("OMO opencode must be an object"))?;
    for (key, source) in [
        ("agents", crate::refs::ReferenceSource::OmoAgent),
        ("categories", crate::refs::ReferenceSource::OmoCategory),
    ] {
        let Some(entries) = opencode.get(key) else {
            continue;
        };
        let entries = entries
            .as_object()
            .ok_or_else(|| AppError::validation(format!("OMO opencode.{key} must be an object")))?;
        for (name, entry) in entries {
            let entry = entry.as_object().ok_or_else(|| {
                AppError::validation(format!("OMO opencode.{key}.{name} must be an object"))
            })?;
            if let Some(model) = entry.get("model").and_then(Value::as_str) {
                if !model.is_empty() {
                    references.push(crate::refs::ModelReference {
                        source,
                        location: format!("{OMO_OPENCODE_BLOCK}.{key}.{name}.model"),
                        model_ref: ModelRef::parse(model)?,
                    });
                }
            }
        }
    }
    Ok(())
}

fn agents_markdown_files(paths: &ConfigPaths) -> Result<Vec<std::path::PathBuf>, AppError> {
    crate::agents::collect_markdown_files_in_roots(paths)
}
