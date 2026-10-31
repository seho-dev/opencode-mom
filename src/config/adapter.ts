import type { AgentDefinition } from '$src/types/agents.js';
import type {
  AppInfo,
  AppPreferences,
  AppState,
  CommandError,
  OpenCodeInstallation,
  OpenCodeServiceStatus,
  OpenCodeUpdateCheck,
  ProjectFolder,
  ProjectPage,
  UpdateCheck,
} from '$src/types/app.js';
import type { Group, GroupType } from '$src/types/groups.js';
import type {
  IslandAnswer,
  IslandMessage,
  IslandPermissionDecision,
  IslandState,
  IslandView,
} from '$src/types/island.js';
import type { McpDraft, McpList, McpServer, McpUpdate } from '$src/types/mcp.js';
import type {
  CloseMcpProxyUnitRequest,
  ImportMcpProxyServerRequest,
  McpProxyServerDto,
  McpProxySettingsDto,
  McpProxyStateDto,
  MomHttpSettings,
} from '$src/types/mcp-proxy.js';
import type { ModelCatalogEntry, ModelDef, ModelRef } from '$src/types/models.js';
import type { LidState } from '$src/types/power.js';
import type { PromptGenerationRequest } from '$src/types/prompt-generation.js';
import type { ProviderDef } from '$src/types/providers.js';
import type { SkillDraft, SkillEntry, SkillList, SkillUpdate } from '$src/types/skills.js';
import type { TokenUsageRecord } from '$src/types/stats.js';
import { GROUP_TYPE_NATIVE, GROUP_TYPE_OMO, GROUP_TYPE_SLIM } from '$src/utils/constants.js';

type Result<T> = Promise<T>;

// Persisted config schema values are frozen for backward compatibility.
const GROUP_TYPE_WIRE: Record<GroupType, string> = {
  native: 'opencode',
  slim: 'slim',
  omo: 'oh-my-openagent',
};
const WIRE_GROUP_TYPE: Record<string, GroupType> = {
  opencode: GROUP_TYPE_NATIVE,
  slim: GROUP_TYPE_SLIM,
  'oh-my-openagent': GROUP_TYPE_OMO,
  native: GROUP_TYPE_NATIVE,
  omo: GROUP_TYPE_OMO,
};
export const groupTypeToWire = (type: GroupType): string => GROUP_TYPE_WIRE[type];
export const groupTypeFromWire = (value: string): GroupType => WIRE_GROUP_TYPE[value] ?? GROUP_TYPE_NATIVE;

