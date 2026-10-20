use tauri::State;
use uuid::Uuid;

use crate::error::AppError;
use crate::mcp::{self, McpDraft, McpList, McpServer, McpUpdate};
use crate::paths::{reject_project_write, ConfigPaths};
use crate::skills::{self, SkillDraft, SkillEntry, SkillList, SkillUpdate};

#[tauri::command]
pub fn list_mcps(
    state: State<'_, ConfigPaths>,
    project_id: Option<Uuid>,
) -> Result<McpList, AppError> {
    mcp::list(&state.inner().for_project(project_id)?)
}

#[tauri::command]
pub fn get_mcp(
    state: State<'_, ConfigPaths>,
    name: String,
    project_id: Option<Uuid>,
) -> Result<McpServer, AppError> {
    mcp::get(&state.inner().for_project(project_id)?, &name)
}

#[tauri::command]
pub fn create_mcp(
    state: State<'_, ConfigPaths>,
    draft: McpDraft,
    project_id: Option<Uuid>,
) -> Result<McpServer, AppError> {
    reject_project_write(project_id)?;
    mcp::create(state.inner(), draft)
}

#[tauri::command]
pub fn update_mcp(
    state: State<'_, ConfigPaths>,
    draft: McpUpdate,
    project_id: Option<Uuid>,
) -> Result<McpServer, AppError> {
    reject_project_write(project_id)?;
    mcp::update(state.inner(), draft)
}

#[tauri::command]
pub fn delete_mcp(
    state: State<'_, ConfigPaths>,
    name: String,
    project_id: Option<Uuid>,
) -> Result<(), AppError> {
    reject_project_write(project_id)?;
    mcp::delete(state.inner(), &name)
}

#[tauri::command]
pub async fn list_skills(
    state: State<'_, ConfigPaths>,
    project_id: Option<Uuid>,
) -> Result<SkillList, AppError> {
    let paths = state.inner().for_project(project_id)?;
    super::spawn_blocking_task("Skill discovery", move || skills::list(&paths)).await
}

#[tauri::command]
pub async fn get_skill(
    state: State<'_, ConfigPaths>,
    id: String,
    project_id: Option<Uuid>,
) -> Result<SkillEntry, AppError> {
    let paths = state.inner().for_project(project_id)?;
    super::spawn_blocking_task("Read skill", move || skills::get(&paths, &id)).await
}

#[tauri::command]
pub async fn create_skill(
    state: State<'_, ConfigPaths>,
    draft: SkillDraft,
    project_id: Option<Uuid>,
) -> Result<SkillEntry, AppError> {
    reject_project_write(project_id)?;
    let paths = state.inner().clone();
    super::spawn_blocking_task("Create skill", move || skills::create(&paths, draft)).await
}

#[tauri::command]
pub async fn update_skill(
    state: State<'_, ConfigPaths>,
    draft: SkillUpdate,
    project_id: Option<Uuid>,
) -> Result<SkillEntry, AppError> {
    reject_project_write(project_id)?;
    let paths = state.inner().clone();
    super::spawn_blocking_task("Update skill", move || skills::update(&paths, draft)).await
}
