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

const state = (id) => ({
  providers: [],
  agents: [],
  groups: [{ id }],
  preferences: { theme: 'dark', locale: 'en' },
});

test('startup, save, and full refresh use the correct splash and reload paths', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    const states = [];
    const catalogs = [];
    const save = deferred();
    const store = createConfigStore({
      loadAppState() {
        const request = deferred();
        states.push(request);
        return request.promise;
      },
      opencodeListModels() {
        const request = deferred();
        catalogs.push(request);
        return request.promise;
      },
      saveGroup() {
        return save.promise;
      },
    });

    assert.equal(store.splashLoading, true);
    assert.equal(store.loading, true);
    assert.equal(states.length, 1);
    assert.equal(catalogs.length, 1);
    states[0].resolve(state('initial'));
    await new Promise(setImmediate);
    assert.equal(store.loading, false);
    assert.equal(store.splashLoading, true);
    catalogs[0].resolve([]);
    await new Promise(setImmediate);
    assert.equal(store.splashLoading, false);
    assert.equal(store.groups[0].id, 'initial');

    const savedGroup = { id: 'saved' };
    const saving = store.saveGroup(savedGroup);
    assert.equal(store.splashLoading, false);
    assert.equal(states.length, 1);
    save.resolve();
    await new Promise(setImmediate);
    assert.equal(states.length, 2);
    assert.equal(catalogs.length, 2);
    assert.equal(store.loading, true);
    assert.equal(store.splashLoading, false);
    states[1].resolve(state('after-save'));
    await new Promise(setImmediate);
    assert.equal(store.loading, false);
    assert.equal(store.splashLoading, false);
    catalogs[1].resolve([]);
    await saving;
    assert.equal(store.groups[0].id, 'after-save');
    assert.equal(store.splashLoading, false);
    assert.equal(catalogs.length, 2);

    const refreshing = store.refreshAll();
    assert.equal(store.splashLoading, false);
    assert.equal(states.length, 3);
    assert.equal(catalogs.length, 3);
    states[2].resolve(state('refreshed'));
    await new Promise(setImmediate);
    assert.equal(store.groups[0].id, 'refreshed');
    assert.equal(store.splashLoading, false);
    const catalog = [{ modelId: 'new-model' }];
    catalogs[2].resolve(catalog);
    await refreshing;
    assert.deepEqual(store.catalog, catalog);
    assert.equal(store.splashLoading, false);
    assert.equal(store.refreshAllError, null);

    const failing = store.refreshAll();
    assert.equal(store.splashLoading, false);
    assert.equal(states.length, 4);
    assert.equal(catalogs.length, 4);
    states[3].resolve(state('after-failure'));
    catalogs[3].reject(new Error('catalog unavailable'));
    await assert.rejects(failing, { code: 'ipc_error', message: 'catalog unavailable' });
    assert.equal(store.groups[0].id, 'after-failure');
    assert.equal(store.catalogError?.message, 'catalog unavailable');
    assert.equal(store.refreshAllError?.message, 'catalog unavailable');
    assert.equal(store.splashLoading, false);
    assert.deepEqual(store.catalog, catalog);
  } finally {
    await vite.close();
  }
});

