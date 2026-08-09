import type { AppStateResponse, QuickSwitchCommandClient, SwitchGroupResponse } from './contracts';

export const currentGroupId = '11111111-1111-4111-8111-111111111111';
export const nextGroupId = '22222222-2222-4222-8222-222222222222';

export const loadedState: AppStateResponse = {
  groups: [
    { id: currentGroupId, name: 'Default', description: null, categoryMappings: [], agentOverrides: [], openCodeAgentOverrides: [], isEnabled: true, updatedAt: '2026-07-09T00:00:00Z' },
    { id: nextGroupId, name: 'Research', description: null, categoryMappings: [], agentOverrides: [], openCodeAgentOverrides: [], isEnabled: true, updatedAt: '2026-07-09T00:00:00Z' }
  ],
  appState: {
    selectedGroupID: currentGroupId,
    selectedGroupName: 'Default',
    lastSuccessfulWrite: null,
    lastWarningSummary: null,
    lastErrorSummary: null,
    migrationVersion: 2
  },
  discoveredOpenCodeAgentNames: [],
  openCodeAgentDiscoveryError: null
};

export const switchedState: SwitchGroupResponse = {
  outcome: 'success',
  warnings: ['OpenCode config was skipped.'],
  appState: {
    ...loadedState.appState,
    selectedGroupID: nextGroupId,
    selectedGroupName: 'Research',
    lastWarningSummary: { message: 'OpenCode config was skipped.', count: 1 }
  }
};

type FakeClientOptions = Readonly<{
  loadResponse: AppStateResponse;
  switchResponse?: SwitchGroupResponse;
  switchError?: string;
}>;

export const fakeClient = (options: FakeClientOptions): QuickSwitchCommandClient => {
  return {
    loadAppState: async () => options.loadResponse,
    switchGroup: async () => {
      if (options.switchError) return Promise.reject({ code: 'groupDisabled', message: options.switchError, detail: null });
      return options.switchResponse ?? switchedState;
    }
  };
};
