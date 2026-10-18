import assert from 'node:assert/strict';
import test from 'node:test';
import { createServer } from 'vite';

const group = (id, type) => ({
  id,
  name: id,
  description: '',
  type,
  openCodeAgentOverrides: [],
  slimAgentOverrides: null,
  omoAgentOverrides: null,
  omoCategoryMappings: null,
  isEnabled: false,
  updatedAt: '',
});

test('all target-bound Island IPCs forward the caller generation unchanged and never retry conflicts', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const calls = [];
    let failure;
    const result = [];
    const adapter = createTauriAdapter(async (command, args) => {
      calls.push([command, args]);
      if (failure) throw failure;
      return result;
    });
    const answer = { field: 'choice' };
    const cases = [
      ['islandMessages', ['ses_same'], 'island_messages', { sessionId: 'ses_same' }],
      [
        'islandSend',
        ['ses_same', 'text', 'msg_same'],
        'island_send',
        { sessionId: 'ses_same', text: 'text', messageId: 'msg_same' },
      ],
      [
        'islandReplyPermission',
        ['ses_same', 'request_same', 'once'],
        'island_reply_permission',
        { sessionId: 'ses_same', requestId: 'request_same', decision: 'once' },
      ],
      [
        'islandReplyQuestion',
        ['ses_same', 'request_same', answer],
        'island_reply_question',
        { sessionId: 'ses_same', formId: 'request_same', answer },
      ],
      [
        'islandCancelQuestion',
        ['ses_same', 'request_same'],
        'island_cancel_question',
        { sessionId: 'ses_same', formId: 'request_same' },
      ],
      ['islandInterrupt', ['ses_same'], 'island_interrupt', { sessionId: 'ses_same' }],
    ];
    for (const generation of [0, 17]) {
      for (const [method, args, command, payload] of cases) {
        assert.strictEqual(await adapter[method](...args, generation), result);
        assert.deepEqual(calls.at(-1), [command, { ...payload, expectedConnectionGeneration: generation }]);
      }
    }
    failure = { code: 'conflict', message: 'Connection generation changed' };
    for (const [method, args, command, payload] of cases) {
      const before = calls.length;
      await assert.rejects(adapter[method](...args, 17), (cause) => cause === failure);
      assert.equal(calls.length, before + 1);
      assert.deepEqual(calls.at(-1), [command, { ...payload, expectedConnectionGeneration: 17 }]);
    }
  } finally {
    await vite.close();
  }
});

test('group IPC reads wire types and writes canonical types for save and copy', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const calls = [];
    const saved = [];
    const adapter = createTauriAdapter(async (command, args) => {
      calls.push([command, args]);
      if (command === 'load_app_state')
        return {
          providers: [],
          agents: [],
          groups: [
            group('native', 'opencode'),
            group('slim', 'slim'),
            group('omo', 'oh-my-openagent'),
            group('legacy-native', 'native'),
            group('legacy-omo', 'omo'),
            group('unknown', 'unknown-type'),
          ],
          preferences: { theme: 'dark', locale: 'en' },
        };
      if (command === 'save_group') {
        saved.push(args.group);
        return args.group;
      }
      if (command === 'copy_group') return group('copy', 'oh-my-openagent');
      throw new Error(`Unexpected command: ${command}`);
    });

    const state = await adapter.loadAppState();
    assert.deepEqual(
      state.groups.map(({ type }) => type),
      ['native', 'slim', 'omo', 'native', 'omo', 'native'],
    );
    for (const [type, wire] of [
      ['native', 'opencode'],
      ['slim', 'slim'],
      ['omo', 'oh-my-openagent'],
    ]) {
      const input = group(type, type);
      assert.deepEqual(await adapter.saveGroup(input), input);
      assert.equal(input.type, type);
      assert.deepEqual(saved.at(-1), { ...input, type: wire });
    }
    assert.deepEqual(await adapter.copyGroup('omo', 'Copy'), group('copy', 'omo'));
    assert.deepEqual(calls.at(-1), ['copy_group', { id: 'omo', name: 'Copy' }]);
  } finally {
    await vite.close();
  }
});

