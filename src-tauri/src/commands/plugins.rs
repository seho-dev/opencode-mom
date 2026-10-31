use crate::error::AppError;
use crate::opencode::plugins::{
    self as runtime, PluginConfigPreview, PluginRemovalPlan, PluginRemovalTarget, PluginRuntime,
};
use crate::paths::ConfigPaths;

#[tauri::command]
pub async fn list_plugin_runtime(
    state: tauri::State<'_, ConfigPaths>,
) -> Result<PluginRuntime, AppError> {
    let paths = state.inner().clone();
    super::spawn_blocking_task("Plugin runtime", move || runtime::list(&paths)).await
}

#[tauri::command]
pub async fn get_plugin_config_sources(
    state: tauri::State<'_, ConfigPaths>,
) -> Result<PluginConfigPreview, AppError> {
    let paths = state.inner().clone();
    super::spawn_blocking_task("Plugin config preview", move || {
        runtime::config_sources(&paths)
    })
    .await
}

#[tauri::command]
pub async fn update_plugin_package(
    state: tauri::State<'_, ConfigPaths>,
    target: String,
) -> Result<(), AppError> {
    let paths = state.inner().clone();
    super::spawn_blocking_task("Update plugin package", move || {
        runtime::update(&paths, &target)
    })
    .await
}

#[tauri::command]
pub async fn prepare_plugin_removal(
    state: tauri::State<'_, ConfigPaths>,
    target: PluginRemovalTarget,
) -> Result<PluginRemovalPlan, AppError> {
    let paths = state.inner().clone();
    super::spawn_blocking_task("Plugin removal preview", move || {
        runtime::prepare_removal(&paths, target)
    })
    .await
}

#[tauri::command]
pub async fn remove_plugin_declaration(
    state: tauri::State<'_, ConfigPaths>,
    plan: PluginRemovalPlan,
) -> Result<(), AppError> {
    let paths = state.inner().clone();
    super::spawn_blocking_task("Plugin declaration removal", move || {
        runtime::remove_declaration(&paths, plan)
    })
    .await
}
