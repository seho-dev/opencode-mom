import { invoke } from '@tauri-apps/api/core';

import type {
  AppStateResponse,
  CommandError,
  DiscoverOpenCodeAgentsResponse,
  GroupMutationResponse,
  ModelGroup,
  ModelGroupAgentOverride,
  QuickSwitchCommandClient,
  SettingsCommandClient,
  SwitchGroupResponse,
  Uuid
} from './contracts';

export const loadAppState = () => invoke<AppStateResponse>('load_app_state');
export const saveGroup = (group: ModelGroup) =>
  invoke<GroupMutationResponse>('save_group', { request: { group } });
export const copyGroup = (id: Uuid) => invoke<GroupMutationResponse>('copy_group', { request: { id } });
export const deleteGroup = (id: Uuid) => invoke<GroupMutationResponse>('delete_group', { request: { id } });
export const switchGroup = (id: Uuid) =>
  invoke<SwitchGroupResponse>('switch_group', { request: { id } });
export const discoverOpenCodeAgents = (savedOverrides: readonly ModelGroupAgentOverride[]) =>
  invoke<DiscoverOpenCodeAgentsResponse>('discover_open_code_agents', { request: { savedOverrides } });
export const tauriQuickSwitchCommandClient: QuickSwitchCommandClient = {
  loadAppState,
  switchGroup
};

export const tauriSettingsCommandClient: SettingsCommandClient = {
  loadAppState,
  saveGroup,
  copyGroup,
  deleteGroup,
  switchGroup,
  discoverOpenCodeAgents
};

export const commandErrorMessage = (error: unknown): string => {
  if (isCommandError(error)) {
    return error.detail ?? error.message;
  }
  if (hasMessage(error)) {
    return error.message;
  }
  return 'Command failed.';
};

const isCommandError = (value: unknown): value is CommandError => {
  return hasMessage(value) && 'code' in value && 'detail' in value;
};

const hasMessage = (value: unknown): value is Readonly<{ message: string }> => {
  return typeof value === 'object' && value !== null && 'message' in value && typeof value.message === 'string';
};
