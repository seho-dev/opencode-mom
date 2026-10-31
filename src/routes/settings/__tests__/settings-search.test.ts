import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { tick } from 'svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { goto } from '$app/navigation';
import { page } from '$app/state';
import type { CommandAdapter } from '$src/config/adapter.js';
import { createConfigStore } from '$src/config/store.svelte.js';
import { en } from '$src/i18n/dictionaries/en.js';
import { zh } from '$src/i18n/dictionaries/zh.js';
import Harness from '$src/test-utils/harness.svelte';
import Settings from '../+page.svelte';
import SettingsSearch from '../SettingsSearch.svelte';

const native = vi.hoisted(() => ({ isTauri: vi.fn(() => true) }));
vi.mock('@tauri-apps/api/core', () => ({ isTauri: native.isTauri }));
vi.mock('$app/navigation', () => ({ goto: vi.fn().mockResolvedValue(undefined), beforeNavigate: vi.fn() }));
vi.mock('$app/state', () => ({ page: { url: new URL('http://localhost/settings'), params: {} } }));
vi.mock('$src/shell/toast.svelte.js', () => ({ toast: vi.fn() }));

const service = {
  target: { mode: 'explicit', url: 'http://127.0.0.1:4096', hasPassword: true },
  connected: true,
  url: 'http://127.0.0.1:4096',
  pid: 42,
  version: '2.0.0',
  error: null,
};
const installation = {
  binaryPath: '/Users/test/.opencode/bin/opencode',
  version: '2.0.0',
  method: 'curl',
  canUpgrade: true,
  upgradeReason: null,
  manualCommand: null,
};

function makeAdapter() {
  return {
    savePreferences: vi.fn().mockResolvedValue(undefined),
    getAutostart: vi.fn().mockResolvedValue(false),
    setAutostart: vi.fn(),
    getLidProtection: vi.fn().mockResolvedValue({ enabled: true, phase: 'protected', activeSessions: 2 }),
    setLidProtection: vi.fn(),
    getIslandState: vi.fn().mockResolvedValue({ supported: true, enabled: true }),
    setIslandEnabled: vi.fn(),
    getAppInfo: vi.fn().mockResolvedValue({ version: '2.0.0', platform: 'macos' }),
    getOpenCodeInstallation: vi.fn().mockResolvedValue(installation),
    getOpenCodeServiceStatus: vi.fn().mockResolvedValue(service),
    setOpenCodeServiceTarget: vi.fn(),
    checkOpenCodeUpdate: vi.fn().mockResolvedValue({
      installation,
      currentVersion: '2.0.0',
      latestVersion: '2.1.0',
      updateAvailable: true,
    }),
    upgradeOpenCode: vi.fn(),
    checkForUpdates: vi.fn(),
    getMomHttpSettings: vi.fn().mockResolvedValue({ port: 18765, allowLan: false }),
    setMomHttpSettings: vi.fn(),
    getMcpProxySettings: vi.fn().mockResolvedValue({ enabled: true, idleTimeout: 300, maxUnits: 16, revision: 1 }),
    getMcpProxyState: vi.fn().mockResolvedValue({
      actualState: 'running',
      boundAddr: '127.0.0.1:18765',
      securityErrors: [],
      activeUnits: 0,
      activeSessions: 0,
      maxUnits: 16,
      revision: 1,
      generation: 1,
    }),
    saveMcpProxySettings: vi.fn(),
    rotateMcpProxyKey: vi.fn(),
    getGlobalBehavior: vi.fn().mockResolvedValue({
      path: '/global/AGENTS.md',
      resolvedPath: '/resolved/AGENTS.md',
      content: '# Private saved instructions',
      exists: true,
    }),
    saveGlobalBehavior: vi.fn(),
  };
}

