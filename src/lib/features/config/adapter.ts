import type {
  AgentDefinition,
  AppState,
  CommandError,
  Group,
  ModelDef,
  ModelRef,
  ProviderDef,
  Result,
} from './types.js';

export interface CommandAdapter {
  loadAppState(): Result<AppState>;
  listProviders(): Result<ProviderDef[]>;
  createProvider(value: ProviderDef): Result<ProviderDef>;
  updateProvider(value: ProviderDef): Result<ProviderDef>;
  deleteProvider(id: string): Result<void>;
  renameProvider(oldId: string, newId: string): Result<void>;
  revealProviderSecret(providerId: string, key: string): Result<unknown>;
  listModels(): Result<ProviderDef[]>;
  createModel(providerId: string, value: ModelDef): Result<ModelDef>;
  updateModel(providerId: string, value: ModelDef): Result<ModelDef>;
  deleteModel(ref: ModelRef): Result<void>;
  renameModel(ref: ModelRef, newId: string): Result<void>;
  replaceModelReferences(from: ModelRef, to: ModelRef): Result<void>;
  listAgents(): Result<AgentDefinition[]>;
  getAgent(id: string): Result<AgentDefinition>;
  createAgent(value: AgentDefinition): Result<AgentDefinition>;
  updateAgent(value: AgentDefinition): Result<AgentDefinition>;
  deleteAgent(id: string, storage?: AgentDefinition['storage']): Result<void>;
  saveGroup(value: Group): Result<Group>;
  copyGroup(id: string, name: string): Result<Group>;
  deleteGroup(id: string): Result<void>;
  switchGroup(id: string): Result<void>;
  loadConfigDiagnostics(): Result<string[]>;
  validateConfig(): Result<string[]>;
}

type ModelReference = { owner: string; replace: (to: ModelRef) => void };
type AgentMutation = {
  source: NonNullable<AgentDefinition['storage']>;
  fields?: Record<string, unknown>;
  prompt?: string;
  clearFields?: string[];
};
type AgentWrite = AgentDefinition & { mutation?: AgentMutation; clearFields?: string[]; sources?: unknown[] };
type BrowserMockSeed = Partial<AppState> & { conflictOn?: string[] };
const sensitiveOption = (key: string) => /(api.?key|token|secret|password|credential)/i.test(key);
const clone = <T>(value: T): T => structuredClone(value);

