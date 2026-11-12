import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterAll, beforeAll, beforeEach, expect, test, vi } from 'vitest';
import { goto } from '$app/navigation';
import { page as routePage } from '$app/state';
import { makeScopedConfig } from '$src/components/__tests__/scope-config.js';
import { createTauriAdapter } from '$src/config/adapter.js';
import { type ConfigStore, createConfigStore } from '$src/config/store.svelte.js';
import { en } from '$src/i18n/dictionaries/en.js';
import Harness from '$src/test-utils/harness.svelte';
import type { McpServer } from '$src/types/mcp.js';
import McpList from '../+page.svelte';
import McpDetail from '../[id]/+page.svelte';
import { maskConfig, maskTarget } from '../config-view.js';
import DeleteMcpDialog from '../DeleteMcpDialog.svelte';
import McpForm from '../McpForm.svelte';
import McpNew from '../new/+page.svelte';

const page: { params: { id?: string }; url: URL } = routePage;

const scrollIntoView = Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'scrollIntoView');

beforeAll(() => {
  Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', {
    configurable: true,
    value: vi.fn(),
  });
});
afterAll(() => {
  if (scrollIntoView) Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', scrollIntoView);
  else Reflect.deleteProperty(HTMLElement.prototype, 'scrollIntoView');
});

vi.mock('$app/navigation', () => ({
  goto: vi.fn().mockResolvedValue(undefined),
}));
vi.mock('$app/state', async () => {
  const { createSubscriber } = await import('svelte/reactivity');
  let notify = () => {};
  const track = createSubscriber((update) => {
    notify = update;
  });
  let params = { id: 'docs/a b' };
  let url = new URL('http://localhost/mcp');

  return {
    page: {
      get url() {
        track();
        return url;
      },
      set url(value) {
        url = value;
        notify();
      },
      get params() {
        track();
        return params;
      },
      set params(value) {
        params = value;
        notify();
      },
    },
  };
});

const server = {
  name: 'docs/a b',
  type: 'remote',
  disabled: false,
  target: 'https://example.com/mcp',
  sourcePath: '/global/opencode.json',
  sourcePaths: ['/global/opencode.json', '/global/opencode.jsonc'],
  sourceSnapshot: ' opaque-source-v1 \r\n',

  config: {
    type: 'remote',
    url: 'https://example.com/mcp',
    headers: { Authorization: 'secret-header' },
    environment: { KEY: 'secret-env' },
    oauth: { client_id: 'public-id', client_secret: 'secret-oauth' },
    timeout: { startup: 5000, catalog: 10000, execution: 30000 },
  },
} satisfies McpServer;
const disabledUpdate = {
  name: server.name,
  disabled: true,
  expectedDisabled: false,
  expectedSourcePath: server.sourcePath,
  expectedSourceSnapshot: server.sourceSnapshot,
};
const remoteUrl =
  'https://private-user:private-pass@example.com/mcp?token=url-token&api%5Fkey=url-api&key=url-key&client_secret=url-secret&mode=stream';
const remote = {
  ...server,
  target: remoteUrl,
  config: { ...server.config, url: remoteUrl },
} satisfies McpServer;
const command = [
  'node',
  'server.js',
  '--token',
  'command token with spaces',
  '--api-key=command-api',
  '--password',
  'command password with spaces',
  'ACCESS_TOKEN=env-command-token',
  'KEY=env-command-key',
  '--port',
  '3000',
];
const local = {
  ...server,
  name: 'local',
  type: 'local',
  target: command.join(' '),
  config: { type: 'local', command },
} satisfies McpServer;
const credentialValues = [
  'private-user',
  'private-pass',
  'url-token',
  'url-api',
  'url-key',
  'url-secret',
  'command token with spaces',
  'command-api',
  'command password with spaces',
  'env-command-token',
  'env-command-key',
];

function makeConfig(overrides: Record<string, unknown> = {}) {
  return {
    preferences: { locale: 'en', theme: 'dark' },
    listMcps: vi.fn().mockResolvedValue({ data: [server], diagnostics: [] }),
    getMcp: vi.fn().mockResolvedValue(server),
    deleteMcp: vi.fn().mockResolvedValue(undefined),
    reloadOpencode: vi.fn(),
    ...overrides,
  } as unknown as ConfigStore;
}

beforeEach(() => {
  page.params = { id: server.name };
  page.url = new URL('http://localhost/mcp');
  vi.mocked(goto).mockClear();
});

test('list encodes detail links, keeps config status without the hint, and supports delete cancel/failure/retry', async () => {
  const deleteMcp = vi.fn().mockRejectedValueOnce(new Error('write failed')).mockResolvedValueOnce(undefined);
  const listMcps = vi
    .fn()
    .mockResolvedValueOnce({
      data: [server],
      diagnostics: ['Unreadable source'],
    })
    .mockResolvedValueOnce({ data: [], diagnostics: [] });
  const config = makeConfig({ deleteMcp, listMcps });

  render(McpList, {}, { wrapper: Harness, wrapperProps: { config } });

  const link = await screen.findByRole('link', { name: server.name });

  expect(link.getAttribute('href')).toBe('/mcp/docs%2Fa%20b');
  expect(screen.getByRole('link', { name: 'New MCP' }).getAttribute('href')).toBe('/mcp/new');
  expect(screen.getByRole('link', { name: `View ${server.name}` }).getAttribute('href')).toBe('/mcp/docs%2Fa%20b');
  expect(screen.getByRole('link', { name: `Edit ${server.name}` }).getAttribute('href')).toBe(
    '/mcp/docs%2Fa%20b?edit=1',
  );
  expect(screen.queryByText('Enabled')).toBeNull();
  expect(screen.queryByText(/Servers from global configuration|not live connection status/)).toBeNull();
  expect(screen.getByText('Unreadable source')).toBeTruthy();
  await fireEvent.input(screen.getByRole('textbox'), {
    target: { value: 'missing' },
  });
  expect(screen.getByText('No matches')).toBeTruthy();
  await fireEvent.input(screen.getByRole('textbox'), { target: { value: '' } });
  await fireEvent.click(screen.getByRole('button', { name: `Delete ${server.name}` }));

  let dialog = await screen.findByRole('dialog', { name: 'Delete MCP server' });

  expect(within(dialog).getByText('/global/opencode.jsonc')).toBeTruthy();
  await fireEvent.click(within(dialog).getByRole('button', { name: 'Cancel' }));
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect(deleteMcp).not.toHaveBeenCalled();
  await fireEvent.click(screen.getByRole('button', { name: `Delete ${server.name}` }));
  dialog = await screen.findByRole('dialog');
  await fireEvent.click(within(dialog).getByRole('button', { name: 'Delete' }));
  await within(dialog).findByRole('alert');
  expect(screen.getByRole('link', { name: server.name })).toBeTruthy();
  await fireEvent.click(within(dialog).getByRole('button', { name: 'Delete' }));
  await screen.findByText('Removed from global configuration. Reload OpenCode to apply.');
  await screen.findByText('No MCP servers in global configuration.');
  expect(deleteMcp).toHaveBeenNthCalledWith(2, server.name);
  expect(listMcps).toHaveBeenCalledTimes(2);
  expect(config.reloadOpencode).not.toHaveBeenCalled();
});

test('list read failure has retry and loading/empty states', async () => {
  const listMcps = vi
    .fn()
    .mockRejectedValueOnce(new Error('desktop unavailable'))
    .mockResolvedValueOnce({ data: [], diagnostics: [] });

  render(McpList, {}, { wrapper: Harness, wrapperProps: { config: makeConfig({ listMcps }) } });

  const alert = await screen.findByRole('alert');

  expect(alert.textContent).toContain('desktop unavailable');
  await fireEvent.click(within(alert).getByRole('button', { name: 'Retry' }));
  await screen.findByText('No MCP servers in global configuration.');
});

