import assert from 'node:assert/strict';
import test from 'node:test';
import { createServer } from 'vite';

const first = { id: '11111111-1111-4111-8111-111111111111', path: '/projects/first' };
const second = { id: '22222222-2222-4222-8222-222222222222', path: '/projects/second' };
const state = (extra = {}) => ({
  providers: [],
  agents: [{ id: 'global-agent', source: 'custom' }],
  groups: [],
  preferences: { theme: 'dark', locale: 'en' },
  ...extra,
});
function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

test('project management preserves IPC commands, payloads, cancellation, results, and errors', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const calls = [];
    let picked = null;
    let failure;
    const adapter = createTauriAdapter(async (...args) => {
      calls.push(args);
      if (failure) throw failure;
      if (args[0] === 'pick_project_folder') return picked;
      if (args[0] === 'add_project') return first;
    });
    assert.equal(await adapter.pickProjectFolder(), null);
    picked = first.path;
    assert.equal(await adapter.pickProjectFolder(), first.path);
    assert.strictEqual(await adapter.addProject(first.path), first);
    await adapter.selectProject(first.id);
    await adapter.selectProject(null);
    await adapter.removeProject(first.id);
    assert.deepEqual(calls, [
      ['pick_project_folder'],
      ['pick_project_folder'],
      ['add_project', { path: first.path }],
      ['select_project', { id: first.id }],
      ['select_project', { id: null }],
      ['remove_project', { id: first.id }],
    ]);
    failure = { code: 'configuration_failed', message: 'Project persistence failed' };
    await assert.rejects(adapter.pickProjectFolder(), failure);
    await assert.rejects(adapter.addProject(first.path), failure);
    await assert.rejects(adapter.selectProject(first.id), failure);
    await assert.rejects(adapter.removeProject(first.id), failure);
  } finally {
    await vite.close();
  }
});

test('resource IPC omits absent or null global scope and sends only explicit project IDs', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const calls = [];
    const result = {};
    const adapter = createTauriAdapter(async (...args) => {
      calls.push(args);
      return result;
    });
    const agent = { id: 'custom', source: 'custom' };
    const mcp = { name: 'server', config: {} };
    const skill = { id: 'skill', content: 'content' };
    const operations = [
      ['listAgents', 'list_agents', [], undefined],
      ['getAgent', 'get_agent', ['custom'], { id: 'custom' }],
      ['createAgent', 'create_agent', [agent], { agent }],
      ['updateAgent', 'update_agent', [agent], { agent }],
      ['deleteAgent', 'delete_agent', ['custom', 'global_markdown'], { id: 'custom', storage: 'global_markdown' }],
      ['listMcps', 'list_mcps', [], undefined],
      ['getMcp', 'get_mcp', ['server'], { name: 'server' }],
      ['createMcp', 'create_mcp', [mcp], { draft: mcp }],
      ['updateMcp', 'update_mcp', [mcp], { draft: mcp }],
      ['deleteMcp', 'delete_mcp', ['server'], { name: 'server' }],
      ['listSkills', 'list_skills', [], undefined],
      ['getSkill', 'get_skill', ['skill'], { id: 'skill' }],
      ['createSkill', 'create_skill', [skill], { draft: skill }],
      ['updateSkill', 'update_skill', [skill], { draft: skill }],
    ];
    for (const [method, command, args, payload] of operations) {
      assert.strictEqual(await adapter[method](...args), result);
      assert.deepEqual(calls.at(-1), payload ? [command, payload] : [command]);
      assert.strictEqual(await adapter[method](...args, null), result);
      assert.deepEqual(calls.at(-1), payload ? [command, payload] : [command]);
      assert.strictEqual(await adapter[method](...args, first.id), result);
      assert.deepEqual(calls.at(-1), [command, { ...payload, projectId: first.id }]);
    }
  } finally {
    await vite.close();
  }
});

test('project initialization accepts old snapshots and normalizes stale persisted selection', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    let loaded = state();
    const store = createConfigStore({ loadAppState: async () => loaded }, false);
    assert.deepEqual(store.projects, []);
    assert.equal(store.selectedProjectId, null);
    assert.equal(store.projectSaving, false);
    await new Promise(setImmediate);
    assert.deepEqual(store.projects, []);
    assert.equal(store.selectedProjectId, null);
    loaded = state({ projects: [first, second], selectedProjectId: second.id });
    await store.refresh();
    assert.deepEqual(store.projects, [first, second]);
    assert.equal(store.selectedProjectId, second.id);
    loaded = state({ projects: [first], selectedProjectId: second.id });
    await store.refresh();
    assert.deepEqual(store.projects, [first]);
    assert.equal(store.selectedProjectId, null);
  } finally {
    await vite.close();
  }
});

