use crate::error::AppError;
use crate::power::{LidState, PowerService};

#[tauri::command]
pub fn get_lid_protection(service: tauri::State<'_, PowerService>) -> Result<LidState, AppError> {
    service.get()
}

#[tauri::command]
pub fn set_lid_protection(
    enabled: bool,
    service: tauri::State<'_, PowerService>,
) -> Result<LidState, AppError> {
    service.set(enabled)
}
