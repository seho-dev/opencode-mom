use serde_json::Value;

use crate::agents::{self, AgentReference, AgentReferenceIndex, AgentReferenceKind};
use crate::document::JsoncDoc;
use crate::error::AppError;
use crate::models::AppConfig;
use crate::paths::ConfigPaths;
use crate::providers::ModelRef;

// OMO stores OpenCode-specific mappings under the literal `opencode` object.
const OMO_OPENCODE_BLOCK: &str = "opencode";

/// Where a model reference was found. Displayed to users when deletions are blocked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReferenceSource {
    OpenCodeInlineAgent,
    MarkdownAgent,
    ApplicationGroup,
    SlimPreset,
    OmoAgent,
    OmoCategory,
}

impl ReferenceSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OpenCodeInlineAgent => "opencode-inline-agent",
            Self::MarkdownAgent => "markdown-agent",
            Self::ApplicationGroup => "application-group",
            Self::SlimPreset => "slim-preset",
            Self::OmoAgent => "omo-agent",
            Self::OmoCategory => "omo-category",
        }
    }
}

/// One real model binding with a displayable location.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ModelReference {
    pub source: ReferenceSource,
    pub location: String,
    pub model_ref: ModelRef,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModelReferenceIndex {
    references: Vec<ModelReference>,
}

impl ModelReferenceIndex {
    pub fn new(references: impl IntoIterator<Item = ModelReference>) -> Self {
        let mut references: Vec<_> = references.into_iter().collect();
        references.sort();
        references.dedup();
        Self { references }
    }

    pub fn for_model(&self, model_ref: &ModelRef) -> Vec<ModelReference> {
        self.references
            .iter()
            .filter(|reference| &reference.model_ref == model_ref)
            .cloned()
            .collect()
    }

    pub fn for_provider(&self, provider_id: &str) -> Vec<ModelReference> {
        self.references
            .iter()
            .filter(|reference| reference.model_ref.provider_id == provider_id)
            .cloned()
            .collect()
    }
}

/// Builds the index used to protect models from deletion while still referenced.
pub fn collect_model_references(paths: &ConfigPaths) -> Result<ModelReferenceIndex, AppError> {
    let mut references = agents::model_references(paths)?;
    let config = crate::models::load_config(&paths.config_file())?;
    for group in config.groups {
        let group_id = group.id;
        let group_name = group.name.clone();
        add_group_references(
            &mut references,
            group.open_code_agent_overrides.into_iter(),
            group_id,
            &group_name,
            "openCodeAgentOverrides",
        )?;
        if let Some(bindings) = group.slim_agent_overrides {
            add_group_references(
                &mut references,
                bindings.into_iter(),
                group_id,
                &group_name,
                "slimAgentOverrides",
            )?;
        }
        if let Some(bindings) = group.omo_agent_overrides {
            add_group_references(
                &mut references,
                bindings.into_iter(),
                group_id,
                &group_name,
                "omoAgentOverrides",
            )?;
        }
        if let Some(mappings) = group.omo_category_mappings {
            for mapping in mappings {
                references.push(ModelReference {
                    source: ReferenceSource::ApplicationGroup,
                    location: format!(
                        "group.{group_id}.omoCategoryMappings.{}.model",
                        mapping.category_name
                    ),
                    model_ref: ModelRef::parse(&mapping.model_ref).map_err(|error| {
                        AppError::validation(format!(
                            "invalid model reference in group '{}', category '{}': {error}",
                            group_name, mapping.category_name
                        ))
                    })?,
                });
            }
        }
    }
    references.extend(scan_plugin_references(paths)?);
    Ok(ModelReferenceIndex::new(references))
}

/// Scans Slim and OMO files for model references used by deletion protection.
pub fn scan_plugin_references(paths: &ConfigPaths) -> Result<Vec<ModelReference>, AppError> {
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

fn scan_slim(document: &JsoncDoc, references: &mut Vec<ModelReference>) -> Result<(), AppError> {
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
                    references.push(ModelReference {
                        source: ReferenceSource::SlimPreset,
                        location: format!("presets.{preset}.{agent}.model"),
                        model_ref: ModelRef::parse(model)?,
                    });
                }
            }
        }
    }
    Ok(())
}