test('OpenCode reload invokes CLI before full refresh and skips refresh on failure', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
    const commands = [];
    let reload = deferred();
    let catalog;
    const adapter = createTauriAdapter((command) => {
      commands.push(command);
      if (command === 'load_app_state') return Promise.resolve(state('initial'));
      if (command === 'opencode_list_models') return catalog?.promise ?? Promise.resolve([]);
      if (command === 'opencode_reload') return reload.promise;
      throw new Error(`Unexpected command: ${command}`);
    });
    const store = createConfigStore(adapter);
    await new Promise(setImmediate);
    assert.deepEqual(commands, ['opencode_list_models', 'load_app_state']);

    const success = store.reloadOpencode();
    assert.equal(store.reloading, true);
    assert.equal(store.splashLoading, true);
    await store.reloadOpencode();
    assert.deepEqual(commands, ['opencode_list_models', 'load_app_state', 'opencode_reload']);
    catalog = deferred();
    reload.resolve();
    await new Promise(setImmediate);
    assert.equal(store.loading, false);
    assert.equal(store.reloading, true);
    assert.equal(store.splashLoading, true);
    catalog.resolve([]);
    await success;
    assert.deepEqual(commands, [
      'opencode_list_models',
      'load_app_state',
      'opencode_reload',
      'opencode_list_models',
      'load_app_state',
    ]);
    assert.equal(store.reloading, false);
    assert.equal(store.splashLoading, false);
    assert.equal(store.refreshAllError, null);

    reload = deferred();
    const failure = store.reloadOpencode();
    assert.equal(store.splashLoading, true);
    reload.reject({ code: 'configuration_failed', message: 'CLI reload failed' });
    await assert.rejects(failure, { code: 'configuration_failed', message: 'CLI reload failed' });
    assert.deepEqual(commands, [
      'opencode_list_models',
      'load_app_state',
      'opencode_reload',
      'opencode_list_models',
      'load_app_state',
      'opencode_reload',
    ]);
    assert.equal(store.refreshAllError?.message, 'CLI reload failed');
    assert.equal(store.splashLoading, false);
    assert.equal(store.reloading, false);

    reload = deferred();
    catalog = deferred();
    const catalogFailure = store.reloadOpencode();
    reload.resolve();
    await new Promise(setImmediate);
    assert.equal(store.splashLoading, true);
    catalog.reject(new Error('catalog unavailable'));
    await assert.rejects(catalogFailure, { code: 'ipc_error', message: 'catalog unavailable' });
    assert.equal(store.refreshAllError?.message, 'catalog unavailable');
    assert.equal(store.catalogError?.message, 'catalog unavailable');
    assert.equal(store.splashLoading, false);
    assert.equal(store.reloading, false);
  } finally {
    await vite.close();
  }
});

test('automatic refreshes update the catalog once without failing successful mutations', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    let catalogCalls = 0;
    let stateCalls = 0;
    let failCatalog = false;
    const operations = [
      ['createProvider', [{}]],
      ['updateProvider', [{}]],
      ['deleteProvider', ['provider']],
      ['createModel', ['provider', {}]],
      ['updateModel', ['provider', {}]],
      ['deleteModel', ['provider/model']],
      ['createAgent', [{}]],
      ['updateAgent', [{}]],
      ['deleteAgent', ['agent']],
      ['saveGroup', [{}]],
      ['deleteGroup', ['group']],
      ['switchGroup', ['group']],
    ];
    const store = createConfigStore({
      loadAppState() {
        stateCalls += 1;
        return Promise.resolve(state(`state-${stateCalls}`));
      },
      opencodeListModels() {
        catalogCalls += 1;
        return failCatalog ? Promise.reject(new Error('catalog unavailable')) : Promise.resolve([]);
      },
      ...Object.fromEntries(operations.map(([name]) => [name, () => Promise.resolve()])),
    });
    await new Promise(setImmediate);
    assert.equal(catalogCalls, 1);
    for (const [name, args] of operations) {
      failCatalog = name === 'createProvider' || name === 'switchGroup';
      await store[name](...args);
      assert.equal(catalogCalls, stateCalls, `${name} must refresh the catalog exactly once`);
      assert.equal(store.groups[0].id, `state-${stateCalls}`);
      assert.equal(store.catalogError?.message, failCatalog ? 'catalog unavailable' : undefined);
      assert.equal(store.error, null);
      assert.equal(store.draftRecovery, null);
    }
    assert.equal(store.splashLoading, false);
    failCatalog = true;
    const before = stateCalls;
    await store.reloadKeepingDraft();
    assert.equal(stateCalls, before + 1);
    assert.equal(catalogCalls, stateCalls);
    await store.discardDraftAndRefresh();
    assert.equal(stateCalls, before + 2);
    assert.equal(catalogCalls, stateCalls);
    assert.equal(store.splashLoading, false);
  } finally {
    await vite.close();
  }
});

test('startup catalog failure still closes splash without an unhandled rejection', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    const store = createConfigStore({
      loadAppState: () => Promise.resolve(state('initial')),
      opencodeListModels: () => Promise.reject(new Error('catalog unavailable')),
    });
    await new Promise(setImmediate);
    assert.equal(store.loading, false);
    assert.equal(store.splashLoading, false);
    assert.equal(store.catalogError?.message, 'catalog unavailable');
    assert.equal(store.groups[0].id, 'initial');
  } finally {
    await vite.close();
  }
});

