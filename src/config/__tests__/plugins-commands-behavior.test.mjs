import assert from 'node:assert/strict';
import test from 'node:test';
import { createServer } from 'vite';

const projectId = '11111111-1111-4111-8111-111111111111';
const secret = 'private-plugin-command-instructions';
const options = { token: secret, nested: { enabled: true } };
const preview = {
  directory: '/project',
  data: [
    {
      type: 'document',
      path: '/config/opencode.jsonc',
      plugins: ['trusted-plugin', { package: 'private-plugin', options }, { package: 'plain-plugin' }],
    },
    { type: 'document', plugins: [] },
    { type: 'directory', path: '/config/plugins' },
  ],
};
const runtime = {
  directory: '/project',
  data: [
    {
      source: { type: 'package', target: 'trusted-plugin' },
      canUpdate: true,
      canRemove: true,
      features: {},
      state: { status: 'active' },
    },
    {
      source: { type: 'package', target: 'other-plugin' },
      canUpdate: false,
      canRemove: false,
      features: {},
      state: { status: 'active' },
    },
  ],
};
const removalTarget = Object.freeze({ type: 'package', target: 'trusted-plugin' });
const removalPlan = Object.freeze({
  target: removalTarget,
  sources: Object.freeze([
    Object.freeze({ path: '/config/opencode.jsonc', expectedPath: '/resolved/opencode.jsonc', expectedHash: secret }),
    Object.freeze({ path: '/config/extra.jsonc', expectedPath: '/resolved/extra.jsonc', expectedHash: `${secret}-2` }),
  ]),
  serviceKey: `${secret}-service`,
});
const pluginResults = {
  get_plugin_config_sources: preview,
  list_plugin_runtime: runtime,
  prepare_plugin_removal: removalPlan,
};
const commandSnapshot = `---\r\ndescription: ${secret}\r\nagent: build\r\n---\r\n\r\nReview $ARGUMENTS.\r\n`;
const command = { name: 'review', content: secret };
const commandUpdate = {
  id: 'review',
  content: secret,
  expectedPath: '/config/commands/review.md',
  expectedContent: commandSnapshot,
};
const commandDelete = {
  id: commandUpdate.id,
  expectedPath: commandUpdate.expectedPath,
  expectedContent: commandSnapshot,
};
const behavior = {
  content: secret,
  expectedPath: '/config/AGENTS.md',
  expectedContent: `# Instructions\n\n${secret}\n\n- Preserve all Markdown.\n`,
};
const pluginOperations = [
  ['getPluginConfigSources', 'get_plugin_config_sources', [], undefined],
  ['listPluginRuntime', 'list_plugin_runtime', [], undefined],
  ['updatePluginPackage', 'update_plugin_package', ['trusted-plugin'], { target: 'trusted-plugin' }],
  ['preparePluginRemoval', 'prepare_plugin_removal', [removalTarget], { target: removalTarget }],
  ['removePluginDeclaration', 'remove_plugin_declaration', [removalPlan], { plan: removalPlan }],
];
const commandOperations = [
  ['listSlashCommands', 'list_slash_commands', [], undefined],
  ['getSlashCommand', 'get_slash_command', ['review'], { id: 'review' }],
  ['createSlashCommand', 'create_slash_command', [command], { draft: command }],
  ['updateSlashCommand', 'update_slash_command', [commandUpdate], { draft: commandUpdate }],
  ['deleteSlashCommand', 'delete_slash_command', [commandDelete], { draft: commandDelete }],
];
const writes = commandOperations.filter(([method]) => /^(create|update|delete)SlashCommand$/.test(method));

function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