test('detail masks secrets, confirms sources, blocks dismissal while deleting, and returns to list only on success', async () => {
  let resolve: () => void = () => {};
  const deleteMcp = vi
    .fn()
    .mockRejectedValueOnce(new Error('failed'))
    .mockImplementationOnce(
      () =>
        new Promise<void>((done) => {
          resolve = done;
        }),
    );
  const config = makeConfig({ deleteMcp });

  render(McpDetail, {}, { wrapper: Harness, wrapperProps: { config } });
  await screen.findByRole('heading', { name: server.name });
  expect(screen.getByText('Enabled')).toBeTruthy();
  expect(screen.queryByText(/Servers from global configuration|not live connection status/)).toBeNull();
  expect(document.body.textContent).not.toContain('secret-header');
  expect(document.body.textContent).not.toContain('secret-env');
  expect(document.body.textContent).not.toContain('secret-oauth');
  expect(document.body.textContent).toContain('public-id');

  for (const key of Object.keys(server.config)) {
    const preview = screen.getByRole('region', {
      name: `${en['mcp.nativeConfig']}: ${key}`,
    });

    expect(preview.tabIndex).toBe(0);
    preview.focus();
    expect(document.activeElement).toBe(preview);
  }

  await fireEvent.click(screen.getByRole('button', { name: 'Delete' }));

  let dialog = await screen.findByRole('dialog');

  await fireEvent.click(within(dialog).getByRole('button', { name: 'Cancel' }));
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect(deleteMcp).not.toHaveBeenCalled();
  await fireEvent.click(screen.getByRole('button', { name: 'Delete' }));
  dialog = await screen.findByRole('dialog');
  await fireEvent.click(within(dialog).getByRole('button', { name: 'Delete' }));
  await within(dialog).findByRole('alert');
  expect(goto).not.toHaveBeenCalled();
  await fireEvent.click(within(dialog).getByRole('button', { name: 'Delete' }));
  expect(
    (
      within(dialog).getByRole('button', {
        name: 'Deleting…',
      }) as HTMLButtonElement
    ).disabled,
  ).toBe(true);
  expect(
    (
      within(dialog).getByRole('button', {
        name: 'Cancel',
      }) as HTMLButtonElement
    ).disabled,
  ).toBe(true);
  await fireEvent.keyDown(dialog, { key: 'Escape' });
  expect(screen.getByRole('dialog')).toBeTruthy();
  resolve();
  await waitFor(() => expect(goto).toHaveBeenCalledExactlyOnceWith('/mcp?removed=1'));
  expect(config.reloadOpencode).not.toHaveBeenCalled();
});

test('detail rejects stale route responses and distinguishes not-found with retry', async () => {
  let resolve: (value: McpServer) => void = () => {};
  const getMcp = vi
    .fn()
    .mockImplementationOnce(
      () =>
        new Promise<McpServer>((done) => {
          resolve = done;
        }),
    )
    .mockRejectedValueOnce({ code: 'not_found', message: 'missing entry' })
    .mockResolvedValueOnce({ ...server, name: 'new' });

  render(McpDetail, {}, { wrapper: Harness, wrapperProps: { config: makeConfig({ getMcp }) } });
  await waitFor(() => expect(getMcp).toHaveBeenCalledOnce());
  page.params = { id: 'new' };

  const alert = await screen.findByRole('alert');

  expect(alert.textContent).toContain('not found in global configuration');
  resolve(server);
  await Promise.resolve();
  expect(screen.queryByRole('heading', { name: server.name })).toBeNull();
  await fireEvent.click(within(alert).getByRole('button', { name: 'Retry' }));
  await screen.findByRole('heading', { name: 'new' });
  expect(getMcp).toHaveBeenLastCalledWith('new');
});

test('masking covers nested config and does not change the native input', () => {
  const original = { ...server.config, options: [{ apiKey: 'secret' }] };
  const rendered = JSON.stringify(maskConfig(original));

  expect(rendered).not.toContain(':"secret"');
  expect(rendered).not.toContain('secret-header');
  expect(rendered).not.toContain('secret-env');
  expect(rendered).not.toContain('secret-oauth');
  expect(rendered).toContain('public-id');
  expect(original.headers).toEqual({ Authorization: 'secret-header' });
});

test('list hides URL and explicit command credentials, including from search', async () => {
  const config = makeConfig({
    listMcps: vi.fn().mockResolvedValue({ data: [remote, local], diagnostics: [] }),
  });

  render(McpList, {}, { wrapper: Harness, wrapperProps: { config } });
  await screen.findByRole('link', { name: local.name });
  for (const secret of credentialValues) expect(document.body.innerHTML).not.toContain(secret);
  expect(document.body.textContent).toContain('example.com/mcp');
  expect(document.body.textContent).toContain('mode=stream');
  expect(document.body.textContent).toContain('--port 3000');
  await fireEvent.input(screen.getByRole('textbox'), {
    target: { value: 'env-command-token' },
  });
  expect(screen.getByText('No matches')).toBeTruthy();
  expect(local.config.command).toEqual(command);
  expect(remote.config.url).toBe(remoteUrl);
});

test.each([remote, local])('detail hides credentials in target and native config: $type', async (entry) => {
  page.params = { id: entry.name };
  render(
    McpDetail,
    {},
    {
      wrapper: Harness,
      wrapperProps: {
        config: makeConfig({ getMcp: vi.fn().mockResolvedValue(entry) }),
      },
    },
  );
  await screen.findByRole('heading', { name: entry.name });

  for (const secret of [...credentialValues, 'secret-header', 'secret-env', 'secret-oauth']) {
    expect(document.body.innerHTML).not.toContain(secret);
  }

  expect(document.body.textContent).toContain('••••••');
});

test('display masking preserves ordinary text and source values, with a safe raw-target fallback', () => {
  const source = {
    url: remoteUrl,
    command,

    options: {
      note: 'ordinary text',
      args: ['--key', 'array-key', '--password="quoted password"', 'PUBLIC=value', '--tokenizer', 'ordinary-tokenizer'],
    },
  };
  const before = structuredClone(source);
  const rendered = JSON.stringify(maskConfig(source));

  for (const secret of [...credentialValues, 'array-key', 'quoted password']) expect(rendered).not.toContain(secret);
  expect(rendered).toContain('ordinary text');
  expect(rendered).toContain('PUBLIC=value');
  expect(rendered).toContain('ordinary-tokenizer');
  expect(rendered).toContain('server.js');
  expect(
    maskTarget({
      type: 'local',
      config: {},
      target: 'node server.js --token "quoted token" API_KEY=raw-key --port 3000',
    }),
  ).toBe('node server.js --token •••••• API_KEY=•••••• --port 3000');
  expect(source).toEqual(before);
});

test.each(['local', 'remote'])(
  'creates a $0 MCP from a complete native object and navigates without reload',
  async (type) => {
    const createMcp = vi.fn().mockImplementation(async (draft) => ({
      ...server,
      name: draft.name,
      type,
      config: draft.config,
    }));
    const config = makeConfig({ createMcp });

    render(McpNew, {}, { wrapper: Harness, wrapperProps: { config } });

    const input = screen.getByLabelText('Native configuration (JSON)') as HTMLTextAreaElement;

    expect(JSON.parse(input.value)).toEqual({
      type: 'local',
      command: [''],
      environment: {},
      disabled: false,
    });

    if (type === 'remote') {
      await fireEvent.click(screen.getByRole('button', { name: 'Type' }));
      await fireEvent.click(screen.getByRole('option', { name: 'Remote' }));
      expect(JSON.parse(input.value)).toMatchObject({
        type: 'remote',
        url: '',
        headers: {},
      });
    }

    const native =
      type === 'local'
        ? {
            type,
            command: ['node', 'server.js'],
            environment: { TOKEN: 'raw-secret' },
            cwd: '/work',
            disabled: true,
            extra: { retained: true },
          }
        : {
            type,
            url: 'https://example.com/mcp',
            headers: { Authorization: 'raw-secret' },

            oauth: {
              client_id: 'public-id',
              client_secret: 'raw-oauth',
              scope: 'read',
              callback_port: 8080,
              redirect_uri: 'http://localhost:8080/callback',
              auth_server_metadata_url: 'https://example.com/.well-known/oauth-authorization-server',
            },

            timeout: { startup: 5000, catalog: 10000, execution: 30000 },
            disabled: false,
            extra: { retained: true },
          };

    await fireEvent.input(screen.getByLabelText('Name'), {
      target: { value: 'new server' },
    });
    await fireEvent.input(input, { target: { value: JSON.stringify(native) } });
    await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
    await waitFor(() => expect(goto).toHaveBeenCalledExactlyOnceWith('/mcp/new%20server?saved=1'));
    expect(createMcp).toHaveBeenCalledExactlyOnceWith({
      name: 'new server',
      config: native,
    });
    expect(config.reloadOpencode).not.toHaveBeenCalled();
  },
);