export function createBrowserMockAdapter(seed: BrowserMockSeed = {}): CommandAdapter {
  let state: AppState = clone({
    providers: seed.providers ?? [],
    agents: seed.agents ?? [],
    groups: seed.groups ?? [],
    diagnostics: seed.diagnostics ?? [],
  });
  const conflictOn = new Set(seed.conflictOn ?? []);
  const fail = (code: CommandError['code'], message: string, detail?: unknown): never => {
    throw { code, message, detail } satisfies CommandError;
  };
  const providerById = (id: string) =>
    state.providers.find((provider) => provider.id === id) ?? fail('not_found', 'Provider not found');
  const modelByRef = (ref: ModelRef) => {
    const [providerId, modelId, extra] = ref.split('/');
    if (!providerId || !modelId || extra) fail('validation_failed', 'Model reference must be provider/model');
    const provider = providerById(providerId as string);
    const model = provider.models[modelId as string] ?? fail('not_found', 'Model not found');
    return { provider, model };
  };
  const agentById = (id: string) =>
    state.agents.find((agent) => agent.id === id) ?? fail('not_found', 'Agent not found');
  const groupById = (id: string) =>
    state.groups.find((group) => group.id === id) ?? fail('not_found', 'Group not found');
  const bindings = (group: Group) => [
    ...group.openCodeAgentOverrides,
    ...(group.slimAgentOverrides ?? []),
    ...(group.omoAgentOverrides ?? []),
    ...(group.omoCategoryMappings ?? []),
  ];
  const modelReferences = (ref: ModelRef) => {
    const references: ModelReference[] = [];
    state.agents.forEach((agent) => {
      if (agent.modelRef === ref)
        references.push({
          owner: `agent.${agent.id}`,
          replace: (to) => {
            agent.modelRef = to;
          },
        });
    });
    state.groups.forEach((group) => {
      const replaceBindings = (entries: { modelRef: ModelRef }[] | null, owner: string) =>
        entries?.forEach((entry) => {
          if (entry.modelRef === ref)
            references.push({
              owner,
              replace: (to) => {
                entry.modelRef = to;
              },
            });
        });
      replaceBindings(group.openCodeAgentOverrides, `group.${group.name}.openCodeAgentOverrides`);
      replaceBindings(group.slimAgentOverrides, `group.${group.name}.slimAgentOverrides`);
      replaceBindings(group.omoAgentOverrides, `group.${group.name}.omoAgentOverrides`);
      replaceBindings(group.omoCategoryMappings, `group.${group.name}.omoCategoryMappings`);
    });
    return references;
  };
  const assertModelExists = (ref: ModelRef) => {
    modelByRef(ref);
  };
  const assertGroup = (group: Group) => {
    bindings(group).forEach((binding) => assertModelExists(binding.modelRef));
  };
  const sourceMap = (agent: AgentDefinition, storage: AgentDefinition['storage']) => {
    if (storage === 'inline') return (agent.inline ?? {}) as Record<string, unknown>;
    return (agent.markdown ?? {}) as Record<string, unknown>;
  };
  const sourcePreviews = (agent: AgentDefinition) => {
    const sources: Array<Record<string, unknown>> = [];
    if (agent.inline)
      sources.push({
        storage: 'inline',
        raw: clone(agent.inline),
        fields: clone(agent.inline),
        prompt: typeof agent.inline['prompt'] === 'string' ? agent.inline['prompt'] : undefined,
      });
    if (agent.markdown)
      sources.push({
        storage: agent.storage === 'project_markdown' ? 'project_markdown' : 'global_markdown',
        raw: clone(agent.markdown),
        fields: clone(agent.markdown),
        prompt: typeof agent.markdown['prompt'] === 'string' ? agent.markdown['prompt'] : undefined,
      });
    return sources;
  };
  const publicAgent = (agent: AgentDefinition) => {
    const source =
      agent.source === 'inline'
        ? sourceMap(agent, 'inline')
        : agent.source === 'markdown'
          ? sourceMap(agent, agent.storage ?? 'global_markdown')
          : { ...sourceMap(agent, 'inline'), ...sourceMap(agent, 'global_markdown') };
    const result = clone({ ...agent, ...source, sources: sourcePreviews(agent) }) as AgentWrite &
      Record<string, unknown>;
    if (typeof result['modelRef'] !== 'string' && typeof source['model'] === 'string')
      result.modelRef = source['model'] as ModelRef;
    if (typeof result['model'] === 'string') {
      result.modelRef = result['model'] as ModelRef;
      delete result['model'];
    }
    return result as AgentDefinition;
  };
  const applyAgentMutation = (current: AgentDefinition, value: AgentWrite) => {
    const mutation = value.mutation;
    if (!mutation?.source) return clone(value) as AgentDefinition;
    const storage = mutation.source;
    const fields = clone(mutation.fields ?? {});
    const clearFields = [...new Set([...(mutation.clearFields ?? []), ...(value.clearFields ?? [])])];
    if (clearFields.some((field) => !field.trim())) fail('validation_failed', 'Agent fields to clear cannot be empty');
    if (clearFields.some((field) => Object.prototype.hasOwnProperty.call(fields, field)))
      fail('validation_failed', 'An agent field cannot be set and cleared at the same time');
    if (typeof fields['modelRef'] === 'string') {
      fields['model'] = fields['modelRef'];
      delete fields['modelRef'];
    }
    if (typeof fields['model'] === 'string' && fields['model']) assertModelExists(fields['model'] as ModelRef);
    if (mutation['prompt'] !== undefined) fields['prompt'] = mutation['prompt'];
    const next = clone(current);
    const target = storageMap(next, storage);
    Object.assign(target, fields);
    clearFields.forEach((field) => {
      delete target[field];
      if (field === 'model') delete target['modelRef'];
    });
    if (storage === 'inline') next.inline = target;
    else next.markdown = target;
    next.storage = storage;
    next.source = next.inline && next.markdown ? 'both' : next.inline ? 'inline' : 'markdown';
    const effective = next.source === 'both' ? { ...next.inline, ...next.markdown } : target;
    Object.assign(next, effective);
    if (typeof (next as AgentWrite & Record<string, unknown>)['model'] === 'string') {
      next.modelRef = (next as AgentWrite & Record<string, unknown>)['model'] as ModelRef;
      delete (next as AgentWrite & Record<string, unknown>)['model'];
    }
    return publicAgent(next);
  };
  const storageMap = (agent: AgentDefinition, storage: AgentDefinition['storage']) => {
    if (storage === 'inline') return { ...(agent.inline ?? {}) };
    return { ...(agent.markdown ?? {}) };
  };
  const replaceReferences = (from: ModelRef, to: ModelRef) => {
    assertModelExists(from);
    assertModelExists(to);
    modelReferences(from).forEach((reference) => reference.replace(to));
  };
  const providerReferences = (id: string) => [
    ...state.agents.filter((agent) => agent.modelRef?.startsWith(`${id}/`)).map((agent) => `agent.${agent.id}`),
    ...state.groups.flatMap((group) =>
      bindings(group)
        .filter((binding) => binding.modelRef.startsWith(`${id}/`))
        .map(() => `group.${group.name}`),
    ),
  ];
  const agentReferences = (id: string) =>
    state.groups
      .filter((group) => bindings(group).some((binding) => 'agentName' in binding && binding.agentName === id))
      .map((group) => group.name);

  return {
    async loadAppState() {
      return { ...clone(state), agents: state.agents.map(publicAgent) };
    },
    async listProviders() {
      return clone(state.providers);
    },
    async createProvider(value) {
      if (!value.id.trim() || state.providers.some((provider) => provider.id === value.id))
        fail('validation_failed', 'Provider ID is invalid or already exists');
      const provider = { ...clone(value), models: {} };
      state.providers = [...state.providers, provider];
      return clone(provider);
    },
    async updateProvider(value) {
      const index = state.providers.findIndex((provider) => provider.id === value.id);
      if (index < 0) fail('not_found', 'Provider not found');
      const current = state.providers[index] as ProviderDef;
      if (JSON.stringify(current.models) !== JSON.stringify(value.models))
        fail('validation_failed', 'Provider updates cannot change models directly');
      state.providers[index] = clone(value);
      return clone(value);
    },
    async deleteProvider(id) {
      const provider = providerById(id);
      if (Object.keys(provider.models).length) fail('references_blocked', 'Provider still contains models');
      const references = providerReferences(id);
      if (references.length) fail('references_blocked', 'Provider is still referenced', references);
      state.providers = state.providers.filter((candidate) => candidate.id !== id);
    },
    async renameProvider(oldId, newId) {
      const nextId = newId.trim();
      if (!nextId || state.providers.some((provider) => provider.id === nextId))
        fail('validation_failed', 'New provider ID is invalid or already exists');
      const provider = providerById(oldId);
      const replacements = Object.keys(provider.models).map(
        (modelId) => [`${oldId}/${modelId}` as ModelRef, `${nextId}/${modelId}` as ModelRef] as const,
      );
      provider.id = nextId;
      replacements.forEach(([from, to]) => modelReferences(from).forEach((reference) => reference.replace(to)));
    },
    async revealProviderSecret(providerId, key) {
      const value = providerById(providerId).options?.[key];
      if (value === undefined) fail('not_found', 'Provider option not found');
      if (!sensitiveOption(key)) fail('validation_failed', 'Only sensitive provider options can be revealed');
      return clone(value);
    },
    async listModels() {
      return clone(state.providers);
    },
    async createModel(providerId, value) {
      const provider = providerById(providerId);
      if (!value.id.trim() || provider.models[value.id])
        fail('validation_failed', 'Model ID is invalid or already exists');
      provider.models[value.id] = clone(value);
      return clone(value);
    },
    async updateModel(providerId, value) {
      const provider = providerById(providerId);
      if (!provider.models[value.id]) fail('not_found', 'Model not found');
      provider.models[value.id] = clone(value);
      return clone(value);
    },
    async deleteModel(ref) {
      const { provider, model } = modelByRef(ref);
      const references = modelReferences(ref);
      if (references.length)
        fail(
          'references_blocked',
          'Model is still referenced',
          references.map((reference) => reference.owner),
        );
      delete provider.models[model.id];
    },
    async renameModel(ref, newId) {
      const { provider, model } = modelByRef(ref);
      const nextId = newId.trim();
      if (!nextId || provider.models[nextId]) fail('validation_failed', 'New model ID is invalid or already exists');
      const nextRef = `${provider.id}/${nextId}` as ModelRef;
      delete provider.models[model.id];
      provider.models[nextId] = { ...model, id: nextId };
      modelReferences(ref).forEach((reference) => reference.replace(nextRef));
    },
    async replaceModelReferences(from, to) {
      replaceReferences(from, to);
    },
    async listAgents() {
      return state.agents.map(publicAgent);
    },
    async getAgent(id) {
      return publicAgent(agentById(id));
    },
    async createAgent(value) {
      if (!value.id.trim() || state.agents.some((agent) => agent.id === value.id))
        fail('validation_failed', 'Agent ID is invalid or already exists');
      if (value.modelRef) assertModelExists(value.modelRef);
      const agent = (value as AgentWrite).mutation
        ? applyAgentMutation({ id: value.id, source: value.source, storage: value.storage }, value as AgentWrite)
        : publicAgent(clone(value));
      state.agents = [...state.agents, agent];
      return publicAgent(agent);
    },
    async updateAgent(value) {
      if (conflictOn.has('update_agent')) fail('conflict', 'Configuration changed externally.');
      const current = agentById(value.id);
      const next = (value as AgentWrite).mutation
        ? applyAgentMutation(current, value as AgentWrite)
        : publicAgent(clone(value));
      if (next.modelRef) assertModelExists(next.modelRef);
      state.agents = state.agents.map((agent) => (agent.id === value.id ? next : agent));
      return publicAgent(next);
    },
    async deleteAgent(id, storage) {
      const agent = agentById(id);
      if (['build', 'plan', 'general', 'explore'].includes(id))
        fail('references_blocked', 'Built-in agents cannot be deleted');
      const references = agentReferences(id);
      if (references.length) fail('references_blocked', 'Agent is still referenced by a group', references);
      if (agent.source === 'both') {
        if (!storage) fail('validation_failed', 'Deleting a dual-source agent requires choosing a source');
        if (storage === 'inline')
          state.agents = state.agents.map((candidate) =>
            candidate.id === id
              ? { ...candidate, source: 'markdown', storage: 'global_markdown', inline: undefined }
              : candidate,
          );
        else
          state.agents = state.agents.map((candidate) =>
            candidate.id === id
              ? { ...candidate, source: 'inline', storage: 'inline', markdown: undefined }
              : candidate,
          );
        return;
      }
      state.agents = state.agents.filter((candidate) => candidate.id !== id);
    },
    async saveGroup(value) {
      if (conflictOn.has('save_group')) fail('conflict', 'Configuration changed externally.');
      assertGroup(value);
      if (
        state.groups.some(
          (group) => group.id !== value.id && group.name.trim().toLowerCase() === value.name.trim().toLowerCase(),
        )
      )
        fail('validation_failed', 'Group name already exists');
      const group = { ...clone(value), updatedAt: new Date().toISOString() };
      state.groups = state.groups.some((candidate) => candidate.id === group.id)
        ? state.groups.map((candidate) => (candidate.id === group.id ? group : candidate))
        : [...state.groups, group];
      return clone(group);
    },
    async copyGroup(id, name) {
      const group = groupById(id);
      if (
        !name.trim() ||
        state.groups.some((candidate) => candidate.name.trim().toLowerCase() === name.trim().toLowerCase())
      )
        fail('validation_failed', 'New group name is invalid or already exists');
      if (!globalThis.crypto?.randomUUID)
        fail('configuration_failed', 'This environment does not support UUID generation; cannot copy the group');
      const copy = {
        ...clone(group),
        id: globalThis.crypto.randomUUID(),
        name: name.trim(),
        updatedAt: new Date().toISOString(),
      };
      state.groups = [...state.groups, copy];
      return clone(copy);
    },
    async deleteGroup(id) {
      groupById(id);
      state.groups = state.groups.filter((group) => group.id !== id);
    },
    async switchGroup(id) {
      groupById(id);
      state.groups = state.groups.map((group) => ({ ...group, isEnabled: group.id === id }));
    },
    async loadConfigDiagnostics() {
      return clone(state.diagnostics ?? []);
    },
    async validateConfig() {
      state.groups.forEach(assertGroup);
      return clone(state.diagnostics ?? []);
    },
  };
}

