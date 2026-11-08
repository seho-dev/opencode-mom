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
import type { BehaviorDocument, BehaviorUpdate } from '$src/types/behavior.js';
import type {
  SlashCommandDelete,
  SlashCommandDraft,
  SlashCommandEntry,
  SlashCommandList,
  SlashCommandUpdate,
} from '$src/types/commands.js';
import type { Group, GroupType } from '$src/types/groups.js';
import type {
  IslandAnswer,
  IslandMessage,
  IslandPermissionDecision,
  IslandState,
  IslandView,
} from '$src/types/island.js';
import type { LoggingState, LogPage, LogQuery } from '$src/types/logging.js';
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
import type { NotificationPermission, NotificationSettings } from '$src/types/notifications.js';
import type { PluginRemovalPlan, PluginRemovalTarget, PluginRuntime } from '$src/types/plugins.js';
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
export const groupTypeToWire = (type: GroupType): string => GROUP_TYPE_WIRE[type];
export const groupTypeFromWire = (value: unknown): GroupType => {
  switch (value) {
    case 'opencode':
      return GROUP_TYPE_NATIVE;
    case 'slim':
      return GROUP_TYPE_SLIM;
    case 'oh-my-openagent':
      return GROUP_TYPE_OMO;
    default:
      throw {
        code: 'configuration_failed',
        message: 'Configuration contains an unsupported group type.',
      } satisfies CommandError;
  }
};

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
  opencodeListModels(provider?: string): Result<ModelCatalogEntry[]>;
  generatePrompt(request: PromptGenerationRequest): Result<string>;
  opencodeReload(): Result<void>;
  opencodeTokenUsageRecords(): Result<TokenUsageRecord[]>;
  createProvider(value: ProviderDef): Result<ProviderDef>;
  updateProvider(value: ProviderDef): Result<ProviderDef>;
  deleteProvider(id: string): Result<void>;
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
  listPluginRuntime(): Result<PluginRuntime>;
  updatePluginPackage(target: string): Result<void>;
  preparePluginRemoval(target: PluginRemovalTarget): Result<PluginRemovalPlan>;
  removePluginDeclaration(plan: PluginRemovalPlan): Result<void>;
  listSlashCommands(projectId?: string | null): Result<SlashCommandList>;
  getSlashCommand(id: string, projectId?: string | null): Result<SlashCommandEntry>;
  createSlashCommand(draft: SlashCommandDraft, projectId?: string | null): Result<SlashCommandEntry>;
  updateSlashCommand(draft: SlashCommandUpdate, projectId?: string | null): Result<SlashCommandEntry>;
  deleteSlashCommand(draft: SlashCommandDelete, projectId?: string | null): Result<void>;
  getGlobalBehavior(): Result<BehaviorDocument>;
  saveGlobalBehavior(draft: BehaviorUpdate): Result<BehaviorDocument>;
  getAutostart(): Result<boolean>;
  setAutostart(enabled: boolean): Result<boolean>;
  getLidProtection(): Result<LidState>;
  setLidProtection(enabled: boolean): Result<LidState>;
  getNotificationSettings(): Result<NotificationSettings>;
  getNotificationPermission(): Result<NotificationPermission>;
  requestNotificationPermission(): Result<NotificationPermission>;
  openNotificationSettings(): Result<void>;
  setNotificationSettings(settings: NotificationSettings): Result<NotificationSettings>;
  getLoggingState(): Result<LoggingState>;
  setLoggingEnabled(enabled: boolean): Result<LoggingState>;
  queryLogs(query: LogQuery): Result<LogPage>;
  openLogsDirectory(): Result<void>;
  clearLogs(): Result<LoggingState>;
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
  rotateMomHttpKey(): Result<string>;
  getMcpProxySettings(): Result<McpProxySettingsDto>;
  saveMcpProxySettings(settings: McpProxySettingsDto, expectedRevision: number): Result<void>;
  getMcpProxyServer(id: string): Result<McpProxyServerDto>;
  createMcpProxyServer(server: McpProxyServerDto, expectedRevision: number): Result<McpProxyServerDto>;
  updateMcpProxyServer(server: McpProxyServerDto, expectedRevision: number): Result<McpProxyServerDto>;
  deleteMcpProxyServer(id: string, expectedRevision: number): Result<void>;
  importMcpProxyServer(request: ImportMcpProxyServerRequest): Result<McpProxyServerDto>;
  closeMcpProxyUnit(request: CloseMcpProxyUnitRequest): Result<void>;
  saveGroup(value: Group): Result<Group>;
  deleteGroup(id: string): Result<void>;
  switchGroup(id: string): Result<void>;
  savePreferences(preferences: AppPreferences): Result<void>;
}

export type TauriInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

// Persisted group.type uses frozen wire values; canonicalize at the IPC boundary only.
const readGroup = (group: Group): Group => ({ ...group, type: groupTypeFromWire(group.type) });
const writeGroup = (group: Group): Group => ({ ...group, type: groupTypeToWire(group.type) as GroupType });
const projectScope = (projectId?: string | null) => (projectId == null ? {} : { projectId });

