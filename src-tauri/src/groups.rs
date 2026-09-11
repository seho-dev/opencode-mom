use uuid::Uuid;

use crate::document::{JsoncDoc, OMO_SCHEMA, OPENCODE_SCHEMA, SLIM_SCHEMA};
use crate::error::AppError;
use crate::models::{self, AppConfig, AppSelectionState, GroupType, ModelGroup};
use crate::paths::ConfigPaths;
use crate::projection;

fn load(paths: &ConfigPaths) -> Result<AppConfig, AppError> {
    let config = models::load_config(&paths.config_file())?;
    config.validate()?;
    Ok(config)
}

fn find_enabled_group<'a>(
    config: &'a AppConfig,
    group_id: Uuid,
) -> Result<&'a ModelGroup, AppError> {
    let group = config
        .groups
        .iter()
        .find(|group| group.id == group_id)
        .ok_or_else(|| AppError::not_found("group not found"))?;
    if !group.is_enabled {
        return Err(AppError::validation("group is disabled"));
    }
    Ok(group)
}

/// Saves a group and re-projects Slim/OMO/OpenCode files when the group is selected or
/// was selected before. Plain sequential writes.
pub fn save_group(
    paths: &ConfigPaths,
    mut group: ModelGroup,
) -> Result<(ModelGroup, AppConfig), AppError> {
    let mut config = load(paths)?;
    if config
        .groups
        .iter()
        .any(|item| item.id != group.id && item.name.trim().eq_ignore_ascii_case(group.name.trim()))
    {
        return Err(AppError::validation("duplicate group name"));
    }
    validate_group_drafts(paths, &config, &group)?;
    let previous = config
        .groups
        .iter()
        .find(|item| item.id == group.id)
        .cloned();
    group.updated_at = time::OffsetDateTime::now_utc();

    if config.state.selected_group_id == Some(group.id) && !group.is_enabled {
        return Err(AppError::validation(
            "the selected group cannot be disabled; select another group first",
        ));
    }

    if let Some(existing) = config.groups.iter_mut().find(|item| item.id == group.id) {
        *existing = group.clone();
    } else {
        config.groups.push(group.clone());
    }
    if config.state.selected_group_id == Some(group.id) {
        config.state.selected_group_name = Some(group.name.clone());
    }
    config.validate()?;

    persist_group_lifecycle(paths, previous.as_ref(), &group, config)?;
    let config = load(paths)?;
    Ok((group, config))
}

pub fn copy_group(
    paths: &ConfigPaths,
    id: Uuid,
    name: Option<String>,
) -> Result<(ModelGroup, AppConfig), AppError> {
    let mut config = load(paths)?;
    let source = config
        .groups
        .iter()
        .find(|item| item.id == id)
        .cloned()
        .ok_or_else(|| AppError::not_found("group not found"))?;
    let copied = ModelGroup {
        id: Uuid::new_v4(),
        name: match name {
            Some(name) => {
                let name = name.trim().to_owned();
                if name.is_empty()
                    || config
                        .groups
                        .iter()
                        .any(|group| group.name.trim().eq_ignore_ascii_case(&name))
                {
                    return Err(AppError::validation("duplicate group name"));
                }
                name
            }
            None => unique_copy_group_name(&source.name, &config.groups),
        },
        updated_at: time::OffsetDateTime::now_utc(),
        ..source
    };
    config.groups.push(copied.clone());
    config.validate()?;
    models::save_config(&paths.config_file(), &config)?;
    Ok((copied, config))
}

pub fn delete_group(paths: &ConfigPaths, id: Uuid) -> Result<AppConfig, AppError> {
    let mut config = load(paths)?;
    let index = config
        .groups
        .iter()
        .position(|item| item.id == id)
        .ok_or_else(|| AppError::not_found("group not found"))?;
    let deleted = config.groups.remove(index);
    let selected = config.state.selected_group_id == Some(deleted.id);
    if selected {
        config.state = AppSelectionState::default();
    }
    config.validate()?;
    if deleted.group_type == GroupType::Slim {
        let slim_file = paths.slim_file();
        let source = JsoncDoc::read(&slim_file, SLIM_SCHEMA)?;
        let document = projection::delete_slim_preset(&source, &deleted.name)?;
        document.save(&slim_file)?;
    }
    if selected && deleted.group_type == GroupType::OhMyOpenagent {
        let omo_file = paths.omo_file();
        let source = JsoncDoc::read(&omo_file, OMO_SCHEMA)?;
        let document = projection::clear_omo(&source)?;
        document.save(&omo_file)?;
    }

    models::save_config(&paths.config_file(), &config)?;
    Ok(config)
}

/// Switches the active selection and projects the group into the managed target files.
/// Switching to the already-selected group is a no-op.
pub fn switch_group(paths: &ConfigPaths, group_id: Uuid) -> Result<(), AppError> {
    let mut config = load(paths)?;
    let group = find_enabled_group(&config, group_id)?.clone();
    if config.state.selected_group_id == Some(group_id) {
        return Ok(());
    }

    if group.group_type == GroupType::Slim {
        let slim_file = paths.slim_file();
        let source = JsoncDoc::read(&slim_file, SLIM_SCHEMA)?;
        let document = projection::project_slim_active(&group, &source)?;
        document.save(&slim_file)?;
    }
    if group.group_type == GroupType::OhMyOpenagent {
        let omo_file = paths.omo_file();
        let source = JsoncDoc::read(&omo_file, OMO_SCHEMA)?;
        let document = projection::project_omo(&group, &source)?;
        document.save(&omo_file)?;
    }
    if projection::has_effective_opencode_overrides(&group) {
        let opencode_file = paths.opencode_file();
        let source = JsoncDoc::read(&opencode_file, OPENCODE_SCHEMA)?;
        let (document, warnings) = projection::project_opencode(&group, &source)?;
        document.save(&opencode_file)?;
        for warning in warnings {
            eprintln!("warning: {warning}");
        }
    }

    config.state = AppSelectionState {
        selected_group_id: Some(group.id),
        selected_group_name: Some(group.name.clone()),
    };
    models::save_config(&paths.config_file(), &config)
}