function mountPage(tab: 'general' | 'mcp-proxy' = 'general', overrides: Partial<ReturnType<typeof makeAdapter>> = {}) {
  page.url = new URL(`http://localhost/settings${tab === 'mcp-proxy' ? '?tab=mcp-proxy' : ''}`);
  const adapter = { ...makeAdapter(), ...overrides };
  const config = createConfigStore(adapter as unknown as CommandAdapter, false, false);
  const view = render(Settings, {}, { wrapper: Harness, wrapperProps: { config } });
  return { ...view, config, adapter };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

async function settle() {
  for (let step = 0; step < 8; step++) {
    await Promise.resolve();
    await tick();
  }
}

function results() {
  return screen.getByRole('list', { name: 'Matching settings' });
}

function activePanel() {
  return within(screen.getByRole('tabpanel'));
}

function expectNoWrites(adapter: ReturnType<typeof makeAdapter>) {
  for (const call of [
    adapter.setAutostart,
    adapter.setLidProtection,
    adapter.setIslandEnabled,
    adapter.setOpenCodeServiceTarget,
    adapter.upgradeOpenCode,
    adapter.checkForUpdates,
    adapter.setMomHttpSettings,
    adapter.saveMcpProxySettings,
    adapter.rotateMcpProxyKey,
    adapter.saveGlobalBehavior,
  ])
    expect(call).not.toHaveBeenCalled();
}

beforeEach(() => {
  page.url = new URL('http://localhost/settings');
  native.isTauri.mockReturnValue(true);
  vi.mocked(goto).mockClear();
  Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', { configurable: true, value: vi.fn() });
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
  Reflect.deleteProperty(HTMLElement.prototype, 'scrollIntoView');
});

test('matches bilingual labels, descriptions and technical names locally, and follows live locale changes', async () => {
  const adapter = makeAdapter();
  const config = createConfigStore(adapter as unknown as CommandAdapter, false, false);
  const onselect = vi.fn();
  render(SettingsSearch, { onselect }, { wrapper: Harness, wrapperProps: { config } });
  const search = screen.getByRole('searchbox', { name: 'Search settings' }) as HTMLInputElement;
  expect(search.name).toBe('settings-search');
  expect(search.autocomplete).toBe('off');

  for (const [query, label] of [
    ['  ＣＬＩ  ', en['settings.opencodeUpdates']],
    ['18765', en['settings.http.port']],
    ['allowLan', en['settings.http.allowLan']],
    ['default plugin', en['settings.defaultPlugin']],
    ['MCP maxUnits', en['settings.proxy.maxUnits']],
    ['idleTimeout', en['settings.proxy.idleTimeout']],
    ['局域网', en['settings.http.allowLan']],
    ['默认插件', en['settings.defaultPlugin']],
    ['登录', en['settings.autostart']],
    ['password', en['settings.servicePassword']],
  ] as const) {
    await fireEvent.input(search, { target: { value: query } });
    expect(within(results()).getByText(label)).toBeTruthy();
  }
  await config.setLocale('zh');
  await tick();
  expect(search).toBe(screen.getByRole('searchbox', { name: '搜索设置' }));
  expect(search.value).toBe('password');
  expect(screen.getByText(zh['settings.servicePassword'])).toBeTruthy();
  await fireEvent.input(search, { target: { value: 'MCP maxUnits' } });
  const chineseResults = screen.getByRole('list', { name: '匹配的设置' });
  await fireEvent.click(within(chineseResults).getByRole('button', { name: /最大并发/ }));
  expect(onselect).toHaveBeenCalledWith(
    expect.objectContaining({ tab: 'mcp-proxy', target: 'settings-proxy-capacity' }),
  );
  await config.setLocale('en');
  await tick();
  expect(within(results()).getByText(en['settings.proxy.maxUnits'])).toBeTruthy();
});

test('empty, whitespace, no-match and Escape clear return to the unchanged settings view', async () => {
  const { adapter } = mountPage();
  const port = await activePanel().findByRole('spinbutton', { name: 'Port' });
  const search = screen.getByRole('searchbox', { name: 'Search settings' }) as HTMLInputElement;
  expect(screen.queryByRole('list', { name: 'Matching settings' })).toBeNull();
  await fireEvent.input(search, { target: { value: 'not-a-real-setting-xyz' } });
  expect(screen.getByText(en['settings.searchEmpty'])).toBeTruthy();
  expect(activePanel().getByRole('spinbutton', { name: 'Port' })).toBe(port);
  await fireEvent.click(screen.getAllByRole('button', { name: 'Clear search' }).at(-1) as HTMLButtonElement);
  expect(search.value).toBe('');
  expect(document.activeElement).toBe(search);
  expect(screen.queryByRole('list', { name: 'Matching settings' })).toBeNull();
  await fireEvent.input(search, { target: { value: '   ' } });
  expect(screen.queryByRole('list', { name: 'Matching settings' })).toBeNull();
  await fireEvent.input(search, { target: { value: 'port' } });
  await fireEvent.keyDown(search, { key: 'Escape' });
  expect(search.value).toBe('');
  expect(activePanel().getByRole('spinbutton', { name: 'Port' })).toBe(port);
  expectNoWrites(adapter);
  expect(adapter.checkOpenCodeUpdate).not.toHaveBeenCalled();
  expect(goto).not.toHaveBeenCalled();
});

test('search and same-tab navigation retain URL, password, clear-password and HTTP drafts without remounting native panels', async () => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
  const { adapter, config, unmount } = mountPage();
  await settle();
  const url = activePanel().getByRole('textbox', { name: 'OpenCode backend URL' }) as HTMLInputElement;
  const password = activePanel().getByLabelText('Service password (optional)') as HTMLInputElement;
  const clear = activePanel().getByRole('checkbox', {
    name: 'Clear the saved password when saving',
  });
  const port = activePanel().getByRole('spinbutton', { name: 'Port' }) as HTMLInputElement;
  const lid = screen.getByRole('checkbox', { name: 'Stay awake with lid closed' });
  const island = screen.getByRole('checkbox', { name: 'OpenCode mini island' });
  await fireEvent.input(url, { target: { value: 'http://127.0.0.1:54321/draft-only-url' } });
  await fireEvent.input(password, { target: { value: 'draft-only-private-password' } });
  await fireEvent.click(clear);
  await fireEvent.input(port, { target: { value: '23456' } });
  const search = screen.getByRole('searchbox', { name: 'Search settings' });
  const counts = [
    adapter.getAppInfo,
    adapter.getOpenCodeInstallation,
    adapter.getOpenCodeServiceStatus,
    adapter.getIslandState,
  ].map((call) => call.mock.calls.length);
  for (const query of ['password', 'draft-only-private-password', 'draft-only-url', '23456']) {
    await fireEvent.input(search, { target: { value: query } });
    if (query !== 'password') expect(screen.queryByRole('list', { name: 'Matching settings' })).toBeNull();
  }
  await fireEvent.input(search, { target: { value: 'password' } });
  await fireEvent.click(within(results()).getByRole('button', { name: /Service password/ }));
  expect(document.activeElement?.id).toBe('opencode-service-title');
  await fireEvent.keyDown(search, { key: 'Escape' });
  expect(activePanel().getByRole('textbox', { name: 'OpenCode backend URL' })).toBe(url);
  expect(url.value).toBe('http://127.0.0.1:54321/draft-only-url');
  expect(password.value).toBe('draft-only-private-password');
  expect(clear.getAttribute('aria-checked')).toBe('true');
  expect(port.value).toBe('23456');
  expect(screen.getByRole('checkbox', { name: 'Stay awake with lid closed' })).toBe(lid);
  expect(screen.getByRole('checkbox', { name: 'OpenCode mini island' })).toBe(island);
  expect(
    [adapter.getAppInfo, adapter.getOpenCodeInstallation, adapter.getOpenCodeServiceStatus, adapter.getIslandState].map(
      (call) => call.mock.calls.length,
    ),
  ).toEqual(counts);
  expect(adapter.getAutostart).toHaveBeenCalledOnce();
  expect(adapter.getMomHttpSettings).toHaveBeenCalledOnce();
  expect(adapter.getMcpProxySettings).not.toHaveBeenCalled();
  expect(adapter.checkOpenCodeUpdate).not.toHaveBeenCalled();
  expect(goto).not.toHaveBeenCalled();
  expectNoWrites(adapter);
  expect(config.draftRecovery).toBeNull();
  await vi.advanceTimersByTimeAsync(2000);
  expect(adapter.getLidProtection).toHaveBeenCalledTimes(2);
  expect(adapter.getMomHttpSettings).toHaveBeenCalledTimes(2);
  expect(port.value).toBe('23456');
  expect(password.value).toBe('draft-only-private-password');
  await fireEvent.click(clear);
  expect(password.disabled).toBe(false);
  expect(password.value).toBe('draft-only-private-password');
  for (const input of document.querySelectorAll('input')) expect(input.name).not.toBe('');
  unmount();
  expect(password.isConnected).toBe(false);
  await vi.advanceTimersByTimeAsync(4000);
  expect(adapter.getLidProtection).toHaveBeenCalledTimes(2);
  expect(adapter.getMomHttpSettings).toHaveBeenCalledTimes(2);
});

