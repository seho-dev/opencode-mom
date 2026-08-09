use std::fmt;
use std::path::Path;

use time::OffsetDateTime;
use uuid::Uuid;

use crate::core::draft_state::{
    open_code_agent_mapping_presentation, OpenCodeAgentMappingPresentation, OpenCodeDiscoveryState,
};
use crate::core::models::{AppSelectionState, ModelGroup, ModelGroupAgentOverride};
use crate::core::paths::{ConfigPathError, ConfigPaths, HomeEnv};
use crate::core::repository::{
    AppStateRepository, ModelGroupRepository, OhMyOpenAgentConfigRepository, OpenCodeConfigError,
    OpenCodeConfigRepository, RepositoryError,
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
    pub presentation: OpenCodeAgentMappingPresentation,
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
        let groups = self
            .model_groups()
            .load()
            .map_err(ApplicationError::LoadGroups)?;
        let app_state = self
            .app_state()
            .load()
            .map_err(ApplicationError::LoadAppState)?;
        let discovery = self.discovered_open_code_agents();
        Ok(LoadedAppState {
            groups,
            app_state,
            discovered_open_code_agent_names: discovery.agent_names,
            open_code_agent_discovery_error: discovery.error,
        })
    }

    pub fn save_group(&self, group: ModelGroup) -> Result<GroupMutation, ApplicationError> {
        let group_id = group.id;
        let mut groups = self
            .model_groups()
            .load()
            .map_err(ApplicationError::LoadGroups)?;
        if groups.iter().any(|candidate| {
            candidate.id != group_id
                && candidate
                    .name
                    .trim()
                    .eq_ignore_ascii_case(group.name.trim())
        }) {
            return Err(ApplicationError::DuplicateGroupName);
        }
        match groups.iter_mut().find(|candidate| candidate.id == group_id) {
            Some(existing) => *existing = group.clone(),
            None => groups.push(group.clone()),
        }
        self.model_groups()
            .save(&groups)
            .map_err(ApplicationError::SaveGroups)?;
        let state = self
            .app_state()
            .load()
            .map_err(ApplicationError::LoadAppState)?;
        if state.selected_group_id == Some(group_id) {
            self.switch_use_case()
                .save_active_group_projection(group_id)
                .map_err(ApplicationError::Switch)?;
        }
        let app_state = self
            .app_state()
            .load()
            .map_err(ApplicationError::LoadAppState)?;
        Ok(GroupMutation {
            group,
            groups,
            app_state,
        })
    }

    pub fn copy_group(&self, id: Uuid) -> Result<GroupMutation, ApplicationError> {
        let mut groups = self
            .model_groups()
            .load()
            .map_err(ApplicationError::LoadGroups)?;
        let source = groups
            .iter()
            .find(|group| group.id == id)
            .cloned()
            .ok_or(ApplicationError::GroupNotFound)?;
        let copied = ModelGroup {
            id: Uuid::new_v4(),
            name: format!("{} Copy", source.name),
            updated_at: OffsetDateTime::now_utc(),
            ..source
        };
        groups.push(copied.clone());
        self.model_groups()
            .save(&groups)
            .map_err(ApplicationError::SaveGroups)?;
        let app_state = self
            .app_state()
            .load()
            .map_err(ApplicationError::LoadAppState)?;
        Ok(GroupMutation {
            group: copied,
            groups,
            app_state,
        })
    }

    pub fn delete_group(&self, id: Uuid) -> Result<GroupMutation, ApplicationError> {
        let groups = self
            .model_groups()
            .load()
            .map_err(ApplicationError::LoadGroups)?;
        let deleted = groups
            .iter()
            .find(|group| group.id == id)
            .cloned()
            .ok_or(ApplicationError::GroupNotFound)?;
        let remaining = groups
            .into_iter()
            .filter(|group| group.id != id)
            .collect::<Vec<_>>();
        self.model_groups()
            .save(&remaining)
            .map_err(ApplicationError::SaveGroups)?;
        let mut app_state = self
            .app_state()
            .load()
            .map_err(ApplicationError::LoadAppState)?;
        if app_state.selected_group_id == Some(id) {
            app_state.selected_group_id = None;
            app_state.selected_group_name = None;
            self.app_state()
                .save(&app_state)
                .map_err(ApplicationError::SaveAppState)?;
        }
        Ok(GroupMutation {
            group: deleted,
            groups: remaining,
            app_state,
        })
    }

    pub fn switch_group(&self, id: Uuid) -> Result<GroupSwitch, ApplicationError> {
        let outcome = self
            .switch_use_case()
            .switch_to(id)
            .map_err(ApplicationError::Switch)?;
        let app_state = self
            .app_state()
            .load()
            .map_err(ApplicationError::LoadAppState)?;
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

    pub fn discover_open_code_agents(
        &self,
        saved_overrides: Vec<ModelGroupAgentOverride>,
    ) -> OpenCodeAgentDiscovery {
        let discovery = self.discovered_open_code_agents();
        let names = discovery
            .agent_names
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        let discovery_state = match discovery.error.clone() {
            Some(error) => OpenCodeDiscoveryState::Failure(error),
            None => OpenCodeDiscoveryState::Success,
        };
        let presentation =
            open_code_agent_mapping_presentation(&saved_overrides, &names, discovery_state);
        OpenCodeAgentDiscovery {
            agent_names: discovery.agent_names,
            error: discovery.error,
            presentation,
        }
    }

    pub(crate) fn recover_pending_transaction(&self) -> std::io::Result<()> {
        recover_pending_transaction(&self.paths.omo_switch_dir())
    }

    fn switch_use_case(&self) -> SwitchGroupUseCase<fn() -> OffsetDateTime> {
        SwitchGroupUseCase::new(
            SwitchGroupRepositories {
                model_groups: self.model_groups(),
                app_state: self.app_state(),
                backups_root: self.paths.omo_switch_dir(),
                opencode: self.opencode(),
                oh_my_openagent: self.oh_my_openagent(),
            },
            OffsetDateTime::now_utc,
        )
    }

    fn model_groups(&self) -> ModelGroupRepository {
        ModelGroupRepository::new(self.paths.groups_file())
    }

    fn app_state(&self) -> AppStateRepository {
        AppStateRepository::new(self.paths.state_file())
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

struct DiscoveredOpenCodeAgentNames {
    agent_names: Vec<String>,
    error: Option<String>,
}
