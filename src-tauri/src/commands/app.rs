use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;

use crate::agents;
use crate::commands::agent::AgentDefinitionDto;
use crate::error::AppError;
use crate::models;
use crate::paths::ConfigPaths;
use crate::providers;

type CommandResult<T> = Result<T, AppError>;

#[tauri::command]
pub fn load_app_state(state: State<'_, ConfigPaths>) -> CommandResult<AppStateResponse> {
    let paths = state.inner();
    let config = models::load_config(&paths.config_file())?;
    config.validate()?;
    // Read selection and preferences before `groups` is moved out of `config`.
    let selected_group_id = config.state.selected_group_id;
    let preferences = config.state.preferences;
    Ok(AppStateResponse {
        providers: providers::list_providers(&paths.opencode_file())?,
        agents: agents::list(paths)?
            .into_iter()
            .map(Into::into)
            .collect::<Vec<_>>(),
        groups: config.groups,
        selected_group_id,
        projects: config.projects,
        selected_project_id: config.state.selected_project_id,
        preferences,
    })
}

#[tauri::command]
pub async fn pick_project_folder(app: AppHandle) -> CommandResult<Option<String>> {
    tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .blocking_pick_folder()
            .map(|folder| {
                let path = folder.into_path().map_err(|error| {
                    AppError::configuration(format!("pick project folder: {error}"))
                })?;
                path.into_os_string()
                    .into_string()
                    .map_err(|_| AppError::validation("project path must be valid UTF-8"))
            })
            .transpose()
    })
    .await
    .map_err(|error| AppError::configuration(format!("pick project folder: {error}")))?
}

#[tauri::command]
pub fn add_project(
    state: State<'_, ConfigPaths>,
    path: String,
) -> CommandResult<models::ProjectFolder> {
    let root = state
        .inner()
        .validate_project_directory(std::path::Path::new(&path))?;
    let path = root
        .to_str()
        .ok_or_else(|| AppError::validation("project path must be valid UTF-8"))?;
    models::add_project(&state.inner().config_file(), path)
}

#[tauri::command]
pub fn remove_project(state: State<'_, ConfigPaths>, id: Uuid) -> CommandResult<()> {
    models::remove_project(&state.inner().config_file(), id)
}

#[tauri::command]
pub fn select_project(state: State<'_, ConfigPaths>, id: Option<Uuid>) -> CommandResult<()> {
    models::select_project(&state.inner().config_file(), id)
}

#[tauri::command]
pub fn save_preferences(
    app: AppHandle,
    state: State<'_, ConfigPaths>,
    preferences: crate::models::AppPreferences,
) -> CommandResult<()> {
    let paths = state.inner();
    let write = models::config_write_lock();
    let mut config = models::load_config(&paths.config_file())?;
    config.state.preferences = preferences;
    models::save_config(&paths.config_file(), &config)?;
    if let Some(service) = app.try_state::<crate::island::IslandService>() {
        service.update_preferences(preferences);
    }
    drop(write);
    if let Err(error) = app.emit_to("tray", "app:preferences-changed", ()) {
        eprintln!("failed to notify tray window of preference change: {error}");
    }
    if let Err(error) = app.emit_to("island", "app:preferences-changed", ()) {
        eprintln!("failed to notify island window of preference change: {error}");
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
    pub projects: Vec<models::ProjectFolder>,
    pub selected_project_id: Option<Uuid>,
    pub preferences: crate::models::AppPreferences,
}

#[cfg(test)]
mod tests;
