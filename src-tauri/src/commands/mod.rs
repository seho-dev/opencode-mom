mod errors;
mod types;

pub use types::{
    AppStateResponse, CommandError, CommandErrorCode, CopyGroupRequest, DeleteGroupRequest,
    DiscoverOpenCodeAgentsRequest, DiscoverOpenCodeAgentsResponse, GroupMutationResponse,
    SaveGroupRequest, SwitchGroupOutcome, SwitchGroupRequest, SwitchGroupResponse,
};

use crate::application::{
    GroupApplicationService, GroupMutation, GroupSwitch, GroupSwitchOutcome, LoadedAppState,
    OpenCodeAgentDiscovery,
};
use crate::commands::errors::command_error_from_application;
use crate::commands::types::open_code_agent_mapping_presentation;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn load_app_state(
    application: State<'_, GroupApplicationService>,
) -> Result<AppStateResponse, CommandError> {
    application
        .load_app_state()
        .map(app_state_response)
        .map_err(command_error_from_application)
}

#[tauri::command]
pub fn save_group(
    app: AppHandle,
    application: State<'_, GroupApplicationService>,
    request: SaveGroupRequest,
) -> Result<GroupMutationResponse, CommandError> {
    let response = application
        .save_group(request.group)
        .map(group_mutation_response)
        .map_err(command_error_from_application)?;
    crate::app_shell::refresh_tray(&app);
    Ok(response)
}

#[tauri::command]
pub fn copy_group(
    app: AppHandle,
    application: State<'_, GroupApplicationService>,
    request: CopyGroupRequest,
) -> Result<GroupMutationResponse, CommandError> {
    let response = application
        .copy_group(request.id)
        .map(group_mutation_response)
        .map_err(command_error_from_application)?;
    crate::app_shell::refresh_tray(&app);
    Ok(response)
}

#[tauri::command]
pub fn delete_group(
    app: AppHandle,
    application: State<'_, GroupApplicationService>,
    request: DeleteGroupRequest,
) -> Result<GroupMutationResponse, CommandError> {
    let response = application
        .delete_group(request.id)
        .map(group_mutation_response)
        .map_err(command_error_from_application)?;
    crate::app_shell::refresh_tray(&app);
    Ok(response)
}

#[tauri::command]
pub fn switch_group(
    app: AppHandle,
    application: State<'_, GroupApplicationService>,
    request: SwitchGroupRequest,
) -> Result<SwitchGroupResponse, CommandError> {
    let response = application
        .switch_group(request.id)
        .map(switch_group_response)
        .map_err(command_error_from_application)?;
    crate::app_shell::refresh_tray(&app);
    Ok(response)
}

#[tauri::command]
pub fn discover_open_code_agents(
    application: State<'_, GroupApplicationService>,
    request: DiscoverOpenCodeAgentsRequest,
) -> Result<DiscoverOpenCodeAgentsResponse, CommandError> {
    let saved_overrides = request.saved_overrides;
    let discovery = application.discover_open_code_agents();
    let presentation = open_code_agent_mapping_presentation(
        &saved_overrides,
        &discovery.agent_names,
        discovery.error.as_deref(),
    );
    Ok(discover_open_code_agents_response(discovery, presentation))
}

fn app_state_response(state: LoadedAppState) -> AppStateResponse {
    AppStateResponse {
        groups: state.groups,
        app_state: state.app_state,
        discovered_open_code_agent_names: state.discovered_open_code_agent_names,
        open_code_agent_discovery_error: state.open_code_agent_discovery_error,
    }
}

fn group_mutation_response(mutation: GroupMutation) -> GroupMutationResponse {
    GroupMutationResponse {
        group: mutation.group,
        groups: mutation.groups,
        app_state: mutation.app_state,
    }
}

fn switch_group_response(switch: GroupSwitch) -> SwitchGroupResponse {
    SwitchGroupResponse {
        outcome: match switch.outcome {
            GroupSwitchOutcome::Success => SwitchGroupOutcome::Success,
            GroupSwitchOutcome::NoOp => SwitchGroupOutcome::NoOp,
        },
        warnings: switch.warnings,
        app_state: switch.app_state,
    }
}

fn discover_open_code_agents_response(
    discovery: OpenCodeAgentDiscovery,
    presentation: types::OpenCodeAgentMappingPresentation,
) -> DiscoverOpenCodeAgentsResponse {
    DiscoverOpenCodeAgentsResponse {
        agent_names: discovery.agent_names,
        error: discovery.error,
        presentation,
    }
}
