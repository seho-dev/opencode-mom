use tauri::State;

use crate::behavior::{self, BehaviorDocument, BehaviorUpdate};
use crate::error::AppError;
use crate::paths::ConfigPaths;

#[tauri::command]
pub async fn get_global_behavior(
    state: State<'_, ConfigPaths>,
) -> Result<BehaviorDocument, AppError> {
    let paths = state.inner().clone();
    super::spawn_blocking_task("Read Global instructions", move || behavior::get(&paths)).await
}

#[tauri::command]
pub async fn save_global_behavior(
    state: State<'_, ConfigPaths>,
    draft: BehaviorUpdate,
) -> Result<BehaviorDocument, AppError> {
    let paths = state.inner().clone();
    super::spawn_blocking_task("Save Global instructions", move || {
        behavior::save(&paths, draft)
    })
    .await
}
