use crate::error::AppError;

#[tauri::command]
pub async fn get_autostart(app: tauri::AppHandle) -> Result<bool, AppError> {
    super::spawn_blocking_task("Read autostart", move || {
        #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
        {
            use tauri_plugin_autostart::ManagerExt;
            app.autolaunch()
                .is_enabled()
                .map_err(|error| AppError::configuration(format!("read autostart: {error}")))
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            let _ = app;
            Err(AppError::configuration(
                "autostart is unsupported on this platform",
            ))
        }
    })
    .await
}

#[tauri::command]
pub async fn set_autostart(app: tauri::AppHandle, enabled: bool) -> Result<bool, AppError> {
    super::spawn_blocking_task("Set autostart", move || {
        #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
        {
            use tauri_plugin_autostart::ManagerExt;
            let manager = app.autolaunch();
            if enabled {
                manager.enable()
            } else {
                manager.disable()
            }
            .map_err(|error| AppError::configuration(format!("set autostart: {error}")))?;
            manager
                .is_enabled()
                .map_err(|error| AppError::configuration(format!("read autostart: {error}")))
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            let _ = (app, enabled);
            Err(AppError::configuration(
                "autostart is unsupported on this platform",
            ))
        }
    })
    .await
}
