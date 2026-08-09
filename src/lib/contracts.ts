export type Uuid = string;

export type ModelGroupCategoryMapping = Readonly<{ categoryName: string; modelRef: string }>;
export type ModelGroupAgentOverride = Readonly<{ agentName: string; modelRef: string }>;

export type ModelGroup = Readonly<{
  id: Uuid;
  name: string;
  description: string | null;
  categoryMappings: readonly ModelGroupCategoryMapping[];
  agentOverrides: readonly ModelGroupAgentOverride[];
  openCodeAgentOverrides: readonly ModelGroupAgentOverride[];
  isEnabled: boolean;
  updatedAt: string;
}>;

export type LastSuccessfulWriteMetadata = Readonly<{
  target: string;
  wroteAt: string;
  backupPath: string | null;
}>;

export type AppSelectionState = Readonly<{
  selectedGroupID: Uuid | null;
  selectedGroupName: string | null;
  lastSuccessfulWrite: LastSuccessfulWriteMetadata | null;
  lastWarningSummary: { readonly message: string; readonly count: number } | null;
  lastErrorSummary: { readonly message: string; readonly count: number } | null;
  migrationVersion: number;
}>;

export type AppStateResponse = Readonly<{
  groups: readonly ModelGroup[];
  appState: AppSelectionState;
  discoveredOpenCodeAgentNames: readonly string[];
  openCodeAgentDiscoveryError: string | null;
}>;

export type CommandErrorCode =
  | 'missingHome' | 'loadGroupsFailed' | 'saveGroupsFailed' | 'loadAppStateFailed'
  | 'saveAppStateFailed' | 'groupNotFound' | 'groupDisabled' | 'missingOpenCodeConfig'
  | 'malformedOpenCodeConfig' | 'loadOhMyConfigFailed' | 'backupFailed' | 'writeFailed'
  | 'rollbackFailed' | 'duplicateGroupName';

export type CommandError = Readonly<{ code: CommandErrorCode; message: string; detail: string | null }>;
export type SwitchOutcome = 'success' | 'noOp';
export type SwitchGroupResponse = Readonly<{
  outcome: SwitchOutcome;
  warnings: readonly string[];
  appState: AppSelectionState;
}>;

export type GroupMutationResponse = Readonly<{
  group: ModelGroup;
  groups: readonly ModelGroup[];
  appState: AppSelectionState;
}>;

export type OpenCodeAgentDiscoveredRow = Readonly<{
  id: string; agentName: string; modelRef: string; isEditable: boolean;
}>;
export type OpenCodeAgentOverrideInfoRow = Readonly<{
  id: string; agentName: string; modelRef: string; status: string; message: string;
}>;
export type OpenCodeAgentMappingPresentation = Readonly<{
  discoveredRows: readonly OpenCodeAgentDiscoveredRow[];
  staleOverrides: readonly OpenCodeAgentOverrideInfoRow[];
  preservedOverrides: readonly OpenCodeAgentOverrideInfoRow[];
  discoveryError: string | null;
  isReadOnly: boolean;
  allowsCustomAgentCreation: boolean;
}>;
export type DiscoverOpenCodeAgentsResponse = Readonly<{
  agentNames: readonly string[];
  error: string | null;
  presentation: OpenCodeAgentMappingPresentation;
}>;

export type QuickSwitchCommandClient = Readonly<{
  loadAppState: () => Promise<AppStateResponse>;
  switchGroup: (id: Uuid) => Promise<SwitchGroupResponse>;
}>;

export type SettingsCommandClient = Readonly<{
  loadAppState: () => Promise<AppStateResponse>;
  saveGroup: (group: ModelGroup) => Promise<GroupMutationResponse>;
  copyGroup: (id: Uuid) => Promise<GroupMutationResponse>;
  deleteGroup: (id: Uuid) => Promise<GroupMutationResponse>;
  switchGroup: (id: Uuid) => Promise<SwitchGroupResponse>;
  discoverOpenCodeAgents: (savedOverrides: readonly ModelGroupAgentOverride[]) => Promise<DiscoverOpenCodeAgentsResponse>;
}>;
