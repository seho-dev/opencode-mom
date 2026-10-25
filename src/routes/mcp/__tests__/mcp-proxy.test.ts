import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { tick } from 'svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { goto } from '$app/navigation';
import { page } from '$app/state';
import type { ConfigStore } from '$src/config/store.svelte.js';
import { toast } from '$src/shell/toast.svelte.js';
import Harness from '$src/test-utils/harness.svelte';
import type { McpProxyServerDto, McpProxyStateDto } from '$src/types/mcp-proxy.js';
import McpPage from '../+page.svelte';
import ProxyPanel from '../ProxyPanel.svelte';
import ProxyServerForm from '../ProxyServerForm.svelte';
import ProxyNewPage from '../proxy/new/+page.svelte';

vi.mock('$src/shell/toast.svelte.js', () => ({ toast: vi.fn() }));

vi.mock('$app/navigation', () => ({
  goto: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('$app/state', async () => {
  const { createSubscriber } = await import('svelte/reactivity');
  let notify: () => void = () => {};
  const track = createSubscriber((update) => {
    notify = update;
  });
  const state = {
    url: new URL('http://localhost/mcp'),
    params: {},
  };
  return {
    page: {
      get url() {
        track();
        return state.url;
      },
      set url(value: URL) {
        state.url = value;
        notify();
      },
      get params() {
        track();
        return state.params;
      },
      set params(value: Record<string, string>) {
        state.params = value;
        notify();
      },
    },
  };
});

const defaultProxyState: McpProxyStateDto = {
  actual: true,
  desired: true,
  actualState: 'running',
  desiredState: true,
  boundAddr: '127.0.0.1:23456',
  port: 23456,
  allowLan: false,
  totalServices: 1,
  enabledServices: 1,
  activeUnits: 2,
  activeSessions: 7,
  availableUnits: 30,
  maxUnits: 32,
  securityErrors: [],
  services: [
    {
      id: 'fs-server',
      name: 'Filesystem Server',
      enabled: true,
      protocol: 'legacy',
      activeSessions: 2,
    },
  ],
  revision: 3,
  generation: 1,
};

const fullServer: McpProxyServerDto = {
  id: 'fs-server',
  name: 'Filesystem Server',
  enabled: true,
  command: ['npx', '-y', '@modelcontextprotocol/server-filesystem', '/tmp'],
  cwd: '/tmp',
  env: { DEBUG: '1' },
  protocol: 'legacy',
  startupTimeout: 30,
  executionTimeout: 300,
};

function makeConfig(overrides: Record<string, unknown> = {}) {
  return {
    preferences: { locale: 'en', theme: 'dark' },
    selectedProjectId: null,
    saving: false,
    listMcps: vi.fn().mockResolvedValue({ data: [], diagnostics: [] }),
    getMcpProxyState: vi.fn().mockResolvedValue(defaultProxyState),
    getMcpProxySettings: vi.fn().mockResolvedValue({
      enabled: true,
      idleTimeout: 300,
      maxUnits: 16,
      revision: 2,
    }),
    saveMcpProxySettings: vi.fn().mockResolvedValue(undefined),
    getMcpProxyServer: vi.fn().mockResolvedValue(fullServer),
    createMcpProxyServer: vi.fn().mockResolvedValue(fullServer),
    updateMcpProxyServer: vi.fn().mockResolvedValue(fullServer),
    deleteMcpProxyServer: vi.fn().mockResolvedValue(undefined),
    importMcpProxyServer: vi.fn().mockResolvedValue(fullServer),
    closeMcpProxyUnit: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  } as unknown as ConfigStore;
}

beforeEach(() => {
  page.url = new URL('http://localhost/mcp');
  vi.mocked(goto).mockClear();
  vi.mocked(toast).mockClear();
});

afterEach(() => {
  vi.useRealTimers();
});

test.each(['edit', 'toggle'] as const)(
  '%s keeps its lookup revision across polling and ignores duplicate clicks',
  async (action) => {
    let resolve!: (value: McpProxyServerDto) => void;
    const pending = new Promise<McpProxyServerDto>((yes) => {
      resolve = yes;
    });
    const getMcpProxyServer = vi.fn().mockReturnValue(pending);
    const getMcpProxyState = vi
      .fn()
      .mockResolvedValueOnce(defaultProxyState)
      .mockResolvedValueOnce({ ...defaultProxyState, revision: 4 })
      .mockResolvedValue({ ...defaultProxyState, revision: 5 });
    const config = makeConfig({ getMcpProxyServer, getMcpProxyState });
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    render(ProxyPanel, {}, { wrapper: Harness, wrapperProps: { config } });
    await vi.advanceTimersByTimeAsync(0);
    await tick();
    const button = screen.getByRole('button', { name: action === 'edit' ? 'Edit' : 'Disabled' });
    await fireEvent.click(button);
    await fireEvent.click(button);
    expect(getMcpProxyServer).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(2000);
    await tick();
    expect(screen.getByText('Revision: 4')).toBeDefined();

    resolve(fullServer);
    await pending;
    await tick();
    if (action === 'edit') {
      await vi.advanceTimersByTimeAsync(2000);
      await tick();
      expect(screen.getByText('Revision: 5')).toBeDefined();
      await fireEvent.click(screen.getByRole('button', { name: 'Save Server' }));
    }
    expect(config.updateMcpProxyServer).toHaveBeenCalledExactlyOnceWith(
      { ...fullServer, enabled: action === 'edit' },
      3,
    );
  },
);

test('status stays loading until readback and becomes unknown after a failed refresh', async () => {
  let resolve!: (value: McpProxyStateDto) => void;
  const pending = new Promise<McpProxyStateDto>((yes) => {
    resolve = yes;
  });
  const config = makeConfig({
    getMcpProxyState: vi.fn().mockReturnValueOnce(pending).mockRejectedValue(new Error('Read failed')),
  });
  render(ProxyPanel, {}, { wrapper: Harness, wrapperProps: { config } });
  expect(screen.getAllByText('Loading configuration...').length).toBeGreaterThan(0);
  expect(screen.queryByText('Stopped')).toBeNull();
  resolve(defaultProxyState);
  await screen.findByText('Running');
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh' }));
  await screen.findByText('Read failed');
  expect(screen.getByText('Status unknown')).toBeDefined();
  expect(screen.queryByText('Running')).toBeNull();
  expect(screen.queryByText('Stopped')).toBeNull();
});

test('failed initial state read stays unknown rather than stopped', async () => {
  const config = makeConfig({ getMcpProxyState: vi.fn().mockRejectedValue(new Error('Read failed')) });
  render(ProxyPanel, {}, { wrapper: Harness, wrapperProps: { config } });
  await screen.findByText('Read failed');
  expect(screen.getByText('Status unknown')).toBeDefined();
  expect(screen.queryByText('Stopped')).toBeNull();
});

test('gateway does not save settings read after unmount', async () => {
  const settings = { enabled: true, idleTimeout: 300, maxUnits: 16, revision: 2 };
  let resolve!: (value: typeof settings) => void;
  const pending = new Promise<typeof settings>((yes) => {
    resolve = yes;
  });
  const config = makeConfig({ getMcpProxySettings: vi.fn().mockReturnValue(pending) });
  const { unmount } = render(ProxyPanel, {}, { wrapper: Harness, wrapperProps: { config } });
  await screen.findByText('Filesystem Server');
  await fireEvent.click(screen.getByRole('checkbox', { name: 'Enable MCP Gateway' }));
  unmount();
  resolve(settings);
  await pending;
  await tick();
  expect(config.saveMcpProxySettings).not.toHaveBeenCalled();
  expect(toast).not.toHaveBeenCalled();
});

test.each(['edit', 'toggle'] as const)('%s ignores a server lookup completed after unmount', async (action) => {
  let resolve!: (value: McpProxyServerDto) => void;
  const pending = new Promise<McpProxyServerDto>((yes) => {
    resolve = yes;
  });
  const getMcpProxyServer = vi.fn().mockReturnValue(pending);
  const config = makeConfig({ getMcpProxyServer });
  const { unmount } = render(ProxyPanel, {}, { wrapper: Harness, wrapperProps: { config } });
  await screen.findByText('Filesystem Server');
  expect(screen.getByText('Active Units: 2 / 32')).toBeDefined();
  expect(screen.getByText('Active Sessions: 7')).toBeDefined();
  await fireEvent.click(screen.getByRole('button', { name: action === 'edit' ? 'Edit' : 'Disabled' }));
  expect(getMcpProxyServer).toHaveBeenCalledWith('fs-server');
  unmount();
  resolve(fullServer);
  await pending;
  await tick();
  expect(config.updateMcpProxyServer).not.toHaveBeenCalled();
  expect(screen.queryByRole('dialog')).toBeNull();
  expect(toast).not.toHaveBeenCalled();
});

test('does not toast an edit lookup error after unmount', async () => {
  let reject!: (reason: unknown) => void;
  const pending = new Promise<McpProxyServerDto>((_, no) => {
    reject = no;
  });
  const config = makeConfig({ getMcpProxyServer: vi.fn().mockReturnValue(pending) });
  const { unmount } = render(ProxyPanel, {}, { wrapper: Harness, wrapperProps: { config } });
  await screen.findByText('Filesystem Server');
  await fireEvent.click(screen.getByRole('button', { name: 'Edit' }));
  unmount();
  reject(new Error('Lookup failed'));
  await pending.catch(() => {});
  await tick();
  expect(toast).not.toHaveBeenCalled();
});

test('server enabled switch binds its named checkbox to the saved draft', async () => {
  const config = makeConfig();
  render(
    ProxyServerForm,
    { dialog: false, server: fullServer, expectedRevision: 3 },
    { wrapper: Harness, wrapperProps: { config } },
  );
  const checkbox = screen.getByRole('checkbox', { name: 'Enabled' }) as HTMLInputElement;
  expect(checkbox.id).toBe('proxy-srv-enabled');
  expect(checkbox.name).toBe('proxy-srv-enabled');
  expect(checkbox.checked).toBe(true);
  await fireEvent.click(checkbox);
  expect(checkbox.checked).toBe(false);
  await fireEvent.click(screen.getByRole('button', { name: 'Save Server' }));
  expect(config.updateMcpProxyServer).toHaveBeenCalledExactlyOnceWith({ ...fullServer, enabled: false }, 3);
});

test('create does not mutate or navigate when its revision read finishes after unmount', async () => {
  let resolve!: (value: McpProxyStateDto) => void;
  const pending = new Promise<McpProxyStateDto>((yes) => {
    resolve = yes;
  });
  const config = makeConfig({ getMcpProxyState: vi.fn().mockReturnValue(pending) });
  const { unmount } = render(ProxyNewPage, {}, { wrapper: Harness, wrapperProps: { config } });
  await fireEvent.input(screen.getByLabelText('Server ID'), { target: { value: 'custom-mcp' } });
  await fireEvent.input(screen.getByRole('textbox', { name: /Command/ }), { target: { value: 'npx' } });
  await fireEvent.submit(screen.getByRole('form'));
  await fireEvent.submit(screen.getByRole('form'));
  expect(config.getMcpProxyState).toHaveBeenCalledTimes(1);
  unmount();
  resolve(defaultProxyState);
  await pending;
  await tick();
  expect(config.createMcpProxyServer).not.toHaveBeenCalled();
  expect(goto).not.toHaveBeenCalled();
  expect(toast).not.toHaveBeenCalled();
});

test.each(['create', 'update'] as const)(
  '%s completion after unmount does not call onSaved or navigate',
  async (action) => {
    let resolve!: (value: McpProxyServerDto) => void;
    const pending = new Promise<McpProxyServerDto>((yes) => {
      resolve = yes;
    });
    const mutation = vi.fn().mockReturnValue(pending);
    const onSaved = vi.fn();
    const config = makeConfig({ [action === 'create' ? 'createMcpProxyServer' : 'updateMcpProxyServer']: mutation });
    const { unmount } =
      action === 'create'
        ? render(ProxyNewPage, {}, { wrapper: Harness, wrapperProps: { config } })
        : render(
            ProxyServerForm,
            { dialog: false, server: fullServer, expectedRevision: 3, onSaved },
            { wrapper: Harness, wrapperProps: { config } },
          );
    if (action === 'create') {
      await fireEvent.input(screen.getByLabelText('Server ID'), { target: { value: 'custom-mcp' } });
      await fireEvent.input(screen.getByRole('textbox', { name: /Command/ }), { target: { value: 'npx' } });
    }
    await fireEvent.submit(screen.getByRole('form'));
    await waitFor(() => expect(mutation).toHaveBeenCalledTimes(1));
    await fireEvent.submit(screen.getByRole('form'));
    expect(mutation).toHaveBeenCalledTimes(1);
    unmount();
    resolve(fullServer);
    await pending;
    await tick();
    expect(onSaved).not.toHaveBeenCalled();
    expect(goto).not.toHaveBeenCalled();
    expect(toast).not.toHaveBeenCalled();
  },
);

test('import ignores duplicate submissions and late completion after unmount', async () => {
  let resolve!: (value: McpProxyServerDto) => void;
  const pending = new Promise<McpProxyServerDto>((yes) => {
    resolve = yes;
  });
  const config = makeConfig({
    listMcps: vi.fn().mockResolvedValue({
      data: [{ name: 'local-fs', type: 'local', sourcePath: '/tmp/opencode.json' }],
      diagnostics: [],
    }),
    importMcpProxyServer: vi.fn().mockReturnValue(pending),
  });
  const { unmount } = render(ProxyNewPage, {}, { wrapper: Harness, wrapperProps: { config } });
  await fireEvent.click(screen.getByRole('button', { name: 'Import Server' }));
  await screen.findByRole('button', { name: 'local-fs (local)' });
  await fireEvent.submit(screen.getByRole('form'));
  await fireEvent.submit(screen.getByRole('form'));
  expect(config.importMcpProxyServer).toHaveBeenCalledExactlyOnceWith({
    scope: 'global',
    serverName: 'local-fs',
    expectedSourcePath: '/tmp/opencode.json',
  });
  unmount();
  resolve(fullServer);
  await pending;
  await tick();
  expect(config.createMcpProxyServer).not.toHaveBeenCalled();
  expect(goto).not.toHaveBeenCalled();
  expect(toast).not.toHaveBeenCalled();
});

test('server form preserves argument boundaries and literal environment values', async () => {
  const server: McpProxyServerDto = {
    ...fullServer,
    command: ['node', '', '""', "''", '  spaced  ', '', ''],
    env: Object.fromEntries([
      ['SPACED', '  value  '],
      ['EMPTY', ''],
      ['EQUALS', 'a=b=c'],
      ['QUOTED', '"literal value"'],
      ['app.config-key', 'ok'],
      ['__proto__', 'literal'],
    ]),
  };
  const config = makeConfig();
  render(
    ProxyServerForm,
    {
      open: true,
      server,
      expectedRevision: 3,
      onSaved: vi.fn(),
    },
    { wrapper: Harness, wrapperProps: { config } },
  );
  await screen.findByRole('dialog');
  const env = screen.getByLabelText('Environment Variables') as HTMLTextAreaElement;
  await fireEvent.input(env, { target: { value: `${env.value}\n  # comment\n  MORE =  more  ` } });
  await fireEvent.click(screen.getByRole('button', { name: 'Save Server' }));
  await waitFor(() =>
    expect(config.updateMcpProxyServer).toHaveBeenCalledWith(
      {
        ...server,
        env: { ...server.env, MORE: '  more  ' },
      },
      3,
    ),
  );
});

test('MCP page displays tabs and switches between config and proxy', async () => {
  const config = makeConfig();
  render(McpPage, {}, { wrapper: Harness, wrapperProps: { config } });

  expect(screen.getByRole('tab', { name: 'Configuration' })).toBeDefined();
  expect(screen.getByRole('tab', { name: 'MCP Proxy' })).toBeDefined();

  // Click MCP Proxy tab
  const proxyTab = screen.getByRole('tab', { name: 'MCP Proxy' });
  await fireEvent.click(proxyTab);

  expect(vi.mocked(goto)).toHaveBeenCalled();
  const calledUrl = vi.mocked(goto).mock.calls[0]?.[0] as string;
  expect(calledUrl).toContain('tab=proxy');
});

test('ProxyPanel renders running status, server list and supports close unit', async () => {
  page.url = new URL('http://localhost/mcp?tab=proxy');
  const closeMcpProxyUnit = vi.fn().mockResolvedValue(undefined);
  const config = makeConfig({ closeMcpProxyUnit });

  render(McpPage, {}, { wrapper: Harness, wrapperProps: { config } });

  await waitFor(() => {
    expect(config.getMcpProxyState).toHaveBeenCalled();
  });

  expect(await screen.findByText('127.0.0.1:23456')).toBeDefined();
  expect(screen.getByText('Filesystem Server')).toBeDefined();
  expect(screen.getByText('fs-server')).toBeDefined();

  // Click Close Unit button
  const buttons = screen.getAllByRole('button', { name: /Close Unit/i });
  const triggerBtn = buttons[0];
  if (!triggerBtn) throw new Error('Trigger button not found');
  await fireEvent.click(triggerBtn);

  // Modal appears with input
  const input = screen.getByRole('textbox', { name: 'Enter unit or session ID to close:' });
  await fireEvent.input(input, { target: { value: 'unit-abc' } });

  const modalButtons = screen.getAllByRole('button', { name: /Close Unit/i });
  const confirmBtn = modalButtons[1];
  if (!confirmBtn) throw new Error('Confirm button not found');
  await fireEvent.click(confirmBtn);

  await waitFor(() => {
    expect(closeMcpProxyUnit).toHaveBeenCalledWith({
      unitId: 'unit-abc',
      expectedGeneration: 1,
    });
  });
});

test('ProxyPanel master switch toggles gateway enablement', async () => {
  page.url = new URL('http://localhost/mcp?tab=proxy');
  const getMcpProxySettings = vi.fn().mockResolvedValue({
    enabled: true,
    idleTimeout: 300,
    maxUnits: 16,
    revision: 2,
  });
  const saveMcpProxySettings = vi.fn().mockResolvedValue(undefined);
  const config = makeConfig({ getMcpProxySettings, saveMcpProxySettings });

  render(McpPage, {}, { wrapper: Harness, wrapperProps: { config } });
  await waitFor(() => expect(config.getMcpProxyState).toHaveBeenCalled());

  const masterSwitch = screen.getByRole('checkbox', { name: 'Enable MCP Gateway' });
  expect((masterSwitch as HTMLInputElement).checked).toBe(true);

  await fireEvent.click(masterSwitch);
  await waitFor(() => {
    expect(saveMcpProxySettings).toHaveBeenCalledWith(expect.objectContaining({ enabled: false, revision: 2 }), 2);
  });
});

test('Add Server button links to /mcp/proxy/new', async () => {
  page.url = new URL('http://localhost/mcp?tab=proxy');
  const config = makeConfig();
  render(McpPage, {}, { wrapper: Harness, wrapperProps: { config } });
  await waitFor(() => expect(config.getMcpProxyState).toHaveBeenCalled());

  const addLink = screen.getByRole('link', { name: 'Add Server' });
  expect(addLink.getAttribute('href')).toBe('/mcp/proxy/new');
});

test('ProxyNewPage creates proxy server and navigates on save', async () => {
  const createMcpProxyServer = vi.fn().mockResolvedValue(fullServer);
  const config = makeConfig({ createMcpProxyServer });

  render(ProxyNewPage, {}, { wrapper: Harness, wrapperProps: { config } });

  await screen.findByRole('heading', { name: 'Add Proxy Server' });
  const idInput = screen.getByLabelText('Server ID');
  const cmdInput = screen.getByLabelText('Command & Arguments');

  await fireEvent.input(idInput, { target: { value: 'custom-mcp' } });
  await fireEvent.input(cmdInput, { target: { value: 'npx\n-y\ncustom-pkg' } });

  const saveBtn = screen.getByRole('button', { name: 'Save Server' });
  await fireEvent.click(saveBtn);

  await waitFor(() => {
    expect(createMcpProxyServer).toHaveBeenCalledWith(
      expect.objectContaining({
        id: 'custom-mcp',
        command: ['npx', '-y', 'custom-pkg'],
      }),
      3,
    );
  });
  expect(goto).toHaveBeenCalledWith('/mcp?tab=proxy');
});