test('proxy drafts and revisions survive search, empty results, locale changes and polling; header refresh remains active-tab only', async () => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
  const { adapter, config, unmount } = mountPage('mcp-proxy');
  await settle();
  const timeout = activePanel().getByRole('spinbutton', { name: 'Session Idle Timeout (seconds)' }) as HTMLInputElement;
  const units = activePanel().getByRole('spinbutton', { name: 'Maximum Concurrent Units' }) as HTMLInputElement;
  await fireEvent.input(timeout, { target: { value: '7654' } });
  await fireEvent.input(units, { target: { value: '77' } });
  adapter.getMcpProxySettings.mockResolvedValue({ enabled: false, idleTimeout: 500, maxUnits: 32, revision: 2 });
  const search = screen.getByRole('searchbox', { name: 'Search settings' });
  for (const query of ['token', '7654', 'maxUnits', '']) {
    await fireEvent.input(search, { target: { value: query } });
    if (query === '7654') expect(screen.queryByRole('list', { name: 'Matching settings' })).toBeNull();
  }
  await config.setLocale('zh');
  await tick();
  expect(timeout.value).toBe('7654');
  await config.setLocale('en');
  await tick();
  expect(activePanel().getByRole('spinbutton', { name: 'Session Idle Timeout (seconds)' })).toBe(timeout);
  await fireEvent.click(
    within(document.querySelector('.page-head') as HTMLElement).getByRole('button', { name: 'Refresh' }),
  );
  await settle();
  expect(adapter.getMcpProxySettings).toHaveBeenCalledTimes(2);
  await vi.advanceTimersByTimeAsync(2000);
  expect(adapter.getMcpProxySettings).toHaveBeenCalledTimes(3);
  expect(timeout.value).toBe('7654');
  expect(units.value).toBe('77');
  expect(adapter.getMomHttpSettings).not.toHaveBeenCalled();
  expect(adapter.getAutostart).not.toHaveBeenCalled();
  expect(adapter.getLidProtection).not.toHaveBeenCalled();
  expect(adapter.getAppInfo).not.toHaveBeenCalled();
  expectNoWrites(adapter);
  expect(adapter.checkOpenCodeUpdate).not.toHaveBeenCalled();
  expect(goto).not.toHaveBeenCalled();
  adapter.saveMcpProxySettings.mockResolvedValue(undefined);
  await fireEvent.click(screen.getByRole('button', { name: 'Save Gateway Settings' }));
  await settle();
  expect(adapter.saveMcpProxySettings).toHaveBeenCalledExactlyOnceWith(
    { enabled: true, idleTimeout: 7654, maxUnits: 77, revision: 1 },
    1,
  );
  unmount();
  const reads = adapter.getMcpProxySettings.mock.calls.length;
  await vi.advanceTimersByTimeAsync(4000);
  expect(adapter.getMcpProxySettings).toHaveBeenCalledTimes(reads);
});

