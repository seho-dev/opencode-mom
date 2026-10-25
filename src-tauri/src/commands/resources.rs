use tauri::State;
use uuid::Uuid;

use crate::error::AppError;
use crate::mcp::{self, McpDraft, McpList, McpServer, McpUpdate};
use crate::paths::{reject_project_write, ConfigPaths};
use crate::skills::{self, SkillDraft, SkillEntry, SkillList, SkillUpdate};

#[tauri::command]
pub async fn list_mcps(
    state: State<'_, ConfigPaths>,
    project_id: Option<Uuid>,
) -> Result<McpList, AppError> {
    let paths = state.inner().clone();
    super::spawn_blocking_task("MCP discovery", move || {
        mcp::list(&paths.for_project(project_id)?)
    })
    .await
}

#[tauri::command]
pub async fn get_mcp(
    state: State<'_, ConfigPaths>,
    name: String,
    project_id: Option<Uuid>,
) -> Result<McpServer, AppError> {
    let paths = state.inner().clone();
    super::spawn_blocking_task("Read MCP", move || {
        mcp::get(&paths.for_project(project_id)?, &name)
    })
    .await
}

#[tauri::command]
pub async fn create_mcp(
    state: State<'_, ConfigPaths>,
    draft: McpDraft,
    project_id: Option<Uuid>,
) -> Result<McpServer, AppError> {
    reject_project_write(project_id)?;
    let paths = state.inner().clone();
    super::spawn_blocking_task("Create MCP", move || mcp::create(&paths, draft)).await
}

#[tauri::command]
pub async fn update_mcp(
    state: State<'_, ConfigPaths>,
    draft: McpUpdate,
    project_id: Option<Uuid>,
) -> Result<McpServer, AppError> {
    reject_project_write(project_id)?;
    let paths = state.inner().clone();
    super::spawn_blocking_task("Update MCP", move || mcp::update(&paths, draft)).await
}

#[tauri::command]
pub async fn delete_mcp(
    state: State<'_, ConfigPaths>,
    name: String,
    project_id: Option<Uuid>,
) -> Result<(), AppError> {
    reject_project_write(project_id)?;
    let paths = state.inner().clone();
    super::spawn_blocking_task("Delete MCP", move || mcp::delete(&paths, &name)).await
}

#[tauri::command]
pub async fn list_skills(
    state: State<'_, ConfigPaths>,
    project_id: Option<Uuid>,
) -> Result<SkillList, AppError> {
    let paths = state.inner().clone();
    super::spawn_blocking_task("Skill discovery", move || {
        skills::list(&paths.for_project(project_id)?)
    })
    .await
}

#[tauri::command]
pub async fn get_skill(
    state: State<'_, ConfigPaths>,
    id: String,
    project_id: Option<Uuid>,
) -> Result<SkillEntry, AppError> {
    let paths = state.inner().clone();
    super::spawn_blocking_task("Read skill", move || {
        skills::get(&paths.for_project(project_id)?, &id)
    })
    .await
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
