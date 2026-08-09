use serde_json::{Map, Value};

use crate::core::document::{OhMyOpenAgentDocument, OpenCodeDocument};
use crate::core::models::ModelGroup;

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectionResult<D> {
    pub document: D,
    pub warnings: Vec<String>,
}

pub struct OhMyOpenAgentProjectionService;

impl OhMyOpenAgentProjectionService {
    pub fn project(
        group: &ModelGroup,
        existing_document: &OhMyOpenAgentDocument,
    ) -> ProjectionResult<OhMyOpenAgentDocument> {
        let mut raw = existing_document.raw().clone();
        let agents = project_section(
            existing_document.agents(),
            group.agent_overrides.iter().filter_map(|override_row| {
                let model_ref = override_row.model_ref.trim();
                (!model_ref.is_empty()).then(|| (override_row.agent_name.as_str(), model_ref))
            }),
        );
        let categories = project_section(
            existing_document.categories(),
            group.category_mappings.iter().filter_map(|mapping| {
                let model_ref = mapping.model_ref.trim();
                (!model_ref.is_empty()).then(|| (mapping.category_name.as_str(), model_ref))
            }),
        );

        raw.insert("agents".to_owned(), Value::Object(agents));
        raw.insert("categories".to_owned(), Value::Object(categories));

        ProjectionResult {
            document: OhMyOpenAgentDocument::from_raw(raw),
            warnings: Vec::new(),
        }
    }
}

pub struct OpenCodeProjectionService;

impl OpenCodeProjectionService {
    pub fn project(
        group: &ModelGroup,
        existing_document: &OpenCodeDocument,
    ) -> ProjectionResult<OpenCodeDocument> {
        let effective_overrides: Vec<(&str, &str)> = group
            .open_code_agent_overrides
            .iter()
            .filter_map(|override_row| {
                let model_ref = override_row.model_ref.trim();
                (!model_ref.is_empty()).then(|| (override_row.agent_name.as_str(), model_ref))
            })
            .collect();
        let mut raw = existing_document.raw().clone();
        let Some(Value::Object(existing_agents)) = raw.get("agent") else {
            let warnings = effective_overrides
                .iter()
                .map(|(agent_name, _model_ref)| {
                    format!(
                        "OpenCode config has no valid top-level 'agent' object; skipped model override for '{agent_name}'."
                    )
                })
                .collect();
            return ProjectionResult {
                document: existing_document.clone(),
                warnings,
            };
        };

        let mut agents = existing_agents.clone();
        let mut warnings = Vec::new();
        for (agent_name, model_ref) in effective_overrides {
            match agents.get(agent_name) {
                Some(Value::Object(entry)) => {
                    let mut projected_entry = entry.clone();
                    projected_entry.insert("model".to_owned(), Value::String(model_ref.to_owned()));
                    agents.insert(agent_name.to_owned(), Value::Object(projected_entry));
                }
                Some(
                    Value::Null
                    | Value::Bool(_)
                    | Value::Number(_)
                    | Value::String(_)
                    | Value::Array(_),
                )
                | None => warnings.push(format!(
                    "OpenCode agent '{agent_name}' was not found; skipped model override."
                )),
            }
        }

        raw.insert("agent".to_owned(), Value::Object(agents));
        ProjectionResult {
            document: OpenCodeDocument::from_raw(raw),
            warnings,
        }
    }
}

fn project_section<'a>(
    existing_section: &Map<String, Value>,
    selected_models: impl Iterator<Item = (&'a str, &'a str)>,
) -> Map<String, Value> {
    let mut projected = Map::new();
    for (name, model_ref) in selected_models {
        let mut entry = match existing_section.get(name) {
            Some(Value::Object(object)) => object.clone(),
            Some(
                Value::Null
                | Value::Bool(_)
                | Value::Number(_)
                | Value::String(_)
                | Value::Array(_),
            )
            | None => Map::new(),
        };
        entry.insert("model".to_owned(), Value::String(model_ref.to_owned()));
        projected.insert(name.to_owned(), Value::Object(entry));
    }
    projected
}
