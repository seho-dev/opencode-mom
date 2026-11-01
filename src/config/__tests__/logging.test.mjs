import assert from 'node:assert/strict';
import test from 'node:test';
import { createServer } from 'vite';

test('logging commands preserve exact IPC names, query fields, results, and one-shot failures', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { LOG_TYPES } = await vite.ssrLoadModule('/src/types/logging.ts');
    assert.deepEqual(LOG_TYPES, [
      'app',
      'config',
      'group',
      'resource',
      'opencode',
      'http',
      'mcp_proxy',
      'power',
      'island',
      'logging',
    ]);
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const state = { enabled: true, recording: false, directory: '/logs', error: null, droppedRecords: 3 };
    const page = { records: [], nextCursor: 'next-page', invalidLines: 2 };
    const query = {
      type: 'mcp_proxy',
      status: 'error',
      from: '2026-11-01T00:00:00Z',
      to: '2026-11-02T00:00:00Z',
      operationId: 'operation-1',
      cursor: 'previous-page',
      limit: 20,
    };
    const calls = [];
    let failure;
    const adapter = createTauriAdapter(async (...args) => {
      calls.push(args);
      if (failure) throw failure;
      if (args[0] === 'query_logs') return page;
      return args[0] === 'open_logs_directory' ? undefined : state;
    });
    const cases = [
      ['getLoggingState', [], ['get_logging_state'], state],
      ['setLoggingEnabled', [true], ['set_logging_enabled', { enabled: true }], state],
      ['setLoggingEnabled', [false], ['set_logging_enabled', { enabled: false }], state],
      ['queryLogs', [query], ['query_logs', { query }], page],
      ['queryLogs', [{}], ['query_logs', { query: {} }], page],
      ['openLogsDirectory', [], ['open_logs_directory'], undefined],
      ['clearLogs', [], ['clear_logs'], state],
    ];
    assert.deepEqual(calls, []);
    for (const [method, args, expected, result] of cases) {
      assert.equal(await adapter[method](...args), result);
      assert.deepEqual(calls.at(-1), expected);
      if (args[0] === query) assert.equal(calls.at(-1)[1].query, query);
    }
    assert.equal(calls.length, cases.length);
    failure = { code: 'configuration_failed', message: 'private failure', detail: 'private detail' };
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

test('logging facades stay lazy and never retain payloads, results, or private failures in shared state', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    const calls = [];
    const state = { enabled: false, recording: false, directory: '/private/logs', error: null, droppedRecords: 0 };
    const page = {
      records: [
        {
          schemaVersion: 1,
          timestamp: '2026-11-01T00:00:00Z',
          sessionId: 'private-session',
          sequence: 1,
          operationId: 'private-operation',
          type: 'config',
          status: 'success',
          event: 'config_saved',
          message: 'Configuration saved',
          durationMs: null,
          errorCode: null,
          context: { step: 'write', count: 1, targetWritten: true },
        },
      ],
      nextCursor: null,
      invalidLines: 0,
    };
    let failure;
    let finish;
    const pending = new Promise((resolve) => {
      finish = resolve;
    });
    const adapter = createTauriAdapter(async (...args) => {
      calls.push(args);
      if (failure) throw failure;
      await pending;
      if (args[0] === 'query_logs') return page;
      return args[0] === 'open_logs_directory' ? undefined : state;
    });
    const store = createConfigStore(adapter, false, false);
    const before = JSON.stringify(store);
    assert.deepEqual(calls, []);
    const query = { operationId: 'private-operation', cursor: 'private-cursor', limit: 10 };
    const cases = [
      ['getLoggingState', [], ['get_logging_state'], state],
      ['setLoggingEnabled', [false], ['set_logging_enabled', { enabled: false }], state],
      ['queryLogs', [query], ['query_logs', { query }], page],
      ['openLogsDirectory', [], ['open_logs_directory'], undefined],
      ['clearLogs', [], ['clear_logs'], state],
    ];
    const requests = cases.map(([method, args, expected]) => {
      const request = store[method](...args);
      assert.deepEqual(calls.at(-1), expected);
      assert.equal(JSON.stringify(store), before);
      return request;
    });
    finish(undefined);
    const results = await Promise.all(requests);
    for (const [index, result] of results.entries()) assert.equal(result, cases[index][3]);
    assert.equal(JSON.stringify(store), before);
    failure = { code: 'conflict', message: 'private failure', detail: { path: '/private/logs' } };
    for (const [method, args, expected] of cases) {
      const count = calls.length;
      await assert.rejects(store[method](...args), (cause) => cause === failure);
      assert.equal(calls.length, count + 1);
      assert.deepEqual(calls.at(-1), expected);
      assert.equal(JSON.stringify(store), before);
    }
  } finally {
    await vite.close();
  }
});

test('global initialization and refresh never prefetch logging or add logging preferences', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    const calls = [];
    const preferences = { theme: 'dark', locale: 'en', defaultPlugin: 'native' };
    const adapter = createTauriAdapter(async (...args) => {
      calls.push(args);
      if (args[0] === 'load_app_state') return { providers: [], agents: [], groups: [], preferences };
      assert.equal(args[0], 'opencode_list_models');
      return [];
    });
    const store = createConfigStore(adapter);
    await store.refresh();
    assert.deepEqual(calls, [
      ['opencode_list_models', { provider: undefined }],
      ['load_app_state'],
      ['opencode_list_models', { provider: undefined }],
      ['load_app_state'],
    ]);
    assert.deepEqual(store.preferences, preferences);
    assert.equal(store.loading, false);
  } finally {
    await vite.close();
  }
});
