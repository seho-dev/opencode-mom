use tauri::State;

use crate::error::AppError;
use crate::opencode_cli::{self, ModelCatalogEntry};
use crate::paths::ConfigPaths;

type CommandResult<T> = Result<T, AppError>;

fn paths(state: &State<'_, ConfigPaths>) -> ConfigPaths {
    state.inner().clone()
}

#[tauri::command]
pub fn opencode_list_models(
    state: State<'_, ConfigPaths>,
    provider: Option<String>,
) -> CommandResult<Vec<ModelCatalogEntry>> {
    opencode_cli::list_models_via_cli(&paths(&state), provider.as_deref())
}

#[tauri::command]
pub fn opencode_resolve_binary() -> CommandResult<String> {
    Ok(opencode_cli::resolve_opencode_binary()
        .display()
        .to_string())
}
