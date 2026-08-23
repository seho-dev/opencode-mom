use serde_json::Value;
use tauri::State;
use uuid::Uuid;

use crate::agents::{
    self, AgentCreate, AgentDefinition, AgentMutation, AgentReferenceIndex, AgentStorage,
};
use crate::document::{JsoncDoc, OMO_SCHEMA, OPENCODE_SCHEMA, SLIM_SCHEMA};
use crate::dto::{AgentDefinitionDto, AgentStorageDto, AppStateResponse};
use crate::error::AppError;
use crate::groups;
use crate::models::{self, ModelDef, ModelGroup, ProviderDef};
use crate::paths::ConfigPaths;
use crate::providers::{self, ModelRef};
use crate::refs;

type CommandResult<T> = Result<T, AppError>;

fn paths(state: &State<'_, ConfigPaths>) -> ConfigPaths {
    state.inner().clone()
}

#[tauri::command]
pub fn load_app_state(state: State<'_, ConfigPaths>) -> CommandResult<AppStateResponse> {
    let paths = paths(&state);
    let config = models::load_config(&paths.config_file())?;
    config.validate()?;
    Ok(AppStateResponse {
        providers: refs::list_providers_redacted(&paths)?,
        agents: agents::list(&paths)?
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>(),
        groups: config.groups,
        diagnostics: config_diagnostics(&paths),
    })
}

#[tauri::command]
pub fn list_providers(state: State<'_, ConfigPaths>) -> CommandResult<Vec<ProviderDef>> {
    refs::list_providers_redacted(&paths(&state))
}

#[tauri::command]
pub fn reveal_provider_option(
    state: State<'_, ConfigPaths>,
    provider_id: String,
    key: String,
) -> CommandResult<Value> {
    providers::reveal_sensitive_option(&paths(&state).opencode_file(), &provider_id, &key)
}

#[tauri::command]
pub fn create_provider(
    state: State<'_, ConfigPaths>,
    provider: ProviderDef,
) -> CommandResult<ProviderDef> {
    providers::create_provider(&paths(&state).opencode_file(), provider)
        .map(providers::redact_provider_secrets)
}

#[tauri::command]
pub fn update_provider(
    state: State<'_, ConfigPaths>,
    provider: ProviderDef,
) -> CommandResult<ProviderDef> {
    providers::update_provider(&paths(&state).opencode_file(), provider)
        .map(providers::redact_provider_secrets)
}

#[tauri::command]
pub fn delete_provider(state: State<'_, ConfigPaths>, id: String) -> CommandResult<()> {
    let paths = paths(&state);
    let references = refs::provider_is_referenced(&paths, &id)?;
    if !references.is_empty() {
        return Err(AppError::references(
            format!("{id} is referenced by {} location(s)", references.len()),
            refs::model_reference_locations(&references),
        ));
    }
    providers::delete_provider(&paths.opencode_file(), &id)
}

#[tauri::command]
pub fn list_models(state: State<'_, ConfigPaths>) -> CommandResult<Vec<ProviderDef>> {
    refs::list_providers_redacted(&paths(&state))
}

#[tauri::command]
pub fn create_model(
    state: State<'_, ConfigPaths>,
    provider_id: String,
    model: ModelDef,
) -> CommandResult<ModelDef> {
    providers::create_model(&paths(&state).opencode_file(), &provider_id, model)
}

#[tauri::command]
pub fn update_model(
    state: State<'_, ConfigPaths>,
    provider_id: String,
    model: ModelDef,
) -> CommandResult<ModelDef> {
    let model_ref = ModelRef::new(&provider_id, &model.id)?;
    providers::update_model(&paths(&state).opencode_file(), &model_ref, model)
}

#[tauri::command]
pub fn delete_model(state: State<'_, ConfigPaths>, model_ref: String) -> CommandResult<()> {
    let paths = paths(&state);
    let model_ref = ModelRef::parse(&model_ref)?;
    let references = refs::model_is_referenced(&paths, &model_ref)?;
    if !references.is_empty() {
        return Err(AppError::references(
            format!(
                "{} is referenced by {} location(s)",
                model_ref.as_str(),
                references.len()
            ),
            refs::model_reference_locations(&references),
        ));
    }
    providers::delete_model(&paths.opencode_file(), &model_ref)
}

