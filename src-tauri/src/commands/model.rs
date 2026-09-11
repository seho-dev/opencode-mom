use tauri::State;

use crate::error::AppError;
use crate::models::{ModelDef, ProviderDef};
use crate::paths::ConfigPaths;
use crate::providers::{self, ModelRef};
use crate::refs;

type CommandResult<T> = Result<T, AppError>;

fn paths(state: &State<'_, ConfigPaths>) -> ConfigPaths {
    state.inner().clone()
}

#[tauri::command]
pub fn list_models(state: State<'_, ConfigPaths>) -> CommandResult<Vec<ProviderDef>> {
    providers::list_providers(&paths(&state).opencode_file())
}

#[tauri::command]
pub fn create_model(
    state: State<'_, ConfigPaths>,
    provider_id: String,
    model: ModelDef,
) -> CommandResult<ModelDef> {
    providers::create_model(&paths(&state).opencode_file(), &provider_id, model)
}

#[tauri::command]
pub fn update_model(
    state: State<'_, ConfigPaths>,
    provider_id: String,
    model: ModelDef,
) -> CommandResult<ModelDef> {
    let model_ref = ModelRef::new(&provider_id, &model.id)?;
    providers::update_model(&paths(&state).opencode_file(), &model_ref, model)
}

#[tauri::command]
pub fn delete_model(state: State<'_, ConfigPaths>, model_ref: String) -> CommandResult<()> {
    let paths = paths(&state);
    let model_ref = ModelRef::parse(&model_ref)?;
    let references = refs::model_is_referenced(&paths, &model_ref)?;
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
}
