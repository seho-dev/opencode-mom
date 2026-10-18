import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterAll, beforeAll, beforeEach, expect, test, vi } from 'vitest';
import { goto } from '$app/navigation';
import { page as routePage } from '$app/state';
import { makeScopedConfig } from '$src/components/__tests__/scope-config.js';
import type { ConfigStore } from '$src/config/store.svelte.js';
import Harness from '$src/test-utils/harness.svelte';
import type { McpServer } from '$src/types/mcp.js';
import McpList from '../+page.svelte';
import McpDetail from '../[id]/+page.svelte';
import { maskConfig, maskTarget } from '../config-view.js';
import McpNew from '../new/+page.svelte';

const page: { params: { id?: string }; url: URL } = routePage;

const scrollIntoView = Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'scrollIntoView');
beforeAll(() => {
  Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', { configurable: true, value: vi.fn() });
});
afterAll(() => {
  if (scrollIntoView) Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', scrollIntoView);
  else Reflect.deleteProperty(HTMLElement.prototype, 'scrollIntoView');
});

vi.mock('$app/navigation', () => ({ goto: vi.fn().mockResolvedValue(undefined) }));
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
  config: {
    type: 'remote',
    url: 'https://example.com/mcp',
    headers: { Authorization: 'secret-header' },
    environment: { KEY: 'secret-env' },
    oauth: { client_id: 'public-id', client_secret: 'secret-oauth' },
    timeout: { startup: 5000, catalog: 10000, execution: 30000 },
  },
} satisfies McpServer;
const remoteUrl =
  'https://private-user:private-pass@example.com/mcp?token=url-token&api%5Fkey=url-api&key=url-key&client_secret=url-secret&mode=stream';
const remote = { ...server, target: remoteUrl, config: { ...server.config, url: remoteUrl } } satisfies McpServer;
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
    .mockResolvedValueOnce({ data: [server], diagnostics: ['Unreadable source'] })
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
  expect(screen.getByText('Enabled')).toBeTruthy();
  expect(screen.queryByText(/Servers from global configuration|not live connection status/)).toBeNull();
  expect(screen.getByText('Unreadable source')).toBeTruthy();
  await fireEvent.input(screen.getByRole('textbox'), { target: { value: 'missing' } });
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
  expect((within(dialog).getByRole('button', { name: 'Deleting…' }) as HTMLButtonElement).disabled).toBe(true);
  expect((within(dialog).getByRole('button', { name: 'Cancel' }) as HTMLButtonElement).disabled).toBe(true);
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
  const config = makeConfig({ listMcps: vi.fn().mockResolvedValue({ data: [remote, local], diagnostics: [] }) });
  render(McpList, {}, { wrapper: Harness, wrapperProps: { config } });
  await screen.findByRole('link', { name: local.name });
  for (const secret of credentialValues) expect(document.body.innerHTML).not.toContain(secret);
  expect(document.body.textContent).toContain('example.com/mcp');
  expect(document.body.textContent).toContain('mode=stream');
  expect(document.body.textContent).toContain('--port 3000');
  await fireEvent.input(screen.getByRole('textbox'), { target: { value: 'env-command-token' } });
  expect(screen.getByText('No matches')).toBeTruthy();
  expect(local.config.command).toEqual(command);
  expect(remote.config.url).toBe(remoteUrl);
});

