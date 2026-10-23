use tauri::{AppHandle, Manager, State};

use super::spawn_blocking_task;
use crate::error::AppError;
use crate::opencode::{
    self, installation, ModelCatalogEntry, OpenCodeInstallation, OpenCodeServiceStatus,
    OpenCodeUpdateCheck, TokenUsageRecord,
};
use crate::paths::ConfigPaths;

type CommandResult<T> = Result<T, AppError>;

#[tauri::command]
pub async fn opencode_list_models(
    state: State<'_, ConfigPaths>,
    provider: Option<String>,
) -> CommandResult<Vec<ModelCatalogEntry>> {
    let paths = state.inner().clone();
    spawn_blocking_task("OpenCode model catalog", move || {
        opencode::list_models_via_service(&paths, provider.as_deref())
    })
    .await
}

#[tauri::command]
pub async fn opencode_get_service_status() -> CommandResult<OpenCodeServiceStatus> {
    spawn_blocking_task("OpenCode service status", opencode::target::get_status).await
}

#[tauri::command]
pub async fn opencode_set_service_target(
    app: AppHandle,
    url: Option<String>,
    password: Option<String>,
) -> CommandResult<OpenCodeServiceStatus> {
    spawn_blocking_task("OpenCode service target", move || {
        let power = app.state::<crate::power::PowerService>();
        let island = app.state::<crate::island::IslandService>();
        opencode::target::set_target(
            url,
            password,
            || {
                Ok((
                    power.prepare_target_refresh()?,
                    island.prepare_target_refresh(),
                ))
            },
            |(power, island)| {
                power.invalidate();
                island.invalidate()
            },
            |generation| {
                power.finish_target_refresh();
                if island.finish_target_refresh(&app, generation).is_err() {
                    eprintln!("OpenCode target saved; Mini Island refresh unavailable");
                }
            },
        )
    })
    .await
}

#[tauri::command]
pub fn opencode_resolve_binary() -> CommandResult<String> {
    Ok(opencode::resolve_opencode_binary().display().to_string())
}

#[tauri::command]
pub async fn opencode_get_installation() -> CommandResult<OpenCodeInstallation> {
    spawn_blocking_task("OpenCode installation", installation::get_installation).await
}

#[tauri::command]
pub async fn opencode_check_update() -> CommandResult<OpenCodeUpdateCheck> {
    spawn_blocking_task("OpenCode update check", installation::check_for_update).await
}

#[tauri::command]
pub async fn opencode_upgrade(
    expected_binary_path: String,
    expected_version: String,
    target_version: String,
) -> CommandResult<OpenCodeInstallation> {
    spawn_blocking_task("OpenCode upgrade", move || {
        installation::upgrade(&expected_binary_path, &expected_version, &target_version)
    })
    .await
}

#[tauri::command]
pub async fn opencode_reload() -> CommandResult<()> {
    spawn_blocking_task("OpenCode reload", opencode::reload_via_service).await
}

#[tauri::command]
pub async fn opencode_active_sessions() -> CommandResult<usize> {
    spawn_blocking_task(
        "OpenCode active sessions",
        opencode::active_sessions_via_service,
    )
    .await
}

#[tauri::command]
pub async fn opencode_token_usage_records() -> CommandResult<Vec<TokenUsageRecord>> {
    spawn_blocking_task(
        "OpenCode token usage",
        opencode::token_usage_records_via_service,
    )
    .await
}
