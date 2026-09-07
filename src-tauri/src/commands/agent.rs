use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tauri::State;

use crate::agents::{
    self, AgentCreate, AgentDefinition, AgentFieldOverride, AgentMutation, AgentReferenceIndex,
    AgentSource, AgentStorage, InlineAgentSource, MarkdownAgentSource,
};
use crate::error::AppError;
use crate::paths::ConfigPaths;
use crate::providers;
use crate::refs;

type CommandResult<T> = Result<T, AppError>;

fn paths(state: &State<'_, ConfigPaths>) -> ConfigPaths {
    state.inner().clone()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDefinitionDto {
    pub id: String,
    pub source: AgentSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<AgentStorageDto>,
    #[serde(default)]
    pub sources: Vec<AgentSourcePreviewDto>,
    #[serde(default)]
    pub overrides: Vec<AgentFieldOverride>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutation: Option<AgentMutation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline: Option<Map<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<Map<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective: Option<Map<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "top_p")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steps: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission: Option<Map<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<std::collections::BTreeMap<String, bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Map<String, Value>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStorageDto {
    Inline,
    GlobalMarkdown,
    ProjectMarkdown,
}

impl From<AgentStorageDto> for AgentStorage {
    fn from(storage: AgentStorageDto) -> Self {
        match storage {
            AgentStorageDto::Inline => Self::Inline,
            AgentStorageDto::GlobalMarkdown => Self::GlobalMarkdown,
            AgentStorageDto::ProjectMarkdown => Self::ProjectMarkdown,
        }
    }
}

impl From<AgentStorage> for AgentStorageDto {
    fn from(storage: AgentStorage) -> Self {
        match storage {
            AgentStorage::Inline => Self::Inline,
            AgentStorage::GlobalMarkdown => Self::GlobalMarkdown,
            AgentStorage::ProjectMarkdown => Self::ProjectMarkdown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSourcePreviewDto {
    pub storage: AgentStorageDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub raw: Value,
    pub fields: Map<String, Value>,
    pub prompt: Option<String>,
}

impl From<AgentDefinition> for AgentDefinitionDto {
    fn from(agent: AgentDefinition) -> Self {
        let storage = match agent.source {
            AgentSource::Inline => Some(AgentStorageDto::Inline),
            AgentSource::Markdown => unique_markdown_storage(&agent),
            AgentSource::Both => None,
        };
        let inline_source = agent.inline.clone();
        let inline = inline_source.clone().and_then(|source| object(source.raw));
        let markdown = markdown_fields(&agent.markdown);
        let effective = object(agent.effective);
        let fields = effective.as_ref();
        Self {
            id: agent.id,
            source: agent.source,
            storage,
            sources: source_previews(inline_source, &agent.markdown),
            overrides: agent.overrides,
            mutation: None,
            inline,
            markdown,
            model_ref: string_field(fields, "model"),
            mode: string_field(fields, "mode"),
            description: string_field(fields, "description"),
            disable: bool_field(fields, "disable"),
            hidden: bool_field(fields, "hidden"),
            color: string_field(fields, "color"),
            variant: string_field(fields, "variant"),
            prompt: string_field(fields, "prompt"),
            temperature: number_field(fields, "temperature"),
            top_p: number_field(fields, "top_p"),
            steps: unsigned_field(fields, "steps"),
            permission: object_field(fields, "permission"),
            tools: bool_map_field(fields, "tools"),
            options: object_field(fields, "options"),
            effective,
        }
    }
}

fn source_previews(
    inline: Option<InlineAgentSource>,
    markdown: &[MarkdownAgentSource],
) -> Vec<AgentSourcePreviewDto> {
    let mut sources = Vec::new();
    if let Some(inline) = inline {
        sources.push(AgentSourcePreviewDto {
            storage: AgentStorageDto::Inline,
            path: None,
            fields: inline.raw.as_object().cloned().unwrap_or_default(),
            raw: inline.raw,
            prompt: None,
        });
    }
    sources.extend(markdown.iter().map(|source| {
        let mut fields = source.frontmatter.clone();
        fields.remove("prompt");
        AgentSourcePreviewDto {
            storage: source.storage.into(),
            path: Some(source.path.display().to_string()),
            raw: Value::String(source.raw.clone()),
            fields,
            prompt: Some(source.prompt.clone()),
        }
    }));
    sources
}

fn unique_markdown_storage(agent: &AgentDefinition) -> Option<AgentStorageDto> {
    let mut storages = agent.markdown.iter().map(|source| source.storage);
    let storage = storages.next()?;
    storages
        .all(|candidate| candidate == storage)
        .then_some(storage.into())
}

fn markdown_fields(markdown: &[MarkdownAgentSource]) -> Option<Map<String, Value>> {
    let mut fields = Map::new();
    for source in markdown {
        fields.extend(source.frontmatter.clone());
        fields.insert("prompt".to_owned(), Value::String(source.prompt.clone()));
    }
    (!fields.is_empty()).then_some(fields)
}

fn object(value: Value) -> Option<Map<String, Value>> {
    match value {
        Value::Object(value) => Some(value),
        _ => None,
    }
}

fn string_field(fields: Option<&Map<String, Value>>, key: &str) -> Option<String> {
    fields?.get(key)?.as_str().map(ToOwned::to_owned)
}

fn bool_field(fields: Option<&Map<String, Value>>, key: &str) -> Option<bool> {
    fields?.get(key)?.as_bool()
}

fn number_field(fields: Option<&Map<String, Value>>, key: &str) -> Option<f64> {
    fields?.get(key)?.as_f64()
}

fn unsigned_field(fields: Option<&Map<String, Value>>, key: &str) -> Option<u64> {
    fields?.get(key)?.as_u64()
}

fn object_field(fields: Option<&Map<String, Value>>, key: &str) -> Option<Map<String, Value>> {
    fields?.get(key)?.as_object().cloned()
}

fn bool_map_field(
    fields: Option<&Map<String, Value>>,
    key: &str,
) -> Option<std::collections::BTreeMap<String, bool>> {
    fields?
        .get(key)?
        .as_object()?
        .iter()
        .map(|(key, value)| Some((key.clone(), value.as_bool()?)))
        .collect()
}

#[tauri::command]
pub fn list_agents(state: State<'_, ConfigPaths>) -> CommandResult<Vec<AgentDefinitionDto>> {
    let agents = agents::list(&paths(&state))?;
    Ok(agents.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub fn get_agent(state: State<'_, ConfigPaths>, id: String) -> CommandResult<AgentDefinitionDto> {
    agents::get(&paths(&state), &id).map(Into::into)
}

#[tauri::command]
pub fn create_agent(
    state: State<'_, ConfigPaths>,
    agent: AgentDefinitionDto,
) -> CommandResult<AgentDefinitionDto> {
    let paths = paths(&state);
    let storage = agent.storage.map(Into::into).ok_or_else(|| {
        AppError::validation("agent creation requires an explicit storage target")
    })?;
    let mutation = agent.mutation.clone().unwrap_or_default();
    validate_agent_model_reference(&paths, &mutation.fields)?;
    agents::create(
        &paths,
        AgentCreate {
            id: agent.id,
            storage,
            fields: mutation.fields,
            prompt: mutation.prompt,
        },
    )
    .map(Into::into)
}

#[tauri::command]
pub fn update_agent(
    state: State<'_, ConfigPaths>,
    agent: AgentDefinitionDto,
) -> CommandResult<AgentDefinitionDto> {
    let paths = paths(&state);
    let storage = agent
        .storage
        .map(Into::into)
        .ok_or_else(|| AppError::validation("agent update requires an explicit storage target"))?;
    let mutation = agent.mutation.clone().unwrap_or_default();
    validate_agent_model_reference(&paths, &mutation.fields)?;
    agents::update(
        &paths,
        &agent.id,
        storage,
        AgentMutation {
            fields: mutation.fields,
            prompt: mutation.prompt,
            clear_fields: mutation.clear_fields,
        },
    )
    .map(Into::into)
}

#[tauri::command]
pub fn delete_agent(
    state: State<'_, ConfigPaths>,
    id: String,
    storage: Option<AgentStorageDto>,
) -> CommandResult<()> {
    let paths = paths(&state);
    let storage: AgentStorage = match storage {
        Some(storage) => storage.into(),
        None => {
            let current: AgentDefinition = agents::get(&paths, &id)?;
            match current.source {
                AgentSource::Inline => AgentStorage::Inline,
                AgentSource::Both => {
                    return Err(AppError::validation(
                        "agent has multiple sources; provide an explicit storage target",
                    ));
                }
                AgentSource::Markdown => {
                    let mut storages = current.markdown.iter().map(|source| source.storage);
                    let first = storages.next().ok_or_else(|| {
                        AppError::validation(
                            "agent has multiple sources; provide an explicit storage target",
                        )
                    })?;
                    if storages.all(|candidate| candidate == first) {
                        first
                    } else {
                        return Err(AppError::validation(
                            "agent has multiple sources; provide an explicit storage target",
                        ));
                    }
                }
            }
        }
    };
    let references: AgentReferenceIndex = refs::collect_agent_references(&paths)?;
    agents::delete(&paths, &id, storage, &references).map(|_| ())
}

fn validate_agent_model_reference(
    paths: &ConfigPaths,
    fields: &serde_json::Map<String, Value>,
) -> CommandResult<()> {
    let Some(model) = fields.get("model") else {
        return Ok(());
    };
    let model = model
        .as_str()
        .ok_or_else(|| AppError::validation("agent model must be a string when provided"))?;
    if model.is_empty() {
        return Ok(());
    }
    let model_ref = crate::providers::ModelRef::parse(model).map_err(|error| {
        AppError::validation(format!("agent model must be provider/model: {error}"))
    })?;
    let provider = providers::get_provider(&paths.opencode_file(), &model_ref.provider_id)
        .map_err(|error| {
            AppError::validation(format!(
                "agent model '{}' does not exist: {error}",
                model_ref.as_str()
            ))
        })?;
    if provider.models.contains_key(&model_ref.model_id) {
        Ok(())
    } else {
        Err(AppError::validation(format!(
            "agent model '{}' does not exist",
            model_ref.as_str()
        )))
    }
}
