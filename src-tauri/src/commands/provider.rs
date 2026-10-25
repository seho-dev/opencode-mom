use tauri::State;

use crate::error::AppError;
use crate::models::ProviderDef;
use crate::paths::ConfigPaths;
use crate::providers;
use crate::refs;

type CommandResult<T> = Result<T, AppError>;

#[tauri::command]
pub async fn list_providers(state: State<'_, ConfigPaths>) -> CommandResult<Vec<ProviderDef>> {
    let paths = state.inner().clone();
    super::spawn_blocking_task("Provider discovery", move || {
        providers::list_providers(&paths.opencode_file())
    })
    .await
}

#[tauri::command]
pub async fn create_provider(
    state: State<'_, ConfigPaths>,
    provider: ProviderDef,
) -> CommandResult<ProviderDef> {
    let paths = state.inner().clone();
    super::spawn_blocking_task("Create provider", move || {
        providers::create_provider(&paths.opencode_file(), provider)
    })
    .await
}

#[tauri::command]
pub async fn update_provider(
    state: State<'_, ConfigPaths>,
    provider: ProviderDef,
) -> CommandResult<ProviderDef> {
    let paths = state.inner().clone();
    super::spawn_blocking_task("Update provider", move || {
        providers::update_provider(&paths.opencode_file(), provider)
    })
    .await
}

#[tauri::command]
pub async fn delete_provider(state: State<'_, ConfigPaths>, id: String) -> CommandResult<()> {
    let paths = state.inner().clone();
    super::spawn_blocking_task("Delete provider", move || {
        let references = refs::collect_model_references(&paths)?.for_provider(&id);
        if !references.is_empty() {
            return Err(AppError::references(
                format!("{id} is referenced by {} location(s)", references.len()),
                refs::model_reference_locations(&references),
            ));
        }
        providers::delete_provider(&paths.opencode_file(), &id)
    })
    .await
}
