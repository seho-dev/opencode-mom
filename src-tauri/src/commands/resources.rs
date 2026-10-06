use tauri::State;

use crate::error::AppError;
use crate::mcp::{self, McpDraft, McpList, McpServer, McpUpdate};
use crate::paths::ConfigPaths;
use crate::skills::{self, SkillDraft, SkillEntry, SkillList, SkillUpdate};

#[tauri::command]
pub fn list_mcps(state: State<'_, ConfigPaths>) -> Result<McpList, AppError> {
    mcp::list(state.inner())
}

#[tauri::command]
pub fn get_mcp(state: State<'_, ConfigPaths>, name: String) -> Result<McpServer, AppError> {
    mcp::get(state.inner(), &name)
}

#[tauri::command]
pub fn create_mcp(state: State<'_, ConfigPaths>, draft: McpDraft) -> Result<McpServer, AppError> {
    mcp::create(state.inner(), draft)
}

#[tauri::command]
pub fn update_mcp(state: State<'_, ConfigPaths>, draft: McpUpdate) -> Result<McpServer, AppError> {
    mcp::update(state.inner(), draft)
}

#[tauri::command]
pub fn delete_mcp(state: State<'_, ConfigPaths>, name: String) -> Result<(), AppError> {
    mcp::delete(state.inner(), &name)
}

#[tauri::command]
pub async fn list_skills(state: State<'_, ConfigPaths>) -> Result<SkillList, AppError> {
    let paths = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || skills::list(&paths))
        .await
        .map_err(|error| AppError::configuration(format!("discover skills: {error}")))?
}

#[tauri::command]
pub async fn get_skill(state: State<'_, ConfigPaths>, id: String) -> Result<SkillEntry, AppError> {
    let paths = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || skills::get(&paths, &id))
        .await
        .map_err(|error| AppError::configuration(format!("read skill: {error}")))?
}

#[tauri::command]
pub async fn create_skill(
    state: State<'_, ConfigPaths>,
    draft: SkillDraft,
) -> Result<SkillEntry, AppError> {
    let paths = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || skills::create(&paths, draft))
        .await
        .map_err(|_| AppError::configuration("Create skill operation failed"))?
}

#[tauri::command]
pub async fn update_skill(
    state: State<'_, ConfigPaths>,
    draft: SkillUpdate,
) -> Result<SkillEntry, AppError> {
    let paths = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || skills::update(&paths, draft))
        .await
        .map_err(|_| AppError::configuration("Update skill operation failed"))?
}