fn scan_omo(document: &JsoncDoc, references: &mut Vec<ModelReference>) -> Result<(), AppError> {
    let Some(opencode) = document.raw().get(OMO_OPENCODE_BLOCK) else {
        return Ok(());
    };
    let opencode = opencode
        .as_object()
        .ok_or_else(|| AppError::validation("OMO opencode must be an object"))?;
    for (key, source) in [
        ("agents", ReferenceSource::OmoAgent),
        ("categories", ReferenceSource::OmoCategory),
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
                    references.push(ModelReference {
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

fn add_group_references(
    references: &mut Vec<ModelReference>,
    bindings: impl IntoIterator<Item = crate::models::AgentModelBinding>,
    group_id: uuid::Uuid,
    group_name: &str,
    field: &str,
) -> Result<(), AppError> {
    for binding in bindings {
        references.push(ModelReference {
            source: ReferenceSource::ApplicationGroup,
            location: format!("group.{group_id}.{field}.{}.model", binding.agent_name),
            model_ref: ModelRef::parse(&binding.model_ref).map_err(|error| {
                AppError::validation(format!(
                    "invalid model reference in group '{group_name}', field '{field}', agent '{}': {error}",
                    binding.agent_name
                ))
            })?,
        });
    }
    Ok(())
}

/// Formats the locations of blocking model references into a single detail string.
pub fn model_reference_locations(references: &[ModelReference]) -> String {
    references
        .iter()
        .map(|reference| format!("{} at {}", reference.source.as_str(), reference.location))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Collects every agent name referenced by groups and the native OpenCode config.
pub fn collect_agent_references(paths: &ConfigPaths) -> Result<AgentReferenceIndex, AppError> {
    let mut references = AgentReferenceIndex::new();
    for definition in agents::list(paths)? {
        if let Some(inline) = definition.inline {
            scan_permission_references(
                &mut references,
                &inline.raw,
                &format!("agents.{}.inline", definition.id),
            );
        }
        for markdown in definition.markdown {
            scan_permission_references(
                &mut references,
                &Value::Object(markdown.frontmatter),
                &format!("{}:{}", markdown.path.display(), definition.id),
            );
        }
    }
    let config: AppConfig = crate::models::load_config(&paths.config_file())?;
    for group in config.groups {
        let owner_prefix = format!("group.{}", group.id);
        add_group_agent_references(
            &mut references,
            &group.open_code_agent_overrides,
            &format!("{owner_prefix}.openCodeAgentOverrides"),
        );
        if let Some(bindings) = &group.slim_agent_overrides {
            add_group_agent_references(
                &mut references,
                bindings,
                &format!("{owner_prefix}.slimAgentOverrides"),
            );
        }
        if let Some(bindings) = &group.omo_agent_overrides {
            add_group_agent_references(
                &mut references,
                bindings,
                &format!("{owner_prefix}.omoAgentOverrides"),
            );
        }
    }

    let document =
        crate::document::JsoncDoc::read(&paths.opencode_file(), crate::document::OPENCODE_SCHEMA)?;
    let opencode = document.raw();
    if let Some(agent) = opencode.get("default_agent").and_then(Value::as_str) {
        add_agent_reference(
            &mut references,
            agent,
            AgentReferenceKind::DefaultAgent,
            "default_agent".to_owned(),
        );
    }
    if let Some(commands) = opencode.get("commands").and_then(Value::as_object) {
        for (name, command) in commands {
            if let Some(agent) = command
                .as_object()
                .and_then(|command| command.get("agent"))
                .and_then(Value::as_str)
            {
                add_agent_reference(
                    &mut references,
                    agent,
                    AgentReferenceKind::Command,
                    format!("commands.{name}.agent"),
                );
            }
        }
    }
    scan_permission_references(&mut references, &Value::Object(document.raw().clone()), "");
    Ok(references)
}

fn add_group_agent_references(
    references: &mut AgentReferenceIndex,
    bindings: &[crate::models::AgentModelBinding],
    owner_prefix: &str,
) {
    for binding in bindings {
        add_agent_reference(
            references,
            &binding.agent_name,
            AgentReferenceKind::Group,
            format!("{owner_prefix}.{}.agentName", binding.agent_name),
        );
    }
}

fn add_agent_reference(
    references: &mut AgentReferenceIndex,
    agent_id: &str,
    kind: AgentReferenceKind,
    owner: String,
) {
    if agent_id.is_empty() {
        return;
    }
    references
        .entry(agent_id.to_owned())
        .or_default()
        .push(AgentReference { kind, owner });
}

/// Walks a config subtree for V2 `permissions` arrays. Each `subagent` rule with a
/// concrete (wildcard-free) resource references that agent ID.
fn scan_permission_references(
    references: &mut AgentReferenceIndex,
    value: &Value,
    owner_prefix: &str,
) {
    let Some(object) = value.as_object() else {
        if let Some(values) = value.as_array() {
            for nested in values {
                scan_permission_references(references, nested, owner_prefix);
            }
        }
        return;
    };
    if let Some(rules) = object.get("permissions").and_then(Value::as_array) {
        for (index, rule) in rules.iter().enumerate() {
            let Some(rule) = rule.as_object() else {
                continue;
            };
            if rule.get("action").and_then(Value::as_str) != Some("subagent") {
                continue;
            }
            let Some(agent) = rule.get("resource").and_then(Value::as_str) else {
                continue;
            };
            if agent.is_empty() || agent.contains(['*', '?']) {
                continue;
            }
            let owner = if owner_prefix.is_empty() {
                format!("permissions[{index}]")
            } else {
                format!("{owner_prefix}.permissions[{index}]")
            };
            add_agent_reference(references, agent, AgentReferenceKind::PermissionTask, owner);
        }
    }
    for (key, nested) in object {
        if owner_prefix.is_empty() && key == "agents" {
            continue;
        }
        let nested_prefix = if owner_prefix.is_empty() {
            key.clone()
        } else {
            format!("{owner_prefix}.{key}")
        };
        scan_permission_references(references, nested, &nested_prefix);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    struct TestEnv {
        home: PathBuf,
        paths: ConfigPaths,
    }

    impl TestEnv {
        fn new() -> Self {
            let home = std::env::temp_dir()
                .join(format!("opencode-mom-refs-test-{}", uuid::Uuid::new_v4()));
            let paths = ConfigPaths::for_home(&home);
            fs::create_dir_all(paths.config_file().parent().unwrap()).unwrap();
            fs::create_dir_all(paths.opencode_file().parent().unwrap()).unwrap();
            Self { home, paths }
        }
    }

    impl Drop for TestEnv {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.home);
        }
    }

    #[test]
    fn inline_agent_with_subagent_permission_produces_single_reference() {
        let env = TestEnv::new();
        fs::write(env.paths.config_file(), r#"{"groups":[]}"#).unwrap();
        let opencode_content = serde_json::json!({
            "agents": {
                "caller": {
                    "permissions": [
                        {
                            "action": "subagent",
                            "resource": "worker"
                        }
                    ]
                }
            }
        });
        fs::write(env.paths.opencode_file(), opencode_content.to_string()).unwrap();

        let references =
            collect_agent_references(&env.paths).expect("collect agent references succeeds");
        let worker_refs = references
            .get("worker")
            .expect("worker should be referenced");

        assert_eq!(
            worker_refs.len(),
            1,
            "worker should only be referenced once: {worker_refs:?}"
        );
        assert_eq!(worker_refs[0].owner, "agents.caller.inline.permissions[0]");
        assert_eq!(worker_refs[0].kind, AgentReferenceKind::PermissionTask);
    }
}
