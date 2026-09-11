use tauri::State;

use crate::error::AppError;
use crate::models::ProviderDef;
use crate::paths::ConfigPaths;
use crate::providers;
use crate::refs;

type CommandResult<T> = Result<T, AppError>;

fn paths(state: &State<'_, ConfigPaths>) -> ConfigPaths {
    state.inner().clone()
}

#[tauri::command]
pub fn list_providers(state: State<'_, ConfigPaths>) -> CommandResult<Vec<ProviderDef>> {
    providers::list_providers(&paths(&state).opencode_file())
}

#[tauri::command]
pub fn list_custom_providers(state: State<'_, ConfigPaths>) -> CommandResult<Vec<ProviderDef>> {
    let all = providers::list_providers(&paths(&state).opencode_file())?;
    let custom_ids = providers::custom_provider_ids(&paths(&state).opencode_file())?;
    Ok(all
        .into_iter()
        .filter(|p| custom_ids.contains(&p.name))
        .collect())
}

#[tauri::command]
pub fn create_provider(
    state: State<'_, ConfigPaths>,
    provider: ProviderDef,
) -> CommandResult<ProviderDef> {
    providers::create_provider(&paths(&state).opencode_file(), provider)
}

#[tauri::command]
pub fn update_provider(
    state: State<'_, ConfigPaths>,
    provider: ProviderDef,
) -> CommandResult<ProviderDef> {
    providers::update_provider(&paths(&state).opencode_file(), provider)
}

#[tauri::command]
pub fn delete_provider(state: State<'_, ConfigPaths>, id: String) -> CommandResult<()> {
    let paths = paths(&state);
    let references = refs::provider_is_referenced(&paths, &id)?;
    if !references.is_empty() {
        return Err(AppError::references(
            format!("{id} is referenced by {} location(s)", references.len()),
            refs::model_reference_locations(&references),
        ));
    }
    providers::delete_provider(&paths.opencode_file(), &id)
}
