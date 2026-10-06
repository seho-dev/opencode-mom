import type { AgentDefinition } from '$src/types/agents.js';
import type { AppPreferences, AppState, CommandError, LocalePreference, ThemePreference } from '$src/types/app.js';
import type { Group } from '$src/types/groups.js';
import type { McpDraft, McpList, McpServer, McpUpdate } from '$src/types/mcp.js';
import type { ModelCatalogEntry, ModelDef, ModelRef } from '$src/types/models.js';
import type { LidState } from '$src/types/power.js';
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

export function createConfigStore(adapter: CommandAdapter, catalogOnRefresh = true) {
  let providers = $state<ProviderDef[]>([]);
  let agents = $state<AgentDefinition[]>([]);
  let groups = $state<Group[]>([]);
  let selectedGroupId = $state<string | null>(null);
  let loading = $state(true);
  let splashLoading = $state(true);
  let initializing = true;
  let stateRequests = 0;
  let stateRequestSeq = 0;
  let saving = $state(false);
  let switching = $state(false);
  let reloading = $state(false);
  let error = $state<CommandError | null>(null);
  let notice = $state<string | null>(null);
  let draftRecovery = $state<DraftRecovery | null>(null);
  let formResetVersion = $state(0);
  // New: opencode CLI model catalog (builtin + custom), fetched via wrapper
  let catalog = $state<ModelCatalogEntry[]>([]);
  let catalogLoading = $state(false);
  let catalogError = $state<CommandError | null>(null);
  let refreshAllError = $state<CommandError | null>(null);
  // Guards against out-of-order catalog responses when loads overlap (startup prefetch, group switch, manual refresh).
  let catalogRequestSeq = 0;
  let preferences = $state<AppPreferences>({ theme: 'dark', locale: 'en' });
  function syncPreferencesToDom() {
    if (typeof document === 'undefined') return;
    document.documentElement.setAttribute('data-theme', preferences.theme);
    document.documentElement.lang = preferences.locale;
  }
  const apply = (state: AppState, resetForms = false) => {
    providers = state.providers;
    agents = state.agents;
    groups = state.groups;
    selectedGroupId = state.selectedGroupId ?? null;
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
      const secretDraft = ['createMcp', 'updateMcp', 'createSkill', 'updateSkill'].includes(operation);
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
  async function refresh(keepDraft = false, rejectCatalog = false) {
    loading = true;
    stateRequests += 1;
    const request = ++stateRequestSeq;
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
      if (request === stateRequestSeq) apply(state, !keepDraft);
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
    const seq = ++catalogRequestSeq;
    catalogLoading = true;
    catalogError = null;
    try {
      const result = await adapter.opencodeListModels(provider);
      if (seq === catalogRequestSeq) catalog = result;
      return result;
    } catch (cause) {
      const err = serializeError(cause);
      if (seq === catalogRequestSeq) catalogError = err;
      throw err;
    } finally {
      if (seq === catalogRequestSeq) catalogLoading = false;
    }
  }
  const finishInitialization = () => {
    initializing = false;
    if (!reloading) splashLoading = false;
  };
  void refresh().then(finishInitialization, finishInitialization);
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
    refresh,
    refreshAll,
    reloadOpencode,
    reloadKeepingDraft,
    discardDraftAndRefresh,
    continueEditing,
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
    async createAgent(value: AgentDefinition) {
      await run('createAgent', value, () => adapter.createAgent(value));
      await refresh();
    },
    async updateAgent(value: AgentDefinition) {
      await run('updateAgent', value, () => adapter.updateAgent(value));
      await refresh();
    },
    async deleteAgent(id: string, storage?: AgentDefinition['storage']) {
      await run('deleteAgent', { id, storage }, () => adapter.deleteAgent(id, storage));
      await refresh();
    },
    listMcps(): Promise<McpList> {
      return adapter.listMcps();
    },
    getMcp(name: string): Promise<McpServer> {
      return adapter.getMcp(name);
    },
    createMcp(draft: McpDraft): Promise<McpServer> {
      return run('createMcp', draft, () => adapter.createMcp(draft));
    },
    updateMcp(draft: McpUpdate): Promise<McpServer> {
      return run('updateMcp', draft, () => adapter.updateMcp(draft));
    },
    async deleteMcp(name: string): Promise<void> {
      await run('deleteMcp', { name }, () => adapter.deleteMcp(name));
    },
    listSkills(): Promise<SkillList> {
      return adapter.listSkills();
    },
    getSkill(id: string): Promise<SkillEntry> {
      return adapter.getSkill(id);
    },
    createSkill(draft: SkillDraft): Promise<SkillEntry> {
      return run('createSkill', draft, () => adapter.createSkill(draft));
    },
    updateSkill(draft: SkillUpdate): Promise<SkillEntry> {
      return run('updateSkill', draft, () => adapter.updateSkill(draft));
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
    loadTokenUsageRecords(): Promise<TokenUsageRecord[]> {
      return adapter.opencodeTokenUsageRecords();
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