test('plugins stay Global-only while commands and behavior preserve snapshots, nullable fields and command scopes', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const calls = [];
    const result = { data: [], diagnostics: [] };
    let failure;
    const adapter = createTauriAdapter(async (...args) => {
      calls.push(args);
      if (failure) throw failure;
      return pluginResults[args[0]] ?? result;
    });
    for (const [method, ipc, args, payload] of pluginOperations) {
      for (const input of [args, [...args, undefined], [...args, null], [...args, projectId]]) {
        assert.strictEqual(await adapter[method](...input), pluginResults[ipc] ?? result);
        assert.deepEqual(calls.at(-1), payload ? [ipc, payload] : [ipc]);
      }
    }
    for (const [method, ipc, args, payload] of commandOperations) {
      for (const scope of [undefined, null, projectId]) {
        assert.strictEqual(await adapter[method](...args, scope), pluginResults[ipc] ?? result);
        assert.deepEqual(
          calls.at(-1),
          scope ? [ipc, { ...payload, projectId: scope }] : payload ? [ipc, payload] : [ipc],
        );
      }
    }
    assert.strictEqual(await adapter.getGlobalBehavior(), result);
    assert.deepEqual(calls.at(-1), ['get_global_behavior']);
    for (const draft of [behavior, { content: secret, expectedPath: null, expectedContent: null }]) {
      assert.strictEqual(await adapter.saveGlobalBehavior(draft), result);
      assert.deepEqual(calls.at(-1), ['save_global_behavior', { draft }]);
    }
    failure = { code: 'conflict', message: secret, detail: preview };
    for (const [method, , args] of [...pluginOperations, ...commandOperations])
      await assert.rejects(adapter[method](...args, projectId), failure);
    await assert.rejects(adapter.getGlobalBehavior(), failure);
    await assert.rejects(adapter.saveGlobalBehavior(behavior), failure);
  } finally {
    await vite.close();
  }
});

test('plugin facades stay Global-only with a selected project and commands retain explicit scope without global refresh', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    const calls = [];
    const result = { data: [], diagnostics: [] };
    const store = createConfigStore(
      createTauriAdapter(async (...args) => {
        calls.push(args);
        if (args[0] === 'load_app_state')
          return {
            providers: [],
            agents: [],
            groups: [],
            projects: [{ id: projectId, path: '/project' }],
            selectedProjectId: projectId,
            preferences: { theme: 'dark', locale: 'en' },
          };
        if (args[0] === 'opencode_list_models') return [{ ref: 'provider/model' }];
        return pluginResults[args[0]] ?? result;
      }),
    );
    await new Promise(setImmediate);
    assert.deepEqual(
      calls.map(([ipc]) => ipc),
      ['opencode_list_models', 'load_app_state'],
    );
    calls.length = 0;
    const original = Object.fromEntries(
      ['providers', 'agents', 'groups', 'preferences', 'catalog', 'serviceTargetVersion', 'formResetVersion'].map(
        (key) => [key, store[key]],
      ),
    );
    assert.equal(store.selectedProjectId, projectId);
    for (const [method, ipc, args, payload] of pluginOperations) {
      for (const input of [args, [...args, undefined], [...args, null], [...args, projectId]]) {
        assert.strictEqual(await store[method](...input), pluginResults[ipc] ?? result);
        assert.deepEqual(calls.at(-1), payload ? [ipc, payload] : [ipc]);
      }
    }
    for (const [method, ipc, args, payload] of commandOperations) {
      for (const scope of [undefined, null, projectId]) {
        assert.strictEqual(await store[method](...args, scope), pluginResults[ipc] ?? result);
        assert.deepEqual(calls.at(-1), scope ? [ipc, { ...payload, projectId }] : payload ? [ipc, payload] : [ipc]);
      }
    }
    assert.strictEqual(await store.getGlobalBehavior(), result);
    assert.deepEqual(calls.at(-1), ['get_global_behavior']);
    assert.strictEqual(await store.saveGlobalBehavior(behavior), result);
    assert.deepEqual(calls.at(-1), ['save_global_behavior', { draft: behavior }]);
    for (const [key, value] of Object.entries(original)) assert.strictEqual(store[key], value);
    for (const key of [
      'plugins',
      'pluginRuntime',
      'pluginConfigSources',
      'pluginConfigPreview',
      'options',
      'slashCommands',
      'behavior',
    ])
      assert.equal(key in store, false);
    assert.ok(!JSON.stringify(store).includes(secret));
    assert.equal(store.selectedProjectId, projectId);
    assert.equal(store.notice, 'saveGlobalBehavior');
    assert.equal(store.saving, false);
    assert.equal(store.loading, false);
    assert.equal(store.draftRecovery, null);
    assert.ok(calls.every(([ipc]) => !['load_app_state', 'opencode_list_models'].includes(ipc)));
  } finally {
    await vite.close();
  }
});

