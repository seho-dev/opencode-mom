import assert from 'node:assert/strict';
import test from 'node:test';
import { createServer } from 'vite';

test('resource facades stay lazy and MCP deletion does not refresh the global catalog', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const calls = [];
    let failure;
    const list = { data: [], diagnostics: ['one unavailable source'] };
    const lid = { enabled: false, phase: 'disabled' };
    const adapter = createTauriAdapter(async (command, args) => {
      calls.push([command, args]);
      if (command === 'load_app_state')
        return { providers: [], agents: [], groups: [], preferences: { theme: 'dark', locale: 'en' } };
      if (command === 'opencode_list_models') return [];
      if (failure) throw failure;
      if (command === 'get_lid_protection' || command === 'set_lid_protection') return lid;
      if (command === 'get_autostart' || command === 'set_autostart') return false;
      return list;
    });
    const store = createConfigStore(adapter);
    await new Promise(setImmediate);
    assert.deepEqual(
      calls.map(([command]) => command),
      ['opencode_list_models', 'load_app_state'],
    );
    calls.length = 0;
    assert.strictEqual(await store.listMcps(), list);
    assert.strictEqual(await store.getMcp('test'), list);
    assert.strictEqual(await store.listSkills(), list);
    assert.strictEqual(await store.getSkill('Test'), list);
    assert.equal(await store.getAutostart(), false);
    assert.equal(await store.setAutostart(true), false);
    assert.strictEqual(await store.getLidProtection(), lid);
    assert.strictEqual(await store.setLidProtection(true), lid);
    assert.strictEqual(await store.setLidProtection(false), lid);
    assert.equal(store.notice, null);
    await store.deleteMcp('test');
    assert.equal(store.notice, 'deleteMcp');
    assert.equal(store.saving, false);
    assert.deepEqual(
      calls.map(([command]) => command),
      [
        'list_mcps',
        'get_mcp',
        'list_skills',
        'get_skill',
        'get_autostart',
        'set_autostart',
        'get_lid_protection',
        'set_lid_protection',
        'set_lid_protection',
        'delete_mcp',
      ],
    );
    failure = { code: 'configuration_failed', message: 'OS registration unavailable' };
    store.clearNotice();
    await assert.rejects(store.setAutostart(true), failure);
    await assert.rejects(store.getLidProtection(), failure);
    await assert.rejects(store.setLidProtection(false), failure);
    await assert.rejects(store.listSkills(), failure);
    assert.equal(store.error, null);
    assert.equal(store.draftRecovery, null);
    assert.equal(store.notice, null);
    await assert.rejects(store.deleteMcp('test'), failure);
    assert.equal(store.error?.message, failure.message);
    assert.deepEqual(store.draftRecovery?.payload, { name: 'test' });
  } finally {
    await vite.close();
  }
});

