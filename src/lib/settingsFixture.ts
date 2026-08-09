import type {
  AppSelectionState,
  AppStateResponse,
  DiscoverOpenCodeAgentsResponse,
  GroupMutationResponse,
  ModelGroup,
  SettingsCommandClient,
  SwitchGroupResponse,
  Uuid
} from './contracts';

const defaultGroupId = '11111111-1111-4111-8111-111111111111';
const researchGroupId = '22222222-2222-4222-8222-222222222222';

let groups: readonly ModelGroup[] = [
  {
    id: defaultGroupId,
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
    categoryMappings: [{ categoryName: 'chat', modelRef: 'openai/gpt-5.1' }],
    agentOverrides: [],
    openCodeAgentOverrides: [],
    isEnabled: true,
    updatedAt: '2026-07-09T00:00:00Z'
  }
];

let appState: AppSelectionState = {
  selectedGroupID: defaultGroupId,
  selectedGroupName: 'Default',
  lastSuccessfulWrite: null,
  lastWarningSummary: null,
  lastErrorSummary: null,
  migrationVersion: 2
};

export const settingsFixtureClient: SettingsCommandClient = {
  loadAppState: async () => appResponse(),
  switchGroup: async (id: Uuid): Promise<SwitchGroupResponse> => {
    const group = groups.find((candidate) => candidate.id === id);
    if (!group) {
      return Promise.reject({ code: 'groupNotFound', message: 'Group not found.', detail: null });
    }
    appState = { ...appState, selectedGroupID: group.id, selectedGroupName: group.name, lastWarningSummary: null };
    return { outcome: 'success', warnings: [], appState };
  },
  saveGroup: async (group: ModelGroup): Promise<GroupMutationResponse> => {
    groups = groups.some((candidate) => candidate.id === group.id) ? groups.map((candidate) => candidate.id === group.id ? group : candidate) : [...groups, group];
    return { group, groups, appState };
  },
  copyGroup: async (id: Uuid): Promise<GroupMutationResponse> => {
    const source = groups.find((group) => group.id === id);
    if (!source) {
      return Promise.reject({ code: 'groupNotFound', message: 'Group not found.', detail: null });
    }
    const group = { ...source, id: `fixture-copy-${groups.length}`, name: `${source.name} Copy`, updatedAt: new Date().toISOString() };
    groups = [...groups, group];
    return { group, groups, appState };
  },
  deleteGroup: async (id: Uuid): Promise<GroupMutationResponse> => {
    const group = groups.find((candidate) => candidate.id === id);
    if (!group) {
      return Promise.reject({ code: 'groupNotFound', message: 'Group not found.', detail: null });
    }
    groups = groups.filter((candidate) => candidate.id !== id);
    return { group, groups, appState };
  },
  discoverOpenCodeAgents: async (savedOverrides) => discovery(savedOverrides, null)
};

export const degradedSettingsFixtureClient: SettingsCommandClient = {
  ...settingsFixtureClient,
  discoverOpenCodeAgents: async (savedOverrides) => discovery(savedOverrides, 'OpenCode config is malformed.')
};

const appResponse = (): AppStateResponse => ({
  groups,
  appState,
  discoveredOpenCodeAgentNames: ['reviewer', 'summarizer'],
  openCodeAgentDiscoveryError: null,
});

const discovery = (savedOverrides: readonly { readonly agentName: string; readonly modelRef: string }[], error: string | null): Promise<DiscoverOpenCodeAgentsResponse> => Promise.resolve(error ? {
  agentNames: [],
  error,
  presentation: { discoveredRows: [], staleOverrides: [], preservedOverrides: savedOverrides.map((override) => ({ id: `preserved:${override.agentName}`, agentName: override.agentName, modelRef: override.modelRef, status: 'Preserved', message: 'Editing disabled until OpenCode agent discovery succeeds.' })), discoveryError: error, isReadOnly: true, allowsCustomAgentCreation: false }
} : {
  agentNames: ['reviewer', 'summarizer'],
  error: null,
  presentation: { discoveredRows: ['reviewer', 'summarizer'].map((agentName) => ({ id: `discovered:${agentName}`, agentName, modelRef: savedOverrides.find((override) => override.agentName === agentName)?.modelRef ?? '', isEditable: true })), staleOverrides: [], preservedOverrides: [], discoveryError: null, isReadOnly: false, allowsCustomAgentCreation: false }
});