test('cross-tab results navigate and focus without activating controls; only explicit navigation changes lifecycles', async () => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
  const { adapter, unmount } = mountPage();
  await settle();
  const search = screen.getByRole('searchbox', { name: 'Search settings' }) as HTMLInputElement;
  await fireEvent.input(search, { target: { value: 'rotate' } });
  expect(adapter.getMcpProxySettings).not.toHaveBeenCalled();
  await fireEvent.click(within(results()).getByRole('button', { name: /Rotate Secret Token/ }));
  await settle();
  expect(screen.getByRole('tab', { name: 'MCP Proxy' }).getAttribute('aria-selected')).toBe('true');
  expect(document.activeElement?.id).toBe('settings-proxy-token');
  expect(screen.queryByRole('dialog')).toBeNull();
  expect(adapter.getMcpProxySettings).toHaveBeenCalledOnce();
  const lidReads = adapter.getLidProtection.mock.calls.length;
  const httpReads = adapter.getMomHttpSettings.mock.calls.length;
  await vi.advanceTimersByTimeAsync(2000);
  expect(adapter.getLidProtection).toHaveBeenCalledTimes(lidReads);
  expect(adapter.getMomHttpSettings).toHaveBeenCalledTimes(httpReads);
  expect(adapter.getMcpProxySettings).toHaveBeenCalledTimes(2);
  await fireEvent.input(search, { target: { value: '18765' } });
  await fireEvent.click(within(results()).getByRole('button', { name: /Port/ }));
  await settle();
  expect(document.activeElement?.id).toBe('settings-http-port');
  expect(screen.getByRole('tab', { name: 'General' }).getAttribute('aria-selected')).toBe('true');
  expect(search.value).toBe('18765');
  expect(goto).toHaveBeenCalledTimes(2);
  expect(vi.mocked(goto).mock.calls[0]?.[0]).toBe('http://localhost/settings?tab=mcp-proxy');
  expectNoWrites(adapter);
  expect(adapter.checkOpenCodeUpdate).not.toHaveBeenCalled();
  unmount();
});

