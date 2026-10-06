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