test('project operations are lazy, deduplicate returned IDs and paths, and remove selected scope to Global', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const calls = [];
    let picked = null;
    let saved = first;
    const store = createConfigStore(
      createTauriAdapter(async (...args) => {
        calls.push(args);
        if (args[0] === 'load_app_state') return state({ projects: [first, second], selectedProjectId: second.id });
        if (args[0] === 'opencode_list_models') return [];
        if (args[0] === 'pick_project_folder') return picked;
        if (args[0] === 'add_project') return saved;
      }),
    );
    await new Promise(setImmediate);
    calls.length = 0;
    const resetVersion = store.formResetVersion;
    assert.equal(await store.pickProjectFolder(), null);
    assert.deepEqual(store.projects, [first, second]);
    assert.equal(store.selectedProjectId, second.id);
    picked = '/picked/alias';
    assert.equal(await store.pickProjectFolder(), picked);
    assert.equal(store.selectedProjectId, second.id);
    assert.strictEqual(await store.addProject(picked), first);
    assert.deepEqual(store.projects, [second, first]);
    assert.equal(store.selectedProjectId, first.id);
    await store.addProject(first.path);
    assert.deepEqual(store.projects, [second, first]);
    saved = { ...first, id: '33333333-3333-4333-8333-333333333333' };
    await store.addProject(first.path);
    assert.deepEqual(store.projects, [second, saved]);
    await store.removeProject(second.id);
    assert.deepEqual(store.projects, [saved]);
    assert.equal(store.selectedProjectId, saved.id);
    await store.selectProject(null);
    assert.equal(store.selectedProjectId, null);
    await store.selectProject(saved.id);
    assert.equal(store.selectedProjectId, saved.id);
    await store.removeProject(saved.id);
    assert.deepEqual(store.projects, []);
    assert.equal(store.selectedProjectId, null);
    assert.deepEqual(calls, [
      ['pick_project_folder'],
      ['pick_project_folder'],
      ['add_project', { path: picked }],
      ['add_project', { path: first.path }],
      ['add_project', { path: first.path }],
      ['remove_project', { id: second.id }],
      ['select_project', { id: null }],
      ['select_project', { id: saved.id }],
      ['remove_project', { id: saved.id }],
    ]);
    assert.equal(store.loading, false);
    assert.equal(store.saving, false);
    assert.equal(store.projectSaving, false);
    assert.equal(store.error, null);
    assert.equal(store.notice, null);
    assert.equal(store.draftRecovery, null);
    assert.equal(store.formResetVersion, resetVersion);
  } finally {
    await vite.close();
  }
});

test('project persistence is guarded independently and failures preserve selection without path recovery', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    const pending = deferred();
    const failure = { code: 'configuration_failed', message: 'Project persistence failed' };
    const groupFailure = { code: 'conflict', message: 'Keep the existing Group draft' };
    let selectCalls = 0;
    const store = createConfigStore(
      {
        loadAppState: async () => state({ projects: [first, second], selectedProjectId: first.id }),
        saveGroup: async () => {
          throw groupFailure;
        },
        pickProjectFolder: async () => {
          throw failure;
        },
        addProject: async () => {
          throw failure;
        },
        removeProject: async () => {
          throw failure;
        },
        selectProject: () => {
          selectCalls += 1;
          return pending.promise;
        },
      },
      false,
    );
    await new Promise(setImmediate);
    await assert.rejects(store.saveGroup({ id: 'draft' }), groupFailure);
    const recovery = store.draftRecovery;
    const existingError = store.error;
    await assert.rejects(store.selectProject('not-saved'), { code: 'not_found' });
    assert.equal(selectCalls, 0);
    const selecting = store.selectProject(second.id);
    assert.equal(store.projectSaving, true);
    assert.equal(store.saving, false);
    assert.equal(store.selectedProjectId, first.id);
    await assert.rejects(store.addProject('/private/path'), { code: 'busy' });
    await assert.rejects(store.selectProject(null), { code: 'busy' });
    assert.equal(store.projectSaving, true);
    pending.reject(failure);
    await assert.rejects(selecting, failure);
    await assert.rejects(store.pickProjectFolder(), failure);
    await assert.rejects(store.addProject('/private/path'), failure);
    await assert.rejects(store.removeProject(first.id), failure);
    assert.deepEqual(store.projects, [first, second]);
    assert.equal(store.selectedProjectId, first.id);
    assert.equal(store.projectSaving, false);
    assert.strictEqual(store.error, existingError);
    assert.equal(store.notice, null);
    assert.strictEqual(store.draftRecovery, recovery);
  } finally {
    await vite.close();
  }
});

