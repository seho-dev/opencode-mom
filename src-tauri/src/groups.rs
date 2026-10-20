use std::path::PathBuf;
use uuid::Uuid;

use crate::document::{JsoncDoc, OMO_SCHEMA, OPENCODE_SCHEMA, SLIM_SCHEMA};
use crate::error::AppError;
use crate::models::{self, AppConfig, GroupType, ModelGroup};
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
    let _write = models::config_write_lock();
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
    let _write = models::config_write_lock();
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

/// Removes a group from opencode-mom's own config only. Target config files are left
/// untouched, so their mappings stay behind as residual configuration.
pub fn delete_group(paths: &ConfigPaths, id: Uuid) -> Result<AppConfig, AppError> {
    let _write = models::config_write_lock();
    let mut config = load(paths)?;
    let index = config
        .groups
        .iter()
        .position(|item| item.id == id)
        .ok_or_else(|| AppError::not_found("group not found"))?;
    let deleted = config.groups.remove(index);
    if config.state.selected_group_id == Some(deleted.id) {
        config.state.selected_group_id = None;
        config.state.selected_group_name = None;
    }
    config.validate()?;
    models::save_config(&paths.config_file(), &config)?;
    Ok(config)
}

/// Switches the active selection and projects the group into the managed target files.
/// Switching to the already-selected group is a no-op.
pub fn switch_group(paths: &ConfigPaths, group_id: Uuid) -> Result<(), AppError> {
    let _write = models::config_write_lock();
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
    if group.group_type == GroupType::Omo {
        let omo_file = paths.omo_file();
        let source = JsoncDoc::read(&omo_file, OMO_SCHEMA)?;
        let document = projection::project_omo(&group, &source)?;
        document.save(&omo_file)?;
    }
    if group.group_type == GroupType::Native && projection::has_effective_opencode_overrides(&group)
    {
        let opencode_file = paths.opencode_file();
        let source = JsoncDoc::read(&opencode_file, OPENCODE_SCHEMA)?;
        let (document, warnings) = projection::project_opencode(&group, &source)?;
        document.save(&opencode_file)?;
        for warning in warnings {
            eprintln!("warning: {warning}");
        }
    }

    config.state.selected_group_id = Some(group.id);
    config.state.selected_group_name = Some(group.name.clone());
    models::save_config(&paths.config_file(), &config)
}

