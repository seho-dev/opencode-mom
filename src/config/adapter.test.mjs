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
