use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use serde::Deserialize;
use serde_json::{Map, Value};

use crate::document::{JsoncDoc, OPENCODE_SCHEMA};
use crate::error::AppError;
use crate::models::{ModelDef, ProviderDef, ProviderOptions};

/// A stable OpenCode model reference. Only `provider/model` is valid.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ModelRef {
    pub provider_id: String,
    pub model_id: String,
}

impl ModelRef {
    pub fn new(
        provider_id: impl Into<String>,
        model_id: impl Into<String>,
    ) -> Result<Self, AppError> {
        let provider_id = provider_id.into();
        let model_id = model_id.into();
        validate_id(&provider_id, IdKind::Provider)?;
        validate_id(&model_id, IdKind::Model)?;
        Ok(Self {
            provider_id,
            model_id,
        })
    }

    pub fn parse(value: &str) -> Result<Self, AppError> {
        let (provider_id, model_id) = value
            .split_once('/')
            .filter(|(_, model_id)| !model_id.contains('/'))
            .ok_or_else(|| invalid_model_ref(value))?;
        Self::new(provider_id, model_id)
    }

    pub fn as_str(&self) -> String {
        format!("{}/{}", self.provider_id, self.model_id)
    }
}

impl fmt::Display for ModelRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.as_str())
    }
}

fn invalid_model_ref(value: &str) -> AppError {
    AppError::validation(format!("invalid model reference: {value}"))
}

pub fn list_providers(opencode_file: &Path) -> Result<Vec<ProviderDef>, AppError> {
    providers_from_root(
        &JsoncDoc::read(opencode_file, OPENCODE_SCHEMA)?
            .raw()
            .clone(),
    )
}

pub fn custom_provider_ids(
    opencode_file: &Path,
) -> Result<std::collections::HashSet<String>, AppError> {
    let doc = JsoncDoc::read(opencode_file, OPENCODE_SCHEMA)?;
    let raw = doc.raw();
    let mut set = std::collections::HashSet::new();
    if let Some(Value::Object(providers)) = raw.get("provider") {
        for key in providers.keys() {
            set.insert(key.clone());
        }
    }
    Ok(set)
}

pub fn is_custom_provider(opencode_file: &Path, provider_id: &str) -> Result<bool, AppError> {
    Ok(custom_provider_ids(opencode_file)?.contains(provider_id))
}

fn ensure_custom_provider(opencode_file: &Path, provider_id: &str) -> Result<(), AppError> {
    if !is_custom_provider(opencode_file, provider_id)? {
        return Err(AppError::validation(format!(
            "model operations are only allowed for custom providers: {provider_id}"
        )));
    }
    Ok(())
}

pub fn get_provider(opencode_file: &Path, provider_id: &str) -> Result<ProviderDef, AppError> {
    validate_id(provider_id, IdKind::Provider)?;
    list_providers(opencode_file)?
        .into_iter()
        .find(|provider| provider.name == provider_id)
        .ok_or_else(|| AppError::not_found(format!("provider not found: {provider_id}")))
}

pub fn create_provider(
    opencode_file: &Path,
    provider: ProviderDef,
) -> Result<ProviderDef, AppError> {
    validate_provider(&provider)?;
    if provider
        .npm
        .as_deref()
        .map_or(true, |npm| npm.trim().is_empty())
    {
        return Err(AppError::validation("provider npm adapter is required"));
    }
    let mut document = JsoncDoc::read(opencode_file, OPENCODE_SCHEMA)?;
    let providers = providers_from_root(&document.raw().clone())?;
    if providers.iter().any(|item| item.name == provider.name) {
        return Err(AppError::validation(format!(
            "provider already exists: {}",
            provider.name
        )));
    }
    document.patch(
        &["provider", &provider.name],
        Some(provider_value(&provider)),
    )?;
    document.save(opencode_file)?;
    Ok(provider)
}