test('MCP new syntax, non-object and target validation do not call IPC; duplicate errors keep draft', async () => {
  const createMcp = vi.fn().mockRejectedValue({ code: 'conflict', message: 'Name already exists' });

  render(McpNew, {}, { wrapper: Harness, wrapperProps: { config: makeConfig({ createMcp }) } });

  const input = screen.getByLabelText('Native configuration (JSON)') as HTMLTextAreaElement;

  await fireEvent.input(screen.getByLabelText('Name'), {
    target: { value: 'duplicate' },
  });

  for (const text of ['{invalid}', '[]', '{"type":"local","command":[]}']) {
    await fireEvent.input(input, { target: { value: text } });
    await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
    expect(screen.getByRole('alert')).toBeTruthy();
    expect(input.value).toBe(text);
    expect(createMcp).not.toHaveBeenCalled();
  }

  const text = JSON.stringify(server.config);

  await fireEvent.input(input, { target: { value: text } });
  await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
  await screen.findByText(/Name already exists/);
  expect(input.value).toBe(text);
  expect(goto).not.toHaveBeenCalled();
});

test('MCP explicit edit uses raw config and immutable expected snapshot; conflict preserves draft, cancel returns to list', async () => {
  const updateMcp = vi.fn().mockRejectedValue({
    code: 'configuration_failed',
    message: 'Source changed externally',
  });

  render(McpDetail, {}, { wrapper: Harness, wrapperProps: { config: makeConfig({ updateMcp }) } });
  await screen.findByRole('heading', { name: server.name });
  expect(document.body.innerHTML).not.toContain('secret-header');
  await fireEvent.click(screen.getByRole('button', { name: 'Edit' }));
  expect(screen.getByText(/including credentials/)).toBeTruthy();

  const input = screen.getByLabelText('Native configuration (JSON)') as HTMLTextAreaElement;

  expect(JSON.parse(input.value)).toEqual(server.config);
  expect((screen.getByLabelText('Name') as HTMLInputElement).disabled).toBe(true);

  const native = {
    ...server.config,
    timeout: { startup: 9000, catalog: 10000, execution: 30000 },
    unknown: { retained: true },
  };

  await fireEvent.input(input, { target: { value: JSON.stringify(native) } });
  await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
  await screen.findByText(/Source changed externally/);
  expect(JSON.parse(input.value)).toEqual(native);
  expect(updateMcp).toHaveBeenCalledExactlyOnceWith({
    name: server.name,
    config: native,
    expectedConfig: server.config,
    expectedSourcePath: server.sourcePath,
    expectedSourceSnapshot: server.sourceSnapshot,
  });
  await fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
  await waitFor(() => expect(goto).toHaveBeenCalledExactlyOnceWith('/mcp'));
  expect(document.body.innerHTML).not.toContain('secret-header');
});

test('edit draft freezes the exact backend snapshot and scope across source updates and failed retries', async () => {
  const source = structuredClone(server);
  const updateMcp = vi.fn().mockRejectedValue({ code: 'conflict', message: 'Refresh before saving' });
  const config = makeScopedConfig({ updateMcp });
  const mounted = render(McpForm, { source, projectId: 'project-a' }, { wrapper: Harness, wrapperProps: { config } });
  const original = structuredClone(source);
  const native = { ...source.config, disabled: true };

  await fireEvent.input(screen.getByLabelText('Native configuration (JSON)'), {
    target: { value: JSON.stringify(native) },
  });
  source.sourceSnapshot = 'new source token';
  source.sourcePath = '/changed/opencode.json';
  source.config.timeout = 123;
  await mounted.rerender({ source, projectId: 'project-b' });
  await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
  await screen.findByText(/Refresh before saving/);
  await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
  await waitFor(() => expect(updateMcp).toHaveBeenCalledTimes(2));

  for (const call of updateMcp.mock.calls) {
    expect(call).toEqual([
      {
        name: original.name,
        config: native,
        expectedConfig: original.config,
        expectedSourcePath: original.sourcePath,
        expectedSourceSnapshot: original.sourceSnapshot,
      },
      'project-a',
    ]);
  }
  expect(document.body.textContent).not.toContain(original.sourceSnapshot);
});

test('MCP save pending disables controls and successful result returns to the list', async () => {
  let resolve: (result: McpServer) => void = () => {};
  const updateMcp = vi.fn().mockImplementation(
    () =>
      new Promise<McpServer>((done) => {
        resolve = done;
      }),
  );
  const config = makeConfig({ updateMcp });

  render(McpDetail, {}, { wrapper: Harness, wrapperProps: { config } });
  await screen.findByRole('heading', { name: server.name });
  await fireEvent.click(screen.getByRole('button', { name: 'Edit' }));
  await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
  expect((screen.getByRole('button', { name: 'Saving…' }) as HTMLButtonElement).disabled).toBe(true);
  expect((screen.getByRole('button', { name: 'Cancel' }) as HTMLButtonElement).disabled).toBe(true);
  resolve({ ...server, disabled: true });
  await waitFor(() => expect(goto).toHaveBeenCalledExactlyOnceWith('/mcp?saved=1'));
  expect(config.reloadOpencode).not.toHaveBeenCalled();
});

test('MCP route changes ignore an old pending save and edit intent opens only after loading', async () => {
  let resolve: (result: McpServer) => void = () => {};

  page.url = new URL('http://localhost/mcp/test?edit=1');

  const updateMcp = vi.fn().mockImplementation(
    () =>
      new Promise<McpServer>((done) => {
        resolve = done;
      }),
  );
  const getMcp = vi
    .fn()
    .mockResolvedValueOnce(server)
    .mockResolvedValueOnce({ ...server, name: 'other' });

  render(
    McpDetail,
    {},
    {
      wrapper: Harness,
      wrapperProps: { config: makeConfig({ updateMcp, getMcp }) },
    },
  );
  await screen.findByLabelText('Native configuration (JSON)');
  await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
  page.url = new URL('http://localhost/mcp/other');
  page.params = { id: 'other' };
  await screen.findByRole('heading', { name: 'other' });
  resolve(server);
  await waitFor(() => expect(screen.queryByRole('heading', { name: server.name })).toBeNull());
  expect(screen.queryByText('Saved to global configuration. Reload OpenCode manually to apply.')).toBeNull();
});

test('MCP Edit query on the same ID opens editor; unmount ignores late creation navigation', async () => {
  const mounted = render(McpDetail, {}, { wrapper: Harness, wrapperProps: { config: makeConfig() } });

  await screen.findByRole('heading', { name: server.name });
  page.url = new URL('http://localhost/mcp/docs?edit=1');
  await screen.findByLabelText('Native configuration (JSON)');
  mounted.unmount();

  let resolve: (result: McpServer) => void = () => {};
  const createMcp = vi.fn().mockImplementation(
    () =>
      new Promise<McpServer>((done) => {
        resolve = done;
      }),
  );
  const creating = render(McpNew, {}, { wrapper: Harness, wrapperProps: { config: makeConfig({ createMcp }) } });

  await fireEvent.input(screen.getByLabelText('Name'), {
    target: { value: 'docs' },
  });
  await fireEvent.input(screen.getByLabelText('Native configuration (JSON)'), {
    target: { value: JSON.stringify(server.config) },
  });
  await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
  creating.unmount();
  resolve(server);
  await Promise.resolve();
  expect(goto).not.toHaveBeenCalled();
});

