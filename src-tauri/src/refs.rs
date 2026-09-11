use serde_json::Value;

use crate::agents::{self, AgentReference, AgentReferenceIndex, AgentReferenceKind};
use crate::error::AppError;
use crate::models::AppConfig;
use crate::paths::ConfigPaths;
use crate::providers::ModelRef;

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
    references.extend(crate::replacement::scan_plugin_references(paths)?);
    Ok(ModelReferenceIndex::new(references))
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
            scan_permission_task_references(
                &mut references,
                &inline.raw,
                &format!("agent.{}.inline", definition.id),
            );
        }
        for markdown in definition.markdown {
            scan_permission_task_references(
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
    if let Some(commands) = opencode.get("command").and_then(Value::as_object) {
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
                    format!("command.{name}.agent"),
                );
            }
        }
    }
    scan_permission_task_references(&mut references, &Value::Object(document.raw().clone()), "");
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

fn scan_permission_task_references(
    references: &mut AgentReferenceIndex,
    value: &Value,
    owner_prefix: &str,
) {
    let Some(object) = value.as_object() else {
        if let Some(values) = value.as_array() {
            for nested in values {
                scan_permission_task_references(references, nested, owner_prefix);
            }
        }
        return;
    };
    if let Some(tasks) = object
        .get("permission")
        .and_then(Value::as_object)
        .and_then(|permission| permission.get("task"))
        .and_then(Value::as_object)
    {
        for agent in tasks.keys() {
            let owner = if owner_prefix.is_empty() {
                format!("permission.task.{agent}")
            } else {
                format!("{owner_prefix}.permission.task.{agent}")
            };
            add_agent_reference(references, agent, AgentReferenceKind::PermissionTask, owner);
        }
    }
    for (key, nested) in object {
        let nested_prefix = if owner_prefix.is_empty() {
            key.clone()
        } else {
            format!("{owner_prefix}.{key}")
        };
        scan_permission_task_references(references, nested, &nested_prefix);
    }
}

/// Convenience wrapper used by provider deletion protection.
pub fn provider_is_referenced(
    paths: &ConfigPaths,
    provider_id: &str,
) -> Result<Vec<ModelReference>, AppError> {
    Ok(collect_model_references(paths)?.for_provider(provider_id))
}

/// Convenience wrapper used by model deletion protection.
pub fn model_is_referenced(
    paths: &ConfigPaths,
    model_ref: &ModelRef,
) -> Result<Vec<ModelReference>, AppError> {
    Ok(collect_model_references(paths)?.for_model(model_ref))
}