test('resource writes return saved entries without refresh and never retain secret drafts in recovery', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const calls = [];
    let failure;
    const saved = { id: 'Saved', kind: 'local', content: 'saved content' };
    const store = createConfigStore(
      createTauriAdapter(async (command, args) => {
        calls.push([command, args]);
        if (command === 'load_app_state')
          return { providers: [], agents: [], groups: [], preferences: { theme: 'dark', locale: 'en' } };
        if (command === 'opencode_list_models') return [];
        if (failure) throw failure;
        return saved;
      }),
    );
    await new Promise(setImmediate);
    calls.length = 0;
    const mcpDraft = {
      name: 'test',
      config: { type: 'local', command: ['run'], environment: { TOKEN: 'real-secret' } },
    };
    const mcpUpdate = { ...mcpDraft, expectedConfig: mcpDraft.config, expectedSourcePath: '/managed/config' };
    const skillDraft = { id: 'Saved', content: 'secret full Markdown' };
    const skillUpdate = { ...skillDraft, expectedContent: 'secret old Markdown', expectedPath: '/managed/SKILL.md' };
    for (const [method, draft] of [
      ['createMcp', mcpDraft],
      ['updateMcp', mcpUpdate],
      ['createSkill', skillDraft],
      ['updateSkill', skillUpdate],
    ]) {
      assert.strictEqual(await store[method](draft), saved);
      assert.equal(store.notice, method);
      assert.equal(store.saving, false);
    }
    assert.deepEqual(calls, [
      ['create_mcp', { draft: mcpDraft }],
      ['update_mcp', { draft: mcpUpdate }],
      ['create_skill', { draft: skillDraft }],
      ['update_skill', { draft: skillUpdate }],
    ]);
    failure = { code: 'configuration_failed', message: 'real-secret secret full Markdown', detail: mcpDraft };
    for (const [method, draft] of [
      ['createMcp', mcpDraft],
      ['updateMcp', mcpUpdate],
      ['createSkill', skillDraft],
      ['updateSkill', skillUpdate],
    ]) {
      await assert.rejects(store[method](draft), {
        code: 'configuration_failed',
        message: 'Configuration operation failed. Review the draft in the original form and retry.',
      });
      assert.equal(store.draftRecovery?.payload, null);
      assert.equal(store.error?.detail, undefined);
      assert.ok(!JSON.stringify(store.draftRecovery).includes('secret'));
      assert.equal(store.saving, false);
    }
  } finally {
    await vite.close();
  }
});

test('manual update and project facades stay lazy and do not touch config loading, refresh, or recovery', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const calls = [];
    const info = { version: '0.1.0', platform: 'windows' };
    const update = { currentVersion: '0.1.0', latestVersion: null, updateAvailable: false };
    let failure;
    const store = createConfigStore(
      createTauriAdapter(async (...args) => {
        calls.push(args);
        if (args[0] === 'load_app_state')
          return { providers: [], agents: [], groups: [], preferences: { theme: 'dark', locale: 'en' } };
        if (args[0] === 'opencode_list_models') return [];
        if (failure) throw failure;
        if (args[0] === 'get_app_info') return info;
        if (args[0] === 'check_for_updates') return update;
      }),
    );
    await new Promise(setImmediate);
    assert.deepEqual(calls, [['opencode_list_models', { provider: undefined }], ['load_app_state']]);
    calls.length = 0;
    assert.strictEqual(await store.getAppInfo(), info);
    assert.strictEqual(await store.checkForUpdates(), update);
    assert.equal(await store.openProjectPage('repository'), undefined);
    assert.equal(await store.openProjectPage('releases'), undefined);
    assert.deepEqual(calls, [
      ['get_app_info'],
      ['check_for_updates'],
      ['open_project_page', { page: 'repository' }],
      ['open_project_page', { page: 'releases' }],
    ]);
    failure = { code: 'configuration_failed', message: 'Update check unavailable' };
    await assert.rejects(store.getAppInfo(), failure);
    await assert.rejects(store.checkForUpdates(), failure);
    await assert.rejects(store.openProjectPage('releases'), failure);
    assert.equal(calls.length, 7);
    assert.equal(store.loading, false);
    assert.equal(store.saving, false);
    assert.equal(store.error, null);
    assert.equal(store.notice, null);
    assert.equal(store.draftRecovery, null);
  } finally {
    await vite.close();
  }
});

