use tauri::{Emitter, Manager, State, WebviewWindow};
use uuid::Uuid;

use crate::error::AppError;
use crate::groups;
use crate::models::ModelGroup;
use crate::paths::ConfigPaths;

type CommandResult<T> = Result<T, AppError>;

fn paths(state: &State<'_, ConfigPaths>) -> ConfigPaths {
    state.inner().clone()
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
pub fn switch_group(
    window: WebviewWindow,
    state: State<'_, ConfigPaths>,
    id: Uuid,
) -> CommandResult<()> {
    groups::switch_group(&paths(&state), id)?;
    if window.label() == "tray" {
        if let Err(error) = window
            .app_handle()
            .emit_to("main", "tray:config-changed", ())
        {
            eprintln!("failed to notify main window of group switch: {error}");
        }
    }
    Ok(())
}