export type TauriInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
export function createTauriAdapter(invoke: TauriInvoke): CommandAdapter {
  return {
    loadAppState: () => invoke('load_app_state'),
    listProviders: () => invoke('list_providers'),
    createProvider: (value) => invoke('create_provider', { provider: value }),
    updateProvider: (value) => invoke('update_provider', { provider: value }),
    deleteProvider: (id) => invoke('delete_provider', { id }),
    renameProvider: (oldId, newId) => invoke('rename_provider', { oldId, newId }),
    revealProviderSecret: (providerId, key) => invoke('reveal_provider_option', { providerId, key }),
    listModels: () => invoke('list_models'),
    createModel: (providerId, value) => invoke('create_model', { providerId, model: value }),
    updateModel: (providerId, value) => invoke('update_model', { providerId, model: value }),
    deleteModel: (modelRef) => invoke('delete_model', { modelRef }),
    renameModel: (modelRef, newId) => invoke('rename_model', { from: modelRef, newModelId: newId }),
    replaceModelReferences: (from, to) => invoke('replace_model_references', { from, to }),
    listAgents: () => invoke('list_agents'),
    getAgent: (id) => invoke('get_agent', { id }),
    createAgent: (value) => invoke('create_agent', { agent: value }),
    updateAgent: (value) => invoke('update_agent', { agent: value }),
    deleteAgent: (id, storage) => invoke('delete_agent', { id, storage }),
    saveGroup: (value) => invoke('save_group', { group: value }),
    copyGroup: (id, name) => invoke('copy_group', { id, name }),
    deleteGroup: (id) => invoke('delete_group', { id }),
    switchGroup: (id) => invoke('switch_group', { id }),
    loadConfigDiagnostics: () => invoke('load_config_diagnostics'),
    validateConfig: () => invoke('validate_config'),
  };
}
export function createCommandAdapter(): CommandAdapter {
  if (typeof window === 'undefined' || !(window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__)
    return createBrowserMockAdapter();
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
