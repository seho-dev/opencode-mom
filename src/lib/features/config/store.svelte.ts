import type { CommandAdapter } from './adapter.js';
import type {
  AgentDefinition,
  AppState,
  Group,
  ModelCatalogEntry,
  ModelDef,
  ProviderDef,
  ModelRef,
  CommandError,
} from './types.js';

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
    const record = value as Record<string, unknown>;
    const code =
      typeof record['code'] === 'string' && errorCodes.has(record['code'] as CommandError['code'])
        ? (record['code'] as CommandError['code'])
        : 'ipc_error';
    const message =
      typeof record['message'] === 'string'
        ? record['message']
        : value instanceof Error
          ? value.message
          : 'Configuration operation failed. Review the draft in the original form and retry.';
    return { code, message, detail: record['detail'] ?? (code === 'ipc_error' ? value : undefined) };
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

export function createConfigStore(adapter: CommandAdapter) {
  let providers = $state<ProviderDef[]>([]);
  let agents = $state<AgentDefinition[]>([]);
  let groups = $state<Group[]>([]);
  let loading = $state(true);
  let saving = $state(false);
  let switching = $state(false);
  let error = $state<CommandError | null>(null);
  let notice = $state<string | null>(null);
  let draftRecovery = $state<DraftRecovery | null>(null);
  let formResetVersion = $state(0);
  // New: opencode CLI model catalog (builtin + custom), fetched via wrapper
  let catalog = $state<ModelCatalogEntry[]>([]);
  let catalogLoading = $state(false);
  let catalogError = $state<CommandError | null>(null);
  // Guards against out-of-order catalog responses when loads overlap (startup prefetch, group switch, manual refresh).
  let catalogRequestSeq = 0;
  const apply = (state: AppState, resetForms = false) => {
    providers = state.providers;
    agents = state.agents;
    groups = state.groups;
    if (resetForms) formResetVersion += 1;
  };
  const run = async <T>(operation: string, payload: unknown, action: () => Promise<T>) => {
    error = null;
    saving = true;
    try {
      const result = await action();
      draftRecovery = null;
      return result;
    } catch (cause) {
      const normalized = serializeError(cause);
      error = normalized;
      draftRecovery = {
        operation,
        payload: snapshot(payload),
        error: normalized,
        conflict: normalized.code === 'conflict',
      };
      throw normalized;
    } finally {
      saving = false;
    }
  };
  async function refresh(keepDraft = false) {
    loading = true;
    if (!keepDraft) {
      error = null;
      draftRecovery = null;
    }
    try {
      apply(await adapter.loadAppState(), !keepDraft);
    } catch (cause) {
      error = serializeError(cause);
    } finally {
      loading = false;
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
  refresh();
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
    get loading() {
      return loading;
    },
    get saving() {
      return saving;
    },
    get switching() {
      return switching;
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
    clearNotice() {
      notice = null;
    },
    refresh,
    reloadKeepingDraft,
    discardDraftAndRefresh,
    continueEditing,
    async createProvider(value: ProviderDef) {
      await run('createProvider', value, () => adapter.createProvider(value));
      await refresh();
      await loadCatalog();
    },
    async updateProvider(value: ProviderDef) {
      await run('updateProvider', value, () => adapter.updateProvider(value));
      await refresh();
      await loadCatalog();
    },
    async deleteProvider(id: string) {
      await run('deleteProvider', { id }, () => adapter.deleteProvider(id));
      await refresh();
      await loadCatalog();
    },
    async createModel(providerId: string, value: ModelDef) {
      await run('createModel', { providerId, value }, () => adapter.createModel(providerId, value));
      await refresh();
      await loadCatalog();
    },
    async updateModel(providerId: string, value: ModelDef) {
      await run('updateModel', { providerId, value }, () => adapter.updateModel(providerId, value));
      await refresh();
      await loadCatalog();
    },
    async deleteModel(ref: ModelRef) {
      await run('deleteModel', { ref }, () => adapter.deleteModel(ref));
      await refresh();
      await loadCatalog();
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
        notice = 'Group switched and configuration applied.';
        await refresh();
        try {
          await loadCatalog();
        } catch {
          // Best-effort: the switch already succeeded; catalog failures are surfaced on the Models page.
        }
      } finally {
        switching = false;
      }
    },
    loadCatalog,
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
