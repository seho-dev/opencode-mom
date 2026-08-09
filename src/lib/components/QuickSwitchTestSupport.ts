import type { AppStateResponse, QuickSwitchCommandClient, SwitchGroupResponse } from '../contracts';

export const currentGroupId = '11111111-1111-4111-8111-111111111111';
export const nextGroupId = '22222222-2222-4222-8222-222222222222';

export const appState: AppStateResponse = {
  groups: [
    { id: currentGroupId, name: 'Default', description: 'Primary local model set', categoryMappings: [{ categoryName: 'build', modelRef: 'anthropic/claude-sonnet-4' }], agentOverrides: [{ agentName: 'planner', modelRef: 'openai/gpt-5.1' }], openCodeAgentOverrides: [{ agentName: 'reviewer', modelRef: 'anthropic/claude-opus-4' }], isEnabled: true, updatedAt: '2026-07-09T00:00:00Z' },
    { id: nextGroupId, name: 'Research', description: 'Long context experiments', categoryMappings: [], agentOverrides: [], openCodeAgentOverrides: [], isEnabled: true, updatedAt: '2026-07-09T00:00:00Z' }
  ],
  appState: {
    selectedGroupID: currentGroupId,
    selectedGroupName: 'Default',
    lastSuccessfulWrite: null,
    lastWarningSummary: { message: 'OpenCode config was skipped.', count: 1 },
    lastErrorSummary: null,
    migrationVersion: 2
  },
  discoveredOpenCodeAgentNames: [],
  openCodeAgentDiscoveryError: null
};

export type ClientOptions = Readonly<{
  state?: AppStateResponse;
  loadResponse?: Promise<AppStateResponse>;
  switchResponse?: SwitchGroupResponse;
  switchError?: string;
}>;

export const client = (options: ClientOptions = {}): QuickSwitchCommandClient => ({
  loadAppState: async () => options.loadResponse ?? options.state ?? appState,
  switchGroup: async () => {
    if (options.switchError) return Promise.reject({ code: 'groupDisabled', message: options.switchError, detail: null });
    return options.switchResponse ?? { outcome: 'success', warnings: [], appState: appState.appState };
  }
});

export const noEnabledGroupsState: AppStateResponse = { ...appState, groups: appState.groups.map((group) => ({ ...group, isEnabled: false })) };
export const stateWithoutWarning: AppStateResponse = { ...appState, appState: { ...appState.appState, lastWarningSummary: null } };
