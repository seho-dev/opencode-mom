import type {
  AgentDefinition,
  AppState,
  CommandError,
  Group,
  ModelCatalogEntry,
  ModelDef,
  ModelRef,
  ProviderDef,
  Result,
} from './types.js';

export interface CommandAdapter {
  loadAppState(): Result<AppState>;
  listProviders(): Result<ProviderDef[]>;
  listCustomProviders(): Result<ProviderDef[]>;
  opencodeListModels(provider?: string): Result<ModelCatalogEntry[]>;
  opencodeResolveBinary(): Result<string>;
  createProvider(value: ProviderDef): Result<ProviderDef>;
  updateProvider(value: ProviderDef): Result<ProviderDef>;
  deleteProvider(id: string): Result<void>;
  listModels(): Result<ProviderDef[]>;
  createModel(providerId: string, value: ModelDef): Result<ModelDef>;
  updateModel(providerId: string, value: ModelDef): Result<ModelDef>;
  deleteModel(ref: ModelRef): Result<void>;
  listAgents(): Result<AgentDefinition[]>;
  getAgent(id: string): Result<AgentDefinition>;
  createAgent(value: AgentDefinition): Result<AgentDefinition>;
  updateAgent(value: AgentDefinition): Result<AgentDefinition>;
  deleteAgent(id: string, storage?: AgentDefinition['storage']): Result<void>;
  saveGroup(value: Group): Result<Group>;
  copyGroup(id: string, name: string): Result<Group>;
  deleteGroup(id: string): Result<void>;
  switchGroup(id: string): Result<void>;
}

export type TauriInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
export function createTauriAdapter(invoke: TauriInvoke): CommandAdapter {
  return {
    loadAppState: () => invoke('load_app_state'),
    listProviders: () => invoke('list_providers'),
    listCustomProviders: () => invoke('list_custom_providers'),
    opencodeListModels: (provider) => invoke('opencode_list_models', { provider }),
    opencodeResolveBinary: () => invoke('opencode_resolve_binary'),
    createProvider: (value) => invoke('create_provider', { provider: value }),
    updateProvider: (value) => invoke('update_provider', { provider: value }),
    deleteProvider: (id) => invoke('delete_provider', { id }),
    listModels: () => invoke('list_models'),
    createModel: (providerId, value) => invoke('create_model', { providerId, model: value }),
    updateModel: (providerId, value) => invoke('update_model', { providerId, model: value }),
    deleteModel: (modelRef) => invoke('delete_model', { modelRef }),
    listAgents: () => invoke('list_agents'),
    getAgent: (id) => invoke('get_agent', { id }),
    createAgent: (value) => invoke('create_agent', { agent: value }),
    updateAgent: (value) => invoke('update_agent', { agent: value }),
    deleteAgent: (id, storage) => invoke('delete_agent', { id, storage }),
    saveGroup: (value) => invoke('save_group', { group: value }),
    copyGroup: (id, name) => invoke('copy_group', { id, name }),
    deleteGroup: (id) => invoke('delete_group', { id }),
    switchGroup: (id) => invoke('switch_group', { id }),
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