test('cross-tab search retains both panels and all drafts while inactive polling stays paused', async () => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
  const confirm = vi.spyOn(window, 'confirm');
  const { adapter } = mountPage();
  await settle();
  const url = activePanel().getByRole('textbox', { name: 'OpenCode backend URL' }) as HTMLInputElement;
  const password = activePanel().getByLabelText('Service password (optional)') as HTMLInputElement;
  const clear = activePanel().getByRole('checkbox', {
    name: 'Clear the saved password when saving',
  });
  const port = activePanel().getByRole('spinbutton', { name: 'Port' }) as HTMLInputElement;
  await fireEvent.input(url, { target: { value: 'http://127.0.0.1:54321/keep-me' } });
  await fireEvent.input(password, { target: { value: 'keep-private-password' } });
  await fireEvent.click(clear);
  await fireEvent.input(port, { target: { value: '23456' } });
  const search = screen.getByRole('searchbox', { name: 'Search settings' });
  await fireEvent.input(search, { target: { value: 'idleTimeout' } });
  await fireEvent.click(within(results()).getByRole('button', { name: /Session Idle Timeout/ }));
  await settle();
  expect(screen.getByRole('tab', { name: 'MCP Proxy' }).getAttribute('aria-selected')).toBe('true');
  expect((document.getElementById('settings-tabpanel-general') as HTMLElement).hidden).toBe(true);
  expect(url.isConnected).toBe(true);
  expect(password.value).toBe('keep-private-password');
  expect(clear.getAttribute('aria-checked')).toBe('true');
  expect(adapter.getMcpProxySettings).toHaveBeenCalledOnce();
  const timeout = activePanel().getByRole('spinbutton', { name: 'Session Idle Timeout (seconds)' }) as HTMLInputElement;
  const units = activePanel().getByRole('spinbutton', { name: 'Maximum Concurrent Units' }) as HTMLInputElement;
  await fireEvent.input(timeout, { target: { value: '7654' } });
  await fireEvent.input(units, { target: { value: '77' } });
  const generalReads = [
    adapter.getAutostart,
    adapter.getLidProtection,
    adapter.getMomHttpSettings,
    adapter.getAppInfo,
    adapter.getIslandState,
    adapter.getOpenCodeInstallation,
    adapter.getOpenCodeServiceStatus,
  ];
  const generalCounts = generalReads.map((call) => call.mock.calls.length);
  await vi.advanceTimersByTimeAsync(4000);
  expect(generalReads.map((call) => call.mock.calls.length)).toEqual(generalCounts);

  await fireEvent.input(search, { target: { value: 'password' } });
  await fireEvent.click(within(results()).getByRole('button', { name: /Service password/ }));
  await settle();
  expect(document.activeElement?.id).toBe('opencode-service-title');
  expect(activePanel().getByRole('textbox', { name: 'OpenCode backend URL' })).toBe(url);
  expect(activePanel().getByLabelText('Service password (optional)')).toBe(password);
  expect(activePanel().getByRole('checkbox', { name: 'Clear the saved password when saving' })).toBe(clear);
  expect(activePanel().getByRole('spinbutton', { name: 'Port' })).toBe(port);
  expect(url.value).toBe('http://127.0.0.1:54321/keep-me');
  expect(password.value).toBe('keep-private-password');
  expect(clear.getAttribute('aria-checked')).toBe('true');
  expect(port.value).toBe('23456');
  expect(adapter.getIslandState).toHaveBeenCalledTimes(2);
  expect(adapter.getOpenCodeServiceStatus).toHaveBeenCalledTimes(2);
  expect(adapter.getOpenCodeInstallation).toHaveBeenCalledOnce();
  const proxyReads = adapter.getMcpProxySettings.mock.calls.length;
  await vi.advanceTimersByTimeAsync(4000);
  expect(adapter.getMcpProxySettings).toHaveBeenCalledTimes(proxyReads);

  await fireEvent.input(search, { target: { value: 'maxUnits' } });
  await fireEvent.click(within(results()).getByRole('button', { name: /Maximum Concurrent Units/ }));
  await settle();
  expect(document.activeElement?.id).toBe('settings-proxy-capacity');
  expect(activePanel().getByRole('spinbutton', { name: 'Session Idle Timeout (seconds)' })).toBe(timeout);
  expect(activePanel().getByRole('spinbutton', { name: 'Maximum Concurrent Units' })).toBe(units);
  expect(timeout.value).toBe('7654');
  expect(units.value).toBe('77');
  expect(confirm).not.toHaveBeenCalled();
  expect(goto).toHaveBeenCalledTimes(3);
  expectNoWrites(adapter);
});

test('inactive native info completion cannot start CLI installation or Island reads until re-entry', async () => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
  const info = deferred<{ version: string; platform: string }>();
  const { adapter } = mountPage('general', { getAppInfo: vi.fn().mockReturnValue(info.promise) });
  await settle();
  expect(adapter.getAppInfo).toHaveBeenCalledTimes(3);
  await fireEvent.click(screen.getByRole('tab', { name: 'MCP Proxy' }));
  info.resolve({ version: '2.0.0', platform: 'macos' });
  await settle();
  await vi.advanceTimersByTimeAsync(4000);
  expect(adapter.getOpenCodeInstallation).not.toHaveBeenCalled();
  expect(adapter.getOpenCodeServiceStatus).not.toHaveBeenCalled();
  expect(adapter.getIslandState).not.toHaveBeenCalled();
  await fireEvent.click(screen.getByRole('tab', { name: 'General' }));
  await settle();
  expect(adapter.getOpenCodeInstallation).toHaveBeenCalledOnce();
  expect(adapter.getOpenCodeServiceStatus).toHaveBeenCalledOnce();
  expect(adapter.getIslandState).toHaveBeenCalledOnce();
  expect(adapter.getAppInfo).toHaveBeenCalledTimes(5);
  expectNoWrites(adapter);
});

