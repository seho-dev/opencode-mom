import assert from 'node:assert/strict';
import test from 'node:test';
import { withServer } from './vite-harness.mjs';

const state = () => ({ providers: [], agents: [], groups: [], preferences: { theme: 'dark', locale: 'en' } });

test('configuration boundaries reject invalid data and preserve failure ownership', async (t) => {
  await withServer(async (vite) => {
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');

    await t.test('group decoding rejects unknown and non-wire types at receipt', async () => {
      for (const type of ['native', 'omo', 'unknown-type', 'toString', null]) {
        const adapter = createTauriAdapter(async () => ({ ...state(), groups: [{ id: 'group', type }] }));
        await assert.rejects(adapter.loadAppState(), { code: 'configuration_failed' });
      }
    });

    await t.test('failed full state reads reject without discarding recovery or resetting forms', async () => {
      const store = createConfigStore(
        {
          createProvider: async () => {
            throw new Error('draft failed');
          },
          loadAppState: async () => {
            throw new Error('state unavailable');
          },
          opencodeListModels: async () => [],
        },
        true,
        false,
      );
      await assert.rejects(store.createProvider({ name: 'draft', models: {} }));
      const recovery = store.draftRecovery;
      const version = store.formResetVersion;
      await assert.rejects(store.refreshAll(), { message: 'state unavailable' });
      assert.equal(store.error.message, 'state unavailable');
      assert.strictEqual(store.draftRecovery, recovery);
      assert.equal(store.formResetVersion, version);
      assert.equal(store.loading, false);
      assert.equal(store.catalogLoading, false);
    });

    await t.test('explicit catalog failures have one global feedback owner', async () => {
      const store = createConfigStore(
        {
          loadAppState: async () => state(),
          opencodeListModels: async () => {
            throw new Error('catalog unavailable');
          },
        },
        true,
        false,
      );
      await assert.rejects(store.refreshAll(), { message: 'catalog unavailable' });
      assert.equal(store.error.message, 'catalog unavailable');
      assert.equal(store.catalogError, null);
    });

    await t.test('a completed read cannot clear recovery created while it was pending', async () => {
      let finishRead;
      const store = createConfigStore(
        {
          loadAppState: () =>
            new Promise((resolve) => {
              finishRead = resolve;
            }),
          createProvider: async () => {
            throw new Error('new draft failed');
          },
        },
        false,
        false,
      );
      const reading = store.refreshAll();
      await assert.rejects(store.createProvider({ name: 'new-draft', models: {} }));
      const recovery = store.draftRecovery;
      finishRead(state());
      await reading;
      assert.strictEqual(store.draftRecovery, recovery);
      assert.equal(store.formResetVersion, 0);
    });

    await t.test('a committed mutation remains successful when its state readback fails', async () => {
      const store = createConfigStore(
        {
          createProvider: async (value) => value,
          loadAppState: async () => {
            throw new Error('readback unavailable');
          },
        },
        false,
        false,
      );
      await assert.doesNotReject(store.createProvider({ name: 'saved', models: {} }));
      assert.equal(store.error.message, 'readback unavailable');
      assert.equal(store.draftRecovery, null);
      assert.equal(store.saving, false);
    });

    await t.test('reload IPC failures stay observable after an independent state refresh', async () => {
      let failReload;
      const store = createConfigStore(
        {
          opencodeReload: () =>
            new Promise((_, reject) => {
              failReload = reject;
            }),
          loadAppState: async () => state(),
        },
        false,
        false,
      );
      const reloading = store.reloadOpencode();
      await store.refresh();
      const rejected = assert.rejects(reloading, { message: 'reload unavailable' });
      failReload(new Error('reload unavailable'));
      await rejected;
      assert.equal(store.error.message, 'reload unavailable');
      assert.equal(store.reloading, false);
      assert.equal(store.splashLoading, false);
    });
  });
});