test('conflict recovery snapshots nested drafts and keeps or discards them on refresh', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    let stateCalls = 0;
    const store = createConfigStore({
      loadAppState: () => Promise.resolve(state(`state-${++stateCalls}`)),
      opencodeListModels: () => Promise.resolve([]),
      saveGroup: () => Promise.reject({ code: 'conflict', message: 'Group changed on disk' }),
    });
    await new Promise(setImmediate);
    const draft = { id: 'draft', overrides: [{ model: { id: 'original' } }] };
    const initialResetVersion = store.formResetVersion;
    await assert.rejects(store.saveGroup(draft), { code: 'conflict', message: 'Group changed on disk' });
    assert.equal(store.draftRecovery?.operation, 'saveGroup');
    assert.equal(store.draftRecovery?.conflict, true);
    assert.deepEqual(store.draftRecovery?.payload, draft);
    assert.notStrictEqual(store.draftRecovery?.payload, draft);
    assert.notStrictEqual(store.draftRecovery?.payload.overrides, draft.overrides);
    draft.overrides[0].model.id = 'edited after conflict';

    await store.reloadKeepingDraft();
    assert.deepEqual(store.draftRecovery?.payload, {
      id: 'draft',
      overrides: [{ model: { id: 'original' } }],
    });
    assert.equal(store.error?.code, 'conflict');
    assert.equal(store.formResetVersion, initialResetVersion);
    assert.equal(store.groups[0].id, 'state-2');

    await store.discardDraftAndRefresh();
    assert.equal(store.draftRecovery, null);
    assert.equal(store.error, null);
    assert.equal(store.formResetVersion, initialResetVersion + 1);
    assert.equal(store.groups[0].id, 'state-3');
  } finally {
    await vite.close();
  }
});

test('switchGroup keeps a successful mutation when its catalog refresh fails', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    const switchRequest = deferred();
    let selected = 'initial';
    let failCatalog = false;
    const store = createConfigStore({
      loadAppState: () => Promise.resolve(state(selected)),
      opencodeListModels: () =>
        failCatalog ? Promise.reject(new Error('catalog unavailable')) : Promise.resolve([{ modelId: 'original' }]),
      switchGroup: (id) => {
        selected = id;
        return switchRequest.promise;
      },
    });
    await new Promise(setImmediate);
    const switching = store.switchGroup('second');
    assert.equal(store.switching, true);
    assert.equal(store.saving, true);
    switchRequest.resolve();
    await switching;
    assert.equal(store.groups[0].id, 'second');
    assert.equal(store.switching, false);
    assert.equal(store.saving, false);
    assert.equal(store.catalogError, null);
    assert.equal(store.error, null);

    failCatalog = true;
    await store.switchGroup('third');
    assert.equal(store.groups[0].id, 'third');
    assert.deepEqual(store.catalog, [{ modelId: 'original' }]);
    assert.equal(store.catalogError?.message, 'catalog unavailable');
    assert.equal(store.error, null);
    assert.equal(store.refreshAllError, null);
    assert.equal(store.draftRecovery, null);
    assert.equal(store.switching, false);
    assert.equal(store.saving, false);
    assert.equal(store.loading, false);
  } finally {
    await vite.close();
  }
});

test('latest catalog request owns catalog, loading, and errors despite stale completions', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    const requests = [];
    const store = createConfigStore({
      loadAppState: () => Promise.resolve(state('initial')),
      opencodeListModels: () => {
        const request = deferred();
        requests.push(request);
        return request.promise;
      },
    });
    requests[0].resolve([{ modelId: 'startup' }]);
    await new Promise(setImmediate);

    const older = store.refreshCatalog();
    const newer = store.refreshCatalog();
    requests[2].resolve([{ modelId: 'latest' }]);
    await newer;
    requests[1].resolve([{ modelId: 'stale' }]);
    await older;
    assert.deepEqual(store.catalog, [{ modelId: 'latest' }]);
    assert.equal(store.catalogLoading, false);

    const staleFailure = store.refreshCatalog();
    const latest = store.refreshCatalog();
    requests[4].resolve([{ modelId: 'newest' }]);
    await latest;
    requests[3].reject(new Error('stale failure'));
    await assert.rejects(staleFailure, { code: 'ipc_error', message: 'stale failure' });
    assert.equal(store.catalogError, null);
    assert.equal(store.catalogLoading, false);
    assert.deepEqual(store.catalog, [{ modelId: 'newest' }]);

    const failed = store.refreshCatalog();
    requests[5].reject(new Error('latest failure'));
    await assert.rejects(failed, { code: 'ipc_error', message: 'latest failure' });
    assert.equal(store.catalogError?.message, 'latest failure');
    assert.equal(store.catalogLoading, false);
    assert.deepEqual(store.catalog, [{ modelId: 'newest' }]);
  } finally {
    await vite.close();
  }
});
