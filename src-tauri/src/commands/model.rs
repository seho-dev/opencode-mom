use tauri::State;

use crate::error::AppError;
use crate::models::{ModelDef, ProviderDef};
use crate::paths::ConfigPaths;
use crate::providers::{self, ModelRef};
use crate::refs;

type CommandResult<T> = Result<T, AppError>;

#[tauri::command]
pub async fn list_models(state: State<'_, ConfigPaths>) -> CommandResult<Vec<ProviderDef>> {
    let paths = state.inner().clone();
    super::spawn_blocking_task("Model discovery", move || {
        providers::list_providers(&paths.opencode_file())
    })
    .await
}

#[tauri::command]
pub async fn create_model(
    state: State<'_, ConfigPaths>,
    provider_id: String,
    model: ModelDef,
) -> CommandResult<ModelDef> {
    let paths = state.inner().clone();
    super::spawn_blocking_task("Create model", move || {
        providers::create_model(&paths.opencode_file(), &provider_id, model)
    })
    .await
}

#[tauri::command]
pub async fn update_model(
    state: State<'_, ConfigPaths>,
    provider_id: String,
    model: ModelDef,
) -> CommandResult<ModelDef> {
    let paths = state.inner().clone();
    super::spawn_blocking_task("Update model", move || {
        let model_ref = ModelRef::new(&provider_id, &model.id)?;
        providers::update_model(&paths.opencode_file(), &model_ref, model)
    })
    .await
}

#[tauri::command]
pub async fn delete_model(state: State<'_, ConfigPaths>, model_ref: String) -> CommandResult<()> {
    let paths = state.inner().clone();
    super::spawn_blocking_task("Delete model", move || {
        let model_ref = ModelRef::parse(&model_ref)?;
        let references = refs::collect_model_references(&paths)?.for_model(&model_ref);
        if !references.is_empty() {
            return Err(AppError::references(
                format!(
                    "{} is referenced by {} location(s)",
                    model_ref.as_str(),
                    references.len()
                ),
                refs::model_reference_locations(&references),
            ));
        }
        providers::delete_model(&paths.opencode_file(), &model_ref)
    })
    .await
}
