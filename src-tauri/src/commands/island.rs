use serde_json::{Map, Value};
use tauri::{AppHandle, Manager, State};

use crate::error::AppError;
use crate::island::{IslandService, IslandState, IslandView};
use crate::opencode::island::IslandMessage;
use crate::paths::ConfigPaths;

#[tauri::command]
pub(crate) async fn get_island_state(app: AppHandle) -> Result<IslandState, AppError> {
    super::spawn_blocking_task("Read Mini Island state", move || {
        Ok(app.state::<IslandService>().state())
    })
    .await
}

#[tauri::command]
pub(crate) async fn set_island_enabled(
    app: AppHandle,
    paths: State<'_, ConfigPaths>,
    enabled: bool,
) -> Result<IslandState, AppError> {
    let paths = paths.inner().clone();
    super::spawn_blocking_task("Set Mini Island enabled", move || {
        crate::island::on_main(&app, move |app| {
            let service = app.state::<IslandService>();
            service.set_enabled(app, &paths, enabled)?;
            Ok(service.state())
        })
    })
    .await
}

#[tauri::command]
pub(crate) async fn refresh_island(
    app: AppHandle,
    service: State<'_, IslandService>,
) -> Result<IslandState, AppError> {
    service.refresh(&app).await?;
    Ok(service.state())
}

#[tauri::command]
pub(crate) async fn set_island_view(app: AppHandle, view: IslandView) -> Result<(), AppError> {
    let receiver = super::spawn_blocking_task("Set Mini Island view", move || {
        let generation = app.state::<IslandService>().enabled_generation()?;
        crate::island::on_main(&app, move |app| {
            app.state::<IslandService>().set_view(app, view, generation)
        })
    })
    .await?;
    receiver
        .await
        .map_err(|_| AppError::configuration("Mini Island native transition closed"))?
}

#[tauri::command]
pub(crate) async fn island_messages(
    service: State<'_, IslandService>,
    session_id: String,
    expected_connection_generation: u64,
) -> Result<Vec<IslandMessage>, AppError> {
    service
        .messages(session_id, expected_connection_generation)
        .await
}

#[tauri::command]
pub(crate) async fn island_send(
    service: State<'_, IslandService>,
    session_id: String,
    text: String,
    message_id: String,
    expected_connection_generation: u64,
) -> Result<(), AppError> {
    service
        .request(
            expected_connection_generation,
            |api| async move { api.send(&session_id, &text, &message_id).await },
            true,
        )
        .await
}

#[tauri::command]
pub(crate) async fn island_reply_permission(
    service: State<'_, IslandService>,
    session_id: String,
    request_id: String,
    decision: String,
    expected_connection_generation: u64,
) -> Result<(), AppError> {
    service
        .request(
            expected_connection_generation,
            |api| async move {
                api.reply_permission(&session_id, &request_id, &decision)
                    .await
            },
            true,
        )
        .await
}

#[tauri::command]
pub(crate) async fn island_reply_question(
    service: State<'_, IslandService>,
    session_id: String,
    form_id: String,
    answer: Map<String, Value>,
    expected_connection_generation: u64,
) -> Result<(), AppError> {
    service
        .request(
            expected_connection_generation,
            |api| async move { api.reply_question(&session_id, &form_id, answer).await },
            true,
        )
        .await
}

#[tauri::command]
pub(crate) async fn island_cancel_question(
    service: State<'_, IslandService>,
    session_id: String,
    form_id: String,
    expected_connection_generation: u64,
) -> Result<(), AppError> {
    service
        .request(
            expected_connection_generation,
            |api| async move { api.cancel_question(&session_id, &form_id).await },
            true,
        )
        .await
}

#[tauri::command]
pub(crate) async fn island_interrupt(
    service: State<'_, IslandService>,
    session_id: String,
    expected_connection_generation: u64,
) -> Result<(), AppError> {
    service
        .request(
            expected_connection_generation,
            |api| async move { api.interrupt(&session_id).await },
            true,
        )
        .await
}