pub fn update_provider(
    opencode_file: &Path,
    provider: ProviderDef,
) -> Result<ProviderDef, AppError> {
    validate_provider(&provider)?;
    let mut document = JsoncDoc::read(opencode_file, OPENCODE_SCHEMA)?;
    let providers = providers_from_root(&document.raw().clone())?;
    let current = providers
        .iter()
        .find(|item| item.name == provider.name)
        .ok_or_else(|| AppError::not_found(format!("provider not found: {}", provider.name)))?;
    if current.models != provider.models {
        return Err(AppError::validation(format!(
            "provider updates cannot change models directly: provider.{}.models",
            provider.name
        )));
    }
    apply_patches(&mut document, provider_field_patches(&provider))?;
    document.save(opencode_file)?;
    Ok(provider)
}

pub fn delete_provider(opencode_file: &Path, provider_id: &str) -> Result<(), AppError> {
    validate_id(provider_id, IdKind::Provider)?;
    let provider = get_provider(opencode_file, provider_id)?;
    if !provider.models.is_empty() {
        return Err(AppError::validation(format!(
            "provider still contains models: {provider_id}"
        )));
    }
    let mut document = JsoncDoc::read(opencode_file, OPENCODE_SCHEMA)?;
    document.patch(&["provider", provider_id], None)?;
    document.save(opencode_file)?;
    Ok(())
}

pub fn create_model(
    opencode_file: &Path,
    provider_id: &str,
    model: ModelDef,
) -> Result<ModelDef, AppError> {
    validate_id(provider_id, IdKind::Provider)?;
    validate_id(&model.id, IdKind::Model)?;
    ensure_custom_provider(opencode_file, provider_id)?;
    let mut document = JsoncDoc::read(opencode_file, OPENCODE_SCHEMA)?;
    let providers = providers_from_root(&document.raw().clone())?;
    let provider = providers
        .iter()
        .find(|item| item.name == provider_id)
        .ok_or_else(|| AppError::not_found(format!("provider not found: {provider_id}")))?;
    if provider.models.contains_key(&model.id) {
        return Err(AppError::validation(format!(
            "model already exists: {}/{}",
            provider_id, model.id
        )));
    }
    document.patch(
        &["provider", provider_id, "models", &model.id],
        Some(model_value(&model)),
    )?;
    document.save(opencode_file)?;
    Ok(model)
}

pub fn update_model(
    opencode_file: &Path,
    model_ref: &ModelRef,
    model: ModelDef,
) -> Result<ModelDef, AppError> {
    validate_id(&model.id, IdKind::Model)?;
    if model.id != model_ref.model_id {
        return Err(AppError::validation(format!(
            "invalid model ID: {}",
            model.id
        )));
    }
    ensure_custom_provider(opencode_file, &model_ref.provider_id)?;
    let mut document = JsoncDoc::read(opencode_file, OPENCODE_SCHEMA)?;
    ensure_model_exists(&document.raw(), model_ref)?;
    apply_patches(&mut document, model_field_patches(model_ref, &model))?;
    document.save(opencode_file)?;
    Ok(model)
}

pub fn delete_model(opencode_file: &Path, model_ref: &ModelRef) -> Result<(), AppError> {
    ensure_custom_provider(opencode_file, &model_ref.provider_id)?;
    let mut document = JsoncDoc::read(opencode_file, OPENCODE_SCHEMA)?;
    ensure_model_exists(&document.raw(), model_ref)?;
    let path = model_path_vec(model_ref);
    let segments: Vec<&str> = path.iter().map(String::as_str).collect();
    document.patch(&segments, None)?;
    document.save(opencode_file)?;
    Ok(())
}