test('MCP Refresh only rereads its own list and replaces diagnostics without runtime reload', async () => {
  const listMcps = vi
    .fn()
    .mockResolvedValueOnce({ data: [server], diagnostics: ['Old diagnostic'] })
    .mockResolvedValueOnce({ data: [local], diagnostics: ['New diagnostic'] });
  const otherRead = vi.fn();
  const config = makeConfig({
    listMcps,
    listSkills: otherRead,
    refreshAll: otherRead,
    createMcp: otherRead,
    updateMcp: otherRead,
  });

  render(McpList, {}, { wrapper: Harness, wrapperProps: { config } });
  await screen.findByRole('link', { name: server.name });
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh' }));
  await screen.findByRole('link', { name: local.name });
  expect(screen.queryByRole('link', { name: server.name })).toBeNull();
  expect(screen.queryByText('Old diagnostic')).toBeNull();
  expect(screen.getByText('New diagnostic')).toBeTruthy();
  expect(listMcps).toHaveBeenCalledTimes(2);
  expect(otherRead).not.toHaveBeenCalled();
  expect(config.reloadOpencode).not.toHaveBeenCalled();
});

test('project list allows editing, toggling and deletion, hides creation and keeps Global comparison readonly', async () => {
  const projectServer = { ...server, sourcePath: '/work/alpha/opencode.jsonc' };
  const listMcps = vi.fn(async (id?: string) => ({
    data: [id ? projectServer : server],
    diagnostics: [],
  }));
  const config = makeScopedConfig({ listMcps });

  render(McpList, {}, { wrapper: Harness, wrapperProps: { config } });

  const link = await screen.findByRole('link', { name: server.name });

  expect(listMcps).toHaveBeenCalledExactlyOnceWith('project-a');
  expect(link.getAttribute('href')).toBe('/mcp/docs%2Fa%20b?project=project-a');
  expect(screen.queryByRole('link', { name: 'New MCP' })).toBeNull();
  expect(screen.getByRole('link', { name: `Edit ${server.name}` }).getAttribute('href')).toBe(
    '/mcp/docs%2Fa%20b?project=project-a&edit=1',
  );
  expect(screen.getByRole('button', { name: `Delete ${server.name}` })).toBeTruthy();
  await fireEvent.click(screen.getByRole('checkbox', { name: 'Show global configuration' }));
  await waitFor(() => expect(screen.getAllByRole('link', { name: server.name })).toHaveLength(2));
  expect(screen.getAllByRole('link', { name: server.name }).map((entry) => entry.getAttribute('href'))).toEqual([
    '/mcp/docs%2Fa%20b?project=project-a',
    '/mcp/docs%2Fa%20b?readonly=1',
  ]);
  expect(listMcps).toHaveBeenCalledTimes(2);
  const switches = screen.getAllByRole('checkbox', { name: /Disable docs/ });

  expect(switches).toHaveLength(2);
  expect((switches[0] as HTMLInputElement).disabled).toBe(false);
  expect((switches[1] as HTMLInputElement).disabled).toBe(true);
  expect(screen.getAllByRole('link', { name: `Edit ${server.name}` })).toHaveLength(1);
  expect(screen.getAllByRole('button', { name: `Delete ${server.name}` })).toHaveLength(1);
});

test('late project list cannot replace Global after quick return', async () => {
  let resolve: (value: { data: McpServer[]; diagnostics: string[] }) => void = () => {};
  const listMcps = vi
    .fn()
    .mockImplementationOnce(
      () =>
        new Promise((done) => {
          resolve = done;
        }),
    )
    .mockResolvedValueOnce({ data: [local], diagnostics: [] });

  render(
    McpList,
    {},
    {
      wrapper: Harness,
      wrapperProps: { config: makeScopedConfig({ listMcps }) },
    },
  );
  await waitFor(() => expect(listMcps).toHaveBeenCalledWith('project-a'));
  await fireEvent.click(screen.getByRole('button', { name: 'Return to Global' }));
  await screen.findByRole('link', { name: local.name });
  resolve({ data: [server], diagnostics: ['late diagnostic'] });
  await waitFor(() => expect(screen.queryByRole('link', { name: server.name })).toBeNull());
  expect(screen.queryByText('late diagnostic')).toBeNull();
  expect(listMcps).toHaveBeenCalledTimes(2);
});

test('project detail opens edit after retry, preserves frozen scope and source on conflict without changing list selection', async () => {
  page.url = new URL('http://localhost/mcp/docs?project=project-b&edit=1&saved=1');

  const getMcp = vi.fn().mockRejectedValueOnce(new Error('missing root')).mockResolvedValue(server);
  const updateMcp = vi.fn().mockRejectedValue({ code: 'conflict', message: 'Project source changed' });
  const config = makeScopedConfig({ getMcp, updateMcp, deleteMcp: vi.fn() });

  render(McpDetail, {}, { wrapper: Harness, wrapperProps: { config } });

  const alert = await screen.findByRole('alert');

  await fireEvent.click(within(alert).getByRole('button', { name: 'Retry' }));
  await screen.findByRole('heading', { name: server.name });
  expect(getMcp).toHaveBeenNthCalledWith(2, server.name, 'project-b');
  expect(screen.getByText('/other/alpha')).toBeTruthy();
  await fireEvent.click(screen.getByRole('button', { name: 'Edit' }));
  const input = screen.getByLabelText('Native configuration (JSON)') as HTMLTextAreaElement;
  const native = { ...server.config, disabled: true };

  await fireEvent.input(input, { target: { value: JSON.stringify(native) } });
  await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
  await screen.findByText(/Project source changed/);
  expect(updateMcp).toHaveBeenCalledExactlyOnceWith(
    {
      name: server.name,
      config: native,
      expectedConfig: server.config,
      expectedSourcePath: server.sourcePath,
      expectedSourceSnapshot: server.sourceSnapshot,
    },
    'project-b',
  );
  expect(JSON.parse(input.value)).toEqual(native);
  expect(screen.queryByText(/Saved to global configuration/)).toBeNull();
  expect(config.selectedProjectId).toBe('project-a');
  expect(config.selectProject).not.toHaveBeenCalled();
});

test('missing project root stays selected with retry/Global exit and cannot show Global fallback', async () => {
  const listMcps = vi.fn().mockRejectedValue(new Error('Project folder does not exist'));
  const config = makeScopedConfig({ listMcps });

  render(McpList, {}, { wrapper: Harness, wrapperProps: { config } });

  const alert = await screen.findByRole('alert');

  expect(alert.textContent).toContain('Project folder does not exist');
  expect(config.selectedProjectId).toBe('project-a');
  expect(screen.getByText('/work/alpha')).toBeTruthy();
  expect(screen.getByRole('button', { name: 'Return to Global' })).toBeTruthy();
  expect(screen.queryByRole('link', { name: server.name })).toBeNull();
  await fireEvent.click(within(alert).getByRole('button', { name: 'Retry' }));
  await waitFor(() => expect(listMcps).toHaveBeenCalledTimes(2));
  expect(listMcps).toHaveBeenLastCalledWith('project-a');
});

test('MCP Global comparison detail rejects forged edit intent without selecting Global', async () => {
  page.url = new URL('http://localhost/mcp/docs?readonly=1&edit=1');

  const getMcp = vi.fn().mockResolvedValue(server);
  const config = makeScopedConfig({ getMcp });

  render(McpDetail, {}, { wrapper: Harness, wrapperProps: { config } });
  await screen.findByRole('heading', { name: server.name });
  expect(getMcp).toHaveBeenCalledExactlyOnceWith(server.name);
  expect(screen.queryByRole('textbox')).toBeNull();
  expect(screen.queryByRole('button', { name: 'Edit' })).toBeNull();
  expect(screen.queryByRole('button', { name: 'Delete' })).toBeNull();
  expect(screen.getByText(en['scope.globalReadOnly'])).toBeTruthy();
  expect(config.selectedProjectId).toBe('project-a');
});

