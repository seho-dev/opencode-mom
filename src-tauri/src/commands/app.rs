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
    // Read preferences before `groups` is moved out of `config`.
    let preferences = config.state.preferences;
    Ok(AppStateResponse {
        providers: providers::list_providers(&paths.opencode_file())?,
        agents: agents::list(&paths)?
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>(),
        groups: config.groups,
        preferences,
    })
}

#[tauri::command]
pub fn save_preferences(
    state: State<'_, ConfigPaths>,
    preferences: crate::models::AppPreferences,
) -> CommandResult<()> {
    let paths = paths(&state);
    let mut config = models::load_config(&paths.config_file())?;
    config.state.preferences = preferences;
    models::save_config(&paths.config_file(), &config)
}

/// The single serialized response shape for `load_app_state`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStateResponse {
    pub providers: Vec<crate::models::ProviderDef>,
    pub agents: Vec<AgentDefinitionDto>,
    pub groups: Vec<crate::models::ModelGroup>,
    pub preferences: crate::models::AppPreferences,
}
