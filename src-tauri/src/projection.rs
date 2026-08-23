use serde_json::{Map, Value};

use crate::document::JsoncDoc;
use crate::models::{AgentModelBinding, GroupType, ModelGroup, OmoCategoryMapping};

/// Projects the active Slim group: replaces the preset and activates it.
pub fn project_slim_active(
    group: &ModelGroup,
    source: &JsoncDoc,
) -> Result<JsoncDoc, crate::error::AppError> {
    let mut document = source.clone();
    if group.group_type != GroupType::Slim {
        return Ok(document);
    }
    replace_slim_preset(
        &mut document,
        &group.name,
        group.slim_agent_overrides.as_deref(),
    );
    document.patch(&["preset"], Some(Value::String(group.name.clone())))?;
    Ok(document)
}

pub fn update_slim_preset(
    source: &JsoncDoc,
    name: &str,
    overrides: Option<&[AgentModelBinding]>,
) -> Result<JsoncDoc, crate::error::AppError> {
    let mut document = source.clone();
    replace_slim_preset(&mut document, name, overrides);
    Ok(document)
}

pub fn rename_slim_preset(
    source: &JsoncDoc,
    old_name: &str,
    new_name: &str,
) -> Result<JsoncDoc, crate::error::AppError> {
    let mut document = source.clone();
    if slim_preset(source, old_name).is_some() {
        let preset = slim_preset(source, old_name).expect("preset existence checked above");
        document.patch(&["presets", new_name], Some(Value::Object(preset.clone())))?;
        document.patch(&["presets", old_name], None)?;
    }
    if active_slim_preset(source) == Some(old_name) {
        document.patch(&["preset"], Some(Value::String(new_name.to_owned())))?;
    }
    Ok(document)
}

pub fn delete_slim_preset(
    source: &JsoncDoc,
    name: &str,
) -> Result<JsoncDoc, crate::error::AppError> {
    let mut document = source.clone();
    document.patch(&["presets", name], None)?;
    if active_slim_preset(source) == Some(name) {
        document.patch(&["preset"], None)?;
    }
    Ok(document)
}

fn slim_preset<'a>(document: &'a JsoncDoc, name: &str) -> Option<&'a Map<String, Value>> {
    document
        .raw()
        .get("presets")
        .and_then(Value::as_object)
        .and_then(|presets| presets.get(name))
        .and_then(Value::as_object)
}

fn active_slim_preset(document: &JsoncDoc) -> Option<&str> {
    document.raw().get("preset").and_then(Value::as_str)
}

fn replace_slim_preset(
    document: &mut JsoncDoc,
    name: &str,
    overrides: Option<&[AgentModelBinding]>,
) {
    let agents = overrides
        .unwrap_or_default()
        .iter()
        .filter(|binding| {
            !binding.agent_name.trim().is_empty() && !binding.model_ref.trim().is_empty()
        })
        .map(|binding| (binding.agent_name.clone(), binding_value(binding)))
        .collect();
    document
        .patch(&["presets", name], Some(Value::Object(agents)))
        .expect("a parsed JSONC document accepts a preset replacement");
}

fn binding_value(binding: &AgentModelBinding) -> Value {
    model_value(&binding.model_ref, binding.variant.as_deref())
}

/// Replaces the freeform `opencode` agent/category mappings of an active OMO group.
/// Existing entries not represented by the group are deliberately removed.
pub fn project_omo(
    group: &ModelGroup,
    source: &JsoncDoc,
) -> Result<JsoncDoc, crate::error::AppError> {
    let mut document = source.clone();
    if group.group_type != GroupType::OhMyOpenagent {
        return Ok(document);
    }
    let agents = mapping_values(group.omo_agent_overrides.as_deref().unwrap_or_default());
    let categories = category_values(group.omo_category_mappings.as_deref().unwrap_or_default());
    document.patch(&["opencode", "agents"], Some(Value::Object(agents)))?;
    document.patch(&["opencode", "categories"], Some(Value::Object(categories)))?;
    Ok(document)
}

/// Removes mappings owned by the currently selected OMO group while preserving all other
/// OMO configuration.
pub fn clear_omo(source: &JsoncDoc) -> Result<JsoncDoc, crate::error::AppError> {
    let mut document = source.clone();
    document.patch(&["opencode", "agents"], Some(Value::Object(Map::new())))?;
    document.patch(&["opencode", "categories"], Some(Value::Object(Map::new())))?;
    Ok(document)
}

pub fn has_effective_opencode_overrides(group: &ModelGroup) -> bool {
    group
        .open_code_agent_overrides
        .iter()
        .any(is_effective_binding)
}

/// Patches native OpenCode agent models/variants for the group. Never creates agents;
/// missing ones produce warnings.
pub fn project_opencode(
    group: &ModelGroup,
    source: &JsoncDoc,
) -> Result<(JsoncDoc, Vec<String>), crate::error::AppError> {
    let mut document = source.clone();
    let mut warnings = Vec::new();
    if !document.raw().get("agent").is_some_and(Value::is_object) {
        for binding in group
            .open_code_agent_overrides
            .iter()
            .filter(|binding| is_effective_binding(binding))
        {
            warnings.push(format!(
                "OpenCode config has no valid top-level 'agent' object; skipped model override for '{}'.",
                binding.agent_name
            ));
        }
        return Ok((document, warnings));
    }
    for binding in group
        .open_code_agent_overrides
        .iter()
        .filter(|binding| is_effective_binding(binding))
    {
        let agents = document.raw().get("agent").and_then(Value::as_object);
        if !agents
            .and_then(|agents| agents.get(&binding.agent_name))
            .is_some_and(Value::is_object)
        {
            warnings.push(format!(
                "OpenCode agent '{}' was not found; skipped model override.",
                binding.agent_name
            ));
            continue;
        }
        let path = ["agent", binding.agent_name.as_str(), "model"];
        document.patch(&path, Some(Value::String(binding.model_ref.clone())))?;
        let variant_path = ["agent", binding.agent_name.as_str(), "variant"];
        let variant = binding
            .variant
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(|value| Value::String(value.to_owned()));
        document.patch(&variant_path, variant)?;
    }
    Ok((document, warnings))
}

fn is_effective_binding(binding: &AgentModelBinding) -> bool {
    !binding.agent_name.trim().is_empty() && !binding.model_ref.trim().is_empty()
}

fn mapping_values(bindings: &[AgentModelBinding]) -> Map<String, Value> {
    bindings
        .iter()
        .filter(|binding| is_effective_binding(binding))
        .map(|binding| {
            (
                binding.agent_name.clone(),
                model_value(&binding.model_ref, binding.variant.as_deref()),
            )
        })
        .collect()
}

fn category_values(bindings: &[OmoCategoryMapping]) -> Map<String, Value> {
    bindings
        .iter()
        .filter(|binding| {
            !binding.category_name.trim().is_empty() && !binding.model_ref.trim().is_empty()
        })
        .map(|binding| {
            (
                binding.category_name.clone(),
                model_value(&binding.model_ref, binding.variant.as_deref()),
            )
        })
        .collect()
}

fn model_value(model: &str, variant: Option<&str>) -> Value {
    let mut value = Map::new();
    value.insert("model".to_owned(), Value::String(model.to_owned()));
    if let Some(variant) = variant.filter(|value| !value.trim().is_empty()) {
        value.insert("variant".to_owned(), Value::String(variant.to_owned()));
    }
    Value::Object(value)
}