function readNotificationPermission(value: unknown): NotificationPermission {
  switch (value) {
    case 'prompt':
    case 'granted':
    case 'denied':
    case 'unavailable':
    case 'unsupported':
      return value;
    default:
      throw {
        code: 'configuration_failed',
        message: 'Notification permission could not be confirmed.',
      } satisfies CommandError;
  }
}
function readNotificationSettings(value: unknown): NotificationSettings {
  if (
    !value ||
    typeof value !== 'object' ||
    !('enabled' in value) ||
    typeof value.enabled !== 'boolean' ||
    !('agentCompleted' in value) ||
    typeof value.agentCompleted !== 'boolean' ||
    !('approvalRequired' in value) ||
    typeof value.approvalRequired !== 'boolean' ||
    !('agentError' in value) ||
    typeof value.agentError !== 'boolean'
  )
    throw {
      code: 'configuration_failed',
      message: 'Notification settings could not be confirmed.',
    } satisfies CommandError;
  return {
    enabled: value.enabled,
    agentCompleted: value.agentCompleted,
    approvalRequired: value.approvalRequired,
    agentError: value.agentError,
  };
}

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
    opencodeListModels: (provider) => invoke('opencode_list_models', { provider }),
    generatePrompt: (request) => invoke('opencode_generate_prompt', { request }),
    opencodeReload: () => invoke('opencode_reload'),
    opencodeTokenUsageRecords: () => invoke('opencode_token_usage_records'),
    createProvider: (value) => invoke('create_provider', { provider: value }),
    updateProvider: (value) => invoke('update_provider', { provider: value }),
    deleteProvider: (id) => invoke('delete_provider', { id }),
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
    listPluginRuntime: () => invoke('list_plugin_runtime'),
    updatePluginPackage: (target) => invoke('update_plugin_package', { target }),
    preparePluginRemoval: (target) => invoke('prepare_plugin_removal', { target }),
    removePluginDeclaration: (plan) => invoke('remove_plugin_declaration', { plan }),
    listSlashCommands: (projectId) =>
      projectId == null ? invoke('list_slash_commands') : invoke('list_slash_commands', { projectId }),
    getSlashCommand: (id, projectId) => invoke('get_slash_command', { id, ...projectScope(projectId) }),
    createSlashCommand: (draft, projectId) => invoke('create_slash_command', { draft, ...projectScope(projectId) }),
    updateSlashCommand: (draft, projectId) => invoke('update_slash_command', { draft, ...projectScope(projectId) }),
    deleteSlashCommand: (draft, projectId) => invoke('delete_slash_command', { draft, ...projectScope(projectId) }),
    getGlobalBehavior: () => invoke('get_global_behavior'),
    saveGlobalBehavior: (draft) => invoke('save_global_behavior', { draft }),
    getAutostart: () => invoke('get_autostart'),
    setAutostart: (enabled) => invoke('set_autostart', { enabled }),
    getLidProtection: () => invoke('get_lid_protection'),
    setLidProtection: (enabled) => invoke('set_lid_protection', { enabled }),
    getNotificationSettings: async () => readNotificationSettings(await invoke('get_notification_settings')),
    getNotificationPermission: async () => readNotificationPermission(await invoke('get_notification_permission')),
    requestNotificationPermission: async () =>
      readNotificationPermission(await invoke('request_notification_permission')),
    openNotificationSettings: () => invoke('open_notification_settings'),
    setNotificationSettings: async (settings) =>
      readNotificationSettings(await invoke('set_notification_settings', { settings })),
    getLoggingState: () => invoke('get_logging_state'),
    setLoggingEnabled: (enabled) => invoke('set_logging_enabled', { enabled }),
    queryLogs: (query) => invoke('query_logs', { query }),
    openLogsDirectory: () => invoke('open_logs_directory'),
    clearLogs: () => invoke('clear_logs'),
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
    rotateMomHttpKey: () => invoke('rotate_mom_http_key'),
    getMcpProxySettings: () => invoke('get_mcp_proxy_settings'),
    saveMcpProxySettings: (settings, expectedRevision) =>
      invoke('save_mcp_proxy_settings', { settings, expectedRevision }),
    getMcpProxyServer: (id) => invoke('get_mcp_proxy_server', { id }),
    createMcpProxyServer: (server, expectedRevision) => invoke('create_mcp_proxy_server', { server, expectedRevision }),
    updateMcpProxyServer: (server, expectedRevision) => invoke('update_mcp_proxy_server', { server, expectedRevision }),
    deleteMcpProxyServer: (id, expectedRevision) => invoke('delete_mcp_proxy_server', { id, expectedRevision }),
    importMcpProxyServer: (request) => invoke('import_mcp_proxy_server', { request }),
    closeMcpProxyUnit: (request) => invoke('close_mcp_proxy_unit', { request }),
    saveGroup: async (value) => readGroup(await invoke<Group>('save_group', { group: writeGroup(value) })),
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
