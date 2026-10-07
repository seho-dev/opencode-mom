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