test('same MCP ID in different project queries has isolated identity and ignores late source responses', async () => {
  page.url = new URL('http://localhost/mcp/docs?project=project-a&edit=1');

  let resolve: (value: McpServer) => void = () => {};
  const next = {
    ...server,
    sourcePath: '/other/alpha/opencode.jsonc',
    sourcePaths: ['/other/alpha/opencode.jsonc'],
  };
  const getMcp = vi
    .fn()
    .mockImplementationOnce(
      () =>
        new Promise((done) => {
          resolve = done;
        }),
    )
    .mockResolvedValueOnce(next);
  const config = makeScopedConfig({ getMcp });

  render(McpDetail, {}, { wrapper: Harness, wrapperProps: { config } });
  await waitFor(() => expect(getMcp).toHaveBeenCalledWith(server.name, 'project-a'));
  page.url = new URL('http://localhost/mcp/docs?project=project-b&edit=1');
  await screen.findByLabelText('Native configuration (JSON)');
  expect(getMcp).toHaveBeenLastCalledWith(server.name, 'project-b');
  resolve({
    ...server,
    sourcePath: '/work/alpha/opencode.jsonc',
    sourcePaths: ['/work/alpha/opencode.jsonc'],
  });
  await waitFor(() => expect(screen.queryByText('/work/alpha/opencode.jsonc')).toBeNull());
  expect(screen.getByText('/other/alpha')).toBeTruthy();
  expect(screen.queryByRole('button', { name: 'Edit' })).toBeNull();
  expect(screen.getByLabelText('Native configuration (JSON)')).toBeTruthy();
  expect(config.selectedProjectId).toBe('project-a');
});

test('MCP disabled IPC forwards exact source/state snapshots and only includes explicit project scope', async () => {
  const invoke = vi.fn().mockResolvedValue(server);
  const adapter = createTauriAdapter(invoke);

  const projectDraft = {
    ...disabledUpdate,
    disabled: false,
    expectedDisabled: true,
    expectedSourcePath: '/work/alpha/opencode.jsonc',
  };

  await adapter.setMcpDisabled(disabledUpdate);
  await adapter.setMcpDisabled(projectDraft, 'project-a');
  await adapter.updateMcp(
    {
      name: server.name,
      config: server.config,
      expectedConfig: server.config,
      expectedSourcePath: server.sourcePath,
      expectedSourceSnapshot: server.sourceSnapshot,
    },
    'project-b',
  );
  expect(invoke.mock.calls).toEqual([
    ['set_mcp_disabled', { draft: disabledUpdate }],
    ['set_mcp_disabled', { draft: projectDraft, projectId: 'project-a' }],
    [
      'update_mcp',
      {
        draft: {
          name: server.name,
          config: server.config,
          expectedConfig: server.config,
          expectedSourcePath: server.sourcePath,
          expectedSourceSnapshot: server.sourceSnapshot,
        },
        projectId: 'project-b',
      },
    ],
  ]);
});

test('disabled store mutation returns the backend entry without refresh and sanitizes failed recovery', async () => {
  const setMcpDisabled = vi.fn().mockResolvedValueOnce(server).mockRejectedValueOnce({
    code: 'conflict',
    message: 'secret-header',
    detail: server.config,
  });
  const adapter = {
    setMcpDisabled,
    loadAppState: vi.fn(),
    opencodeListModels: vi.fn(),
  };
  const store = createConfigStore(adapter as unknown as Parameters<typeof createConfigStore>[0], false, false);

  expect(await store.setMcpDisabled(disabledUpdate, 'project-a')).toBe(server);
  expect(setMcpDisabled).toHaveBeenCalledExactlyOnceWith(disabledUpdate, 'project-a');
  expect(store.notice).toBe('setMcpDisabled');
  await expect(store.setMcpDisabled(disabledUpdate)).rejects.toMatchObject({
    code: 'conflict',
  });
  expect(store.saving).toBe(false);
  expect(store.draftRecovery?.payload).toBeNull();
  expect(store.draftRecovery?.conflict).toBe(true);
  expect(JSON.stringify(store.error)).not.toContain('secret-header');
  expect(JSON.stringify(store.draftRecovery)).not.toContain('secret-header');
  expect(adapter.loadAppState).not.toHaveBeenCalled();
  expect(adapter.opencodeListModels).not.toHaveBeenCalled();
});

test('Global switch keeps committed state while pending and on failure; retry uses the unchanged snapshot', async () => {
  let reject: (cause: unknown) => void = () => {};
  const setMcpDisabled = vi
    .fn()
    .mockImplementationOnce(
      () =>
        new Promise((_, fail) => {
          reject = fail;
        }),
    )
    .mockResolvedValueOnce({
      ...server,
      disabled: true,
      config: { ...server.config, disabled: true },
    });
  const config = makeConfig({
    setMcpDisabled,
    listMcps: vi.fn(async () => ({
      data: [{ ...server, disabled: setMcpDisabled.mock.calls.length > 1 }],
      diagnostics: [],
    })),
  });

  render(McpList, {}, { wrapper: Harness, wrapperProps: { config } });
  const control = (await screen.findByRole('checkbox', {
    name: `Disable ${server.name} in Global`,
  })) as HTMLInputElement;

  await fireEvent.click(control);
  expect(control.checked).toBe(true);
  expect(control.disabled).toBe(true);
  expect(control.getAttribute('aria-busy')).toBe('true');
  expect((screen.getByRole('button', { name: 'Refresh' }) as HTMLButtonElement).disabled).toBe(true);
  expect(setMcpDisabled).toHaveBeenCalledExactlyOnceWith(disabledUpdate);
  reject({ code: 'conflict', message: 'secret-header' });
  await screen.findByText(en['mcp.toggleFailed']);
  expect(control.checked).toBe(true);
  expect(control.disabled).toBe(false);
  expect(document.body.innerHTML).not.toContain('secret-header');
  await fireEvent.click(control);
  await screen.findByRole('checkbox', {
    name: `Enable ${server.name} in Global`,
  });
  expect(
    (
      screen.getByRole('checkbox', {
        name: `Enable ${server.name} in Global`,
      }) as HTMLInputElement
    ).checked,
  ).toBe(false);
  expect(setMcpDisabled).toHaveBeenNthCalledWith(2, disabledUpdate);
  expect(config.listMcps).toHaveBeenCalledTimes(2);
  expect(config.reloadOpencode).not.toHaveBeenCalled();
});

test('project switch writes only the owned source, never the Global comparison', async () => {
  const projectServer = {
    ...server,
    disabled: true,
    sourcePath: '/work/alpha/opencode.jsonc',
  };
  const setMcpDisabled = vi.fn().mockResolvedValue({ ...projectServer, disabled: false });
  const listMcps = vi.fn(async (id?: string) => ({
    data: [id ? { ...projectServer, disabled: setMcpDisabled.mock.calls.length === 0 } : server],
    diagnostics: [],
  }));
  const config = makeScopedConfig({ listMcps, setMcpDisabled });

  render(McpList, {}, { wrapper: Harness, wrapperProps: { config } });
  await screen.findByRole('link', { name: server.name });
  await fireEvent.click(screen.getByRole('checkbox', { name: 'Show global configuration' }));
  const comparison = (await screen.findByRole('checkbox', {
    name: `Disable ${server.name} in Global`,
  })) as HTMLInputElement;

  await fireEvent.click(comparison);
  expect(setMcpDisabled).not.toHaveBeenCalled();
  await fireEvent.click(screen.getByRole('checkbox', { name: `Enable ${server.name} in Project` }));
  await screen.findByRole('checkbox', {
    name: `Disable ${server.name} in Project`,
  });
  expect(setMcpDisabled).toHaveBeenCalledExactlyOnceWith(
    {
      ...disabledUpdate,
      disabled: false,
      expectedDisabled: true,
      expectedSourcePath: projectServer.sourcePath,
    },
    'project-a',
  );
  expect(comparison.checked).toBe(true);
  expect(comparison.disabled).toBe(true);
  expect(screen.getByText(en['mcp.projectSaved'])).toBeTruthy();
  expect(listMcps).toHaveBeenCalledTimes(3);
});