export interface CommandAdapter {
  loadAppState(): Result<AppState>;
  getAppInfo(): Result<AppInfo>;
  checkForUpdates(): Result<UpdateCheck>;
  getOpenCodeInstallation(): Result<OpenCodeInstallation>;
  getOpenCodeServiceStatus(): Result<OpenCodeServiceStatus>;
  setOpenCodeServiceTarget(url: string | null, password?: string | null): Result<OpenCodeServiceStatus>;
  checkOpenCodeUpdate(): Result<OpenCodeUpdateCheck>;
  upgradeOpenCode(
    expectedBinaryPath: string,
    expectedVersion: string,
    targetVersion: string,
  ): Result<OpenCodeInstallation>;
  openProjectPage(page: ProjectPage): Result<void>;
  pickProjectFolder(): Result<string | null>;
  addProject(path: string): Result<ProjectFolder>;
  removeProject(id: string): Result<void>;
  selectProject(id: string | null): Result<void>;
  listProviders(): Result<ProviderDef[]>;
  opencodeListModels(provider?: string): Result<ModelCatalogEntry[]>;
  generatePrompt(request: PromptGenerationRequest): Result<string>;
  opencodeResolveBinary(): Result<string>;
  opencodeReload(): Result<void>;
  opencodeTokenUsageRecords(): Result<TokenUsageRecord[]>;
  createProvider(value: ProviderDef): Result<ProviderDef>;
  updateProvider(value: ProviderDef): Result<ProviderDef>;
  deleteProvider(id: string): Result<void>;
  listModels(): Result<ProviderDef[]>;
  createModel(providerId: string, value: ModelDef): Result<ModelDef>;
  updateModel(providerId: string, value: ModelDef): Result<ModelDef>;
  deleteModel(ref: ModelRef): Result<void>;
  listAgents(projectId?: string | null): Result<AgentDefinition[]>;
  getAgent(id: string, projectId?: string | null): Result<AgentDefinition>;
  createAgent(value: AgentDefinition, projectId?: string | null): Result<AgentDefinition>;
  updateAgent(value: AgentDefinition, projectId?: string | null): Result<AgentDefinition>;
  deleteAgent(id: string, storage?: AgentDefinition['storage'], projectId?: string | null): Result<void>;
  listMcps(projectId?: string | null): Result<McpList>;
  getMcp(name: string, projectId?: string | null): Result<McpServer>;
  createMcp(draft: McpDraft, projectId?: string | null): Result<McpServer>;
  updateMcp(draft: McpUpdate, projectId?: string | null): Result<McpServer>;
  deleteMcp(name: string, projectId?: string | null): Result<void>;
  listSkills(projectId?: string | null): Result<SkillList>;
  getSkill(id: string, projectId?: string | null): Result<SkillEntry>;
  createSkill(draft: SkillDraft, projectId?: string | null): Result<SkillEntry>;
  updateSkill(draft: SkillUpdate, projectId?: string | null): Result<SkillEntry>;
  getAutostart(): Result<boolean>;
  setAutostart(enabled: boolean): Result<boolean>;
  getLidProtection(): Result<LidState>;
  setLidProtection(enabled: boolean): Result<LidState>;
  getIslandState(): Result<IslandState>;
  setIslandEnabled(enabled: boolean): Result<IslandState>;
  refreshIsland(): Result<IslandState>;
  setIslandView(view: IslandView): Result<void>;
  islandMessages(sessionId: string, expectedConnectionGeneration: number): Result<IslandMessage[]>;
  islandSend(sessionId: string, text: string, messageId: string, expectedConnectionGeneration: number): Result<void>;
  islandReplyPermission(
    sessionId: string,
    requestId: string,
    decision: IslandPermissionDecision,
    expectedConnectionGeneration: number,
  ): Result<void>;
  islandReplyQuestion(
    sessionId: string,
    formId: string,
    answer: IslandAnswer,
    expectedConnectionGeneration: number,
  ): Result<void>;
  islandCancelQuestion(sessionId: string, formId: string, expectedConnectionGeneration: number): Result<void>;
  islandInterrupt(sessionId: string, expectedConnectionGeneration: number): Result<void>;
  getMcpProxyState(): Result<McpProxyStateDto>;
  getMomHttpSettings(): Result<MomHttpSettings>;
  setMomHttpSettings(settings: MomHttpSettings): Result<void>;
  getMcpProxySettings(): Result<McpProxySettingsDto>;
  saveMcpProxySettings(settings: McpProxySettingsDto, expectedRevision: number): Result<void>;
  rotateMcpProxyKey(): Result<string>;
  getMcpProxyServer(id: string): Result<McpProxyServerDto>;
  createMcpProxyServer(server: McpProxyServerDto, expectedRevision: number): Result<McpProxyServerDto>;
  updateMcpProxyServer(server: McpProxyServerDto, expectedRevision: number): Result<McpProxyServerDto>;
  deleteMcpProxyServer(id: string, expectedRevision: number): Result<void>;
  importMcpProxyServer(request: ImportMcpProxyServerRequest): Result<McpProxyServerDto>;
  closeMcpProxyUnit(request: CloseMcpProxyUnitRequest): Result<void>;
  saveGroup(value: Group): Result<Group>;
  copyGroup(id: string, name: string): Result<Group>;
  deleteGroup(id: string): Result<void>;
  switchGroup(id: string): Result<void>;
  savePreferences(preferences: AppPreferences): Result<void>;
}