test('deleteAgent passes storage through IPC when provided', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const calls = [];
    const adapter = createTauriAdapter(async (command, args) => {
      calls.push([command, args]);
    });
    await adapter.deleteAgent('custom', 'global_markdown');
    await adapter.deleteAgent('inline');
    assert.deepEqual(calls, [
      ['delete_agent', { id: 'custom', storage: 'global_markdown' }],
      ['delete_agent', { id: 'inline', storage: undefined }],
    ]);
  } finally {
    await vite.close();
  }
});

test('app info, manual updates, and project pages preserve IPC names, payloads, results, and errors', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const calls = [];
    const info = { version: '0.1.0', platform: 'macos' };
    const update = { currentVersion: '0.1.0', latestVersion: '0.10.0', updateAvailable: true };
    let failure;
    const adapter = createTauriAdapter(async (...args) => {
      calls.push(args);
      if (failure) throw failure;
      if (args[0] === 'get_app_info') return info;
      if (args[0] === 'check_for_updates') return update;
    });
    assert.strictEqual(await adapter.getAppInfo(), info);
    assert.strictEqual(await adapter.checkForUpdates(), update);
    assert.equal(await adapter.openProjectPage('repository'), undefined);
    assert.equal(await adapter.openProjectPage('releases'), undefined);
    assert.deepEqual(calls, [
      ['get_app_info'],
      ['check_for_updates'],
      ['open_project_page', { page: 'repository' }],
      ['open_project_page', { page: 'releases' }],
    ]);
    failure = { code: 'configuration_failed', message: 'GitHub unavailable' };
    await assert.rejects(adapter.getAppInfo(), failure);
    await assert.rejects(adapter.checkForUpdates(), failure);
    await assert.rejects(adapter.openProjectPage('releases'), failure);
  } finally {
    await vite.close();
  }
});

test('token usage records invoke the no-argument IPC command and preserve timestamp and count data', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const calls = [];
    const records = [
      { time: 1791030514564, input: 1_250_000, output: 20 },
      { time: 1791116914564, input: 0, output: 0 },
    ];
    const adapter = createTauriAdapter(async (...args) => {
      calls.push(args);
      return records;
    });
    assert.strictEqual(await adapter.opencodeTokenUsageRecords(), records);
    assert.deepEqual(calls, [['opencode_token_usage_records']]);
    assert.deepEqual(records, [
      { time: 1791030514564, input: 1_250_000, output: 20 },
      { time: 1791116914564, input: 0, output: 0 },
    ]);
  } finally {
    await vite.close();
  }
});

test('OpenCode installation, explicit checks, and guarded upgrades preserve IPC contracts and errors', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const calls = [];
    const installation = {
      binaryPath: '/exact/bin/opencode',
      version: '2.0.0',
      method: 'curl',
      canUpgrade: true,
      upgradeReason: null,
      manualCommand: null,
    };
    const update = { installation, latestVersion: '2.1.0', updateAvailable: true };
    let failure;
    const adapter = createTauriAdapter(async (...args) => {
      calls.push(args);
      if (failure) throw failure;
      return args[0] === 'opencode_check_update' ? update : installation;
    });
    assert.strictEqual(await adapter.getOpenCodeInstallation(), installation);
    assert.strictEqual(await adapter.checkOpenCodeUpdate(), update);
    assert.strictEqual(await adapter.upgradeOpenCode('/exact/bin/opencode', '2.0.0', '2.1.0'), installation);
    assert.deepEqual(calls, [
      ['opencode_get_installation'],
      ['opencode_check_update'],
      [
        'opencode_upgrade',
        { expectedBinaryPath: '/exact/bin/opencode', expectedVersion: '2.0.0', targetVersion: '2.1.0' },
      ],
    ]);
    failure = { code: 'conflict', message: 'Installation changed. Refresh first.' };
    await assert.rejects(adapter.getOpenCodeInstallation(), failure);
    await assert.rejects(adapter.checkOpenCodeUpdate(), failure);
    await assert.rejects(adapter.upgradeOpenCode('/exact/bin/opencode', '2.0.0', '2.1.0'), failure);
  } finally {
    await vite.close();
  }
});

