use std::sync::Arc;

use tauri::State;

use crate::error::AppError;
use crate::logging::{LogPage, LogQuery, LoggingService, LoggingState};

#[tauri::command]
pub(crate) fn get_logging_state(service: State<'_, Arc<LoggingService>>) -> LoggingState {
    service.state()
}

#[tauri::command]
pub(crate) async fn set_logging_enabled(
    service: State<'_, Arc<LoggingService>>,
    enabled: bool,
) -> Result<LoggingState, AppError> {
    let service = service.inner().clone();
    super::spawn_blocking_task("Logging settings", move || service.set_enabled(enabled)).await
}

#[tauri::command]
pub(crate) async fn query_logs(
    service: State<'_, Arc<LoggingService>>,
    query: LogQuery,
) -> Result<LogPage, AppError> {
    let service = service.inner().clone();
    super::spawn_blocking_task("Query logs", move || service.query(query)).await
}

#[tauri::command]
pub(crate) async fn open_logs_directory(
    service: State<'_, Arc<LoggingService>>,
) -> Result<(), AppError> {
    let service = service.inner().clone();
    super::spawn_blocking_task("Open logs", move || service.open_directory()).await
}

#[tauri::command]
pub(crate) async fn clear_logs(
    service: State<'_, Arc<LoggingService>>,
) -> Result<LoggingState, AppError> {
    let service = service.inner().clone();
    super::spawn_blocking_task("Clear logs", move || service.clear()).await
}
