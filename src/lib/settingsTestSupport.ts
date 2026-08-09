import type { AppSelectionState, AppStateResponse, DiscoverOpenCodeAgentsResponse, GroupMutationResponse, ModelGroup, SettingsCommandClient, SwitchGroupResponse, Uuid } from './contracts';

export const defaultGroupId = '11111111-1111-4111-8111-111111111111';
export const researchGroupId = '22222222-2222-4222-8222-222222222222';

export type ClientOptions = Readonly<{
  saveGroup?: (group: ModelGroup) => Promise<GroupMutationResponse>;
  copyGroup?: SettingsCommandClient['copyGroup'];
  switchGroup?: SettingsCommandClient['switchGroup'];
  discoverOpenCodeAgents?: SettingsCommandClient['discoverOpenCodeAgents'];
  deleteGroup?: SettingsCommandClient['deleteGroup'];
  discoveryError?: string;
}>;

export const fakeSettingsClient = (options: ClientOptions = {}): SettingsCommandClient => {
  let groups = [...baseGroups];
  let appState = baseAppState;
  return {
    loadAppState: async () => response(groups, appState),
    switchGroup: options.switchGroup ?? (async (id: Uuid): Promise<SwitchGroupResponse> => {
      const group = groups.find((candidate) => candidate.id === id);
      if (!group) return Promise.reject({ code: 'groupNotFound', message: 'Group not found.', detail: null });
      appState = { ...appState, selectedGroupID: group.id, selectedGroupName: group.name };
      return { outcome: 'success', warnings: [], appState };
    }),
    saveGroup: async (group) => {
      if (options.saveGroup) return options.saveGroup(group);
      groups = groups.some((candidate) => candidate.id === group.id) ? groups.map((candidate) => candidate.id === group.id ? group : candidate) : [...groups, group];
      return { group, groups, appState };
    },
    copyGroup: options.copyGroup ?? (async (id) => {
      const source = groups.find((group) => group.id === id);
      if (!source) return Promise.reject({ code: 'groupNotFound', message: 'Group not found.', detail: null });
      const group = { ...source, id: '44444444-4444-4444-8444-444444444444', name: `${source.name} Copy` };
      groups = [...groups, group];
      return { group, groups, appState };
    }),
    deleteGroup: options.deleteGroup ?? (async (id) => {
      const group = groups.find((candidate) => candidate.id === id);
      if (!group) return Promise.reject({ code: 'groupNotFound', message: 'Group not found.', detail: null });
      groups = groups.filter((candidate) => candidate.id !== id);
      return { group, groups, appState };
    }),
    discoverOpenCodeAgents: options.discoverOpenCodeAgents ?? (async (savedOverrides) => discovery(savedOverrides, options.discoveryError ?? null))
  };
};

export const baseGroups: readonly ModelGroup[] = [
  { id: defaultGroupId, name: 'Default', description: 'Primary', categoryMappings: [{ categoryName: 'build', modelRef: 'anthropic/claude-sonnet-4' }], agentOverrides: [{ agentName: 'planner', modelRef: 'openai/gpt-5.1' }], openCodeAgentOverrides: [{ agentName: 'reviewer', modelRef: 'anthropic/claude-opus-4' }], isEnabled: true, updatedAt: '2026-07-09T00:00:00Z' },
  { id: researchGroupId, name: 'Research', description: null, categoryMappings: [], agentOverrides: [], openCodeAgentOverrides: [], isEnabled: true, updatedAt: '2026-07-09T00:00:00Z' }
];

export const baseAppState: AppSelectionState = { selectedGroupID: defaultGroupId, selectedGroupName: 'Default', lastSuccessfulWrite: null, lastWarningSummary: null, lastErrorSummary: null, migrationVersion: 2 };

const response = (groups: readonly ModelGroup[], appState: AppSelectionState): AppStateResponse => ({ groups, appState, discoveredOpenCodeAgentNames: ['reviewer', 'summarizer'], openCodeAgentDiscoveryError: null });
const discovery = (savedOverrides: readonly { readonly agentName: string; readonly modelRef: string }[], error: string | null): Promise<DiscoverOpenCodeAgentsResponse> => Promise.resolve(error ? { agentNames: [], error, presentation: { discoveredRows: [], staleOverrides: [], preservedOverrides: savedOverrides.map((override) => ({ id: `preserved:${override.agentName}`, agentName: override.agentName, modelRef: override.modelRef, status: 'Preserved', message: 'Editing disabled until OpenCode agent discovery succeeds.' })), discoveryError: error, isReadOnly: true, allowsCustomAgentCreation: false } } : { agentNames: ['reviewer', 'summarizer'], error: null, presentation: { discoveredRows: ['reviewer', 'summarizer'].map((agentName) => ({ id: `discovered:${agentName}`, agentName, modelRef: savedOverrides.find((override) => override.agentName === agentName)?.modelRef ?? '', isEditable: true })), staleOverrides: [], preservedOverrides: [], discoveryError: null, isReadOnly: false, allowsCustomAgentCreation: false } });
