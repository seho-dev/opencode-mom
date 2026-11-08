use tauri::{Emitter, Manager, State, WebviewWindow};
use uuid::Uuid;

use crate::error::AppError;
use crate::groups;
use crate::logging::{record, LogStatus, LogType};
use crate::models::ModelGroup;
use crate::paths::ConfigPaths;

type CommandResult<T> = Result<T, AppError>;

#[tauri::command]
pub async fn save_group(
    state: State<'_, ConfigPaths>,
    group: ModelGroup,
) -> CommandResult<ModelGroup> {
    let paths = state.inner().clone();
    super::spawn_blocking_task("Save group", move || {
        groups::save_group(&paths, group).map(|(group, _)| group)
    })
    .await
}

#[tauri::command]
pub async fn delete_group(state: State<'_, ConfigPaths>, id: Uuid) -> CommandResult<()> {
    let paths = state.inner().clone();
    super::spawn_blocking_task("Delete group", move || {
        groups::delete_group(&paths, id).map(|_| ())
    })
    .await
}

#[tauri::command]
pub async fn switch_group(
    window: WebviewWindow,
    state: State<'_, ConfigPaths>,
    id: Uuid,
) -> CommandResult<()> {
    let paths = state.inner().clone();
    super::spawn_blocking_task("Switch group", move || groups::switch_group(&paths, id)).await?;
    if window.label() == "tray"
        && window
            .app_handle()
            .emit_to("main", "tray:config-changed", ())
            .is_err()
    {
        record(
            LogType::Group,
            LogStatus::Error,
            "group_switch_committed_notify_failed",
        );
    }

    Ok(())
}
