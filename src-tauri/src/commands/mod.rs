mod errors;
mod types;

pub use types::{
    AppStateResponse, CommandError, CommandErrorCode, CopyGroupRequest, DeleteGroupRequest,
    DiscoverOpenCodeAgentsRequest, DiscoverOpenCodeAgentsResponse, GroupMutationResponse,
    SaveGroupRequest, SwitchGroupOutcome, SwitchGroupRequest, SwitchGroupResponse,
};

use crate::application::{
    GroupMutation, GroupSwitch, GroupSwitchOutcome, LoadedAppState, OpenCodeAgentDiscovery,
};
use crate::commands::errors::command_error_from_runtime;
use crate::LiveAppRuntime;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn load_app_state(
    runtime: State<'_, LiveAppRuntime>,
) -> Result<AppStateResponse, CommandError> {
    runtime
        .load_app_state()
        .map(app_state_response)
        .map_err(command_error_from_runtime)
}

#[tauri::command]
pub fn save_group(
    app: AppHandle,
    runtime: State<'_, LiveAppRuntime>,
    request: SaveGroupRequest,
) -> Result<GroupMutationResponse, CommandError> {
    let response = runtime
        .save_group(request.group)
        .map(group_mutation_response)
        .map_err(command_error_from_runtime)?;
    crate::app_shell::refresh_tray(&app);
    Ok(response)
}

#[tauri::command]
pub fn copy_group(
    app: AppHandle,
    runtime: State<'_, LiveAppRuntime>,
    request: CopyGroupRequest,
) -> Result<GroupMutationResponse, CommandError> {
    let response = runtime
        .copy_group(request.id)
        .map(group_mutation_response)
        .map_err(command_error_from_runtime)?;
    crate::app_shell::refresh_tray(&app);
    Ok(response)
}

#[tauri::command]
pub fn delete_group(
    app: AppHandle,
    runtime: State<'_, LiveAppRuntime>,
    request: DeleteGroupRequest,
) -> Result<GroupMutationResponse, CommandError> {
    let response = runtime
        .delete_group(request.id)
        .map(group_mutation_response)
        .map_err(command_error_from_runtime)?;
    crate::app_shell::refresh_tray(&app);
    Ok(response)
}

#[tauri::command]
pub fn switch_group(
    app: AppHandle,
    runtime: State<'_, LiveAppRuntime>,
    request: SwitchGroupRequest,
) -> Result<SwitchGroupResponse, CommandError> {
    let response = runtime
        .switch_group(request.id)
        .map(switch_group_response)
        .map_err(command_error_from_runtime)?;
    crate::app_shell::refresh_tray(&app);
    Ok(response)
}

#[tauri::command]
pub fn discover_open_code_agents(
    runtime: State<'_, LiveAppRuntime>,
    request: DiscoverOpenCodeAgentsRequest,
) -> Result<DiscoverOpenCodeAgentsResponse, CommandError> {
    Ok(discover_open_code_agents_response(
        runtime.discover_open_code_agents(request.saved_overrides),
    ))
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
) -> DiscoverOpenCodeAgentsResponse {
    DiscoverOpenCodeAgentsResponse {
        agent_names: discovery.agent_names,
        error: discovery.error,
        presentation: discovery.presentation,
    }
}
