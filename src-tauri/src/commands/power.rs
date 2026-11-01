use tauri::{AppHandle, Manager};

use crate::error::AppError;
use crate::power::{LidState, PowerService};

#[tauri::command]
pub async fn get_lid_protection(app: AppHandle) -> Result<LidState, AppError> {
    super::spawn_blocking_task("Read lid protection", move || {
        app.state::<PowerService>().get()
    })
    .await
}

#[tauri::command]
pub async fn set_lid_protection(enabled: bool, app: AppHandle) -> Result<LidState, AppError> {
    super::spawn_blocking_task("Set lid protection", move || {
        app.state::<PowerService>().set(enabled)
    })
    .await
}