test('HTTP in-flight guards survive fast tab round trips and completion while inactive never resumes polling', async () => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
  const read = deferred<{ port: number; allowLan: boolean }>();
  const save = deferred<void>();
  const { adapter } = mountPage('general', {
    getMomHttpSettings: vi.fn().mockReturnValueOnce(read.promise).mockResolvedValue({ port: 23456, allowLan: false }),
    setMomHttpSettings: vi.fn().mockReturnValue(save.promise),
  });
  await settle();
  const port = activePanel().getByRole('spinbutton', { name: 'Port' }) as HTMLInputElement;
  const general = screen.getByRole('tab', { name: 'General' });
  const proxy = screen.getByRole('tab', { name: 'MCP Proxy' });
  await fireEvent.click(proxy);
  await fireEvent.click(general);
  await settle();
  expect(adapter.getMomHttpSettings).toHaveBeenCalledOnce();
  await fireEvent.click(proxy);
  read.resolve({ port: 18765, allowLan: false });
  await settle();
  await vi.advanceTimersByTimeAsync(6000);
  expect(adapter.getMomHttpSettings).toHaveBeenCalledOnce();
  const inactiveGeneral = within(document.getElementById('settings-tabpanel-general') as HTMLElement);
  await fireEvent.click(inactiveGeneral.getByRole('button', { name: 'Save HTTP Settings', hidden: true }));
  await fireEvent.click(inactiveGeneral.getByRole('checkbox', { name: 'Stay awake with lid closed', hidden: true }));
  await fireEvent.click(inactiveGeneral.getByRole('checkbox', { name: 'OpenCode mini island', hidden: true }));
  await fireEvent.submit(
    inactiveGeneral
      .getByRole('button', { name: 'Save connection target', hidden: true })
      .closest('form') as HTMLFormElement,
  );
  expect(adapter.getMomHttpSettings).toHaveBeenCalledOnce();
  expectNoWrites(adapter);
  await fireEvent.click(general);
  await settle();
  expect(activePanel().getByRole('spinbutton', { name: 'Port' })).toBe(port);
  expect(adapter.getMomHttpSettings).toHaveBeenCalledTimes(2);
  await fireEvent.input(port, { target: { value: '24567' } });
  await fireEvent.click(screen.getByRole('button', { name: 'Save HTTP Settings' }));
  await fireEvent.click(proxy);
  save.resolve();
  await settle();
  await vi.advanceTimersByTimeAsync(6000);
  expect(adapter.setMomHttpSettings).toHaveBeenCalledExactlyOnceWith({ port: 24567, allowLan: false });
  expect(adapter.getMomHttpSettings).toHaveBeenCalledTimes(2);
  await fireEvent.click(general);
  await settle();
  expect(adapter.getMomHttpSettings).toHaveBeenCalledTimes(3);
});

test('checked CLI updates survive tab re-entry and a pending upgrade is not cleared or read back while inactive', async () => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
  const upgrade = deferred<void>();
  const { adapter } = mountPage('general', { upgradeOpenCode: vi.fn().mockReturnValue(upgrade.promise) });
  await settle();
  await fireEvent.click(screen.getByRole('button', { name: 'Check CLI updates' }));
  await settle();
  const general = screen.getByRole('tab', { name: 'General' });
  const proxy = screen.getByRole('tab', { name: 'MCP Proxy' });
  await fireEvent.click(proxy);
  await fireEvent.click(general);
  await settle();
  const upgradeButton = screen.getByRole('button', { name: 'Upgrade CLI…' });
  expect(adapter.getOpenCodeInstallation).toHaveBeenCalledOnce();
  expect(adapter.checkOpenCodeUpdate).toHaveBeenCalledOnce();
  await fireEvent.click(upgradeButton);
  await fireEvent.click(screen.getByRole('button', { name: 'Confirm CLI upgrade' }));
  await fireEvent.click(proxy);
  expect(screen.queryByRole('dialog')).toBeNull();
  await fireEvent.click(general);
  await settle();
  expect((screen.getByRole('button', { name: 'Upgrading CLI…' }) as HTMLButtonElement).disabled).toBe(true);
  expect(adapter.getOpenCodeInstallation).toHaveBeenCalledOnce();
  await fireEvent.click(proxy);
  upgrade.resolve();
  await settle();
  expect(adapter.getOpenCodeInstallation).toHaveBeenCalledOnce();
  const serviceReads = adapter.getOpenCodeServiceStatus.mock.calls.length;
  await vi.advanceTimersByTimeAsync(4000);
  expect(adapter.getOpenCodeServiceStatus).toHaveBeenCalledTimes(serviceReads);
  await fireEvent.click(general);
  await settle();
  expect(screen.queryByRole('dialog')).toBeNull();
  expect(adapter.getOpenCodeInstallation).toHaveBeenCalledOnce();
  expect(adapter.getOpenCodeServiceStatus).toHaveBeenCalledTimes(serviceReads + 1);
  expect(adapter.upgradeOpenCode).toHaveBeenCalledExactlyOnceWith(installation.binaryPath, '2.0.0', '2.1.0');
});

