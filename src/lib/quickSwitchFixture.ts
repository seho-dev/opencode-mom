import type { AppStateResponse, QuickSwitchCommandClient, SwitchGroupResponse, Uuid } from './contracts';

const currentGroupId = '11111111-1111-4111-8111-111111111111';
const researchGroupId = '22222222-2222-4222-8222-222222222222';

const fixtureState: AppStateResponse = {
  groups: [
    {
      id: currentGroupId,
      name: 'Default',
      description: 'Primary local model set',
      categoryMappings: [{ categoryName: 'build', modelRef: 'anthropic/claude-sonnet-4' }],
      agentOverrides: [{ agentName: 'planner', modelRef: 'openai/gpt-5.1' }],
      openCodeAgentOverrides: [{ agentName: 'reviewer', modelRef: 'anthropic/claude-opus-4' }],
      isEnabled: true,
      updatedAt: '2026-07-09T00:00:00Z'
    },
    {
      id: researchGroupId,
      name: 'Research',
      description: 'Long context experiments',
      categoryMappings: [],
      agentOverrides: [],
      openCodeAgentOverrides: [],
      isEnabled: true,
      updatedAt: '2026-07-09T00:00:00Z'
    },
    {
      id: '33333333-3333-4333-8333-333333333333',
      name: 'Archived',
      description: 'Disabled legacy mapping',
      categoryMappings: [],
      agentOverrides: [],
      openCodeAgentOverrides: [],
      isEnabled: false,
      updatedAt: '2026-07-09T00:00:00Z'
    }
  ],
  appState: {
    selectedGroupID: currentGroupId,
    selectedGroupName: 'Default',
    lastSuccessfulWrite: null,
    lastWarningSummary: { message: 'OpenCode config was skipped.', count: 1 },
    lastErrorSummary: null,
    migrationVersion: 2
  },
  discoveredOpenCodeAgentNames: ['reviewer'],
  openCodeAgentDiscoveryError: null,
};

export const quickSwitchFixtureClient: QuickSwitchCommandClient = {
  loadAppState: async () => fixtureState,
  switchGroup: async (id: Uuid): Promise<SwitchGroupResponse> => ({
    outcome: 'success',
    warnings: id === researchGroupId ? ['OpenCode config was skipped.'] : [],
    appState: {
      ...fixtureState.appState,
      selectedGroupID: id,
      selectedGroupName: fixtureState.groups.find((group) => group.id === id)?.name ?? null,
      lastWarningSummary: id === researchGroupId ? { message: 'OpenCode config was skipped.', count: 1 } : null
    }
  })
};