#[tauri::command]
pub fn replace_model_references(
    state: State<'_, ConfigPaths>,
    from: String,
    to: String,
) -> CommandResult<()> {
    let paths = paths(&state);
    let from = ModelRef::parse(&from)?;
    let to = ModelRef::parse(&to)?;
    crate::replacement::replace(&paths, &from, &to).map(|_| ())
}

#[tauri::command]
pub fn rename_model(
    state: State<'_, ConfigPaths>,
    from: String,
    new_model_id: String,
) -> CommandResult<()> {
    let paths = paths(&state);
    let from = ModelRef::parse(&from)?;
    crate::replacement::rename_model(&paths, &from, &new_model_id)
}

#[tauri::command]
pub fn rename_provider(
    state: State<'_, ConfigPaths>,
    old_id: String,
    new_id: String,
) -> CommandResult<()> {
    crate::replacement::rename_provider(&paths(&state), &old_id, &new_id)
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
                crate::agents::AgentSource::Inline => AgentStorage::Inline,
                crate::agents::AgentSource::Both => {
                    return Err(AppError::validation(
                        "agent has multiple sources; provide an explicit storage target",
                    ));
                }
                crate::agents::AgentSource::Markdown => {
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

#[tauri::command]
pub fn save_group(state: State<'_, ConfigPaths>, group: ModelGroup) -> CommandResult<ModelGroup> {
    let (group, _) = groups::save_group(&paths(&state), group)?;
    Ok(group)
}

#[tauri::command]
pub fn copy_group(
    state: State<'_, ConfigPaths>,
    id: Uuid,
    name: Option<String>,
) -> CommandResult<ModelGroup> {
    let (group, _) = groups::copy_group(&paths(&state), id, name)?;
    Ok(group)
}

#[tauri::command]
pub fn delete_group(state: State<'_, ConfigPaths>, id: Uuid) -> CommandResult<()> {
    groups::delete_group(&paths(&state), id).map(|_| ())
}

#[tauri::command]
pub fn switch_group(state: State<'_, ConfigPaths>, id: Uuid) -> CommandResult<()> {
    groups::switch_group(&paths(&state), id).map(|_| ())
}

#[tauri::command]
pub fn load_config_diagnostics(state: State<'_, ConfigPaths>) -> CommandResult<Vec<String>> {
    Ok(config_diagnostics(&paths(&state)))
}

#[tauri::command]
pub fn validate_config(state: State<'_, ConfigPaths>) -> CommandResult<Vec<String>> {
    let paths = paths(&state);
    let config = models::load_config(&paths.config_file())?;
    config.validate()?;
    // Parsing every managed document surfaces malformed JSONC as a validation error.
    JsoncDoc::read(&paths.opencode_file(), OPENCODE_SCHEMA)?;
    JsoncDoc::read(&paths.slim_file(), SLIM_SCHEMA)?;
    JsoncDoc::read(&paths.omo_file(), OMO_SCHEMA)?;
    agents::list(&paths)?;
    refs::collect_model_references(&paths)?;
    refs::collect_agent_references(&paths)?;
    Ok(config_diagnostics(&paths))
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
    let model_ref = ModelRef::parse(model).map_err(|error| {
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

/// Surfaces legacy configuration files that the application deliberately ignores.
fn config_diagnostics(paths: &ConfigPaths) -> Vec<String> {
    let mut diagnostics = Vec::new();
    let opencode_directory = paths
        .opencode_file()
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_default();
    for filename in [
        "oh-my-openagent.json",
        "oh-my-openagent.jsonc",
        "oh-my-opencode.json",
        "oh-my-opencode.jsonc",
    ] {
        let path = opencode_directory.join(filename);
        if path.exists() {
            diagnostics.push(format!("ignored legacy configuration: {}", path.display()));
        }
    }
    diagnostics
}