test('stale global snapshots cannot undo persisted project operations and fresh Group snapshots preserve scope', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    let persisted = state({ projects: [first, second], selectedProjectId: first.id });
    let pending;
    const store = createConfigStore(
      {
        loadAppState: () => pending?.promise ?? Promise.resolve(persisted),
        addProject: async () => {
          persisted = state({ projects: [first, second], selectedProjectId: second.id });
          return second;
        },
        selectProject: async (id) => {
          persisted = { ...persisted, selectedProjectId: id };
        },
        removeProject: async (id) => {
          persisted = {
            ...persisted,
            projects: persisted.projects.filter((project) => project.id !== id),
            selectedProjectId: null,
          };
        },
        saveGroup: async () => {},
      },
      false,
    );
    await new Promise(setImmediate);
    for (const action of [
      () => store.selectProject(second.id),
      () => store.addProject(second.path),
      () => store.removeProject(second.id),
    ]) {
      const old = persisted;
      pending = deferred();
      const refreshing = store.refresh();
      await action();
      pending.resolve({ ...old, groups: [{ id: 'still-apply-global-data' }] });
      await refreshing;
      assert.deepEqual(store.projects, persisted.projects);
      assert.equal(store.selectedProjectId, persisted.selectedProjectId);
      assert.equal(store.groups[0].id, 'still-apply-global-data');
      pending = null;
    }
    await store.selectProject(first.id);
    await store.saveGroup({ id: 'global-group' });
    assert.deepEqual(store.projects, [first]);
    assert.equal(store.selectedProjectId, first.id);
    assert.equal(store.error, null);
  } finally {
    await vite.close();
  }
});

test('explicit-scope resource facades never replace global agents and omitted Global writes ignore shared selection', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const calls = [];
    const result = { data: [{ id: 'project-only' }], diagnostics: [] };
    const store = createConfigStore(
      createTauriAdapter(async (...args) => {
        calls.push(args);
        if (args[0] === 'load_app_state') return state({ projects: [first], selectedProjectId: first.id });
        if (args[0] === 'opencode_list_models') return [];
        return result;
      }),
    );
    await new Promise(setImmediate);
    calls.length = 0;
    for (const [method, args] of [
      ['listAgents', []],
      ['getAgent', ['project-only']],
      ['listMcps', []],
      ['getMcp', ['server']],
      ['listSkills', []],
      ['getSkill', ['skill']],
    ]) {
      assert.strictEqual(await store[method](...args, first.id), result);
      assert.equal(calls.at(-1)[1].projectId, first.id);
      assert.strictEqual(await store[method](...args), result);
      assert.equal(calls.at(-1)[1]?.projectId, undefined);
    }
    assert.deepEqual(store.agents, state().agents);
    const agent = { id: 'global-only', source: 'custom' };
    await store.createAgent(agent);
    await store.updateAgent(agent, null);
    await store.deleteAgent(agent.id);
    const mcp = { name: 'global-server', config: {} };
    await store.createMcp(mcp);
    await store.updateMcp(mcp, null);
    await store.deleteMcp(mcp.name);
    const skill = { id: 'global-skill', content: 'body' };
    await store.createSkill(skill);
    await store.updateSkill(skill, null);
    const writes = calls.filter(([command]) => /^(create|update|delete)_/.test(command));
    assert.equal(writes.length, 8);
    assert.ok(writes.every(([, payload]) => !('projectId' in payload)));
    assert.equal(store.selectedProjectId, first.id);
    assert.deepEqual(store.agents, state().agents);
  } finally {
    await vite.close();
  }
});