fn apply_patches(
    document: &mut JsoncDoc,
    patches: Vec<(Vec<String>, Option<Value>)>,
) -> Result<(), AppError> {
    for (path, value) in patches {
        let segments: Vec<_> = path.iter().map(String::as_str).collect();
        document.patch(&segments, value)?;
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderPayload {
    npm: Option<String>,
    options: Option<ProviderOptions>,
    #[serde(default)]
    models: BTreeMap<String, Value>,
}

fn providers_from_root(root: &Map<String, Value>) -> Result<Vec<ProviderDef>, AppError> {
    let providers = match root.get("provider") {
        None => return Ok(Vec::new()),
        Some(Value::Object(providers)) => providers,
        Some(_) => {
            return Err(AppError::validation(
                "invalid OpenCode provider shape at: provider",
            ))
        }
    };
    let mut parsed = providers
        .iter()
        .map(|(name, value)| provider_from_value(name, value))
        .collect::<Result<Vec<_>, _>>()?;
    parsed.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(parsed)
}

pub fn provider_from_root_value(
    root: &Map<String, Value>,
    provider_id: &str,
) -> Result<ProviderDef, AppError> {
    root.get("provider")
        .and_then(Value::as_object)
        .and_then(|providers| providers.get(provider_id))
        .ok_or_else(|| AppError::not_found(format!("provider not found: {provider_id}")))
        .and_then(|value| provider_from_value(provider_id, value))
}

fn provider_from_value(name: &str, value: &Value) -> Result<ProviderDef, AppError> {
    validate_id(name, IdKind::Provider)?;
    let payload: ProviderPayload = serde_json::from_value(value.clone()).map_err(|_| {
        AppError::validation(format!(
            "invalid OpenCode provider shape at: provider.{name}"
        ))
    })?;
    let mut models = BTreeMap::new();
    for (model_id, model_value) in payload.models {
        let model = model_from_value(&model_id, model_value)?;
        models.insert(model_id, model);
    }
    Ok(ProviderDef {
        name: name.to_owned(),
        npm: payload.npm,
        options: payload.options,
        models,
    })
}

pub fn model_from_value(id: &str, value: Value) -> Result<ModelDef, AppError> {
    validate_id(id, IdKind::Model)?;
    #[derive(Deserialize)]
    #[serde(rename_all = "snake_case")]
    struct ModelPayload {
        name: Option<String>,
        family: Option<String>,
        release_date: Option<String>,
        status: Option<crate::models::ModelStatus>,
        reasoning: Option<bool>,
        temperature: Option<f64>,
        tool_call: Option<bool>,
        attachment: Option<bool>,
        interleaved: Option<bool>,
        cost: Option<crate::models::ModelCost>,
        limit: Option<crate::models::ModelLimit>,
        modalities: Option<crate::models::ModelModalities>,
        experimental: Option<bool>,
        options: Option<BTreeMap<String, Value>>,
        headers: Option<BTreeMap<String, String>>,
        variants: Option<BTreeMap<String, crate::models::ModelVariant>>,
    }
    let payload: ModelPayload = serde_json::from_value(value).map_err(|_| {
        AppError::validation(format!(
            "invalid OpenCode provider shape at: provider.*.models.{id}"
        ))
    })?;
    Ok(ModelDef {
        id: id.to_owned(),
        name: payload.name,
        family: payload.family,
        release_date: payload.release_date,
        status: payload.status,
        reasoning: payload.reasoning,
        temperature: payload.temperature,
        tool_call: payload.tool_call,
        attachment: payload.attachment,
        interleaved: payload.interleaved,
        cost: payload.cost,
        limit: payload.limit,
        modalities: payload.modalities,
        experimental: payload.experimental,
        options: payload.options,
        headers: payload.headers,
        variants: payload.variants,
    })
}

fn provider_value(provider: &ProviderDef) -> Value {
    let mut value = Map::new();
    put_optional(&mut value, "npm", provider.npm.clone().map(Value::String));
    put_optional(
        &mut value,
        "options",
        provider.options.clone().and_then(to_value),
    );
    let models = provider
        .models
        .iter()
        .map(|(id, model)| (id.clone(), model_value(model)))
        .collect();
    value.insert("models".to_owned(), Value::Object(models));
    Value::Object(value)
}

pub fn model_value(model: &ModelDef) -> Value {
    let mut value = Map::new();
    put_optional(&mut value, "name", model.name.clone().map(Value::String));
    put_optional(
        &mut value,
        "family",
        model.family.clone().map(Value::String),
    );
    put_optional(
        &mut value,
        "release_date",
        model.release_date.clone().map(Value::String),
    );
    put_optional(&mut value, "status", model.status.and_then(to_value));
    put_optional(&mut value, "reasoning", model.reasoning.map(Value::Bool));
    put_optional(
        &mut value,
        "temperature",
        model
            .temperature
            .and_then(|number| serde_json::Number::from_f64(number).map(Value::Number)),
    );
    put_optional(&mut value, "tool_call", model.tool_call.map(Value::Bool));
    put_optional(&mut value, "attachment", model.attachment.map(Value::Bool));
    put_optional(
        &mut value,
        "interleaved",
        model.interleaved.map(Value::Bool),
    );
    put_optional(&mut value, "cost", model.cost.clone().and_then(to_value));
    put_optional(&mut value, "limit", model.limit.clone().and_then(to_value));
    put_optional(
        &mut value,
        "modalities",
        model.modalities.clone().and_then(to_value),
    );
    put_optional(
        &mut value,
        "experimental",
        model.experimental.map(Value::Bool),
    );
    put_optional(
        &mut value,
        "options",
        model.options.clone().and_then(to_value),
    );
    put_optional(
        &mut value,
        "headers",
        model.headers.clone().and_then(to_value),
    );
    put_optional(
        &mut value,
        "variants",
        model.variants.clone().and_then(to_value),
    );
    Value::Object(value)
}

fn provider_field_patches(provider: &ProviderDef) -> Vec<(Vec<String>, Option<Value>)> {
    let base = vec!["provider".to_owned(), provider.name.clone()];
    [
        ("npm", provider.npm.clone().map(Value::String)),
        ("options", provider.options.clone().and_then(to_value)),
    ]
    .into_iter()
    .map(|(field, value)| {
        let mut path = base.clone();
        path.push(field.to_owned());
        (path, value)
    })
    .collect()
}

const MODEL_FIELDS: &[&str] = &[
    "name",
    "family",
    "release_date",
    "status",
    "reasoning",
    "temperature",
    "tool_call",
    "attachment",
    "interleaved",
    "cost",
    "limit",
    "modalities",
    "experimental",
    "options",
    "headers",
    "variants",
];

fn model_field_patches(
    model_ref: &ModelRef,
    model: &ModelDef,
) -> Vec<(Vec<String>, Option<Value>)> {
    let base = model_path_vec(model_ref);
    let updated = model_value(model)
        .as_object()
        .expect("model value is an object")
        .clone();
    updated
        .iter()
        .map(|(field, value)| {
            let mut path = base.clone();
            path.push(field.clone());
            (path, Some(value.clone()))
        })
        .chain(
            MODEL_FIELDS
                .iter()
                .filter(|field| !updated.contains_key(**field))
                .map(|field| {
                    let mut path = base.clone();
                    path.push((*field).to_owned());
                    (path, None)
                }),
        )
        .collect()
}

pub fn model_path_vec(model_ref: &ModelRef) -> Vec<String> {
    vec![
        "provider".to_owned(),
        model_ref.provider_id.clone(),
        "models".to_owned(),
        model_ref.model_id.clone(),
    ]
}

fn put_optional(target: &mut Map<String, Value>, key: &str, value: Option<Value>) {
    if let Some(value) = value {
        target.insert(key.to_owned(), value);
    }
}

fn to_value<T: serde::Serialize>(value: T) -> Option<Value> {
    serde_json::to_value(value).ok()
}

fn validate_provider(provider: &ProviderDef) -> Result<(), AppError> {
    validate_id(&provider.name, IdKind::Provider)?;
    for (id, model) in &provider.models {
        if id != &model.id {
            return Err(AppError::validation(format!(
                "invalid model ID: {}",
                model.id
            )));
        }
        validate_id(&model.id, IdKind::Model)?;
    }
    Ok(())
}

enum IdKind {
    Provider,
    Model,
}

fn validate_id(value: &str, kind: IdKind) -> Result<(), AppError> {
    if !value.is_empty()
        && value.trim() == value
        && !value.contains('/')
        && !value.chars().any(char::is_whitespace)
    {
        return Ok(());
    }
    Err(match kind {
        IdKind::Provider => AppError::validation(format!("invalid provider ID: {value}")),
        IdKind::Model => AppError::validation(format!("invalid model ID: {value}")),
    })
}

fn ensure_model_exists(root: &Map<String, Value>, model_ref: &ModelRef) -> Result<(), AppError> {
    let exists = provider_from_root_value(root, &model_ref.provider_id)
        .map(|provider| provider.models.contains_key(&model_ref.model_id))
        .or_else(|error| match error.code {
            crate::error::ErrorCode::NotFound => Ok(false),
            _ => Err(error),
        })?;
    if exists {
        Ok(())
    } else {
        Err(AppError::not_found(format!(
            "model not found: {}",
            model_ref.as_str()
        )))
    }
}