test('OpenCode installation/update facades stay lazy and never use global mutations or refresh', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const calls = [];
    const installation = {
      binaryPath: '/exact/bin/opencode',
      version: '2.0.0',
      method: 'npm',
      canUpgrade: true,
      upgradeReason: null,
      manualCommand: null,
    };
    const update = { installation, latestVersion: '2.1.0', updateAvailable: true };
    let failure;
    const store = createConfigStore(
      createTauriAdapter(async (...args) => {
        calls.push(args);
        if (failure) throw failure;
        return args[0] === 'opencode_check_update' ? update : installation;
      }),
      false,
      false,
    );
    assert.deepEqual(calls, []);
    assert.strictEqual(await store.getOpenCodeInstallation(), installation);
    assert.strictEqual(await store.checkOpenCodeUpdate(), update);
    assert.strictEqual(await store.upgradeOpenCode('/exact/bin/opencode', '2.0.0', '2.1.0'), installation);
    assert.deepEqual(calls, [
      ['opencode_get_installation'],
      ['opencode_check_update'],
      [
        'opencode_upgrade',
        { expectedBinaryPath: '/exact/bin/opencode', expectedVersion: '2.0.0', targetVersion: '2.1.0' },
      ],
    ]);
    failure = { code: 'configuration_failed', message: 'Upgrade unavailable' };
    await assert.rejects(store.getOpenCodeInstallation(), failure);
    await assert.rejects(store.checkOpenCodeUpdate(), failure);
    await assert.rejects(store.upgradeOpenCode('/exact/bin/opencode', '2.0.0', '2.1.0'), failure);
    assert.equal(store.loading, false);
    assert.equal(store.saving, false);
    assert.equal(store.error, null);
    assert.equal(store.notice, null);
    assert.equal(store.draftRecovery, null);
    assert.equal(calls.length, 6);
  } finally {
    await vite.close();
  }
});

test('service facades stay lazy; failed writes retain no secrets and successful saves invalidate only target data', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const calls = [];
    const status = {
      target: { mode: 'explicit', url: 'http://127.0.0.1:4096', hasPassword: true },
      connected: false,
      url: null,
      pid: null,
      version: null,
      error: null,
    };
    const catalog = [{ ref: 'provider/old-target' }];
    let failure;
    const store = createConfigStore(
      createTauriAdapter(async (...args) => {
        calls.push(args);
        if (failure) throw failure;
        return args[0] === 'opencode_list_models' ? catalog : status;
      }),
      false,
      false,
    );
    assert.deepEqual(calls, []);
    assert.strictEqual(await store.getOpenCodeServiceStatus(), status);
    await store.loadCatalog();
    const resetVersion = store.formResetVersion;
    assert.equal(store.serviceTargetVersion, 0);
    assert.equal(Object.getOwnPropertyDescriptor(store, 'serviceTargetVersion').set, undefined);
    const preferences = store.preferences;
    failure = { code: 'configuration_failed', message: 'local-secret', detail: { password: 'local-secret' } };
    await assert.rejects(store.setOpenCodeServiceTarget(status.target.url, 'local-secret'), failure);
    assert.equal(store.serviceTargetVersion, 0);
    assert.deepEqual(store.catalog, catalog);
    for (const key of ['error', 'draftRecovery', 'notice']) assert.equal(store[key], null);
    assert.equal(store.saving, false);
    failure = undefined;
    assert.strictEqual(await store.setOpenCodeServiceTarget(status.target.url, 'local-secret'), status);
    assert.equal(store.serviceTargetVersion, 1);
    assert.deepEqual(store.catalog, []);
    assert.equal(store.catalogError, null);
    assert.equal(store.catalogLoading, false);
    assert.equal(store.formResetVersion, resetVersion);
    assert.strictEqual(store.preferences, preferences);
    assert.deepEqual(
      calls.map(([command]) => command),
      [
        'opencode_get_service_status',
        'opencode_list_models',
        'opencode_set_service_target',
        'opencode_set_service_target',
      ],
    );
  } finally {
    await vite.close();
  }
});

