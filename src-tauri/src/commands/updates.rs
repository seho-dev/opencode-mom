use tauri::AppHandle;

use crate::error::AppError;
use crate::updates::{self, AppInfo, UpdateCheck};

#[tauri::command]
pub fn get_app_info(app: AppHandle) -> AppInfo {
    updates::app_info(app.package_info().version.to_string())
}

#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> Result<UpdateCheck, AppError> {
    let version = app.package_info().version.to_string();
    tauri::async_runtime::spawn_blocking(move || updates::check_for_updates(&version))
        .await
        .map_err(|error| AppError::configuration(format!("Check for updates task: {error}")))?
}

#[tauri::command]
pub async fn open_project_page(page: String) -> Result<(), AppError> {
    tauri::async_runtime::spawn_blocking(move || updates::open_project_page(&page))
        .await
        .map_err(|error| AppError::configuration(format!("Open project page task: {error}")))?
}
