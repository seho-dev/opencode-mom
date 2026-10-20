use tauri::State;

use crate::error::AppError;
use crate::models::ProviderDef;
use crate::paths::ConfigPaths;
use crate::providers;
use crate::refs;

type CommandResult<T> = Result<T, AppError>;

#[tauri::command]
pub fn list_providers(state: State<'_, ConfigPaths>) -> CommandResult<Vec<ProviderDef>> {
    providers::list_providers(&state.inner().opencode_file())
}

#[tauri::command]
pub fn create_provider(
    state: State<'_, ConfigPaths>,
    provider: ProviderDef,
) -> CommandResult<ProviderDef> {
    providers::create_provider(&state.inner().opencode_file(), provider)
}

#[tauri::command]
pub fn update_provider(
    state: State<'_, ConfigPaths>,
    provider: ProviderDef,
) -> CommandResult<ProviderDef> {
    providers::update_provider(&state.inner().opencode_file(), provider)
}

#[tauri::command]
pub fn delete_provider(state: State<'_, ConfigPaths>, id: String) -> CommandResult<()> {
    let paths = state.inner();
    let references = refs::collect_model_references(paths)?.for_provider(&id);
    if !references.is_empty() {
        return Err(AppError::references(
            format!("{id} is referenced by {} location(s)", references.len()),
            refs::model_reference_locations(&references),
        ));
    }
    providers::delete_provider(&paths.opencode_file(), &id)
}