test('plugin runtime list stays lazy and Global-only, without global mutation or retry after failure', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    const calls = [];
    let pending;
    const store = createConfigStore(
      createTauriAdapter((...args) => {
        calls.push(args);
        return pending.promise;
      }),
      false,
      false,
    );
    await new Promise(setImmediate);
    assert.deepEqual(calls, []);
    const original = Object.fromEntries(
      [
        'providers',
        'agents',
        'groups',
        'preferences',
        'catalog',
        'serviceTargetVersion',
        'formResetVersion',
        'error',
        'notice',
        'draftRecovery',
      ].map((key) => [key, store[key]]),
    );
    for (const failure of [null, { code: 'configuration_failed', message: 'Runtime refresh failed.' }]) {
      pending = deferred();
      const callCount = calls.length;
      const reading = store.listPluginRuntime(projectId);
      assert.equal(calls.length, callCount + 1);
      assert.deepEqual(calls.at(-1), ['list_plugin_runtime']);
      assert.equal(store.saving, false);
      assert.equal(store.loading, false);
      if (failure) {
        const rejected = assert.rejects(reading, failure);
        pending.reject(failure);
        await rejected;
      } else {
        pending.resolve(runtime);
        assert.strictEqual(await reading, runtime);
      }
      await new Promise(setImmediate);
      assert.equal(calls.length, callCount + 1);
      for (const [key, value] of Object.entries(original)) assert.strictEqual(store[key], value);
    }
    assert.ok(!JSON.stringify(store).includes(secret));
  } finally {
    await vite.close();
  }
});

test('plugin removal echoes complete package and local plans without startup work, global state, reload or retries', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    const calls = [];
    let pending;
    const store = createConfigStore(
      createTauriAdapter((...args) => {
        calls.push(args);
        return pending.promise;
      }),
      false,
      false,
    );
    await new Promise(setImmediate);
    assert.deepEqual(calls, []);
    const original = JSON.stringify(store);
    const localTarget = Object.freeze({ type: 'local', path: '/config/plugins/local.ts' });
    const localPlan = Object.freeze({ ...removalPlan, target: localTarget });
    for (const plan of [removalPlan, localPlan]) {
      const operations = [
        ['preparePluginRemoval', 'prepare_plugin_removal', 'target', plan.target, plan],
        ['removePluginDeclaration', 'remove_plugin_declaration', 'plan', plan, undefined],
      ];
      for (const [method, ipc, field, value, result] of operations) {
        for (const failure of [null, { code: 'conflict', message: secret, detail: plan }]) {
          pending = deferred();
          const count = calls.length;
          const action = store[method](value, projectId);
          assert.equal(calls.length, count + 1);
          assert.deepEqual(calls.at(-1), [ipc, { [field]: value }]);
          assert.strictEqual(calls.at(-1)[1][field], value);
          assert.equal(store.saving, false);
          assert.equal(store.loading, false);
          if (failure) {
            const rejected = assert.rejects(action, failure);
            pending.reject(failure);
            await rejected;
          } else {
            pending.resolve(result);
            assert.strictEqual(await action, result);
          }
          await new Promise(setImmediate);
          assert.equal(calls.length, count + 1);
          assert.equal(JSON.stringify(store), original);
          assert.ok(!JSON.stringify(store).includes(secret));
        }
      }
    }
  } finally {
    await vite.close();
  }
});

test('every command and behavior write redacts recovery, returned errors and deleted snapshots', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    let pending;
    const store = createConfigStore(
      createTauriAdapter(() => pending.promise),
      false,
      false,
    );
    for (const [method, , args] of [...writes, ['saveGlobalBehavior', 'save_global_behavior', [behavior]]]) {
      for (const code of ['conflict', 'busy', 'not_found', 'validation_failed', 'configuration_failed', 'ipc_error']) {
        pending = deferred();
        const writing = store[method](...args);
        assert.equal(store.saving, true);
        const rejected = assert.rejects(writing, (error) => {
          assert.deepEqual(error, {
            code,
            message: 'Configuration operation failed. Review the draft in the original form and retry.',
          });
          return true;
        });
        pending.reject({ code, message: secret, detail: { content: secret, options } });
        await rejected;
        assert.equal(store.draftRecovery.operation, method);
        assert.equal(store.draftRecovery.payload, null);
        assert.equal(store.draftRecovery.conflict, code === 'conflict');
        assert.equal(store.error.detail, undefined);
        assert.ok(!JSON.stringify([store.error, store.draftRecovery]).includes(secret));
        assert.equal(store.saving, false);
      }
    }
  } finally {
    await vite.close();
  }
});

