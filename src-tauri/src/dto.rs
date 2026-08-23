use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::agents::{
    AgentDefinition as CoreAgentDefinition, AgentFieldOverride, AgentSource as CoreAgentSource,
    AgentStorage,
};

/// The single serialized response shape for `load_app_state`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStateResponse {
    pub providers: Vec<crate::models::ProviderDef>,
    pub agents: Vec<AgentDefinitionDto>,
    pub groups: Vec<crate::models::ModelGroup>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSourceDto {
    Inline,
    Markdown,
    Both,
}

impl From<CoreAgentSource> for AgentSourceDto {
    fn from(source: CoreAgentSource) -> Self {
        match source {
            CoreAgentSource::Inline => Self::Inline,
            CoreAgentSource::Markdown => Self::Markdown,
            CoreAgentSource::Both => Self::Both,
        }
    }
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
pub struct AgentDefinitionDto {
    pub id: String,
    pub source: AgentSourceDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<AgentStorageDto>,
    #[serde(default)]
    pub sources: Vec<AgentSourcePreviewDto>,
    #[serde(default)]
    pub overrides: Vec<AgentFieldOverride>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mutation: Option<AgentMutationDto>,
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

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMutationDto {
    #[serde(default)]
    pub fields: Map<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default)]
    pub clear_fields: Vec<String>,
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

impl From<CoreAgentDefinition> for AgentDefinitionDto {
    fn from(agent: CoreAgentDefinition) -> Self {
        let storage = match agent.source {
            CoreAgentSource::Inline => Some(AgentStorageDto::Inline),
            CoreAgentSource::Markdown => unique_markdown_storage(&agent),
            CoreAgentSource::Both => None,
        };
        let inline_source = agent.inline.clone();
        let inline = inline_source.clone().and_then(|source| object(source.raw));
        let markdown = markdown_fields(&agent.markdown);
        let effective = object(agent.effective);
        let fields = effective.as_ref();
        Self {
            id: agent.id,
            source: agent.source.into(),
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
    inline: Option<crate::agents::InlineAgentSource>,
    markdown: &[crate::agents::MarkdownAgentSource],
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

fn unique_markdown_storage(agent: &CoreAgentDefinition) -> Option<AgentStorageDto> {
    let mut storages = agent.markdown.iter().map(|source| source.storage);
    let storage = storages.next()?;
    storages
        .all(|candidate| candidate == storage)
        .then_some(storage.into())
}

fn markdown_fields(markdown: &[crate::agents::MarkdownAgentSource]) -> Option<Map<String, Value>> {
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