export type TauriInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
// Persisted group.type uses frozen wire values; canonicalize at the IPC boundary only.
const readGroup = (group: Group): Group => ({ ...group, type: groupTypeFromWire(group.type) });
const writeGroup = (group: Group): Group => ({ ...group, type: groupTypeToWire(group.type) as GroupType });
const projectScope = (projectId?: string | null) => (projectId == null ? {} : { projectId });
export function createTauriAdapter(invoke: TauriInvoke): CommandAdapter {
  return {
    loadAppState: async () => {
      const state = await invoke<AppState>('load_app_state');
      return { ...state, groups: state.groups.map(readGroup) };
    },
    getAppInfo: () => invoke('get_app_info'),
    checkForUpdates: () => invoke('check_for_updates'),
    getOpenCodeInstallation: () => invoke('opencode_get_installation'),
    getOpenCodeServiceStatus: () => invoke('opencode_get_service_status'),
    setOpenCodeServiceTarget: (url, password) =>
      invoke('opencode_set_service_target', { url, ...(password === undefined ? {} : { password }) }),
    checkOpenCodeUpdate: () => invoke('opencode_check_update'),
    upgradeOpenCode: (expectedBinaryPath, expectedVersion, targetVersion) =>
      invoke('opencode_upgrade', { expectedBinaryPath, expectedVersion, targetVersion }),
    openProjectPage: (page) => invoke('open_project_page', { page }),
    pickProjectFolder: () => invoke('pick_project_folder'),
    addProject: (path) => invoke('add_project', { path }),
    removeProject: (id) => invoke('remove_project', { id }),
    selectProject: (id) => invoke('select_project', { id }),
    listProviders: () => invoke('list_providers'),
    opencodeListModels: (provider) => invoke('opencode_list_models', { provider }),
    generatePrompt: (request) => invoke('opencode_generate_prompt', { request }),
    opencodeResolveBinary: () => invoke('opencode_resolve_binary'),
    opencodeReload: () => invoke('opencode_reload'),
    opencodeTokenUsageRecords: () => invoke('opencode_token_usage_records'),
    createProvider: (value) => invoke('create_provider', { provider: value }),
    updateProvider: (value) => invoke('update_provider', { provider: value }),
    deleteProvider: (id) => invoke('delete_provider', { id }),
    listModels: () => invoke('list_models'),
    createModel: (providerId, value) => invoke('create_model', { providerId, model: value }),
    updateModel: (providerId, value) => invoke('update_model', { providerId, model: value }),
    deleteModel: (modelRef) => invoke('delete_model', { modelRef }),
    listAgents: (projectId) => (projectId == null ? invoke('list_agents') : invoke('list_agents', { projectId })),
    getAgent: (id, projectId) => invoke('get_agent', { id, ...projectScope(projectId) }),
    createAgent: (value, projectId) => invoke('create_agent', { agent: value, ...projectScope(projectId) }),
    updateAgent: (value, projectId) => invoke('update_agent', { agent: value, ...projectScope(projectId) }),
    deleteAgent: (id, storage, projectId) => invoke('delete_agent', { id, storage, ...projectScope(projectId) }),
    listMcps: (projectId) => (projectId == null ? invoke('list_mcps') : invoke('list_mcps', { projectId })),
    getMcp: (name, projectId) => invoke('get_mcp', { name, ...projectScope(projectId) }),
    createMcp: (draft, projectId) => invoke('create_mcp', { draft, ...projectScope(projectId) }),
    updateMcp: (draft, projectId) => invoke('update_mcp', { draft, ...projectScope(projectId) }),
    deleteMcp: (name, projectId) => invoke('delete_mcp', { name, ...projectScope(projectId) }),
    listSkills: (projectId) => (projectId == null ? invoke('list_skills') : invoke('list_skills', { projectId })),
    getSkill: (id, projectId) => invoke('get_skill', { id, ...projectScope(projectId) }),
    createSkill: (draft, projectId) => invoke('create_skill', { draft, ...projectScope(projectId) }),
    updateSkill: (draft, projectId) => invoke('update_skill', { draft, ...projectScope(projectId) }),
    getAutostart: () => invoke('get_autostart'),
    setAutostart: (enabled) => invoke('set_autostart', { enabled }),
    getLidProtection: () => invoke('get_lid_protection'),
    setLidProtection: (enabled) => invoke('set_lid_protection', { enabled }),
    getIslandState: () => invoke('get_island_state', {}),
    setIslandEnabled: (enabled) => invoke('set_island_enabled', { enabled }),
    refreshIsland: () => invoke('refresh_island', {}),
    setIslandView: (view) => invoke('set_island_view', { view }),
    islandMessages: (sessionId, expectedConnectionGeneration) =>
      invoke('island_messages', { sessionId, expectedConnectionGeneration }),
    islandSend: (sessionId, text, messageId, expectedConnectionGeneration) =>
      invoke('island_send', { sessionId, text, messageId, expectedConnectionGeneration }),
    islandReplyPermission: (sessionId, requestId, decision, expectedConnectionGeneration) =>
      invoke('island_reply_permission', { sessionId, requestId, decision, expectedConnectionGeneration }),
    islandReplyQuestion: (sessionId, formId, answer, expectedConnectionGeneration) =>
      invoke('island_reply_question', { sessionId, formId, answer, expectedConnectionGeneration }),
    islandCancelQuestion: (sessionId, formId, expectedConnectionGeneration) =>
      invoke('island_cancel_question', { sessionId, formId, expectedConnectionGeneration }),
    islandInterrupt: (sessionId, expectedConnectionGeneration) =>
      invoke('island_interrupt', { sessionId, expectedConnectionGeneration }),
    getMcpProxyState: () => invoke('get_mcp_proxy_state'),
    getMomHttpSettings: () => invoke('get_mom_http_settings'),
    setMomHttpSettings: (settings) => invoke('set_mom_http_settings', { settings }),
    getMcpProxySettings: () => invoke('get_mcp_proxy_settings'),
    saveMcpProxySettings: (settings, expectedRevision) =>
      invoke('save_mcp_proxy_settings', { settings, expectedRevision }),
    rotateMcpProxyKey: () => invoke('rotate_mcp_proxy_key'),
    getMcpProxyServer: (id) => invoke('get_mcp_proxy_server', { id }),
    createMcpProxyServer: (server, expectedRevision) => invoke('create_mcp_proxy_server', { server, expectedRevision }),
    updateMcpProxyServer: (server, expectedRevision) => invoke('update_mcp_proxy_server', { server, expectedRevision }),
    deleteMcpProxyServer: (id, expectedRevision) => invoke('delete_mcp_proxy_server', { id, expectedRevision }),
    importMcpProxyServer: (request) => invoke('import_mcp_proxy_server', { request }),
    closeMcpProxyUnit: (request) => invoke('close_mcp_proxy_unit', { request }),
    saveGroup: async (value) => readGroup(await invoke<Group>('save_group', { group: writeGroup(value) })),
    copyGroup: async (id, name) => readGroup(await invoke<Group>('copy_group', { id, name })),
    deleteGroup: (id) => invoke('delete_group', { id }),
    switchGroup: (id) => invoke('switch_group', { id }),
    savePreferences: (preferences) => invoke('save_preferences', { preferences }),
  };
}
export function createCommandAdapter(): CommandAdapter {
  const invoke: TauriInvoke = async (command, args) => {
    const { invoke: call } = await import('@tauri-apps/api/core');
    try {
      return await call(command, args);
    } catch (error) {
      throw (
        error && typeof error === 'object' && 'code' in error ? error : { code: 'ipc_error', message: String(error) }
      ) as CommandError;
    }
  };
  return createTauriAdapter(invoke);
}
