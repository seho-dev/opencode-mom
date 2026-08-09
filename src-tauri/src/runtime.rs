use std::fmt;

use crate::application::{
    ApplicationError, GroupApplicationService, GroupMutation, GroupSwitch, LoadedAppState,
    OpenCodeAgentDiscovery,
};
use crate::core::models::{ModelGroup, ModelGroupAgentOverride};
use uuid::Uuid;

pub struct AppRuntime {
    application: GroupApplicationService,
}

#[derive(Debug)]
pub enum RuntimeError {
    Application(ApplicationError),
    StartupRecovery(std::io::Error),
}

impl From<ApplicationError> for RuntimeError {
    fn from(error: ApplicationError) -> Self {
        Self::Application(error)
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Application(error) => write!(formatter, "{error}"),
            Self::StartupRecovery(error) => write!(
                formatter,
                "Pending configuration transaction recovery failed during runtime startup: {error}"
            ),
        }
    }
}

impl std::error::Error for RuntimeError {}

impl AppRuntime {
    pub const fn new(application: GroupApplicationService) -> Self {
        Self { application }
    }

    pub fn startup(&self) -> Result<(), RuntimeError> {
        self.application
            .recover_pending_transaction()
            .map_err(startup_recovery_error)?;
        Ok(())
    }

    pub fn load_app_state(&self) -> Result<LoadedAppState, RuntimeError> {
        self.application.load_app_state().map_err(Into::into)
    }

    pub fn save_group(&self, group: ModelGroup) -> Result<GroupMutation, RuntimeError> {
        self.application.save_group(group).map_err(Into::into)
    }

    pub fn copy_group(&self, id: Uuid) -> Result<GroupMutation, RuntimeError> {
        self.application.copy_group(id).map_err(Into::into)
    }

    pub fn delete_group(&self, id: Uuid) -> Result<GroupMutation, RuntimeError> {
        self.application.delete_group(id).map_err(Into::into)
    }

    pub fn switch_group(&self, id: Uuid) -> Result<GroupSwitch, RuntimeError> {
        self.application.switch_group(id).map_err(Into::into)
    }

    pub fn discover_open_code_agents(
        &self,
        saved_overrides: Vec<ModelGroupAgentOverride>,
    ) -> OpenCodeAgentDiscovery {
        self.application.discover_open_code_agents(saved_overrides)
    }
}

fn startup_recovery_error(error: std::io::Error) -> RuntimeError {
    RuntimeError::StartupRecovery(error)
}