test('successful switch uses only the fresh backend source snapshot for the next write', async () => {
  const updated = {
    ...server,
    disabled: true,
    sourceSnapshot: ' backend-next-snapshot\r\n',
  };
  const setMcpDisabled = vi.fn().mockResolvedValueOnce(updated).mockResolvedValueOnce(server);
  const config = makeConfig({
    setMcpDisabled,
    listMcps: vi.fn(async () => ({
      data: [setMcpDisabled.mock.calls.length === 1 ? updated : server],
      diagnostics: [],
    })),
  });

  render(McpList, {}, { wrapper: Harness, wrapperProps: { config } });
  await fireEvent.click(
    await screen.findByRole('checkbox', {
      name: `Disable ${server.name} in Global`,
    }),
  );
  await fireEvent.click(
    await screen.findByRole('checkbox', {
      name: `Enable ${server.name} in Global`,
    }),
  );
  await waitFor(() => expect(setMcpDisabled).toHaveBeenCalledTimes(2));
  expect(setMcpDisabled).toHaveBeenNthCalledWith(1, disabledUpdate);
  expect(setMcpDisabled).toHaveBeenNthCalledWith(2, {
    ...disabledUpdate,
    disabled: false,
    expectedDisabled: true,
    expectedSourceSnapshot: updated.sourceSnapshot,
  });
  expect(document.body.textContent).not.toContain(updated.sourceSnapshot);
});

test('a failed reread after a successful write reports a read error, not a write failure', async () => {
  const updated = { ...server, disabled: true };
  const listMcps = vi
    .fn()
    .mockResolvedValueOnce({ data: [server], diagnostics: [] })
    .mockRejectedValueOnce(new Error('reread unavailable'));
  const setMcpDisabled = vi.fn().mockResolvedValue(updated);
  const config = makeConfig({ listMcps, setMcpDisabled });

  render(McpList, {}, { wrapper: Harness, wrapperProps: { config } });
  await fireEvent.click(await screen.findByRole('checkbox', { name: `Disable ${server.name} in Global` }));

  await screen.findByText(/reread unavailable/);
  expect(screen.queryByText(en['mcp.toggleFailed'])).toBeNull();
  expect(screen.getByText(en['configuration.saved'])).toBeTruthy();
  expect(setMcpDisabled).toHaveBeenCalledExactlyOnceWith({
    ...disabledUpdate,
    expectedSourceSnapshot: server.sourceSnapshot,
  });
});

test('sequential MCP toggles reread backend between writes so each draft carries the fresh token', async () => {
  const first = {
    ...server,
    disabled: true,
    sourceSnapshot: ' token-after-enable\r\n',
  };
  const second = {
    ...first,
    disabled: false,
    sourceSnapshot: ' token-after-disable \r\n',
  };
  const setMcpDisabled = vi.fn().mockResolvedValueOnce(first).mockResolvedValueOnce(second);
  const listMcps = vi
    .fn()
    .mockResolvedValueOnce({ data: [server], diagnostics: [] })
    .mockResolvedValueOnce({ data: [first], diagnostics: [] })
    .mockResolvedValueOnce({ data: [second], diagnostics: [] });
  const config = makeConfig({ setMcpDisabled, listMcps });

  render(McpList, {}, { wrapper: Harness, wrapperProps: { config } });
  await fireEvent.click(
    await screen.findByRole('checkbox', {
      name: `Disable ${server.name} in Global`,
    }),
  );
  await fireEvent.click(
    await screen.findByRole('checkbox', {
      name: `Enable ${server.name} in Global`,
    }),
  );
  await waitFor(() => expect(setMcpDisabled).toHaveBeenCalledTimes(2));

  expect(listMcps).toHaveBeenCalledTimes(3);
  expect(setMcpDisabled.mock.calls).toEqual([
    [
      {
        ...disabledUpdate,
        expectedDisabled: false,
        expectedSourceSnapshot: server.sourceSnapshot,
      },
    ],
    [
      {
        ...disabledUpdate,
        disabled: false,
        expectedDisabled: true,
        expectedSourceSnapshot: first.sourceSnapshot,
      },
    ],
  ]);
  expect(document.body.textContent).not.toContain(first.sourceSnapshot);
  expect(document.body.textContent).not.toContain(second.sourceSnapshot);
});

test('in-flight MCP toggle blocks a second write instead of reusing the stale token', async () => {
  let resolve: (result: McpServer) => void = () => {};
  const setMcpDisabled = vi.fn().mockImplementation(
    () =>
      new Promise<McpServer>((done) => {
        resolve = done;
      }),
  );
  const config = makeConfig({
    setMcpDisabled,
    listMcps: vi
      .fn()
      .mockImplementationOnce(async () => ({ data: [server], diagnostics: [] }))
      .mockImplementation(async () => ({
        data: [{ ...server, disabled: true, sourceSnapshot: ' later-token\r\n' }],
        diagnostics: [],
      })),
  });

  render(McpList, {}, { wrapper: Harness, wrapperProps: { config } });
  await fireEvent.click(
    await screen.findByRole('checkbox', {
      name: `Disable ${server.name} in Global`,
    }),
  );
  await fireEvent.click(screen.getByRole('checkbox', { name: `Disable ${server.name} in Global` }));
  expect(setMcpDisabled).toHaveBeenCalledExactlyOnceWith({
    ...disabledUpdate,
    expectedSourceSnapshot: server.sourceSnapshot,
  });
  resolve({ ...server, disabled: true, sourceSnapshot: ' later-token\r\n' });
  await screen.findByRole('checkbox', {
    name: `Enable ${server.name} in Global`,
  });
  expect(setMcpDisabled).toHaveBeenCalledTimes(1);
});

test.each(['resolve', 'reject'])(
  'old project toggle %s cannot change the new scope or show stale feedback',
  async (completion) => {
    let resolve: (result: McpServer) => void = () => {};
    let reject: (cause: unknown) => void = () => {};
    const setMcpDisabled = vi.fn().mockImplementation(
      () =>
        new Promise((done, fail) => {
          resolve = done;
          reject = fail;
        }),
    );
    const listMcps = vi.fn().mockResolvedValue({ data: [server], diagnostics: [] });
    const config = makeScopedConfig({ listMcps, setMcpDisabled });

    render(McpList, {}, { wrapper: Harness, wrapperProps: { config } });
    await fireEvent.click(
      await screen.findByRole('checkbox', {
        name: `Disable ${server.name} in Project`,
      }),
    );
    await config.selectProject(null);
    await screen.findByRole('checkbox', {
      name: `Disable ${server.name} in Global`,
    });
    if (completion === 'resolve') resolve({ ...server, disabled: true });
    else reject(new Error('late error'));
    await Promise.resolve();
    expect(
      (
        screen.getByRole('checkbox', {
          name: `Disable ${server.name} in Global`,
        }) as HTMLInputElement
      ).checked,
    ).toBe(true);
    expect(screen.queryByText(en['mcp.toggleFailed'])).toBeNull();
    expect(screen.queryByText(en['mcp.projectSaved'])).toBeNull();
    expect(listMcps).toHaveBeenCalledTimes(2);
  },
);