test('resource reads redact private failures without creating or replacing global recovery', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    const calls = [];
    let failure;
    const store = createConfigStore(
      createTauriAdapter(async (...args) => {
        calls.push(args);
        throw failure;
      }),
      false,
      false,
    );
    const reads = [
      ['getPluginConfigSources', [], ['get_plugin_config_sources']],
      ['listSlashCommands', [projectId], ['list_slash_commands', { projectId }]],
      ['getSlashCommand', ['review', projectId], ['get_slash_command', { id: 'review', projectId }]],
      ['getGlobalBehavior', [], ['get_global_behavior']],
    ];
    for (const code of [
      'not_found',
      'references_blocked',
      'validation_failed',
      'configuration_failed',
      'busy',
      'conflict',
      'ipc_error',
    ]) {
      failure = { code, message: secret, detail: { content: secret, options } };
      for (const [method, args, expectedCall] of reads) {
        const callCount = calls.length;
        await assert.rejects(store[method](...args), (error) => {
          assert.deepEqual(error, { code, message: 'Configuration could not be loaded. Refresh and retry.' });
          assert.ok(!JSON.stringify(error).includes(secret));
          return true;
        });
        assert.equal(calls.length, callCount + 1);
        assert.deepEqual(calls.at(-1), expectedCall);
        for (const key of ['error', 'notice', 'draftRecovery']) assert.equal(store[key], null);
        assert.equal(store.saving, false);
        assert.equal(store.loading, false);
      }
    }
    failure = { code: 'conflict', message: secret, detail: preview };
    await assert.rejects(store.updateSlashCommand(commandUpdate), { code: 'conflict' });
    const originalError = store.error;
    const originalRecovery = store.draftRecovery;
    assert.equal(originalRecovery.payload, null);
    for (const cause of [failure, { code: secret, message: secret }, new Error(secret), secret]) {
      failure = cause;
      for (const [method, args] of reads) {
        await assert.rejects(store[method](...args), {
          code: cause.code === 'conflict' ? 'conflict' : 'ipc_error',
          message: 'Configuration could not be loaded. Refresh and retry.',
        });
        assert.strictEqual(store.error, originalError);
        assert.strictEqual(store.draftRecovery, originalRecovery);
        assert.ok(!JSON.stringify([store.error, store.draftRecovery]).includes(secret));
      }
    }
  } finally {
    await vite.close();
  }
});

test('plugin preview and runtime operations reject old service replies without retries or dispatch during target saves', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    const calls = [];
    let pending;
    let saving;
    const methods = [
      ['getPluginConfigSources', []],
      ['listPluginRuntime', []],
      ['updatePluginPackage', ['trusted-plugin']],
      ['preparePluginRemoval', [removalTarget]],
      ['removePluginDeclaration', [removalPlan]],
    ];
    const store = createConfigStore(
      {
        ...Object.fromEntries(
          methods.map(([method]) => [
            method,
            (...args) => {
              calls.push([method, args]);
              return pending.promise;
            },
          ]),
        ),
        setOpenCodeServiceTarget: () => saving.promise,
      },
      false,
      false,
    );
    for (const [method, args] of methods) {
      pending = deferred();
      const stale = assert.rejects(store[method](...args, projectId), (error) => {
        assert.equal(error.code, 'conflict');
        assert.ok(!JSON.stringify(error).includes(secret));
        return true;
      });
      saving = deferred();
      const targetSave = store.setOpenCodeServiceTarget(null);
      const dispatched = calls.length;
      await assert.rejects(store[method](...args, projectId), { code: 'conflict' });
      assert.equal(calls.length, dispatched);
      saving.resolve({ connected: false });
      await targetSave;
      pending.resolve({ ...preview, directory: '/old-target' });
      await stale;
      assert.equal(store.error, null);
      assert.equal(store.notice, null);
      assert.equal(store.draftRecovery, null);
      assert.equal(store.saving, false);
    }
    assert.equal(store.serviceTargetVersion, methods.length);
    assert.deepEqual(
      calls,
      methods.map(([method, args]) => [method, args]),
    );
  } finally {
    await vite.close();
  }
});
