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
    super::spawn_blocking_task("Check for updates", move || {
        updates::check_for_updates(&version)
    })
    .await
}

#[tauri::command]
pub async fn open_project_page(page: String) -> Result<(), AppError> {
    super::spawn_blocking_task("Open project page", move || {
        updates::open_project_page(&page)
    })
    .await
}