test('toggle completion after unmount does not reread resources or navigate', async () => {
  let resolve: (result: McpServer) => void = () => {};
  const setMcpDisabled = vi.fn().mockImplementation(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  const config = makeConfig({ setMcpDisabled });
  const mounted = render(McpList, {}, { wrapper: Harness, wrapperProps: { config } });

  await fireEvent.click(
    await screen.findByRole('checkbox', {
      name: `Disable ${server.name} in Global`,
    }),
  );
  mounted.unmount();
  resolve({ ...server, disabled: true });
  await Promise.resolve();
  expect(config.listMcps).toHaveBeenCalledOnce();
  expect(goto).not.toHaveBeenCalled();
  expect(screen.queryByText(en['configuration.saved'])).toBeNull();
});

test('forged project creation route provides no form or save action and does not change selection', async () => {
  page.url = new URL('http://localhost/mcp/new?project=project-b');
  const createMcp = vi.fn().mockResolvedValue(server);
  const config = makeScopedConfig({ createMcp });

  render(McpNew, {}, { wrapper: Harness, wrapperProps: { config } });
  expect(screen.getByText('/other/alpha')).toBeTruthy();
  expect(screen.getByText(en['mcp.projectEditOnly'])).toBeTruthy();
  expect(screen.queryByRole('form')).toBeNull();
  expect(screen.queryByRole('button', { name: 'Save' })).toBeNull();
  expect(createMcp).not.toHaveBeenCalled();
  expect(goto).not.toHaveBeenCalled();
  expect(config.selectedProjectId).toBe('project-a');
  expect(config.selectProject).not.toHaveBeenCalled();
});

test('Global readonly creation prevents writes and changing to a project route invalidates pending Global navigation', async () => {
  page.url = new URL('http://localhost/mcp/new?readonly=1');
  let resolve: (result: McpServer) => void = () => {};
  const createMcp = vi.fn().mockImplementation(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  const config = makeScopedConfig({ createMcp });

  render(McpNew, {}, { wrapper: Harness, wrapperProps: { config } });
  expect((screen.getByRole('button', { name: 'Save' }) as HTMLButtonElement).disabled).toBe(true);
  expect((screen.getByLabelText('Native configuration (JSON)') as HTMLTextAreaElement).disabled).toBe(true);
  await fireEvent.submit(screen.getByRole('form', { name: 'New MCP' }));
  expect(createMcp).not.toHaveBeenCalled();
  page.url = new URL('http://localhost/mcp/new');
  await waitFor(() => expect((screen.getByRole('button', { name: 'Save' }) as HTMLButtonElement).disabled).toBe(false));
  await fireEvent.input(screen.getByLabelText('Name'), {
    target: { value: server.name },
  });
  await fireEvent.input(screen.getByLabelText('Native configuration (JSON)'), {
    target: { value: JSON.stringify(server.config) },
  });
  await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
  page.url = new URL('http://localhost/mcp/new?project=project-b');
  await screen.findByText('/other/alpha');
  resolve(server);
  await Promise.resolve();
  expect(goto).not.toHaveBeenCalled();
  expect(screen.queryByLabelText('Name')).toBeNull();
  expect(screen.queryByRole('button', { name: 'Save' })).toBeNull();
  expect(createMcp).toHaveBeenCalledExactlyOnceWith({
    name: server.name,
    config: server.config,
  });
});

test('project detail deletion preserves exact source guards and ignores late completion after a scope change', async () => {
  page.url = new URL('http://localhost/mcp/docs?project=project-b');
  let resolve: () => void = () => {};
  const deleteMcp = vi.fn().mockImplementationOnce(
    () =>
      new Promise<void>((done) => {
        resolve = done;
      }),
  );
  const getMcp = vi.fn().mockResolvedValue(server);
  const config = makeScopedConfig({ getMcp, deleteMcp });

  render(McpDetail, {}, { wrapper: Harness, wrapperProps: { config } });
  await screen.findByRole('heading', { name: server.name });
  expect(screen.getByRole('button', { name: 'Delete' })).toBeTruthy();
  expect(screen.getByRole('button', { name: 'Edit' })).toBeTruthy();
  expect(deleteMcp).not.toHaveBeenCalled();
  await fireEvent.click(await screen.findByRole('button', { name: 'Delete' }));
  await fireEvent.click(
    within(await screen.findByRole('dialog')).getByRole('button', {
      name: 'Delete',
    }),
  );
  page.url = new URL('http://localhost/mcp/docs?project=project-a');
  await waitFor(() => expect(getMcp).toHaveBeenLastCalledWith(server.name, 'project-a'));
  resolve();
  await Promise.resolve();
  expect(goto).not.toHaveBeenCalled();
  expect(deleteMcp).toHaveBeenCalledExactlyOnceWith(server.name, 'project-b', server.sourcePath, server.sourceSnapshot);
});

test('project deletion IPC and store preserve source guards without changing the Global deletion shape', async () => {
  const invoke = vi.fn().mockResolvedValue(undefined);
  const store = createConfigStore(createTauriAdapter(invoke), false, false);

  await store.deleteMcp(server.name);
  await store.deleteMcp(server.name, 'project-a', server.sourcePath, server.sourceSnapshot);
  expect(invoke.mock.calls).toEqual([
    ['delete_mcp', { name: server.name }],
    [
      'delete_mcp',
      {
        name: server.name,
        projectId: 'project-a',
        expectedSourcePath: server.sourcePath,
        expectedSourceSnapshot: server.sourceSnapshot,
      },
    ],
  ]);
  expect(store.notice).toBe('deleteMcp');
  expect(store.saving).toBe(false);
});

test('project list deletion shows only its own source and keeps Global comparison intact through failure and retry', async () => {
  const projectServer = {
    ...server,
    sourcePath: '/work/alpha/opencode.jsonc',
    sourcePaths: ['/work/alpha/opencode.jsonc'],
  };
  const deleteMcp = vi
    .fn()
    .mockRejectedValueOnce({ code: 'conflict', message: 'secret-header' })
    .mockResolvedValueOnce(undefined);
  const listMcps = vi.fn(async (scope?: string) => ({
    data: scope ? (deleteMcp.mock.calls.length > 1 ? [] : [projectServer]) : [server],
    diagnostics: [],
  }));
  const config = makeScopedConfig({ listMcps, deleteMcp });

  render(McpList, {}, { wrapper: Harness, wrapperProps: { config } });
  await screen.findByRole('link', { name: server.name });
  await fireEvent.click(screen.getByRole('checkbox', { name: 'Show global configuration' }));
  await waitFor(() => expect(screen.getAllByRole('link', { name: server.name })).toHaveLength(2));
  expect(screen.getAllByRole('button', { name: `Delete ${server.name}` })).toHaveLength(1);
  await fireEvent.click(screen.getByRole('button', { name: `Delete ${server.name}` }));
  const dialog = await screen.findByRole('dialog');

  expect(within(dialog).getByText(projectServer.sourcePath)).toBeTruthy();
  expect(within(dialog).queryByText(server.sourcePath)).toBeNull();
  expect(within(dialog).getByText(/Global configuration is unchanged/)).toBeTruthy();
  await fireEvent.click(within(dialog).getByRole('button', { name: 'Delete' }));
  await within(dialog).findByRole('alert');
  expect(document.body.innerHTML).not.toContain('secret-header');
  await fireEvent.click(within(dialog).getByRole('button', { name: 'Delete' }));
  await screen.findByText(en['mcp.projectRemoved']);
  await waitFor(() => expect(screen.getAllByRole('link', { name: server.name })).toHaveLength(1));
  expect(screen.getByRole('link', { name: server.name }).getAttribute('href')).toBe('/mcp/docs%2Fa%20b?readonly=1');
  expect(screen.queryByRole('button', { name: `Delete ${server.name}` })).toBeNull();
  expect(screen.queryByRole('link', { name: 'New MCP' })).toBeNull();
  for (const call of deleteMcp.mock.calls)
    expect(call).toEqual([server.name, 'project-a', projectServer.sourcePath, projectServer.sourceSnapshot]);
  expect(listMcps).toHaveBeenCalledTimes(3);
});

test('project delete confirmation freezes source snapshots across background changes and keeps dismissal blocked while pending', async () => {
  let reject: (cause: unknown) => void = () => {};
  let resolve: () => void = () => {};
  const deleteMcp = vi
    .fn()
    .mockImplementationOnce(
      () =>
        new Promise((_, fail) => {
          reject = fail;
        }),
    )
    .mockImplementationOnce(
      () =>
        new Promise<void>((done) => {
          resolve = done;
        }),
    );
  const source = structuredClone(server);
  const original = structuredClone(source);
  const onDeleted = vi.fn();

  render(
    DeleteMcpDialog,
    { server: source, projectId: 'project-a', onDeleted },
    {
      wrapper: Harness,
      wrapperProps: { config: makeScopedConfig({ deleteMcp }) },
    },
  );
  const dialog = await screen.findByRole('dialog');
  source.sourcePath = '/changed/opencode.json';
  source.sourceSnapshot = 'changed token';
  source.sourcePaths.push('/changed/opencode.json');
  await fireEvent.click(within(dialog).getByRole('button', { name: 'Delete' }));
  expect(
    (
      within(dialog).getByRole('button', {
        name: 'Cancel',
      }) as HTMLButtonElement
    ).disabled,
  ).toBe(true);
  await fireEvent.keyDown(dialog, { key: 'Escape' });
  expect(screen.getByRole('dialog')).toBeTruthy();
  reject({ code: 'conflict', message: 'changed source' });
  await within(dialog).findByRole('alert');
  await fireEvent.click(within(dialog).getByRole('button', { name: 'Delete' }));
  for (const call of deleteMcp.mock.calls)
    expect(call).toEqual([original.name, 'project-a', original.sourcePath, original.sourceSnapshot]);
  resolve();
  await waitFor(() => expect(onDeleted).toHaveBeenCalledOnce());
});

test('successful project detail deletion returns to the list with the same explicit scope', async () => {
  page.url = new URL('http://localhost/mcp/docs?project=project-b');
  const deleteMcp = vi.fn().mockResolvedValue(undefined);

  render(
    McpDetail,
    {},
    {
      wrapper: Harness,
      wrapperProps: {
        config: makeScopedConfig({
          getMcp: vi.fn().mockResolvedValue(server),
          deleteMcp,
        }),
      },
    },
  );
  await fireEvent.click(await screen.findByRole('button', { name: 'Delete' }));
  await fireEvent.click(
    within(await screen.findByRole('dialog')).getByRole('button', {
      name: 'Delete',
    }),
  );
  await waitFor(() => expect(goto).toHaveBeenCalledExactlyOnceWith('/mcp?removed=1&project=project-b'));
  expect(deleteMcp).toHaveBeenCalledExactlyOnceWith(server.name, 'project-b', server.sourcePath, server.sourceSnapshot);
});

test('project edit Cancel returns to the list and keeps the project scope', async () => {
  page.url = new URL('http://localhost/mcp/docs%2Fa%20b?project=project-a&edit=1');
  const config = makeScopedConfig({
    getMcp: vi.fn().mockResolvedValue(server),
  });

  render(McpDetail, {}, { wrapper: Harness, wrapperProps: { config } });
  await screen.findByLabelText('Native configuration (JSON)');
  await fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
  await waitFor(() => expect(goto).toHaveBeenCalledExactlyOnceWith('/mcp?project=project-a'));
  expect(config.updateMcp).toBeUndefined();
  expect(config.selectedProjectId).toBe('project-a');
});

test('project edit save success returns to the list with the saved banner and preserves scope', async () => {
  page.url = new URL('http://localhost/mcp/docs%2Fa%20b?project=project-a&edit=1');
  const native = { ...server.config, disabled: true };
  const updateMcp = vi.fn().mockResolvedValue({ ...server, config: native });
  const config = makeScopedConfig({
    getMcp: vi.fn().mockResolvedValue(server),
    updateMcp,
  });

  render(McpDetail, {}, { wrapper: Harness, wrapperProps: { config } });
  await screen.findByLabelText('Native configuration (JSON)');
  await fireEvent.input(screen.getByLabelText('Native configuration (JSON)'), {
    target: { value: JSON.stringify(native) },
  });
  await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
  await waitFor(() => expect(goto).toHaveBeenCalledExactlyOnceWith('/mcp?saved=1&project=project-a'));
  expect(updateMcp).toHaveBeenCalledExactlyOnceWith(
    {
      name: server.name,
      config: native,
      expectedConfig: server.config,
      expectedSourcePath: server.sourcePath,
      expectedSourceSnapshot: server.sourceSnapshot,
    },
    'project-a',
  );
  expect(config.selectedProjectId).toBe('project-a');
  expect(config.selectProject).not.toHaveBeenCalled();
});

test('project edit save failure stays in the editor, keeps the draft, and does not navigate', async () => {
  page.url = new URL('http://localhost/mcp/docs%2Fa%20b?project=project-a&edit=1');
  const native = { ...server.config, disabled: true };
  const updateMcp = vi.fn().mockRejectedValue({ code: 'conflict', message: 'Project source changed' });
  const config = makeScopedConfig({
    getMcp: vi.fn().mockResolvedValue(server),
    updateMcp,
  });

  render(McpDetail, {}, { wrapper: Harness, wrapperProps: { config } });
  await screen.findByLabelText('Native configuration (JSON)');
  await fireEvent.input(screen.getByLabelText('Native configuration (JSON)'), {
    target: { value: JSON.stringify(native) },
  });
  await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
  await screen.findByText(/Project source changed/);
  expect(goto).not.toHaveBeenCalled();
  const input = screen.getByLabelText('Native configuration (JSON)') as HTMLTextAreaElement;

  expect(JSON.parse(input.value)).toEqual(native);
  expect((screen.getByRole('button', { name: 'Save' }) as HTMLButtonElement).disabled).toBe(false);
  expect(config.selectedProjectId).toBe('project-a');
});

test('Global edit Cancel and save success return to the global list scope', async () => {
  page.url = new URL('http://localhost/mcp/docs%2Fa%20b?edit=1');
  const updateMcp = vi.fn().mockResolvedValue(server);
  const config = makeConfig({ updateMcp });
  const mounted = render(McpDetail, {}, { wrapper: Harness, wrapperProps: { config } });

  await screen.findByLabelText('Native configuration (JSON)');
  await fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
  await waitFor(() => expect(goto).toHaveBeenCalledExactlyOnceWith('/mcp'));
  mounted.unmount();
  vi.mocked(goto).mockClear();

  render(McpDetail, {}, { wrapper: Harness, wrapperProps: { config } });
  await screen.findByLabelText('Native configuration (JSON)');
  await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
  await waitFor(() => expect(goto).toHaveBeenCalledExactlyOnceWith('/mcp?saved=1'));
  expect(updateMcp).toHaveBeenCalledExactlyOnceWith({
    name: server.name,
    config: server.config,
    expectedConfig: server.config,
    expectedSourcePath: server.sourcePath,
    expectedSourceSnapshot: server.sourceSnapshot,
  });
});

test('list shows the saved banner only for the matching explicit scope', async () => {
  page.url = new URL('http://localhost/mcp?saved=1&project=project-a');
  const config = makeScopedConfig({
    listMcps: vi.fn().mockResolvedValue({ data: [server], diagnostics: [] }),
  });

  render(McpList, {}, { wrapper: Harness, wrapperProps: { config } });
  await screen.findByRole('link', { name: server.name });
  expect(screen.getByText(en['mcp.projectSaved'])).toBeTruthy();
});

test('direct project form creation cannot submit even if the native form submit is dispatched', async () => {
  const createMcp = vi.fn();
  const config = makeScopedConfig({ createMcp });

  render(McpForm, { projectId: 'project-a' }, { wrapper: Harness, wrapperProps: { config } });

  expect((screen.getByRole('button', { name: 'Save' }) as HTMLButtonElement).disabled).toBe(true);
  await fireEvent.submit(screen.getByRole('form', { name: 'New MCP' }));
  expect(createMcp).not.toHaveBeenCalled();
  expect(goto).not.toHaveBeenCalled();
});
