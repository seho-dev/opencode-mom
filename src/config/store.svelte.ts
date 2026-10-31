import type { AgentDefinition } from '$src/types/agents.js';
import type {
  AppInfo,
  AppPreferences,
  AppState,
  CommandError,
  LocalePreference,
  OpenCodeInstallation,
  OpenCodeServiceStatus,
  OpenCodeUpdateCheck,
  ProjectFolder,
  ProjectPage,
  ThemePreference,
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
import type { Group } from '$src/types/groups.js';
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
import type { CommandAdapter } from './adapter.js';

type DraftRecovery = { operation: string; payload: unknown; error: CommandError; conflict: boolean };
const errorCodes = new Set<CommandError['code']>([
  'not_found',
  'references_blocked',
  'validation_failed',
  'configuration_failed',
  'busy',
  'conflict',
  'ipc_error',
]);
const serializeError = (value: unknown): CommandError => {
  if (value && typeof value === 'object') {
    const record = value as { code?: unknown; message?: unknown; detail?: unknown };
    const code =
      typeof record.code === 'string' && errorCodes.has(record.code as CommandError['code'])
        ? (record.code as CommandError['code'])
        : 'ipc_error';
    const message =
      typeof record.message === 'string'
        ? record.message
        : value instanceof Error
          ? value.message
          : 'Configuration operation failed. Review the draft in the original form and retry.';
    return { code, message, detail: record.detail ?? (code === 'ipc_error' ? value : undefined) };
  }
  return {
    code: 'ipc_error',
    message:
      typeof value === 'string'
        ? value
        : 'Configuration operation failed. Review the draft in the original form and retry.',
    detail: value,
  };
};
const snapshot = <T>(value: T): T => structuredClone(value);
async function readResource<T>(action: () => Promise<T>): Promise<T> {
  try {
    return await action();
  } catch (cause) {
    throw {
      code: serializeError(cause).code,
      message: 'Configuration could not be loaded. Refresh and retry.',
    } satisfies CommandError;
  }
}

export function createConfigStore(adapter: CommandAdapter, catalogOnRefresh = true, initialize = true) {
  let providers = $state<ProviderDef[]>([]);
  let agents = $state<AgentDefinition[]>([]);
  let groups = $state<Group[]>([]);
  let selectedGroupId = $state<string | null>(null);
  let projects = $state<ProjectFolder[]>([]);
  let selectedProjectId = $state<string | null>(null);
  let projectSaving = $state(false);
  let projectVersion = 0;
  let loading = $state(initialize);
  let splashLoading = $state(initialize);
  let initializing = initialize;
  let stateRequests = 0;
  let stateRequestSeq = 0;
  let saving = $state(false);
  let switching = $state(false);
  let reloading = $state(false);
  let error = $state<CommandError | null>(null);
  let notice = $state<string | null>(null);
  let draftRecovery = $state<DraftRecovery | null>(null);
  let formResetVersion = $state(0);
  // OpenCode service model catalog, fetched through the command adapter.
  let catalog = $state<ModelCatalogEntry[]>([]);
  let catalogLoading = $state(false);
  let catalogError = $state<CommandError | null>(null);
  let refreshAllError = $state<CommandError | null>(null);
  // Guards against out-of-order catalog responses when loads overlap (startup prefetch, group switch, manual refresh).
  let catalogRequestSeq = 0;
  let serviceTargetVersion = $state(0);
  let serviceTargetRequests = 0;
  function assertServiceTarget(version: number) {
    if (version !== serviceTargetVersion || serviceTargetRequests > 0)
      throw {
        code: 'conflict',
        message: 'The OpenCode service target changed. Refresh to read the current target.',
      } satisfies CommandError;
  }
  async function runForServiceTarget<T>(action: () => Promise<T>): Promise<T> {
    const targetVersion = serviceTargetVersion;
    assertServiceTarget(targetVersion);
    const result = await action();
    assertServiceTarget(targetVersion);
    return result;
  }
  let preferences = $state<AppPreferences>({ theme: 'dark', locale: 'en' });
  function syncPreferencesToDom() {
    if (typeof document === 'undefined') return;
    document.documentElement.setAttribute('data-theme', preferences.theme);
    document.documentElement.lang = preferences.locale;
  }
  const apply = (state: AppState, resetForms = false, readProjectVersion = projectVersion) => {
    providers = state.providers;
    agents = state.agents;
    groups = state.groups;
    selectedGroupId = state.selectedGroupId ?? null;
    // Older global reads must not revert a completed project persistence operation.
    if (readProjectVersion === projectVersion) {
      projects = state.projects ?? [];
      selectedProjectId = projects.some((project) => project.id === state.selectedProjectId)
        ? (state.selectedProjectId ?? null)
        : null;
    }
    preferences = state.preferences;
    syncPreferencesToDom();
    if (resetForms) formResetVersion += 1;
  };
  async function persistPreferences(next: AppPreferences) {
    const previous = preferences;
    preferences = next;
    syncPreferencesToDom();
    try {
      await adapter.savePreferences(next);
    } catch (cause) {
      preferences = previous;
      syncPreferencesToDom();
      error = serializeError(cause);
    }
  }
  const run = async <T>(operation: string, payload: unknown, action: () => Promise<T>) => {
    error = null;
    saving = true;
    try {
      const result = await action();
      draftRecovery = null;
      notice = operation;
      return result;
    } catch (cause) {
      const secretDraft = [
        'createMcp',
        'updateMcp',
        'createSkill',
        'updateSkill',
        'createSlashCommand',
        'updateSlashCommand',
        'deleteSlashCommand',
        'saveGlobalBehavior',
      ].includes(operation);
      const normalized = secretDraft
        ? {
            code: serializeError(cause).code,
            message: 'Configuration operation failed. Review the draft in the original form and retry.',
          }
        : serializeError(cause);
      error = normalized;
      draftRecovery = {
        operation,
        payload: secretDraft ? null : snapshot(payload),
        error: normalized,
        conflict: normalized.code === 'conflict',
      };
      throw normalized;
    } finally {
      saving = false;
    }
  };
  const runProject = async <T>(action: () => Promise<T>, commit: (result: T) => void): Promise<T> => {
    if (projectSaving)
      throw { code: 'busy', message: 'A project operation is already in progress.' } satisfies CommandError;
    projectSaving = true;
    try {
      const result = await action();
      projectVersion += 1;
      commit(result);
      return result;
    } finally {
      projectSaving = false;
    }
  };
  async function refresh(keepDraft = false, rejectCatalog = false) {
    loading = true;
    stateRequests += 1;
    const request = ++stateRequestSeq;
    const readProjectVersion = projectVersion;
    if (!keepDraft) {
      error = null;
      draftRecovery = null;
    }
    const catalogResult = (catalogOnRefresh ? loadCatalog() : Promise.resolve()).then(
      () => null,
      (cause: unknown) => serializeError(cause),
    );
    try {
      const state = await adapter.loadAppState();
      if (request === stateRequestSeq) apply(state, !keepDraft, readProjectVersion);
    } catch (cause) {
      if (request === stateRequestSeq) error = serializeError(cause);
    } finally {
      if (--stateRequests === 0) loading = false;
    }
    const catalogFailure = await catalogResult;
    if (rejectCatalog && catalogFailure) throw catalogFailure;
  }
  async function refreshAll() {
    refreshAllError = null;
    try {
      await refresh(false, true);
    } catch (cause) {
      refreshAllError = serializeError(cause);
      throw refreshAllError;
    }
  }
  async function reloadOpencode() {
    if (reloading) return;
    reloading = true;
    splashLoading = true;
    refreshAllError = null;
    try {
      await adapter.opencodeReload();
      await refreshAll();
    } catch (cause) {
      refreshAllError = serializeError(cause);
      throw refreshAllError;
    } finally {
      reloading = false;
      if (!initializing) splashLoading = false;
    }
  }
  async function reloadKeepingDraft() {
    await refresh(true);
  }
  async function discardDraftAndRefresh() {
    draftRecovery = null;
    error = null;
    await refresh();
  }
  function continueEditing() {
    error = null;
  }
  async function loadCatalog(provider?: string) {
    const targetVersion = serviceTargetVersion;
    assertServiceTarget(targetVersion);
    const seq = ++catalogRequestSeq;
    catalogLoading = true;
    catalogError = null;
    try {
      const result = await adapter.opencodeListModels(provider);
      assertServiceTarget(targetVersion);
      if (seq === catalogRequestSeq) catalog = result;
      return result;
    } catch (cause) {
      const err = serializeError(cause);
      if (seq === catalogRequestSeq && targetVersion === serviceTargetVersion && serviceTargetRequests === 0)
        catalogError = err;
      throw err;
    } finally {
      if (seq === catalogRequestSeq) catalogLoading = false;
    }
  }
  const finishInitialization = () => {
    initializing = false;
    if (!reloading) splashLoading = false;
  };
  if (initialize) void refresh().then(finishInitialization, finishInitialization);
  return {
    get providers() {
      return providers;
    },
    get agents() {
      return agents;
    },
    get groups() {
      return groups;
    },
    get selectedGroupId() {
      return selectedGroupId;
    },
    get projects() {
      return projects;
    },
    get selectedProjectId() {
      return selectedProjectId;
    },
    get projectSaving() {
      return projectSaving;
    },
    get preferences() {
      return preferences;
    },
    get loading() {
      return loading;
    },
    get splashLoading() {
      return splashLoading;
    },
    get saving() {
      return saving;
    },
    get switching() {
      return switching;
    },
    get reloading() {
      return reloading;
    },
    get error() {
      return error;
    },
    get notice() {
      return notice;
    },
    get draftRecovery() {
      return draftRecovery;
    },
    get formResetVersion() {
      return formResetVersion;
    },
    get serviceTargetVersion() {
      return serviceTargetVersion;
    },
    get catalog() {
      return catalog;
    },
    get catalogLoading() {
      return catalogLoading;
    },
    get catalogError() {
      return catalogError;
    },
    get refreshAllError() {
      return refreshAllError;
    },
    clearNotice() {
      notice = null;
    },
    setTheme(theme: ThemePreference) {
      return persistPreferences({ ...preferences, theme });
    },
    setLocale(locale: LocalePreference) {
      return persistPreferences({ ...preferences, locale });
    },
    applyPreferences(next: AppPreferences) {
      preferences = next;
      syncPreferencesToDom();
    },
    refresh,
    refreshAll,
    reloadOpencode,
    reloadKeepingDraft,
    discardDraftAndRefresh,
    continueEditing,
    getAppInfo(): Promise<AppInfo> {
      return adapter.getAppInfo();
    },
    checkForUpdates(): Promise<UpdateCheck> {
      return adapter.checkForUpdates();
    },
    getOpenCodeInstallation(): Promise<OpenCodeInstallation> {
      return adapter.getOpenCodeInstallation();
    },
    getOpenCodeServiceStatus(): Promise<OpenCodeServiceStatus> {
      return adapter.getOpenCodeServiceStatus();
    },
    generatePrompt(request: PromptGenerationRequest): Promise<string> {
      return runForServiceTarget(() => adapter.generatePrompt(request));
    },
    async setOpenCodeServiceTarget(url: string | null, password?: string | null): Promise<OpenCodeServiceStatus> {
      serviceTargetRequests += 1;
      try {
        const status = await adapter.setOpenCodeServiceTarget(url, password);
        serviceTargetVersion += 1;
        catalogRequestSeq += 1;
        catalog = [];
        catalogError = null;
        catalogLoading = false;
        return status;
      } finally {
        serviceTargetRequests -= 1;
      }
    },
    checkOpenCodeUpdate(): Promise<OpenCodeUpdateCheck> {
      return adapter.checkOpenCodeUpdate();
    },
    upgradeOpenCode(
      expectedBinaryPath: string,
      expectedVersion: string,
      targetVersion: string,
    ): Promise<OpenCodeInstallation> {
      return adapter.upgradeOpenCode(expectedBinaryPath, expectedVersion, targetVersion);
    },
    openProjectPage(page: ProjectPage): Promise<void> {
      return adapter.openProjectPage(page);
    },
    pickProjectFolder(): Promise<string | null> {
      return adapter.pickProjectFolder();
    },
    addProject(path: string): Promise<ProjectFolder> {
      return runProject(
        () => adapter.addProject(path),
        (project) => {
          projects = [...projects.filter((entry) => entry.id !== project.id && entry.path !== project.path), project];
          selectedProjectId = project.id;
        },
      );
    },
    removeProject(id: string): Promise<void> {
      return runProject(
        () => adapter.removeProject(id),
        () => {
          projects = projects.filter((project) => project.id !== id);
          if (selectedProjectId === id) selectedProjectId = null;
        },
      );
    },
    selectProject(id: string | null): Promise<void> {
      return runProject(
        async () => {
          if (id !== null && !projects.some((project) => project.id === id))
            throw { code: 'not_found', message: 'Project not found.' } satisfies CommandError;
          await adapter.selectProject(id);
        },
        () => {
          selectedProjectId = id;
        },
      );
    },
    async createProvider(value: ProviderDef) {
      await run('createProvider', value, () => adapter.createProvider(value));
      await refresh();
    },
    async updateProvider(value: ProviderDef) {
      await run('updateProvider', value, () => adapter.updateProvider(value));
      await refresh();
    },
    async deleteProvider(id: string) {
      await run('deleteProvider', { id }, () => adapter.deleteProvider(id));
      await refresh();
    },
    async createModel(providerId: string, value: ModelDef) {
      await run('createModel', { providerId, value }, () => adapter.createModel(providerId, value));
      await refresh();
    },
    async updateModel(providerId: string, value: ModelDef) {
      await run('updateModel', { providerId, value }, () => adapter.updateModel(providerId, value));
      await refresh();
    },
    async deleteModel(ref: ModelRef) {
      await run('deleteModel', { ref }, () => adapter.deleteModel(ref));
      await refresh();
    },
    listAgents(projectId?: string | null): Promise<AgentDefinition[]> {
      return adapter.listAgents(projectId);
    },
    getAgent(id: string, projectId?: string | null): Promise<AgentDefinition> {
      return adapter.getAgent(id, projectId);
    },
    async createAgent(value: AgentDefinition, projectId?: string | null) {
      await run('createAgent', value, () => adapter.createAgent(value, projectId));
      await refresh();
    },
    async updateAgent(value: AgentDefinition, projectId?: string | null) {
      await run('updateAgent', value, () => adapter.updateAgent(value, projectId));
      await refresh();
    },
    async deleteAgent(id: string, storage?: AgentDefinition['storage'], projectId?: string | null) {
      await run('deleteAgent', { id, storage }, () => adapter.deleteAgent(id, storage, projectId));
      await refresh();
    },
    listMcps(projectId?: string | null): Promise<McpList> {
      return adapter.listMcps(projectId);
    },
    getMcp(name: string, projectId?: string | null): Promise<McpServer> {
      return adapter.getMcp(name, projectId);
    },
    createMcp(draft: McpDraft, projectId?: string | null): Promise<McpServer> {
      return run('createMcp', draft, () => adapter.createMcp(draft, projectId));
    },
    updateMcp(draft: McpUpdate, projectId?: string | null): Promise<McpServer> {
      return run('updateMcp', draft, () => adapter.updateMcp(draft, projectId));
    },
    async deleteMcp(name: string, projectId?: string | null): Promise<void> {
      await run('deleteMcp', { name }, () => adapter.deleteMcp(name, projectId));
    },
    listSkills(projectId?: string | null): Promise<SkillList> {
      return adapter.listSkills(projectId);
    },
    getSkill(id: string, projectId?: string | null): Promise<SkillEntry> {
      return adapter.getSkill(id, projectId);
    },
    createSkill(draft: SkillDraft, projectId?: string | null): Promise<SkillEntry> {
      return run('createSkill', draft, () => adapter.createSkill(draft, projectId));
    },
    updateSkill(draft: SkillUpdate, projectId?: string | null): Promise<SkillEntry> {
      return run('updateSkill', draft, () => adapter.updateSkill(draft, projectId));
    },
    listSlashCommands(projectId?: string | null): Promise<SlashCommandList> {
      return readResource(() => adapter.listSlashCommands(projectId));
    },
    getSlashCommand(id: string, projectId?: string | null): Promise<SlashCommandEntry> {
      return readResource(() => adapter.getSlashCommand(id, projectId));
    },
    createSlashCommand(draft: SlashCommandDraft, projectId?: string | null): Promise<SlashCommandEntry> {
      return run('createSlashCommand', draft, () => adapter.createSlashCommand(draft, projectId));
    },
    updateSlashCommand(draft: SlashCommandUpdate, projectId?: string | null): Promise<SlashCommandEntry> {
      return run('updateSlashCommand', draft, () => adapter.updateSlashCommand(draft, projectId));
    },
    deleteSlashCommand(draft: SlashCommandDelete, projectId?: string | null): Promise<void> {
      return run('deleteSlashCommand', draft, () => adapter.deleteSlashCommand(draft, projectId));
    },
    getGlobalBehavior(): Promise<BehaviorDocument> {
      return readResource(() => adapter.getGlobalBehavior());
    },
    saveGlobalBehavior(draft: BehaviorUpdate): Promise<BehaviorDocument> {
      return run('saveGlobalBehavior', draft, () => adapter.saveGlobalBehavior(draft));
    },
    getAutostart(): Promise<boolean> {
      return adapter.getAutostart();
    },
    setAutostart(enabled: boolean): Promise<boolean> {
      return adapter.setAutostart(enabled);
    },
    getLidProtection(): Promise<LidState> {
      return adapter.getLidProtection();
    },
    setLidProtection(enabled: boolean): Promise<LidState> {
      return adapter.setLidProtection(enabled);
    },
    getIslandState(): Promise<IslandState> {
      return adapter.getIslandState();
    },
    setIslandEnabled(enabled: boolean): Promise<IslandState> {
      return adapter.setIslandEnabled(enabled);
    },
    refreshIsland(): Promise<IslandState> {
      return adapter.refreshIsland();
    },
    setIslandView(view: IslandView): Promise<void> {
      return adapter.setIslandView(view);
    },
    islandMessages(sessionId: string, expectedConnectionGeneration: number): Promise<IslandMessage[]> {
      return adapter.islandMessages(sessionId, expectedConnectionGeneration);
    },
    islandSend(
      sessionId: string,
      text: string,
      messageId: string,
      expectedConnectionGeneration: number,
    ): Promise<void> {
      return adapter.islandSend(sessionId, text, messageId, expectedConnectionGeneration);
    },
    islandReplyPermission(
      sessionId: string,
      requestId: string,
      decision: IslandPermissionDecision,
      expectedConnectionGeneration: number,
    ): Promise<void> {
      return adapter.islandReplyPermission(sessionId, requestId, decision, expectedConnectionGeneration);
    },
    islandReplyQuestion(
      sessionId: string,
      formId: string,
      answer: IslandAnswer,
      expectedConnectionGeneration: number,
    ): Promise<void> {
      return adapter.islandReplyQuestion(sessionId, formId, answer, expectedConnectionGeneration);
    },
    islandCancelQuestion(sessionId: string, formId: string, expectedConnectionGeneration: number): Promise<void> {
      return adapter.islandCancelQuestion(sessionId, formId, expectedConnectionGeneration);
    },
    islandInterrupt(sessionId: string, expectedConnectionGeneration: number): Promise<void> {
      return adapter.islandInterrupt(sessionId, expectedConnectionGeneration);
    },
    getMcpProxyState(): Promise<McpProxyStateDto> {
      return adapter.getMcpProxyState();
    },
    getMomHttpSettings(): Promise<MomHttpSettings> {
      return adapter.getMomHttpSettings();
    },
    setMomHttpSettings(settings: MomHttpSettings): Promise<void> {
      return adapter.setMomHttpSettings(settings);
    },
    getMcpProxySettings(): Promise<McpProxySettingsDto> {
      return adapter.getMcpProxySettings();
    },
    saveMcpProxySettings(settings: McpProxySettingsDto, expectedRevision: number): Promise<void> {
      return adapter.saveMcpProxySettings(settings, expectedRevision);
    },
    rotateMcpProxyKey(): Promise<string> {
      return adapter.rotateMcpProxyKey();
    },
    getMcpProxyServer(id: string): Promise<McpProxyServerDto> {
      return adapter.getMcpProxyServer(id);
    },
    createMcpProxyServer(server: McpProxyServerDto, expectedRevision: number): Promise<McpProxyServerDto> {
      return adapter.createMcpProxyServer(server, expectedRevision);
    },
    updateMcpProxyServer(server: McpProxyServerDto, expectedRevision: number): Promise<McpProxyServerDto> {
      return adapter.updateMcpProxyServer(server, expectedRevision);
    },
    deleteMcpProxyServer(id: string, expectedRevision: number): Promise<void> {
      return adapter.deleteMcpProxyServer(id, expectedRevision);
    },
    importMcpProxyServer(request: ImportMcpProxyServerRequest): Promise<McpProxyServerDto> {
      return adapter.importMcpProxyServer(request);
    },
    closeMcpProxyUnit(request: CloseMcpProxyUnitRequest): Promise<void> {
      return adapter.closeMcpProxyUnit(request);
    },
    async saveGroup(value: Group) {
      await run('saveGroup', value, () => adapter.saveGroup(value));
      await refresh();
    },
    async deleteGroup(id: string) {
      await run('deleteGroup', { id }, () => adapter.deleteGroup(id));
      await refresh();
    },
    async switchGroup(id: string) {
      switching = true;
      try {
        await run('switchGroup', { id }, () => adapter.switchGroup(id));
        await refresh();
      } finally {
        switching = false;
      }
    },
    loadCatalog,
    async loadTokenUsageRecords(): Promise<TokenUsageRecord[]> {
      const targetVersion = serviceTargetVersion;
      assertServiceTarget(targetVersion);
      const records = await adapter.opencodeTokenUsageRecords();
      assertServiceTarget(targetVersion);
      return records;
    },
    async refreshCatalog(provider?: string) {
      return loadCatalog(provider);
    },
    clearCatalogError() {
      catalogError = null;
    },
    models() {
      return providers.flatMap((provider) =>
        Object.values(provider.models).map((model) => ({
          ...model,
          ref: `${provider.name}/${model.id}` as ModelRef,
          providerId: provider.name,
        })),
      );
    },
    catalogModels() {
      return catalog;
    },
    builtinModels() {
      return catalog.filter((m) => !m.isCustom);
    },
    customModels() {
      return catalog.filter((m) => m.isCustom);
    },
  };
}
export type ConfigStore = ReturnType<typeof createConfigStore>;