test('MCP, skills, autostart, and lid protection preserve IPC commands, payloads, and results', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const calls = [];
    const result = { data: [], diagnostics: [] };
    const lid = { enabled: true, phase: 'idle' };
    const adapter = createTauriAdapter(async (command, args) => {
      calls.push([command, args]);
      if (command === 'get_lid_protection' || command === 'set_lid_protection') return lid;
      return command === 'get_autostart' || command === 'set_autostart' ? false : result;
    });
    assert.strictEqual(await adapter.listMcps(), result);
    assert.strictEqual(await adapter.getMcp('local'), result);
    const mcpDraft = { name: 'local', config: { type: 'local', command: ['run'], environment: { TOKEN: 'secret' } } };
    const mcpUpdate = { ...mcpDraft, expectedConfig: mcpDraft.config, expectedSourcePath: '/managed/opencode.jsonc' };
    assert.strictEqual(await adapter.createMcp(mcpDraft), result);
    assert.strictEqual(await adapter.updateMcp(mcpUpdate), result);
    await adapter.deleteMcp('local');
    assert.strictEqual(await adapter.listSkills(), result);
    assert.strictEqual(await adapter.getSkill('CaseSensitive'), result);
    const skillDraft = { id: 'CaseSensitive', content: '---\nname: Display\n---\nbody' };
    const skillUpdate = { ...skillDraft, expectedContent: 'old', expectedPath: '/managed/CaseSensitive/SKILL.md' };
    assert.strictEqual(await adapter.createSkill(skillDraft), result);
    assert.strictEqual(await adapter.updateSkill(skillUpdate), result);
    assert.equal(await adapter.getAutostart(), false);
    assert.equal(await adapter.setAutostart(true), false);
    assert.equal(await adapter.setAutostart(false), false);
    assert.strictEqual(await adapter.getLidProtection(), lid);
    assert.strictEqual(await adapter.setLidProtection(true), lid);
    assert.strictEqual(await adapter.setLidProtection(false), lid);
    assert.deepEqual(calls, [
      ['list_mcps', undefined],
      ['get_mcp', { name: 'local' }],
      ['create_mcp', { draft: mcpDraft }],
      ['update_mcp', { draft: mcpUpdate }],
      ['delete_mcp', { name: 'local' }],
      ['list_skills', undefined],
      ['get_skill', { id: 'CaseSensitive' }],
      ['create_skill', { draft: skillDraft }],
      ['update_skill', { draft: skillUpdate }],
      ['get_autostart', undefined],
      ['set_autostart', { enabled: true }],
      ['set_autostart', { enabled: false }],
      ['get_lid_protection', undefined],
      ['set_lid_protection', { enabled: true }],
      ['set_lid_protection', { enabled: false }],
    ]);
  } finally {
    await vite.close();
  }
});

test('local service target IPC preserves nullable fields and omitted, retained, new, and cleared passwords', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const calls = [];
    const status = {
      target: { mode: 'shared', url: null, hasPassword: false },
      connected: false,
      url: null,
      pid: null,
      version: null,
      error: null,
    };
    let failure;
    const adapter = createTauriAdapter(async (...args) => {
      calls.push(args);
      if (failure) throw failure;
      return status;
    });
    assert.strictEqual(await adapter.getOpenCodeServiceStatus(), status);
    assert.strictEqual(await adapter.setOpenCodeServiceTarget(null), status);
    await adapter.setOpenCodeServiceTarget('http://127.0.0.1:4096');
    await adapter.setOpenCodeServiceTarget('http://127.0.0.1:4096', undefined);
    await adapter.setOpenCodeServiceTarget('http://127.0.0.1:4096', null);
    await adapter.setOpenCodeServiceTarget('http://127.0.0.1:4096', 'local-secret');
    await adapter.setOpenCodeServiceTarget('http://127.0.0.1:4096', '');
    assert.deepEqual(calls, [
      ['opencode_get_service_status'],
      ['opencode_set_service_target', { url: null }],
      ['opencode_set_service_target', { url: 'http://127.0.0.1:4096' }],
      ['opencode_set_service_target', { url: 'http://127.0.0.1:4096' }],
      ['opencode_set_service_target', { url: 'http://127.0.0.1:4096', password: null }],
      ['opencode_set_service_target', { url: 'http://127.0.0.1:4096', password: 'local-secret' }],
      ['opencode_set_service_target', { url: 'http://127.0.0.1:4096', password: '' }],
    ]);
    failure = { code: 'busy', message: 'Disable monitoring and restore power settings first.' };
    await assert.rejects(adapter.getOpenCodeServiceStatus(), failure);
    await assert.rejects(adapter.setOpenCodeServiceTarget(null), failure);
  } finally {
    await vite.close();
  }
});
