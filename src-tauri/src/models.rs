use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(rename = "baseURL", skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDef {
    pub name: String,
    pub package: Option<String>,
    pub settings: Option<ProviderSettings>,
    pub headers: Option<BTreeMap<String, String>>,
    #[serde(default)]
    pub models: BTreeMap<String, ModelDef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ModelDef {
    pub id: String,
    pub name: Option<String>,
    pub family: Option<String>,
    pub disabled: Option<bool>,
    pub capabilities: Option<ModelCapabilities>,
    pub cost: Option<ModelCost>,
    pub limit: Option<ModelLimit>,
    pub settings: Option<BTreeMap<String, Value>>,
    pub headers: Option<BTreeMap<String, String>>,
    pub variants: Option<Vec<ModelVariant>>,
}

/// V2 model capabilities: tool support plus accepted input/output media types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ModelCapabilities {
    pub tools: Option<bool>,
    pub input: Option<Vec<String>>,
    pub output: Option<Vec<String>>,
}

/// V2 cost per million tokens; cache pricing nests under `cache.read` / `cache.write`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ModelCost {
    pub input: Option<f64>,
    pub output: Option<f64>,
    pub cache: Option<ModelCacheCost>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ModelCacheCost {
    pub read: Option<f64>,
    pub write: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ModelLimit {
    pub context: Option<u64>,
    pub input: Option<u64>,
    pub output: Option<u64>,
}

/// One V2 variant entry: `{ "id", "settings"?, "headers"?, "body"? }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ModelVariant {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<BTreeMap<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentModelBinding {
    pub agent_name: String,
    pub model_ref: String,
    pub variant: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OmoCategoryMapping {
    pub category_name: String,
    pub model_ref: String,
    pub variant: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GroupType {
    // Variant names use the canonical native/slim/omo vocabulary. The serde
    // strings are frozen persisted values required for config compatibility.
    #[serde(rename = "opencode")]
    Native,
    #[serde(rename = "slim")]
    Slim,
    #[serde(rename = "oh-my-openagent")]
    Omo,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelGroup {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    #[serde(rename = "type")]
    pub group_type: GroupType,
    #[serde(default)]
    pub open_code_agent_overrides: Vec<AgentModelBinding>,
    #[serde(default)]
    pub slim_agent_overrides: Option<Vec<AgentModelBinding>>,
    #[serde(default)]
    pub omo_agent_overrides: Option<Vec<AgentModelBinding>>,
    #[serde(default)]
    pub omo_category_mappings: Option<Vec<OmoCategoryMapping>>,
    pub is_enabled: bool,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ThemePreference {
    Light,
    #[default]
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum LocalePreference {
    #[default]
    En,
    Zh,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DefaultPlugin {
    #[default]
    Native,
    Slim,
    Omo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppPreferences {
    pub theme: ThemePreference,
    pub locale: LocalePreference,
    #[serde(default)]
    pub default_plugin: DefaultPlugin,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppSelectionState {
    #[serde(rename = "selectedGroupID")]
    pub selected_group_id: Option<Uuid>,
    pub selected_group_name: Option<String>,
    #[serde(default)]
    pub selected_project_id: Option<Uuid>,
    #[serde(default)]
    pub preferences: AppPreferences,
    #[serde(default)]
    pub island_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectFolder {
    pub id: Uuid,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    #[serde(default)]
    pub groups: Vec<ModelGroup>,
    #[serde(default)]
    pub projects: Vec<ProjectFolder>,
    #[serde(default)]
    pub state: AppSelectionState,
}

impl AppConfig {
    /// Lightweight structural validation. Type checking is already done by serde; this only
    /// rejects values serde cannot see.
    pub fn validate(&self) -> Result<(), AppError> {
        for group in &self.groups {
            if group.name.trim().is_empty() {
                return Err(AppError::validation(format!(
                    "group '{}' has an empty name",
                    group.id
                )));
            }
        }
        Ok(())
    }
}

/// Reads the application config file. A missing file is an empty default configuration.
pub fn load_config(path: &Path) -> Result<AppConfig, AppError> {
    let content = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(AppConfig::default())
        }
        Err(error) => return Err(AppError::io("read", path, error)),
    };
    let mut config: AppConfig = serde_json::from_slice(&content).map_err(|error| {
        AppError::validation(format!("application config: invalid JSON: {error}"))
    })?;
    config.state.selected_project_id = config
        .state
        .selected_project_id
        .filter(|id| config.projects.iter().any(|project| project.id == *id));
    Ok(config)
}

pub fn add_project(config_file: &Path, path: &str) -> Result<ProjectFolder, AppError> {
    let _write = config_write_lock();
    let canonical = crate::paths::canonical_project_directory(Path::new(path))?;
    let path = canonical
        .to_str()
        .ok_or_else(|| AppError::validation("project path must be valid UTF-8"))?
        .to_owned();
    let mut config = load_config(config_file)?;
    let project = match config.projects.iter().find(|project| project.path == path) {
        Some(project) => project.clone(),
        None => {
            let project = ProjectFolder {
                id: Uuid::new_v4(),
                path,
            };
            config.projects.push(project.clone());
            project
        }
    };
    config.state.selected_project_id = Some(project.id);
    save_config(config_file, &config)?;
    Ok(project)
}

pub fn remove_project(config_file: &Path, id: Uuid) -> Result<(), AppError> {
    let _write = config_write_lock();
    let mut config = load_config(config_file)?;
    let index = config
        .projects
        .iter()
        .position(|project| project.id == id)
        .ok_or_else(|| AppError::not_found("project not found"))?;
    config.projects.remove(index);
    if config.state.selected_project_id == Some(id) {
        config.state.selected_project_id = None;
    }
    save_config(config_file, &config)
}

pub fn select_project(config_file: &Path, id: Option<Uuid>) -> Result<(), AppError> {
    let _write = config_write_lock();
    let mut config = load_config(config_file)?;
    if id.is_some_and(|id| !config.projects.iter().any(|project| project.id == id)) {
        return Err(AppError::not_found("project not found"));
    }
    config.state.selected_project_id = id;
    save_config(config_file, &config)
}

// ponytail: one app-config writer; split by path only if write contention matters.
static CONFIG_WRITE: Mutex<()> = Mutex::new(());

pub(crate) fn config_write_lock() -> MutexGuard<'static, ()> {
    CONFIG_WRITE
        .lock()
        .unwrap_or_else(|error| error.into_inner())
}

/// Publishes the application config atomically without recovery backups.
pub fn save_config(path: &Path, config: &AppConfig) -> Result<(), AppError> {
    let data = serde_json::to_vec_pretty(config)
        .map_err(|error| AppError::validation(format!("application config: {error}")))?;
    let target = match fs::canonicalize(path) {
        Ok(target) => target,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            crate::resource_file::absent(path)?;
            path.to_owned()
        }
        Err(error) => return Err(AppError::io("resolve application config", path, error)),
    };
    let directory = target
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(directory).map_err(|error| AppError::io("create", directory, error))?;
    let permissions = match fs::metadata(&target) {
        Ok(metadata) if metadata.is_file() && !metadata.permissions().readonly() => {
            Some(metadata.permissions())
        }
        Ok(_) => {
            return Err(AppError::configuration(format!(
                "application config is not a writable regular file: {}",
                path.display()
            )))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(AppError::io("inspect application config", path, error)),
    };
    #[cfg(unix)]
    let directory_handle =
        fs::File::open(directory).map_err(|error| AppError::io("open", directory, error))?;
    let temporary = directory.join(format!(".config-{}.pending", Uuid::new_v4()));
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&temporary)
        .map_err(|error| AppError::io("create", &temporary, error))?;
    let result = (|| {
        file.write_all(&data)
            .map_err(|error| AppError::io("write", &temporary, error))?;
        if let Some(permissions) = permissions {
            file.set_permissions(permissions)
                .map_err(|error| AppError::io("set permissions", &temporary, error))?;
        }
        file.sync_all()
            .map_err(|error| AppError::io("sync", &temporary, error))?;
        drop(file);
        fs::rename(&temporary, &target)
            .map_err(|error| AppError::io("publish application config", path, error))?;
        #[cfg(unix)]
        if directory_handle.sync_all().is_err() {
            // Publication committed; callers must not roll back a successful save.
            eprintln!("Application config saved, but directory sync failed");
        }
        Ok(())
    })();
    let _ = fs::remove_file(&temporary);
    result
}

#[cfg(test)]
mod tests;