/// Persists a group mutation plus all dependent projections in plain sequential writes.
/// Order matters only cosmetically: every write is independent.
fn persist_group_lifecycle(
    paths: &ConfigPaths,
    previous: Option<&ModelGroup>,
    group: &ModelGroup,
    config: AppConfig,
) -> Result<(), AppError> {
    let selected = config.state.selected_group_id == Some(group.id);

    if group.group_type == GroupType::Slim || switches_away_from_slim(previous, group) {
        let slim_file = paths.slim_file();
        let source = JsoncDoc::read(&slim_file, SLIM_SCHEMA)?;
        let mut document = source.clone();
        if group.group_type == GroupType::Slim {
            if let Some(previous) = previous.filter(|item| item.group_type == GroupType::Slim) {
                if previous.name != group.name {
                    document =
                        projection::rename_slim_preset(&document, &previous.name, &group.name)?;
                }
            }
            document = projection::update_slim_preset(
                &document,
                &group.name,
                group.slim_agent_overrides.as_deref(),
            )?;
            if selected {
                document = projection::project_slim_active(group, &document)?;
            }
        } else if let Some(previous) = previous.filter(|item| item.group_type == GroupType::Slim) {
            document = projection::delete_slim_preset(&document, &previous.name)?;
        }
        document.save(&slim_file)?;
    }

    let switched_from_omo = selected
        && previous.is_some_and(|item| item.group_type == GroupType::OhMyOpenagent)
        && group.group_type != GroupType::OhMyOpenagent;
    if (group.group_type == GroupType::OhMyOpenagent && selected) || switched_from_omo {
        let omo_file = paths.omo_file();
        let source = JsoncDoc::read(&omo_file, OMO_SCHEMA)?;
        let document = if group.group_type == GroupType::OhMyOpenagent && selected {
            projection::project_omo(group, &source)?
        } else {
            projection::clear_omo(&source)?
        };
        document.save(&omo_file)?;
    }

    if selected && projection::has_effective_opencode_overrides(group) {
        let opencode_file = paths.opencode_file();
        let source = JsoncDoc::read(&opencode_file, OPENCODE_SCHEMA)?;
        let (document, _) = projection::project_opencode(group, &source)?;
        document.save(&opencode_file)?;
    }

    models::save_config(&paths.config_file(), &config)
}

fn switches_away_from_slim(previous: Option<&ModelGroup>, group: &ModelGroup) -> bool {
    previous.is_some_and(|item| item.group_type == GroupType::Slim)
        && group.group_type != GroupType::Slim
}

fn validate_group_drafts(
    paths: &ConfigPaths,
    _config: &AppConfig,
    group: &ModelGroup,
) -> Result<(), AppError> {
    let providers = crate::providers::list_providers(&paths.opencode_file())?;
    for (field, name, model) in group_model_references(group) {
        let model_ref = crate::providers::ModelRef::parse(model).map_err(|error| {
            AppError::validation(format!(
                "invalid model reference in group '{}', {field} '{name}': {error}",
                group.name
            ))
        })?;
        let exists = providers
            .iter()
            .find(|provider| provider.name == model_ref.provider_id)
            .is_some_and(|provider| provider.models.contains_key(&model_ref.model_id));
        if !exists {
            return Err(AppError::validation(format!(
                "model '{}' does not exist for group '{}', {field} '{name}'",
                model_ref.as_str(),
                group.name
            )));
        }
    }
    Ok(())
}

fn group_model_references<'a>(group: &'a ModelGroup) -> Vec<(&'static str, &'a str, &'a str)> {
    let mut references = Vec::new();
    references.extend(group.open_code_agent_overrides.iter().map(|binding| {
        (
            "openCodeAgentOverrides",
            binding.agent_name.as_str(),
            binding.model_ref.as_str(),
        )
    }));
    references.extend(group.slim_agent_overrides.iter().flatten().map(|binding| {
        (
            "slimAgentOverrides",
            binding.agent_name.as_str(),
            binding.model_ref.as_str(),
        )
    }));
    references.extend(group.omo_agent_overrides.iter().flatten().map(|binding| {
        (
            "omoAgentOverrides",
            binding.agent_name.as_str(),
            binding.model_ref.as_str(),
        )
    }));
    references.extend(group.omo_category_mappings.iter().flatten().map(|mapping| {
        (
            "omoCategoryMappings",
            mapping.category_name.as_str(),
            mapping.model_ref.as_str(),
        )
    }));
    references
}

fn unique_copy_group_name(source: &str, groups: &[ModelGroup]) -> String {
    let base = if source.trim().is_empty() {
        "Copy".to_owned()
    } else {
        format!("{} Copy", source.trim())
    };
    if !groups
        .iter()
        .any(|group| group.name.trim().eq_ignore_ascii_case(&base))
    {
        return base;
    }
    (2..)
        .map(|number| format!("{base} {number}"))
        .find(|candidate| {
            !groups
                .iter()
                .any(|group| group.name.trim().eq_ignore_ascii_case(candidate))
        })
        .expect("unbounded copy suffix iterator")
}