test.each([remote, local])('detail hides credentials in target and native config: $type', async (entry) => {
  page.params = { id: entry.name };
  render(
    McpDetail,
    {},
    { wrapper: Harness, wrapperProps: { config: makeConfig({ getMcp: vi.fn().mockResolvedValue(entry) }) } },
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
    const createMcp = vi
      .fn()
      .mockImplementation(async (draft) => ({ ...server, name: draft.name, type, config: draft.config }));
    const config = makeConfig({ createMcp });
    render(McpNew, {}, { wrapper: Harness, wrapperProps: { config } });
    const input = screen.getByLabelText('Native configuration (JSON)') as HTMLTextAreaElement;
    expect(JSON.parse(input.value)).toEqual({ type: 'local', command: [''], environment: {}, disabled: false });
    if (type === 'remote') {
      await fireEvent.click(screen.getByRole('button', { name: 'Type' }));
      await fireEvent.click(screen.getByRole('option', { name: 'Remote' }));
      expect(JSON.parse(input.value)).toMatchObject({ type: 'remote', url: '', headers: {} });
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
    await fireEvent.input(screen.getByLabelText('Name'), { target: { value: 'new server' } });
    await fireEvent.input(input, { target: { value: JSON.stringify(native) } });
    await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
    await waitFor(() => expect(goto).toHaveBeenCalledExactlyOnceWith('/mcp/new%20server?saved=1'));
    expect(createMcp).toHaveBeenCalledExactlyOnceWith({ name: 'new server', config: native });
    expect(config.reloadOpencode).not.toHaveBeenCalled();
  },
);

test('MCP new syntax, non-object and target validation do not call IPC; duplicate errors keep draft', async () => {
  const createMcp = vi.fn().mockRejectedValue({ code: 'conflict', message: 'Name already exists' });
  render(McpNew, {}, { wrapper: Harness, wrapperProps: { config: makeConfig({ createMcp }) } });
  const input = screen.getByLabelText('Native configuration (JSON)') as HTMLTextAreaElement;
  await fireEvent.input(screen.getByLabelText('Name'), { target: { value: 'duplicate' } });
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

test('MCP explicit edit uses raw config and immutable expected snapshot; conflict preserves draft, cancel restores saved view', async () => {
  const updateMcp = vi.fn().mockRejectedValue({ code: 'configuration_failed', message: 'Source changed externally' });
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
  });
  await fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
  expect(screen.queryByRole('textbox')).toBeNull();
  expect(document.body.innerHTML).not.toContain('secret-header');
  await fireEvent.click(screen.getByRole('button', { name: 'Edit' }));
  expect(JSON.parse((screen.getByLabelText('Native configuration (JSON)') as HTMLTextAreaElement).value)).toEqual(
    server.config,
  );
});

test('MCP save pending disables controls and successful result returns to masked view', async () => {
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
  await screen.findByText('Saved to global configuration. Reload OpenCode manually to apply.');
  expect(screen.queryByRole('textbox')).toBeNull();
  expect(screen.getByText('Disabled')).toBeTruthy();
  expect(document.body.innerHTML).not.toContain('secret-oauth');
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
  render(McpDetail, {}, { wrapper: Harness, wrapperProps: { config: makeConfig({ updateMcp, getMcp }) } });
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
  await fireEvent.input(screen.getByLabelText('Name'), { target: { value: 'docs' } });
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

test('project list passes exact ID, preserves duplicate Global comparison and exposes only readonly links', async () => {
  const projectServer = { ...server, sourcePath: '/work/alpha/opencode.jsonc' };
  const listMcps = vi.fn(async (id?: string) => ({ data: [id ? projectServer : server], diagnostics: [] }));
  const config = makeScopedConfig({ listMcps });
  render(McpList, {}, { wrapper: Harness, wrapperProps: { config } });
  const link = await screen.findByRole('link', { name: server.name });
  expect(listMcps).toHaveBeenCalledExactlyOnceWith('project-a');
  expect(link.getAttribute('href')).toBe('/mcp/docs%2Fa%20b?project=project-a');
  expect(screen.queryByRole('link', { name: 'New MCP' })).toBeNull();
  expect(screen.queryByRole('link', { name: `Edit ${server.name}` })).toBeNull();
  expect(screen.queryByRole('button', { name: `Delete ${server.name}` })).toBeNull();
  await fireEvent.click(screen.getByRole('checkbox', { name: 'Show global configuration' }));
  await waitFor(() => expect(screen.getAllByRole('link', { name: server.name })).toHaveLength(2));
  expect(screen.getAllByRole('link', { name: server.name }).map((entry) => entry.getAttribute('href'))).toEqual([
    '/mcp/docs%2Fa%20b?project=project-a',
    '/mcp/docs%2Fa%20b?readonly=1',
  ]);
  expect(listMcps).toHaveBeenCalledTimes(2);
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
  render(McpList, {}, { wrapper: Harness, wrapperProps: { config: makeScopedConfig({ listMcps }) } });
  await waitFor(() => expect(listMcps).toHaveBeenCalledWith('project-a'));
  await fireEvent.click(screen.getByRole('button', { name: 'Return to Global' }));
  await screen.findByRole('link', { name: local.name });
  resolve({ data: [server], diagnostics: ['late diagnostic'] });
  await waitFor(() => expect(screen.queryByRole('link', { name: server.name })).toBeNull());
  expect(screen.queryByText('late diagnostic')).toBeNull();
  expect(listMcps).toHaveBeenCalledTimes(2);
});

test('project detail ignores edit and saved query, preserves scope on retry and never mutates list selection', async () => {
  page.url = new URL('http://localhost/mcp/docs?project=project-b&edit=1&saved=1');
  const getMcp = vi.fn().mockRejectedValueOnce(new Error('missing root')).mockResolvedValue(server);
  const config = makeScopedConfig({ getMcp, updateMcp: vi.fn(), deleteMcp: vi.fn() });
  render(McpDetail, {}, { wrapper: Harness, wrapperProps: { config } });
  const alert = await screen.findByRole('alert');
  await fireEvent.click(within(alert).getByRole('button', { name: 'Retry' }));
  await screen.findByRole('heading', { name: server.name });
  expect(getMcp).toHaveBeenNthCalledWith(2, server.name, 'project-b');
  expect(screen.getByText('/other/alpha')).toBeTruthy();
  expect(screen.queryByRole('button', { name: 'Edit' })).toBeNull();
  expect(screen.queryByRole('button', { name: 'Delete' })).toBeNull();
  expect(screen.queryByRole('textbox')).toBeNull();
  expect(screen.queryByText(/Saved to global configuration/)).toBeNull();
  expect(config.selectedProjectId).toBe('project-a');
  expect(config.selectProject).not.toHaveBeenCalled();
  expect(config.updateMcp).not.toHaveBeenCalled();
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
  expect(screen.getByText(/Switch the list to Global to edit/)).toBeTruthy();
  expect(config.selectedProjectId).toBe('project-a');
});

test('same MCP ID in different project queries has isolated identity and ignores late source responses', async () => {
  page.url = new URL('http://localhost/mcp/docs?project=project-a&edit=1');
  let resolve: (value: McpServer) => void = () => {};
  const next = { ...server, sourcePath: '/other/alpha/opencode.jsonc', sourcePaths: ['/other/alpha/opencode.jsonc'] };
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
  await screen.findByText('/other/alpha/opencode.jsonc');
  expect(getMcp).toHaveBeenLastCalledWith(server.name, 'project-b');
  resolve({ ...server, sourcePath: '/work/alpha/opencode.jsonc', sourcePaths: ['/work/alpha/opencode.jsonc'] });
  await waitFor(() => expect(screen.queryByText('/work/alpha/opencode.jsonc')).toBeNull());
  expect(screen.getByText('/other/alpha')).toBeTruthy();
  expect(screen.queryByRole('button', { name: 'Edit' })).toBeNull();
  expect(screen.queryByRole('textbox')).toBeNull();
  expect(config.selectedProjectId).toBe('project-a');
});
