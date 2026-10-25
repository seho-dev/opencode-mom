import assert from 'node:assert/strict';
import test from 'node:test';
import { createServer } from 'vite';

test('MCP proxy adapter maps commands and arguments accurately to Tauri IPC', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');

    const calls = [];
    const adapter = createTauriAdapter(async (command, args) => {
      calls.push([command, args]);
      if (command === 'get_mcp_proxy_state') {
        return {
          actual: true,
          desired: true,
          actualState: 'running',
          desiredState: true,
          boundAddr: '127.0.0.1:23456',
          port: 23456,
          allowLan: false,
          totalServices: 1,
          enabledServices: 1,
          activeUnits: 0,
          activeSessions: 0,
          availableUnits: 32,
          maxUnits: 32,
          securityErrors: [],
          services: [],
          revision: 1,
          generation: 1,
        };
      }
      if (command === 'get_mom_http_settings') {
        return { port: 18765, allowLan: false };
      }
      if (command === 'set_mom_http_settings') {
        return undefined;
      }
      if (command === 'get_mcp_proxy_settings') {
        return {
          enabled: true,
          idleTimeout: 300,
          maxUnits: 32,
          revision: 1,
        };
      }
      if (command === 'save_mcp_proxy_settings') {
        return undefined;
      }
      if (command === 'rotate_mcp_proxy_key') {
        return 'test-secret-token-12345';
      }
      if (command === 'get_mcp_proxy_server') {
        return {
          id: args.id,
          name: 'Test Server',
          enabled: true,
          command: ['echo', 'hello'],
          cwd: '/tmp',
          env: {},
          protocol: 'auto',
          startupTimeout: 30,
          executionTimeout: 300,
        };
      }
      if (command === 'create_mcp_proxy_server' || command === 'update_mcp_proxy_server') {
        return args.server;
      }
      if (command === 'delete_mcp_proxy_server') {
        return undefined;
      }
      if (command === 'import_mcp_proxy_server') {
        return {
          id: 'imported',
          name: args.request.serverName,
          enabled: true,
          command: ['npx', 'server'],
          cwd: '',
          env: {},
          protocol: 'auto',
          startupTimeout: 30,
          executionTimeout: 300,
        };
      }
      if (command === 'close_mcp_proxy_unit') {
        return undefined;
      }
      throw new Error(`Unexpected command: ${command}`);
    });

    // 1. getMcpProxyState
    const state = await adapter.getMcpProxyState();
    assert.equal(state.actualState, 'running');
    assert.equal(state.desiredState, true);
    assert.equal(state.activeUnits, 0);
    assert.equal(state.activeSessions, 0);
    assert.deepEqual(calls.at(-1), ['get_mcp_proxy_state', undefined]);

    const httpSettings = await adapter.getMomHttpSettings();
    assert.deepEqual(httpSettings, { port: 18765, allowLan: false });
    assert.deepEqual(calls.at(-1), ['get_mom_http_settings', undefined]);
    await adapter.setMomHttpSettings({ port: 23456, allowLan: true });
    assert.deepEqual(calls.at(-1), ['set_mom_http_settings', { settings: { port: 23456, allowLan: true } }]);

    // 2. getMcpProxySettings
    const settings = await adapter.getMcpProxySettings();
    assert.equal(settings.idleTimeout, 300);
    assert.equal('port' in settings, false);
    assert.deepEqual(calls.at(-1), ['get_mcp_proxy_settings', undefined]);

    // 3. saveMcpProxySettings
    await adapter.saveMcpProxySettings(settings, 1);
    assert.deepEqual(calls.at(-1), ['save_mcp_proxy_settings', { settings, expectedRevision: 1 }]);

    // 4. rotateMcpProxyKey
    const key = await adapter.rotateMcpProxyKey();
    assert.equal(key, 'test-secret-token-12345');
    assert.deepEqual(calls.at(-1), ['rotate_mcp_proxy_key', undefined]);

    // 5. getMcpProxyServer
    const srv = await adapter.getMcpProxyServer('srv-1');
    assert.equal(srv.id, 'srv-1');
    assert.deepEqual(calls.at(-1), ['get_mcp_proxy_server', { id: 'srv-1' }]);

    // 6. createMcpProxyServer
    const created = await adapter.createMcpProxyServer(srv, 2);
    assert.equal(created.id, 'srv-1');
    assert.deepEqual(calls.at(-1), ['create_mcp_proxy_server', { server: srv, expectedRevision: 2 }]);

    // 7. updateMcpProxyServer
    const updated = await adapter.updateMcpProxyServer(srv, 3);
    assert.equal(updated.id, 'srv-1');
    assert.deepEqual(calls.at(-1), ['update_mcp_proxy_server', { server: srv, expectedRevision: 3 }]);

    // 8. deleteMcpProxyServer
    await adapter.deleteMcpProxyServer('srv-1', 4);
    assert.deepEqual(calls.at(-1), ['delete_mcp_proxy_server', { id: 'srv-1', expectedRevision: 4 }]);

    // 9. importMcpProxyServer
    const imported = await adapter.importMcpProxyServer({
      sourceScope: 'global',
      serverName: 'fs',
      expectedSourcePath: '/tmp/opencode.json',
    });
    assert.equal(imported.name, 'fs');
    assert.deepEqual(calls.at(-1), [
      'import_mcp_proxy_server',
      {
        request: {
          sourceScope: 'global',
          serverName: 'fs',
          expectedSourcePath: '/tmp/opencode.json',
        },
      },
    ]);

    // 10. closeMcpProxyUnit
    await adapter.closeMcpProxyUnit({ unitId: 'unit-99', expectedGeneration: 5 });
    assert.deepEqual(calls.at(-1), ['close_mcp_proxy_unit', { request: { unitId: 'unit-99', expectedGeneration: 5 } }]);
  } finally {
    await vite.close();
  }
});