test('catalog and usage replies from an old service target cannot succeed or repopulate caches', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const deferred = () => {
      let resolve;
      const promise = new Promise((done) => {
        resolve = done;
      });
      return { promise, resolve };
    };
    const catalogRead = deferred();
    const usageRead = deferred();
    const save = deferred();
    const calls = [];
    const store = createConfigStore(
      createTauriAdapter(async (command) => {
        calls.push(command);
        if (command === 'opencode_list_models') return catalogRead.promise;
        if (command === 'opencode_token_usage_records') return usageRead.promise;
        if (command === 'opencode_set_service_target') return save.promise;
      }),
      false,
      false,
    );
    const catalogResult = assert.rejects(store.loadCatalog(), { code: 'conflict' });
    const usageResult = assert.rejects(store.loadTokenUsageRecords(), { code: 'conflict' });
    const targetSave = store.setOpenCodeServiceTarget('http://127.0.0.1:5096');
    assert.equal(store.serviceTargetVersion, 0);
    await assert.rejects(store.loadTokenUsageRecords(), { code: 'conflict' });
    await assert.rejects(store.loadCatalog(), { code: 'conflict' });
    save.resolve({ target: { mode: 'explicit', url: 'http://127.0.0.1:5096', hasPassword: false } });
    await targetSave;
    assert.equal(store.serviceTargetVersion, 1);
    usageRead.resolve([{ time: 1, input: 1, output: 1 }]);
    await usageResult;
    catalogRead.resolve([{ ref: 'provider/old-target' }]);
    await catalogResult;
    assert.deepEqual(store.catalog, []);
    assert.equal(store.catalogError, null);
    assert.equal(store.catalogLoading, false);
    assert.deepEqual(calls, ['opencode_list_models', 'opencode_token_usage_records', 'opencode_set_service_target']);
  } finally {
    await vite.close();
  }
});

test('a failed pending target save cannot poison the catalog or unrelated state and allows fresh reads afterward', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    let resolveCatalog;
    let rejectSave;
    const pendingCatalog = new Promise((done) => {
      resolveCatalog = done;
    });
    const pendingSave = new Promise((_, fail) => {
      rejectSave = fail;
    });
    const catalog = [{ ref: 'provider/current-target' }];
    const state = {
      providers: [{ name: 'provider', models: {} }],
      agents: [{ id: 'agent' }],
      groups: [],
      preferences: { locale: 'zh', theme: 'light' },
    };
    const records = [{ time: 1, input: 1, output: 1 }];
    let catalogCalls = 0;
    const store = createConfigStore(
      createTauriAdapter(async (command) => {
        if (command === 'load_app_state') return state;
        if (command === 'opencode_list_models') return ++catalogCalls === 2 ? pendingCatalog : catalog;
        if (command === 'opencode_set_service_target') return pendingSave;
        if (command === 'opencode_token_usage_records') return records;
        if (command === 'save_group') throw { code: 'conflict', message: 'Keep the original group draft.' };
      }),
      false,
      false,
    );
    await store.refresh(true);
    await store.loadCatalog();
    await assert.rejects(store.saveGroup({ id: 'draft', type: 'native' }), { code: 'conflict' });
    const original = {
      error: store.error,
      draftRecovery: store.draftRecovery,
      preferences: store.preferences,
      formResetVersion: store.formResetVersion,
      serviceTargetVersion: store.serviceTargetVersion,
      providers: store.providers,
      agents: store.agents,
      groups: store.groups,
    };
    const readResult = assert.rejects(store.loadCatalog(), { code: 'conflict' });
    const saveResult = assert.rejects(store.setOpenCodeServiceTarget('http://localhost:5096', 'local-secret'), {
      code: 'validation_failed',
    });
    resolveCatalog([{ ref: 'provider/unconfirmed-target' }]);
    await readResult;
    assert.deepEqual(store.catalog, catalog);
    assert.equal(store.catalogError, null);
    assert.equal(store.catalogLoading, false);
    rejectSave({ code: 'validation_failed', message: 'local-secret', detail: { password: 'local-secret' } });
    await saveResult;
    for (const [key, value] of Object.entries(original)) assert.strictEqual(store[key], value);
    assert.equal(store.saving, false);
    assert.ok(!JSON.stringify(store.draftRecovery).includes('local-secret'));
    assert.deepEqual(await store.loadCatalog(), catalog);
    assert.deepEqual(await store.loadTokenUsageRecords(), records);
  } finally {
    await vite.close();
  }
});
