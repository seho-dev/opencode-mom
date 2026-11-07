use tauri::{AppHandle, Manager, State};

use crate::error::AppError;
use crate::models::NotificationSettings;
use crate::notifications::NotificationService;
use crate::paths::ConfigPaths;

#[tauri::command]
pub(crate) async fn get_notification_permission(
    app: AppHandle,
) -> Result<crate::notifications::permission::NotificationPermission, AppError> {
    crate::notifications::permission::get(&app).await
}

#[tauri::command]
pub(crate) async fn request_notification_permission(
    app: AppHandle,
) -> Result<crate::notifications::permission::NotificationPermission, AppError> {
    crate::notifications::permission::request(&app).await
}

#[tauri::command]
pub(crate) async fn open_notification_settings() -> Result<(), AppError> {
    super::spawn_blocking_task(
        "Open notification settings",
        crate::notifications::permission::open_settings,
    )
    .await
}

#[tauri::command]
pub(crate) fn get_notification_settings(
    service: State<'_, NotificationService>,
) -> NotificationSettings {
    service.settings()
}

#[tauri::command]
pub(crate) async fn set_notification_settings(
    app: AppHandle,
    paths: State<'_, ConfigPaths>,
    settings: NotificationSettings,
) -> Result<NotificationSettings, AppError> {
    let paths = paths.inner().clone();
    super::spawn_blocking_task("Set notification settings", move || {
        app.state::<NotificationService>()
            .set_settings(&paths, settings)
    })
    .await
}