test('ConfigStore MCP proxy facades are lazy pass-throughs and do not trigger run() or draftRecovery', async () => {
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');

    let failOnSave = false;
    const httpSettings = { port: 18765, allowLan: false };
    let savedHttpSettings;
    let failOnHttpSave = false;
    const adapter = {
      loadAppState: async () => ({
        groups: [],
        projects: [],
        selectedGroupId: null,
        selectedProjectId: null,
        preferences: { theme: 'dark', locale: 'en' },
        providers: [],
        agents: [],
      }),
      getMcpProxyState: async () => ({
        actual: false,
        desired: false,
        actualState: 'stopped',
        desiredState: false,
        boundAddr: null,
        port: 23456,
        allowLan: false,
        totalServices: 0,
        enabledServices: 0,
        activeUnits: 0,
        activeSessions: 0,
        availableUnits: 32,
        maxUnits: 32,
        securityErrors: [],
        services: [],
        revision: 1,
        generation: 1,
      }),
      getMomHttpSettings: async () => httpSettings,
      setMomHttpSettings: async (settings) => {
        if (failOnHttpSave) throw new Error('HTTP save failed');
        savedHttpSettings = settings;
      },
      getMcpProxySettings: async () => ({
        enabled: true,
        idleTimeout: 300,
        maxUnits: 32,
        revision: 1,
      }),
      saveMcpProxySettings: async () => {
        if (failOnSave) {
          const err = new Error('failed');
          err.code = 'validation_failed';
          throw err;
        }
      },
      rotateMcpProxyKey: async () => 'key-abc',
      getMcpProxyServer: async (id) => ({
        id,
        name: 'Srv',
        enabled: true,
        command: ['sh'],
        cwd: '',
        env: {},
        protocol: 'auto',
        startupTimeout: 30,
        executionTimeout: 300,
      }),
      createMcpProxyServer: async (s) => s,
      updateMcpProxyServer: async (s) => s,
      deleteMcpProxyServer: async () => undefined,
      importMcpProxyServer: async (req) => ({
        id: 'imp',
        name: req.serverName,
        enabled: true,
        command: ['sh'],
        cwd: '',
        env: {},
        protocol: 'auto',
        startupTimeout: 30,
        executionTimeout: 300,
      }),
      closeMcpProxyUnit: async () => undefined,
    };

    const store = createConfigStore(adapter, false, false);

    const state = await store.getMcpProxyState();
    assert.equal(state.actualState, 'stopped');
    assert.equal(state.desiredState, false);
    assert.equal(state.boundAddr, null);

    assert.equal(await store.getMomHttpSettings(), httpSettings);
    const nextHttpSettings = { port: 23456, allowLan: true };
    await store.setMomHttpSettings(nextHttpSettings);
    assert.equal(savedHttpSettings, nextHttpSettings);
    failOnHttpSave = true;
    await assert.rejects(store.setMomHttpSettings(nextHttpSettings), /HTTP save failed/);
    assert.equal(store.draftRecovery, null);
    assert.equal(store.saving, false);

    const settings = await store.getMcpProxySettings();
    assert.equal(settings.idleTimeout, 300);

    // Save should not set store draftRecovery on failure
    failOnSave = true;
    await assert.rejects(store.saveMcpProxySettings(settings, 1));
    assert.equal(store.draftRecovery, null);

    const key = await store.rotateMcpProxyKey();
    assert.equal(key, 'key-abc');
  } finally {
    await vite.close();
  }
});