/// Persists a group mutation plus all dependent projections using a two-phase commit.
/// Phase 1 (Plan / Preflight): in-memory read and transform of all target documents.
/// Phase 2 (Publish): saves all planned documents and finally commits AppConfig.
fn persist_group_lifecycle(
    paths: &ConfigPaths,
    previous: Option<&ModelGroup>,
    group: &ModelGroup,
    config: AppConfig,
) -> Result<(), AppError> {
    let selected = config.state.selected_group_id == Some(group.id);

    let mut slim_plan: Option<(PathBuf, JsoncDoc)> = None;
    let mut omo_plan: Option<(PathBuf, JsoncDoc)> = None;
    let mut opencode_plan: Option<(PathBuf, JsoncDoc)> = None;

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
        slim_plan = Some((slim_file, document));
    }

    let switched_from_omo = selected
        && previous.is_some_and(|item| item.group_type == GroupType::Omo)
        && group.group_type != GroupType::Omo;
    if (group.group_type == GroupType::Omo && selected) || switched_from_omo {
        let omo_file = paths.omo_file();
        let source = JsoncDoc::read(&omo_file, OMO_SCHEMA)?;
        let document = if group.group_type == GroupType::Omo && selected {
            projection::project_omo(group, &source)?
        } else if let Some(previous) = previous {
            projection::remove_omo_mappings(previous, &source)?
        } else {
            source.clone()
        };
        omo_plan = Some((omo_file, document));
    }

    let switched_from_native = selected
        && previous.is_some_and(|item| item.group_type == GroupType::Native)
        && group.group_type != GroupType::Native;
    if selected
        && group.group_type == GroupType::Native
        && projection::has_effective_opencode_overrides(group)
    {
        let opencode_file = paths.opencode_file();
        let source = JsoncDoc::read(&opencode_file, OPENCODE_SCHEMA)?;
        let (document, _) = projection::project_opencode(group, &source)?;
        opencode_plan = Some((opencode_file, document));
    } else if switched_from_native {
        if let Some(previous) =
            previous.filter(|item| projection::has_effective_opencode_overrides(item))
        {
            let opencode_file = paths.opencode_file();
            let source = JsoncDoc::read(&opencode_file, OPENCODE_SCHEMA)?;
            let document = projection::remove_opencode_overrides(previous, &source)?;
            opencode_plan = Some((opencode_file, document));
        }
    }

    if let Some((path, doc)) = slim_plan {
        doc.save(&path)?;
    }
    if let Some((path, doc)) = omo_plan {
        doc.save(&path)?;
    }
    if let Some((path, doc)) = opencode_plan {
        doc.save(&path)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn save_group_two_phase_commit_aborts_before_writing_when_target_is_corrupt() {
        let temp_dir = tempfile::tempdir().unwrap();
        let paths = ConfigPaths::for_home(temp_dir.path());

        fs::create_dir_all(paths.opencode_file().parent().unwrap()).unwrap();
        fs::write(
            paths.opencode_file(),
            r#"{
  "$schema": "https://opencode.ai/config.json",
  "providers": {
    "acme": {
      "models": {
        "fast": {}
      }
    }
  }
}"#,
        )
        .unwrap();

        fs::create_dir_all(paths.slim_file().parent().unwrap()).unwrap();
        fs::write(paths.slim_file(), "{\n  \"presets\": {}\n}\n").unwrap();

        fs::create_dir_all(paths.omo_file().parent().unwrap()).unwrap();
        fs::write(paths.omo_file(), "{\n}\n").unwrap();

        let slim_group = ModelGroup {
            id: Uuid::new_v4(),
            name: "test-slim".to_owned(),
            description: "test".to_owned(),
            group_type: GroupType::Slim,
            open_code_agent_overrides: Vec::new(),
            slim_agent_overrides: Some(vec![crate::models::AgentModelBinding {
                agent_name: "reviewer".to_owned(),
                model_ref: "acme/fast".to_owned(),
                variant: None,
            }]),
            omo_agent_overrides: None,
            omo_category_mappings: None,
            is_enabled: true,
            updated_at: time::OffsetDateTime::now_utc(),
        };

        let (saved_group, _) = save_group(&paths, slim_group).unwrap();
        switch_group(&paths, saved_group.id).unwrap();

        let slim_content_before = fs::read_to_string(paths.slim_file()).unwrap();
        assert!(slim_content_before.contains("test-slim"));
        assert!(slim_content_before.contains("acme/fast"));

        // Corrupt OMO file so Phase 1 parsing will fail.
        fs::write(paths.omo_file(), "{").unwrap();

        // Switch selected group type from Slim to OMO.
        let mut omo_group = saved_group.clone();
        omo_group.group_type = GroupType::Omo;
        omo_group.omo_agent_overrides = Some(vec![crate::models::AgentModelBinding {
            agent_name: "reviewer".to_owned(),
            model_ref: "acme/fast".to_owned(),
            variant: None,
        }]);

        let result = save_group(&paths, omo_group);
        assert!(
            result.is_err(),
            "save_group must fail on corrupt OMO target"
        );

        // Slim file must be completely untouched - preset preserved!
        let slim_content_after = fs::read_to_string(paths.slim_file()).unwrap();
        assert_eq!(
            slim_content_after, slim_content_before,
            "Slim preset must remain unmodified when commit aborts in Phase 1"
        );

        // AppConfig must still hold the original Slim group.
        let config = models::load_config(&paths.config_file()).unwrap();
        let group_in_config = config
            .groups
            .iter()
            .find(|g| g.id == saved_group.id)
            .expect("group exists in config");
        assert_eq!(group_in_config.group_type, GroupType::Slim);
    }
}
