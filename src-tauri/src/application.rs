use std::fmt;
use std::path::Path;

use time::OffsetDateTime;
use uuid::Uuid;

use crate::core::models::{AppSelectionState, ModelGroup};
use crate::core::paths::{ConfigPathError, ConfigPaths, HomeEnv};
use crate::core::repository::{
    ConfigRepository, OhMyOpenAgentConfigRepository, OpenCodeConfigError, OpenCodeConfigRepository,
    RepositoryError,
};
use crate::core::switching::{
    recover_pending_transaction, SwitchError, SwitchGroupRepositories, SwitchGroupUseCase,
    SwitchOutcome,
};

#[derive(Debug)]
pub enum ApplicationError {
    ConfigPath(ConfigPathError),
    LoadGroups(RepositoryError),
    SaveGroups(RepositoryError),
    LoadAppState(RepositoryError),
    SaveAppState(RepositoryError),
    GroupNotFound,
    DuplicateGroupName,
    Switch(SwitchError),
}

impl fmt::Display for ApplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigPath(error) => write!(formatter, "{error}"),
            Self::LoadGroups(error) => write!(formatter, "failed to load groups: {error}"),
            Self::SaveGroups(error) => write!(formatter, "failed to save groups: {error}"),
            Self::LoadAppState(error) => write!(formatter, "failed to load app state: {error}"),
            Self::SaveAppState(error) => write!(formatter, "failed to save app state: {error}"),
            Self::GroupNotFound => formatter.write_str("group not found"),
            Self::DuplicateGroupName => formatter.write_str("duplicate group name"),
            Self::Switch(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for ApplicationError {}

#[derive(Debug, Clone, PartialEq)]
pub struct LoadedAppState {
    pub groups: Vec<ModelGroup>,
    pub app_state: AppSelectionState,
    pub discovered_open_code_agent_names: Vec<String>,
    pub open_code_agent_discovery_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupMutation {
    pub group: ModelGroup,
    pub groups: Vec<ModelGroup>,
    pub app_state: AppSelectionState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupSwitchOutcome {
    Success,
    NoOp,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupSwitch {
    pub outcome: GroupSwitchOutcome,
    pub warnings: Vec<String>,
    pub app_state: AppSelectionState,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OpenCodeAgentDiscovery {
    pub agent_names: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GroupApplicationService {
    paths: ConfigPaths,
}

impl GroupApplicationService {
    pub fn live() -> Result<Self, ApplicationError> {
        ConfigPaths::from_environment()
            .map(|paths| Self { paths })
            .map_err(ApplicationError::ConfigPath)
    }

    pub fn for_home(home: &Path) -> Result<Self, ApplicationError> {
        ConfigPaths::from_home_env(HomeEnv::new(Some(home), None))
            .map(|paths| Self { paths })
            .map_err(ApplicationError::ConfigPath)
    }

    pub fn load_app_state(&self) -> Result<LoadedAppState, ApplicationError> {
        let config = self.config().load().map_err(ApplicationError::LoadGroups)?;
        let discovery = self.discovered_open_code_agents();
        Ok(LoadedAppState {
            groups: config.groups,
            app_state: config.state,
            discovered_open_code_agent_names: discovery.agent_names,
            open_code_agent_discovery_error: discovery.error,
        })
    }

    pub fn save_group(&self, group: ModelGroup) -> Result<GroupMutation, ApplicationError> {
        let group_id = group.id;
        let mut config = self.config().load().map_err(ApplicationError::LoadGroups)?;
        if config.groups.iter().any(|candidate| {
            candidate.id != group_id
                && candidate
                    .name
                    .trim()
                    .eq_ignore_ascii_case(group.name.trim())
        }) {
            return Err(ApplicationError::DuplicateGroupName);
        }
        match config
            .groups
            .iter_mut()
            .find(|candidate| candidate.id == group_id)
        {
            Some(existing) => *existing = group.clone(),
            None => config.groups.push(group.clone()),
        }
        if config.state.selected_group_id == Some(group_id) {
            self.switch_use_case()
                .save_active_group_projection_for_config(config.clone())
                .map_err(ApplicationError::Switch)?;
        } else {
            self.config()
                .save(&config)
                .map_err(ApplicationError::SaveGroups)?;
        }
        let app_state = self
            .config()
            .load()
            .map_err(ApplicationError::LoadAppState)?
            .state;
        Ok(GroupMutation {
            group,
            groups: config.groups,
            app_state,
        })
    }

    pub fn copy_group(&self, id: Uuid) -> Result<GroupMutation, ApplicationError> {
        let mut config = self.config().load().map_err(ApplicationError::LoadGroups)?;
        let source = config
            .groups
            .iter()
            .find(|group| group.id == id)
            .cloned()
            .ok_or(ApplicationError::GroupNotFound)?;
        let copied_name = unique_copy_group_name(&source.name, &config.groups);
        let copied = ModelGroup {
            id: Uuid::new_v4(),
            name: copied_name,
            updated_at: OffsetDateTime::now_utc(),
            ..source
        };
        config.groups.push(copied.clone());
        self.config()
            .save(&config)
            .map_err(ApplicationError::SaveGroups)?;
        Ok(GroupMutation {
            group: copied,
            groups: config.groups,
            app_state: config.state,
        })
    }

    pub fn delete_group(&self, id: Uuid) -> Result<GroupMutation, ApplicationError> {
        let mut config = self.config().load().map_err(ApplicationError::LoadGroups)?;
        let deleted = config
            .groups
            .iter()
            .find(|group| group.id == id)
            .cloned()
            .ok_or(ApplicationError::GroupNotFound)?;
        let remaining = config
            .groups
            .iter()
            .cloned()
            .into_iter()
            .filter(|group| group.id != id)
            .collect::<Vec<_>>();
        config.groups = remaining.clone();
        if config.state.selected_group_id == Some(id) {
            config.state.selected_group_id = None;
            config.state.selected_group_name = None;
        }
        self.config()
            .save(&config)
            .map_err(ApplicationError::SaveGroups)?;
        Ok(GroupMutation {
            group: deleted,
            groups: remaining,
            app_state: config.state,
        })
    }

    pub fn switch_group(&self, id: Uuid) -> Result<GroupSwitch, ApplicationError> {
        let outcome = self
            .switch_use_case()
            .switch_to(id)
            .map_err(ApplicationError::Switch)?;
        let app_state = self
            .config()
            .load()
            .map_err(ApplicationError::LoadAppState)?
            .state;
        match outcome {
            SwitchOutcome::Success { warnings, .. } => Ok(GroupSwitch {
                outcome: GroupSwitchOutcome::Success,
                warnings,
                app_state,
            }),
            SwitchOutcome::NoOp => Ok(GroupSwitch {
                outcome: GroupSwitchOutcome::NoOp,
                warnings: vec!["Already using this group.".to_owned()],
                app_state,
            }),
        }
    }

    pub fn discover_open_code_agents(&self) -> OpenCodeAgentDiscovery {
        let discovery = self.discovered_open_code_agents();
        OpenCodeAgentDiscovery {
            agent_names: discovery.agent_names,
            error: discovery.error,
        }
    }

    pub(crate) fn recover_pending_transaction(&self) -> std::io::Result<()> {
        recover_pending_transaction(&self.paths.omo_switch_dir())
    }

    fn switch_use_case(&self) -> SwitchGroupUseCase<fn() -> OffsetDateTime> {
        SwitchGroupUseCase::new(
            SwitchGroupRepositories {
                config: self.config(),
                backups_root: self.paths.omo_switch_dir(),
                opencode: self.opencode(),
                oh_my_openagent: self.oh_my_openagent(),
            },
            OffsetDateTime::now_utc,
        )
    }

    fn config(&self) -> ConfigRepository {
        ConfigRepository::new(self.paths.config_file())
    }

    fn opencode(&self) -> OpenCodeConfigRepository {
        OpenCodeConfigRepository::new(self.paths.opencode_file())
    }

    fn oh_my_openagent(&self) -> OhMyOpenAgentConfigRepository {
        OhMyOpenAgentConfigRepository::new(self.paths.oh_my_openagent_file())
    }

    fn discovered_open_code_agents(&self) -> DiscoveredOpenCodeAgentNames {
        match self.opencode().load() {
            Ok(document) => {
                if document
                    .raw()
                    .get("agent")
                    .is_some_and(serde_json::Value::is_object)
                {
                    let mut agent_names = document.agents().keys().cloned().collect::<Vec<_>>();
                    agent_names.sort_by_key(|name| name.to_lowercase());
                    DiscoveredOpenCodeAgentNames {
                        agent_names,
                        error: None,
                    }
                } else {
                    DiscoveredOpenCodeAgentNames {
                        agent_names: Vec::new(),
                        error: Some(
                            "OpenCode config has no valid top-level agent object.".to_owned(),
                        ),
                    }
                }
            }
            Err(OpenCodeConfigError::FileNotFound { .. }) => DiscoveredOpenCodeAgentNames {
                agent_names: Vec::new(),
                error: Some("OpenCode config not found.".to_owned()),
            },
            Err(
                OpenCodeConfigError::ReadFailed { .. }
                | OpenCodeConfigError::MalformedConfig { .. }
                | OpenCodeConfigError::WriteFailed { .. },
            ) => DiscoveredOpenCodeAgentNames {
                agent_names: Vec::new(),
                error: Some("OpenCode config is malformed.".to_owned()),
            },
        }
    }
}

fn unique_copy_group_name(source_name: &str, groups: &[ModelGroup]) -> String {
    let source_name = source_name.trim();
    let copy_name = if source_name.is_empty() {
        "Copy".to_owned()
    } else {
        format!("{source_name} Copy")
    };

    if !group_name_exists(&copy_name, groups) {
        return copy_name;
    }

    let mut copy_number = 2;
    loop {
        let candidate = format!("{copy_name} {copy_number}");
        if !group_name_exists(&candidate, groups) {
            return candidate;
        }
        copy_number += 1;
    }
}

fn group_name_exists(candidate: &str, groups: &[ModelGroup]) -> bool {
    groups
        .iter()
        .any(|group| group.name.trim().eq_ignore_ascii_case(candidate.trim()))
}

struct DiscoveredOpenCodeAgentNames {
    agent_names: Vec<String>,
    error: Option<String>,
}
