use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

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
    // Read selection and preferences before `groups` is moved out of `config`.
    let selected_group_id = config.state.selected_group_id;
    let preferences = config.state.preferences;
    Ok(AppStateResponse {
        providers: providers::list_providers(&paths.opencode_file())?,
        agents: agents::list(&paths)?
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>(),
        groups: config.groups,
        selected_group_id,
        preferences,
    })
}

#[tauri::command]
pub fn save_preferences(
    app: AppHandle,
    state: State<'_, ConfigPaths>,
    preferences: crate::models::AppPreferences,
) -> CommandResult<()> {
    let paths = paths(&state);
    let mut config = models::load_config(&paths.config_file())?;
    config.state.preferences = preferences;
    models::save_config(&paths.config_file(), &config)?;
    if let Err(error) = app.emit_to("tray", "app:preferences-changed", ()) {
        eprintln!("failed to notify tray window of preference change: {error}");
    }
    Ok(())
}

/// The single serialized response shape for `load_app_state`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStateResponse {
    pub providers: Vec<crate::models::ProviderDef>,
    pub agents: Vec<AgentDefinitionDto>,
    pub groups: Vec<crate::models::ModelGroup>,
    pub selected_group_id: Option<Uuid>,
    pub preferences: crate::models::AppPreferences,
}

#[cfg(test)]
mod tests;