test.each(['read', 'save'] as const)(
  'service pending %s and local drafts survive tab round trips without overlapping operations',
  async (operation) => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    const pending = deferred<typeof service>();
    const { adapter } = mountPage(
      'general',
      operation === 'read'
        ? {
            getOpenCodeServiceStatus: vi
              .fn()
              .mockResolvedValueOnce(service)
              .mockReturnValueOnce(pending.promise)
              .mockResolvedValue(service),
          }
        : { setOpenCodeServiceTarget: vi.fn().mockReturnValue(pending.promise) },
    );
    await settle();
    const url = activePanel().getByRole('textbox', { name: 'OpenCode backend URL' }) as HTMLInputElement;
    const password = activePanel().getByLabelText('Service password (optional)') as HTMLInputElement;
    await fireEvent.input(url, { target: { value: 'http://127.0.0.1:54321/retained' } });
    await fireEvent.input(password, { target: { value: 'retained-secret' } });
    await fireEvent.click(
      screen.getByRole('button', {
        name: operation === 'read' ? 'Refresh connection' : 'Save connection target',
      }),
    );
    const reads = adapter.getOpenCodeServiceStatus.mock.calls.length;
    const general = screen.getByRole('tab', { name: 'General' });
    const proxy = screen.getByRole('tab', { name: 'MCP Proxy' });
    await fireEvent.click(proxy);
    await fireEvent.click(general);
    await settle();
    expect(adapter.getOpenCodeServiceStatus).toHaveBeenCalledTimes(reads);
    expect((screen.getByRole('button', { name: 'Check CLI updates' }) as HTMLButtonElement).disabled).toBe(true);
    expect(url.value).toBe('http://127.0.0.1:54321/retained');
    expect(password.value).toBe('retained-secret');
    await fireEvent.click(proxy);
    pending.resolve({ ...service, target: { ...service.target, url: 'http://127.0.0.1:54321/retained' } });
    await settle();
    await vi.advanceTimersByTimeAsync(4000);
    expect(adapter.getOpenCodeServiceStatus).toHaveBeenCalledTimes(reads);
    await fireEvent.click(general);
    await settle();
    expect(adapter.getOpenCodeServiceStatus).toHaveBeenCalledTimes(reads + 1);
    expect(activePanel().getByRole('textbox', { name: 'OpenCode backend URL' })).toBe(url);
    expect(activePanel().getByLabelText('Service password (optional)')).toBe(password);
    expect(url.value).toBe('http://127.0.0.1:54321/retained');
    expect(password.value).toBe(operation === 'save' ? '' : 'retained-secret');
    expect(adapter.getOpenCodeInstallation).toHaveBeenCalledOnce();
    expect(adapter.setOpenCodeServiceTarget).toHaveBeenCalledTimes(operation === 'save' ? 1 : 0);
  },
);

test('search never opens a checked CLI upgrade or restarts its installation inspection', async () => {
  const { adapter } = mountPage();
  const check = await screen.findByRole('button', { name: 'Check CLI updates' });
  await waitFor(() => expect((check as HTMLButtonElement).disabled).toBe(false));
  await fireEvent.click(check);
  await screen.findByRole('button', { name: 'Upgrade CLI…' });
  const search = screen.getByRole('searchbox', { name: 'Search settings' });
  await fireEvent.input(search, { target: { value: 'upgrade' } });
  await fireEvent.click(within(results()).getByRole('button', { name: /CLI updates/ }));
  expect(document.activeElement?.id).toBe('opencode-settings-title');
  expect(screen.queryByRole('dialog')).toBeNull();
  expect(adapter.checkOpenCodeUpdate).toHaveBeenCalledOnce();
  expect(adapter.getOpenCodeInstallation).toHaveBeenCalledOnce();
  expectNoWrites(adapter);
});

