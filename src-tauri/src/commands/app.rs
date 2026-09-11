use serde::Serialize;
use tauri::State;

use crate::agents;
use crate::commands::agent::AgentDefinitionDto;
use crate::error::AppError;
use crate::models;
use crate::paths::ConfigPaths;
use crate::providers;

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
        providers: providers::list_providers(&paths.opencode_file())?,
        agents: agents::list(&paths)?
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>(),
        groups: config.groups,
    })
}

/// The single serialized response shape for `load_app_state`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStateResponse {
    pub providers: Vec<crate::models::ProviderDef>,
    pub agents: Vec<AgentDefinitionDto>,
    pub groups: Vec<crate::models::ModelGroup>,
}
