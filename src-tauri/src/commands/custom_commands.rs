use tauri::State;
use uuid::Uuid;

use crate::custom_commands::{
    self, SlashCommandDelete, SlashCommandDraft, SlashCommandEntry, SlashCommandList,
    SlashCommandUpdate,
};
use crate::error::AppError;
use crate::paths::ConfigPaths;

#[tauri::command]
pub async fn list_slash_commands(
    state: State<'_, ConfigPaths>,
    project_id: Option<Uuid>,
) -> Result<SlashCommandList, AppError> {
    let paths = state.inner().clone();
    super::spawn_blocking_task("Slash-command discovery", move || {
        custom_commands::list(&paths.for_project(project_id)?)
    })
    .await
}

#[tauri::command]
pub async fn get_slash_command(
    state: State<'_, ConfigPaths>,
    id: String,
    project_id: Option<Uuid>,
) -> Result<SlashCommandEntry, AppError> {
    let paths = state.inner().clone();
    super::spawn_blocking_task("Read slash command", move || {
        custom_commands::get(&paths.for_project(project_id)?, &id)
    })
    .await
}

#[tauri::command]
pub async fn create_slash_command(
    state: State<'_, ConfigPaths>,
    draft: SlashCommandDraft,
    project_id: Option<Uuid>,
) -> Result<SlashCommandEntry, AppError> {
    let paths = state.inner().clone();
    super::spawn_blocking_task("Create slash command", move || {
        custom_commands::create(&paths.for_project(project_id)?, draft)
    })
    .await
}

#[tauri::command]
pub async fn update_slash_command(
    state: State<'_, ConfigPaths>,
    draft: SlashCommandUpdate,
    project_id: Option<Uuid>,
) -> Result<SlashCommandEntry, AppError> {
    let paths = state.inner().clone();
    super::spawn_blocking_task("Update slash command", move || {
        custom_commands::update(&paths.for_project(project_id)?, draft)
    })
    .await
}

#[tauri::command]
pub async fn delete_slash_command(
    state: State<'_, ConfigPaths>,
    draft: SlashCommandDelete,
    project_id: Option<Uuid>,
) -> Result<(), AppError> {
    let paths = state.inner().clone();
    super::spawn_blocking_task("Delete slash command", move || {
        custom_commands::delete(&paths.for_project(project_id)?, draft)
    })
    .await
}
