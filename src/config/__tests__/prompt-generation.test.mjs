import assert from 'node:assert/strict';
import test from 'node:test';
import { createServer } from 'vite';

function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

test('adapter and store forward exact prompt generation payload for agent and command kinds', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    const calls = [];

    const adapter = createTauriAdapter(async (command, args) => {
      calls.push([command, args]);
      if (command === 'opencode_generate_prompt') {
        return `generated for ${args.request.kind}:${args.request.preset}`;
      }
      return null;
    });

    const defaultRequest = {
      kind: 'agent',
      preset: 'generate',
      instructions: 'Create an autonomous code reviewer',
      currentPrompt: '',
    };
    const adapterResult = await adapter.generatePrompt(defaultRequest);
    assert.equal(adapterResult, 'generated for agent:generate');
    assert.deepEqual(calls.at(-1), ['opencode_generate_prompt', { request: defaultRequest }]);

    const store = createConfigStore(adapter, false, false);
    const explicitRequest = {
      kind: 'command',
      preset: 'shorten',
      instructions: 'Make instructions compact',
      currentPrompt: 'You are an assistant who answers concisely.',
      model: 'anthropic/claude-3-7-sonnet',
      variant: 'high',
    };
    const storeResult = await store.generatePrompt(explicitRequest);
    assert.equal(storeResult, 'generated for command:shorten');
    assert.deepEqual(calls.at(-1), ['opencode_generate_prompt', { request: explicitRequest }]);
  } finally {
    await vite.close();
  }
});

test('store.generatePrompt passes through raw errors without retrying and keeps secrets out of store state', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    const calls = [];
    let pendingFailure = null;
    let nextResult = null;

    const adapter = createTauriAdapter(async (command, args) => {
      calls.push([command, args]);
      if (pendingFailure) {
        throw pendingFailure;
      }
      return nextResult;
    });

    const store = createConfigStore(adapter, false, false);
    const secretInstructions = 'private-prompt-instruction-987';
    const secretPrompt = 'private-system-prompt-body-654';
    const secretResult = 'private-generated-output-321';

    pendingFailure = {
      code: 'validation_failed',
      message: 'Model is temporarily unavailable',
      detail: { reason: 'quota_exceeded' },
    };

    let caughtError;
    await assert.rejects(
      store.generatePrompt({
        kind: 'agent',
        preset: 'improve',
        instructions: secretInstructions,
        currentPrompt: secretPrompt,
      }),
      (err) => {
        caughtError = err;
        return err.code === 'validation_failed';
      },
    );

    assert.deepEqual(caughtError, pendingFailure);
    assert.equal(calls.length, 1);
    assert.equal(store.saving, false);
    assert.equal(store.loading, false);
    assert.equal(store.notice, null);
    assert.equal(store.error, null);
    assert.equal(store.draftRecovery, null);

    const serializedStateAfterError = JSON.stringify(store);
    assert.ok(!serializedStateAfterError.includes(secretInstructions));
    assert.ok(!serializedStateAfterError.includes(secretPrompt));

    pendingFailure = null;
    nextResult = secretResult;

    const successfulResult = await store.generatePrompt({
      kind: 'command',
      preset: 'generate',
      instructions: secretInstructions,
      currentPrompt: secretPrompt,
      model: 'openai/gpt-4o',
    });

    assert.equal(successfulResult, secretResult);
    assert.equal(calls.length, 2);
    assert.equal(store.saving, false);
    assert.equal(store.loading, false);
    assert.equal(store.notice, null);
    assert.equal(store.error, null);
    assert.equal(store.draftRecovery, null);

    const serializedStateAfterSuccess = JSON.stringify(store);
    assert.ok(!serializedStateAfterSuccess.includes(secretInstructions));
    assert.ok(!serializedStateAfterSuccess.includes(secretPrompt));
    assert.ok(!serializedStateAfterSuccess.includes(secretResult));
    assert.equal('currentPrompt' in store, false);
    assert.equal('prompt' in store, false);
    assert.equal('instructions' in store, false);

    assert.ok(calls.every(([command]) => command === 'opencode_generate_prompt'));
  } finally {
    await vite.close();
  }
});

test('store.generatePrompt guards against concurrent service target saves and stale late replies', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    const calls = [];
    let targetDeferred = null;
    let generateDeferred = null;

    const adapter = createTauriAdapter(async (command, args) => {
      calls.push([command, args]);
      if (command === 'set_opencode_service_target') {
        return targetDeferred.promise;
      }
      if (command === 'opencode_generate_prompt') {
        return generateDeferred.promise;
      }
      return null;
    });

    const store = createConfigStore(adapter, false, false);
    const request = {
      kind: 'agent',
      preset: 'generate',
      instructions: 'instructions during target change',
      currentPrompt: '',
    };

    targetDeferred = deferred();
    const targetSavePromise = store.setOpenCodeServiceTarget('http://localhost:4096', null);
    const dispatchedBefore = calls.length;

    await assert.rejects(store.generatePrompt(request), { code: 'conflict' });
    assert.equal(calls.length, dispatchedBefore);

    targetDeferred.resolve({ connected: true, activeService: 'local' });
    await targetSavePromise;
    assert.equal(store.serviceTargetVersion, 1);

    generateDeferred = deferred();
    const inFlightPrompt = store.generatePrompt(request);
    assert.equal(calls.length, dispatchedBefore + 1);

    targetDeferred = deferred();
    const targetSavePromise2 = store.setOpenCodeServiceTarget('http://localhost:4097', null);
    targetDeferred.resolve({ connected: true, activeService: 'remote' });
    await targetSavePromise2;
    assert.equal(store.serviceTargetVersion, 2);

    generateDeferred.resolve('late prompt arrival');
    await assert.rejects(inFlightPrompt, { code: 'conflict' });

    assert.equal(store.saving, false);
    assert.equal(store.loading, false);
    assert.equal(store.notice, null);
    assert.equal(store.error, null);
    assert.equal(store.draftRecovery, null);
  } finally {
    await vite.close();
  }
});