test('browser search still locates desktop-only settings without fetching native data', async () => {
  native.isTauri.mockReturnValue(false);
  const { adapter } = mountPage('mcp-proxy');
  const search = screen.getByRole('searchbox', { name: 'Search settings' });
  await fireEvent.input(search, { target: { value: 'password' } });
  await fireEvent.click(within(results()).getByRole('button', { name: /Service password/ }));
  expect(document.activeElement?.id).toBe('opencode-settings-title');
  expect(screen.getByText(en['settings.opencodeDesktopOnly'])).toBeTruthy();
  expect(adapter.getOpenCodeServiceStatus).not.toHaveBeenCalled();
  expect(adapter.getOpenCodeInstallation).not.toHaveBeenCalled();
  expect(adapter.getIslandState).not.toHaveBeenCalled();
  expectNoWrites(adapter);
});

test('Behavior search is static, third-tab drafts survive and no hidden Behavior native work starts', async () => {
  const { adapter } = mountPage();
  await settle();
  const password = activePanel().getByLabelText('Service password (optional)') as HTMLInputElement;
  await fireEvent.input(password, { target: { value: 'keep-secret-across-behavior' } });
  const search = screen.getByRole('searchbox', { name: 'Search settings' });
  expect(adapter.getGlobalBehavior).not.toHaveBeenCalled();
  await fireEvent.input(search, { target: { value: 'AGENTS.md' } });
  expect(adapter.getGlobalBehavior).not.toHaveBeenCalled();
  await fireEvent.click(within(results()).getByRole('button', { name: /Global behavior/ }));
  await settle();
  const input = activePanel().getByLabelText(en['settings.behaviorContent']) as HTMLTextAreaElement;
  expect(document.activeElement?.id).toBe('settings-behavior-title');
  expect(adapter.getGlobalBehavior).toHaveBeenCalledOnce();
  await fireEvent.input(input, { target: { value: 'behavior-private-draft-marker' } });
  for (const value of ['behavior-private-draft-marker', 'Private saved instructions', '/resolved/AGENTS.md']) {
    await fireEvent.input(search, { target: { value } });
    expect(screen.queryByRole('list', { name: 'Matching settings' })).toBeNull();
  }
  const general = screen.getByRole('tab', { name: 'General' });
  await fireEvent.click(general);
  await settle();
  expect(activePanel().getByLabelText('Service password (optional)')).toBe(password);
  expect(password.value).toBe('keep-secret-across-behavior');
  await fireEvent.keyDown(general, { key: 'End' });
  await settle();
  expect(document.activeElement?.id).toBe('settings-tab-behavior');
  expect(activePanel().getByLabelText(en['settings.behaviorContent'])).toBe(input);
  expect(input.value).toBe('behavior-private-draft-marker');
  expect(adapter.getGlobalBehavior).toHaveBeenCalledOnce();
  for (const [url] of vi.mocked(goto).mock.calls) expect(String(url)).not.toContain('private');
  expectNoWrites(adapter);
});

test('settings four regions name locale', async () => {
  native.isTauri.mockReturnValue(true);
  const { config } = mountPage();
  await settle();

  const appRegion = screen.getByRole('region', { name: en['settings.appBehavior'] });
  const opencodeRegion = screen.getByRole('region', { name: en['settings.opencodeTitle'] });
  const httpRegion = screen.getByRole('region', { name: en['settings.http.title'] });
  const updatesRegion = screen.getByRole('region', { name: en['settings.updates'] });

  expect(appRegion.compareDocumentPosition(opencodeRegion)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
  expect(opencodeRegion.compareDocumentPosition(httpRegion)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
  expect(httpRegion.compareDocumentPosition(updatesRegion)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);

  const autostart = within(appRegion).getByRole('checkbox', { name: en['settings.autostart'] });
  const island = within(appRegion).getByRole('checkbox', { name: en['settings.island'] });
  const lid = within(appRegion).getByRole('checkbox', { name: en['settings.lid'] });
  expect(autostart.compareDocumentPosition(island)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
  expect(island.compareDocumentPosition(lid)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);

  const urlField = within(opencodeRegion).getByRole('textbox', { name: en['settings.serviceUrl'] });
  const cliBtn = within(opencodeRegion).getByRole('button', { name: en['settings.opencodeCheck'] });
  expect(urlField.compareDocumentPosition(cliBtn)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);

  const port = within(httpRegion).getByRole('spinbutton', { name: en['settings.http.port'] });
  const allowLan = within(httpRegion).getByRole('checkbox', { name: en['settings.http.allowLan'] });
  expect(port).toBeTruthy();
  expect(allowLan).toBeTruthy();

  const updatesBtn = within(updatesRegion).getByRole('button', { name: en['settings.checkUpdates'] });
  expect(updatesBtn).toBeTruthy();
  expect(screen.getByText(en['settings.supportProject'])).toBeTruthy();

  config.setLocale('zh');
  await tick();
  expect(screen.getByRole('region', { name: zh['settings.appBehavior'] })).toBeTruthy();
  config.setLocale('en');
  await tick();
});
