import assert from 'node:assert/strict';
import test from 'node:test';
import { createServer } from 'vite';

test('notification commands exact IPC names, settings results, failures, and no retry/state mutation', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const settings = { enabled: false, agentCompleted: true, approvalRequired: true, agentError: true };
    const calls = [];
    let failure;
    const adapter = createTauriAdapter(async (...args) => {
      calls.push(args);
      if (failure) throw failure;
      if (args[0] === 'get_notification_settings') return settings;
      return args[0] === 'set_notification_settings' ? args[1].settings : settings;
    });
    const cases = [
      ['getNotificationSettings', [], ['get_notification_settings'], settings],
      ['setNotificationSettings', [settings], ['set_notification_settings', { settings }], settings],
    ];
    assert.deepEqual(calls, []);
    for (const [method, args, expected, result] of cases) {
      assert.deepEqual(await adapter[method](...args), result);
      assert.deepEqual(calls.at(-1), expected);
    }
    assert.equal(calls.length, cases.length);
    failure = { code: 'configuration_failed', message: 'notification save failed', detail: 'detail' };
    for (const [method, args, expected] of cases) {
      const before = calls.length;
      await assert.rejects(adapter[method](...args), (cause) => cause === failure);
      assert.equal(calls.length, before + 1);
      assert.deepEqual(calls.at(-1), expected);
    }
  } finally {
    await vite.close();
  }
});

test('notification facade lazy, no global refresh/recovery or payload retention', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    const calls = [];
    const settings = { enabled: true, agentCompleted: false, approvalRequired: true, agentError: true };
    const adapter = createTauriAdapter(async (...args) => {
      calls.push(args);
      return args[0] === 'get_notification_settings' ? settings : (args[1]?.settings ?? settings);
    });
    const store = createConfigStore(adapter, false, false);
    const before = JSON.stringify(store);
    const result = await store.getNotificationSettings();
    assert.deepEqual(result, settings);
    assert.deepEqual(calls.at(-1), ['get_notification_settings']);
    assert.equal(JSON.stringify(store), before);
    await store.setNotificationSettings({ ...settings, enabled: false });
    assert.deepEqual(calls.at(-1), ['set_notification_settings', { settings: { ...settings, enabled: false } }]);
    assert.equal(JSON.stringify(store), before);
  } finally {
    await vite.close();
  }
});

// Permission commands are query/request-only: they must never write settings,
// never retry, and never be triggered by reads.
test('permission commands are no-argument, exact IPC, and never mutate preferences', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    const states = ['prompt', 'granted', 'denied', 'unavailable', 'unsupported'];
    const calls = [];
    let failure;
    let index = 0;
    const adapter = createTauriAdapter(async (...args) => {
      calls.push(args);
      if (failure) throw failure;
      switch (args[0]) {
        case 'get_notification_permission':
          return states[index++];
        case 'request_notification_permission':
          return 'granted';
        case 'open_notification_settings':
          return null;
        default:
          return null;
      }
    });
    const store = createConfigStore(adapter, false, false);
    const before = JSON.stringify(store);

    for (const state of states) {
      const result = await store.getNotificationPermission();
      assert.equal(result, state);
      assert.deepEqual(calls.at(-1), ['get_notification_permission']);
    }
    // Reads are query-only: they never request permission as a side effect.
    assert.ok(!calls.some((call) => call[0] === 'request_notification_permission'));
    assert.ok(calls.every((call) => call.length === 1));
    assert.equal(await store.requestNotificationPermission(), 'granted');
    assert.deepEqual(calls.at(-1), ['request_notification_permission']);
    await store.openNotificationSettings();
    assert.deepEqual(calls.at(-1), ['open_notification_settings']);
    assert.equal(JSON.stringify(store), before);

    failure = { code: 'configuration_failed', message: 'permission request failed' };
    const count = calls.length;
    await assert.rejects(store.requestNotificationPermission(), (cause) => cause === failure);
    assert.equal(calls.length, count + 1);
    await assert.rejects(store.getNotificationPermission(), (cause) => cause === failure);
    assert.equal(calls.length, count + 2);
    assert.equal(JSON.stringify(store), before);
  } finally {
    await vite.close();
  }
});

// Malformed wire values must fail closed with the fixed contract error and never echo
// the payload back to callers.
test('malformed permission results fail closed for get and request without echoing input', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    const malformed = [
      undefined,
      null,
      'grant',
      'Granted',
      '',
      { permission: 'granted' },
      [{ permission: 'denied' }],
      1,
      true,
    ];
    // A denied response must stay denied: the adapter never upgrades it to granted.
    const denied = createConfigStore(
      createTauriAdapter(async () => 'denied'),
      false,
      false,
    );
    assert.equal(await denied.getNotificationPermission(), 'denied');
    assert.equal(await denied.requestNotificationPermission(), 'denied');

    for (const method of ['getNotificationPermission', 'requestNotificationPermission']) {
      for (const value of malformed) {
        let calls = 0;
        const store = createConfigStore(
          createTauriAdapter(async (command) => {
            calls += 1;
            assert.equal(
              command,
              method === 'getNotificationPermission'
                ? 'get_notification_permission'
                : 'request_notification_permission',
            );
            return value;
          }),
          false,
          false,
        );
        const before = JSON.stringify(store);
        await assert.rejects(store[method](), (cause) => {
          assert.equal(calls, 1, 'no retry on malformed result');
          assert.equal(cause.code, 'configuration_failed');
          assert.equal(cause.message, 'Notification permission could not be confirmed.');
          const echoed = typeof value === 'string' ? value : JSON.stringify(value);
          if (echoed) assert.ok(!cause.message.includes(echoed));
          return true;
        });
        assert.equal(calls, 1);
        assert.equal(JSON.stringify(store), before);
      }
    }
  } finally {
    await vite.close();
  }
});
