use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::AppError;
use crate::paths::ConfigPaths;

use super::transport::{self, ManagedConnection};

/// Frontend-facing catalog entry; local config remains authoritative for metadata.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCatalogEntry {
    pub provider_id: String,
    pub model_id: String,
    #[serde(rename = "ref")]
    pub model_ref: String,
    pub name: String,
    pub is_custom: bool,
    pub limit: Option<Value>,
    pub variants: Option<Value>,
}

#[derive(Deserialize)]
struct Catalog {
    data: Vec<ApiModel>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiModel {
    id: String,
    #[serde(rename = "providerID")]
    provider_id: String,
    #[serde(rename = "modelID")]
    model_id: String,
    enabled: bool,
}

pub(super) fn model_refs(
    connection: &ManagedConnection,
    deadline: Instant,
) -> Result<BTreeSet<String>, AppError> {
    let directory = connection.directory().and_then(|path| path.to_str());
    let query: Vec<_> = directory
        .into_iter()
        .map(|directory| ("location[directory]", directory))
        .collect();
    let catalog: super::active::JsonObject<Catalog> = connection.get_json(
        &["model"],
        &query,
        deadline,
        8 * 1024 * 1024,
        "model catalog",
        "8 MiB",
    )?;
    let mut refs = BTreeSet::new();
    for model in catalog.0.data {
        if [&model.id, &model.provider_id, &model.model_id]
            .into_iter()
            .any(|value| {
                value.is_empty()
                    || value.len() > 4096
                    || value.contains(char::is_whitespace)
                    || value.chars().any(char::is_control)
            })
            || model.provider_id.contains('/')
        {
            return Err(AppError::configuration(
                "Invalid OpenCode model catalog identity",
            ));
        }
        if model.enabled {
            refs.insert(format!("{}/{}", model.provider_id, model.model_id));
        }
    }
    Ok(refs)
}

/// Lists enabled models from the selected local service plus all configured models.
pub fn list_models_via_service(
    paths: &ConfigPaths,
    provider_filter: Option<&str>,
) -> Result<Vec<ModelCatalogEntry>, AppError> {
    let deadline = Instant::now() + Duration::from_secs(20);
    let connection = transport::connect(deadline)?;
    list_models(paths, provider_filter, &connection, deadline)
}

fn list_models(
    paths: &ConfigPaths,
    provider_filter: Option<&str>,
    connection: &ManagedConnection,
    deadline: Instant,
) -> Result<Vec<ModelCatalogEntry>, AppError> {
    let mut refs = model_refs(connection, deadline)?;

    let config_providers = crate::providers::list_providers(&paths.opencode_file())?;
    let custom_ids = crate::providers::custom_provider_ids(&paths.opencode_file())?;

    // Config models are authoritative for metadata and must stay listed.
    for provider in &config_providers {
        for model_id in provider.models.keys() {
            refs.insert(format!("{}/{}", provider.name, model_id));
        }
    }

    let mut catalog = Vec::with_capacity(refs.len());
    for model_ref in refs {
        let Some((provider_id, model_id)) = model_ref.split_once('/') else {
            continue;
        };
        if provider_filter.is_some_and(|filter| provider_id != filter) {
            continue;
        }
        let config_model = config_providers
            .iter()
            .find(|provider| provider.name == provider_id)
            .and_then(|provider| provider.models.get(model_id));
        let name = config_model
            .and_then(|model| model.name.clone())
            .unwrap_or_else(|| model_id.to_owned());
        let is_custom = custom_ids.contains(provider_id);
        catalog.push(ModelCatalogEntry {
            provider_id: provider_id.to_owned(),
            model_id: model_id.to_owned(),
            model_ref,
            name,
            is_custom,
            limit: config_model
                .and_then(|model| model.limit.clone())
                .and_then(|limit| serde_json::to_value(limit).ok()),
            variants: config_model
                .and_then(|model| model.variants.clone())
                .and_then(|variants| serde_json::to_value(variants).ok()),
        });
    }
    connection.ensure_current()?;
    Ok(catalog)
}

#[cfg(test)]
mod tests;
