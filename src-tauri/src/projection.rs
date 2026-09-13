use serde_json::{Map, Value};

use crate::document::JsoncDoc;
use crate::models::{AgentModelBinding, GroupType, ModelGroup, OmoCategoryMapping};

/// Projects the active Slim group: merges its preset entries and activates the preset.
pub fn project_slim_active(
    group: &ModelGroup,
    source: &JsoncDoc,
) -> Result<JsoncDoc, crate::error::AppError> {
    let mut document = source.clone();
    if group.group_type != GroupType::Slim {
        return Ok(document);
    }
    merge_slim_preset(
        &mut document,
        &group.name,
        group.slim_agent_overrides.as_deref(),
    );
    document.patch(&["preset"], Some(Value::String(group.name.clone())))?;
    Ok(document)
}

/// Merges the group's Slim overrides into its named preset, preserving residual entries.
pub fn update_slim_preset(
    source: &JsoncDoc,
    name: &str,
    overrides: Option<&[AgentModelBinding]>,
) -> Result<JsoncDoc, crate::error::AppError> {
    let mut document = source.clone();
    merge_slim_preset(&mut document, name, overrides);
    Ok(document)
}

pub fn rename_slim_preset(
    source: &JsoncDoc,
    old_name: &str,
    new_name: &str,
) -> Result<JsoncDoc, crate::error::AppError> {
    let mut document = source.clone();
    if let Some(preset) = slim_preset(source, old_name) {
        let entries: Vec<(String, Value)> =
            preset.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        for (key, value) in entries {
            document.patch(&["presets", new_name, key.as_str()], Some(value))?;
        }
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

fn merge_slim_preset(document: &mut JsoncDoc, name: &str, overrides: Option<&[AgentModelBinding]>) {
    let bindings: Vec<(&str, Value)> = overrides
        .unwrap_or_default()
        .iter()
        .filter(|binding| {
            !binding.agent_name.trim().is_empty() && !binding.model_ref.trim().is_empty()
        })
        .map(|binding| (binding.agent_name.as_str(), binding_value(binding)))
        .collect();
    if bindings.is_empty() {
        if slim_preset(document, name).is_none() {
            document
                .patch(&["presets", name], Some(Value::Object(Map::new())))
                .expect("a parsed JSONC document accepts a preset creation");
        }
        return;
    }
    for (agent, value) in bindings {
        document
            .patch(&["presets", name, agent], Some(value))
            .expect("a parsed JSONC document accepts a preset patch");
    }
}

fn binding_value(binding: &AgentModelBinding) -> Value {
    model_value(&binding.model_ref, binding.variant.as_deref())
}

/// Patches the freeform `opencode` agent/category mappings of an active OMO group.
/// Existing entries not represented by the group are residual and left untouched.
pub fn project_omo(
    group: &ModelGroup,
    source: &JsoncDoc,
) -> Result<JsoncDoc, crate::error::AppError> {
    let mut document = source.clone();
    if group.group_type != GroupType::Omo {
        return Ok(document);
    }
    for (name, value) in mapping_values(group.omo_agent_overrides.as_deref().unwrap_or_default()) {
        document.patch(&["opencode", "agents", name.as_str()], Some(value))?;
    }
    for (name, value) in category_values(group.omo_category_mappings.as_deref().unwrap_or_default())
    {
        document.patch(&["opencode", "categories", name.as_str()], Some(value))?;
    }
    Ok(document)
}

/// Removes only the OMO mappings owned by the group; residual entries are preserved.
pub fn remove_omo_mappings(
    group: &ModelGroup,
    source: &JsoncDoc,
) -> Result<JsoncDoc, crate::error::AppError> {
    let mut document = source.clone();
    for (name, _) in mapping_values(group.omo_agent_overrides.as_deref().unwrap_or_default()) {
        document.patch(&["opencode", "agents", name.as_str()], None)?;
    }
    for (name, _) in category_values(group.omo_category_mappings.as_deref().unwrap_or_default()) {
        document.patch(&["opencode", "categories", name.as_str()], None)?;
    }
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
    if group.group_type != GroupType::Native {
        return Ok((document, warnings));
    }
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

/// Removes only the OpenCode model/variant keys owned by the group; unrelated agent
/// fields are preserved.
pub fn remove_opencode_overrides(
    group: &ModelGroup,
    source: &JsoncDoc,
) -> Result<JsoncDoc, crate::error::AppError> {
    let mut document = source.clone();
    for binding in group
        .open_code_agent_overrides
        .iter()
        .filter(|binding| is_effective_binding(binding))
    {
        document.patch(&["agent", binding.agent_name.as_str(), "model"], None)?;
        document.patch(&["agent", binding.agent_name.as_str(), "variant"], None)?;
    }
    Ok(document)
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
