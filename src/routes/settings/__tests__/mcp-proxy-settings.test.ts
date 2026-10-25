import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { tick } from 'svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { goto } from '$app/navigation';
import { page } from '$app/state';
import type { ConfigStore } from '$src/config/store.svelte.js';
import Harness from '$src/test-utils/harness.svelte';
import type { McpProxySettingsDto, McpProxyStateDto, MomHttpSettings } from '$src/types/mcp-proxy.js';
import SettingsPage from '../+page.svelte';
import McpProxySettings from '../McpProxySettings.svelte';

vi.mock('$app/navigation', () => ({
  goto: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('$src/shell/toast.svelte.js', () => ({
  toast: vi.fn(),
}));

vi.mock('$app/state', async () => {
  const { createSubscriber } = await import('svelte/reactivity');
  let notify: () => void = () => {};
  const track = createSubscriber((update) => {
    notify = update;
  });
  const state = {
    url: new URL('http://localhost/settings'),
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

const defaultSettings: McpProxySettingsDto = {
  enabled: true,
  idleTimeout: 300,
  maxUnits: 32,
  revision: 5,
};

const defaultHttpSettings: MomHttpSettings = { port: 23456, allowLan: false };

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
  activeUnits: 0,
  activeSessions: 7,
  availableUnits: 32,
  maxUnits: 32,
  securityErrors: [],
  services: [],
  revision: 5,
  generation: 2,
};

function makeConfig(overrides: Record<string, unknown> = {}) {
  return {
    preferences: { locale: 'en', theme: 'dark' },
    getAutostart: vi.fn().mockResolvedValue(false),
    setAutostart: vi.fn().mockResolvedValue(true),
    getLidProtection: vi.fn().mockResolvedValue({ enabled: false, phase: 'disabled' }),
    getAppInfo: vi.fn().mockResolvedValue({ version: '2.0.0', platform: 'macos' }),
    checkForUpdates: vi.fn(),
    getMomHttpSettings: vi.fn().mockResolvedValue(defaultHttpSettings),
    setMomHttpSettings: vi.fn().mockResolvedValue(undefined),
    getMcpProxySettings: vi.fn().mockResolvedValue(defaultSettings),
    getMcpProxyState: vi.fn().mockResolvedValue(defaultProxyState),
    saveMcpProxySettings: vi.fn().mockResolvedValue(undefined),
    rotateMcpProxyKey: vi.fn().mockResolvedValue('test-rotated-key-xyz'),
    ...overrides,
  } as unknown as ConfigStore;
}

beforeEach(() => {
  page.url = new URL('http://localhost/settings');
  vi.mocked(goto).mockClear();
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

async function settle() {
  await tick();
  await Promise.resolve();
  await tick();
}

test.each([false, true])(
  'HTTP=%s retains in-flight read/save guards across activation and never polls while inactive',
  async (http) => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    let finishRead!: (value: McpProxySettingsDto | MomHttpSettings) => void;
    let finishSave!: () => void;
    const pendingRead = new Promise<McpProxySettingsDto | MomHttpSettings>((resolve) => {
      finishRead = resolve;
    });
    const pendingSave = new Promise<void>((resolve) => {
      finishSave = resolve;
    });
    const defaults = http ? defaultHttpSettings : defaultSettings;
    const getSettings = vi.fn().mockReturnValueOnce(pendingRead).mockResolvedValue(defaults);
    const setSettings = vi.fn().mockReturnValue(pendingSave);
    const config = makeConfig({
      [http ? 'getMomHttpSettings' : 'getMcpProxySettings']: getSettings,
      [http ? 'setMomHttpSettings' : 'saveMcpProxySettings']: setSettings,
    });
    const view = render(McpProxySettings, { http, active: false }, { wrapper: Harness, wrapperProps: { config } });
    await settle();
    expect(getSettings).not.toHaveBeenCalled();
    expect(config.getMcpProxyState).not.toHaveBeenCalled();
    await view.rerender({ active: true });
    await view.rerender({ active: false });
    await view.rerender({ active: true });
    expect(getSettings).toHaveBeenCalledOnce();
    expect(vi.getTimerCount()).toBe(0);
    await view.rerender({ active: false });
    finishRead(defaults);
    await settle();
    await vi.advanceTimersByTimeAsync(6000);
    expect(getSettings).toHaveBeenCalledOnce();
    expect(vi.getTimerCount()).toBe(0);
    await fireEvent.click(screen.getByRole('button', { name: http ? 'Save HTTP Settings' : 'Save Gateway Settings' }));
    expect(setSettings).not.toHaveBeenCalled();
    await view.rerender({ active: true });
    await settle();
    const input = screen.getByLabelText(http ? 'Port' : 'Session Idle Timeout (seconds)') as HTMLInputElement;
    await fireEvent.input(input, { target: { value: '24567' } });
    await fireEvent.click(screen.getByRole('button', { name: http ? 'Save HTTP Settings' : 'Save Gateway Settings' }));
    expect(setSettings).toHaveBeenCalledOnce();
    await view.rerender({ active: false });
    await view.rerender({ active: true });
    expect(getSettings).toHaveBeenCalledTimes(2);
    await view.rerender({ active: false });
    finishSave();
    await settle();
    await vi.advanceTimersByTimeAsync(6000);
    expect(getSettings).toHaveBeenCalledTimes(2);
    expect(vi.getTimerCount()).toBe(0);
    await view.rerender({ active: true });
    await settle();
    expect(screen.getByLabelText(http ? 'Port' : 'Session Idle Timeout (seconds)')).toBe(input);
    expect(getSettings).toHaveBeenCalledTimes(3);
    await vi.advanceTimersByTimeAsync(2000);
    expect(getSettings).toHaveBeenCalledTimes(4);
  },
);

test.each([false, true])('HTTP=%s polls without replacing a dirty draft or its expected revision', async (http) => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
  const baseline = http ? defaultHttpSettings : defaultSettings;
  const getSettings = vi.fn().mockResolvedValue({ ...baseline });
  const getMcpProxyState = vi.fn().mockResolvedValue(defaultProxyState);
  const config = makeConfig({
    [http ? 'getMomHttpSettings' : 'getMcpProxySettings']: getSettings,
    getMcpProxyState,
  });
  render(McpProxySettings, { http }, { wrapper: Harness, wrapperProps: { config } });
  await settle();

  const field = http ? 'port' : 'idleTimeout';
  const input = screen.getByLabelText(http ? 'Port' : 'Session Idle Timeout (seconds)') as HTMLInputElement;
  getSettings.mockResolvedValue({ ...baseline, [field]: 23457, ...(!http && { revision: 6 }) });
  await vi.advanceTimersByTimeAsync(2000);
  expect(input.value).toBe('23457');

  await fireEvent.input(input, { target: { value: '24567' } });
  getSettings.mockResolvedValue({ ...baseline, [field]: 23458, ...(!http && { revision: 7 }) });
  getMcpProxyState.mockResolvedValue({ ...defaultProxyState, activeSessions: 9 });
  await vi.advanceTimersByTimeAsync(2000);
  expect(input.value).toBe('24567');

  await fireEvent.click(screen.getByRole('button', { name: http ? 'Save HTTP Settings' : 'Save Gateway Settings' }));
  await settle();
  if (http) {
    expect(config.setMomHttpSettings).toHaveBeenCalledExactlyOnceWith({ ...defaultHttpSettings, port: 24567 });
    expect(config.saveMcpProxySettings).not.toHaveBeenCalled();
  } else {
    expect(config.saveMcpProxySettings).toHaveBeenCalledExactlyOnceWith(
      { ...defaultSettings, idleTimeout: 24567, revision: 6 },
      6,
    );
    expect(config.setMomHttpSettings).not.toHaveBeenCalled();
  }
});

test.each([
  [false, false],
  [false, true],
  [true, false],
  [true, true],
])('HTTP=%s save failure=%s preserves polling and refreshes successful saves', async (http, fails) => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
  let resolve!: () => void;
  let reject!: (reason: unknown) => void;
  const pending = new Promise<void>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  const baseline = http ? defaultHttpSettings : defaultSettings;
  const getSettings = vi.fn().mockResolvedValue({ ...baseline });
  const saveSettings = vi.fn().mockReturnValue(pending);
  const config = makeConfig({
    [http ? 'getMomHttpSettings' : 'getMcpProxySettings']: getSettings,
    [http ? 'setMomHttpSettings' : 'saveMcpProxySettings']: saveSettings,
  });
  const { unmount } = render(McpProxySettings, { http }, { wrapper: Harness, wrapperProps: { config } });
  await settle();

  const field = http ? 'port' : 'idleTimeout';
  const input = screen.getByLabelText(http ? 'Port' : 'Session Idle Timeout (seconds)') as HTMLInputElement;
  const save = screen.getByRole('button', { name: http ? 'Save HTTP Settings' : 'Save Gateway Settings' });
  await fireEvent.input(input, { target: { value: '24567' } });
  await fireEvent.click(save);
  await vi.advanceTimersByTimeAsync(6000);
  expect(getSettings).toHaveBeenCalledTimes(1);

  if (fails) {
    reject(new Error('Save failed'));
  } else {
    getSettings.mockResolvedValue({ ...baseline, [field]: 24568, ...(!http && { revision: 6 }) });
    resolve();
  }
  await settle();
  expect((save as HTMLButtonElement).disabled).toBe(false);
  expect(getSettings).toHaveBeenCalledTimes(fails ? 1 : 2);
  expect(input.value).toBe(fails ? '24567' : '24568');

  await vi.advanceTimersByTimeAsync(2000);
  expect(getSettings).toHaveBeenCalledTimes(fails ? 2 : 3);
  expect(input.value).toBe(fails ? '24567' : '24568');
  unmount();
  await vi.advanceTimersByTimeAsync(4000);
  expect(getSettings).toHaveBeenCalledTimes(fails ? 2 : 3);
});

test.each([false, true])('HTTP=%s does not start a save while a status read is in flight', async (http) => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
  const baseline = http ? defaultHttpSettings : defaultSettings;
  let resolve!: (value: McpProxySettingsDto | MomHttpSettings) => void;
  const pending = new Promise<McpProxySettingsDto | MomHttpSettings>((yes) => {
    resolve = yes;
  });
  const getSettings = vi
    .fn()
    .mockResolvedValueOnce({ ...baseline })
    .mockReturnValueOnce(pending);
  const config = makeConfig({ [http ? 'getMomHttpSettings' : 'getMcpProxySettings']: getSettings });
  render(McpProxySettings, { http }, { wrapper: Harness, wrapperProps: { config } });
  await settle();
  await vi.advanceTimersByTimeAsync(2000);
  const save = screen.getByRole('button', {
    name: http ? 'Save HTTP Settings' : 'Save Gateway Settings',
  }) as HTMLButtonElement;
  expect(save.disabled).toBe(true);
  await fireEvent.click(save);
  expect(config.saveMcpProxySettings).not.toHaveBeenCalled();
  expect(config.setMomHttpSettings).not.toHaveBeenCalled();
  await vi.advanceTimersByTimeAsync(6000);
  expect(getSettings).toHaveBeenCalledTimes(2);
  resolve({ ...baseline });
  await settle();
  expect(save.disabled).toBe(false);
});

test('settings switches to MCP Proxy tab and does not read autostart on proxy tab', async () => {
  page.url = new URL('http://localhost/settings?tab=mcp-proxy');
  const getAutostart = vi.fn().mockResolvedValue(false);
  const config = makeConfig({ getAutostart });

  render(SettingsPage, {}, { wrapper: Harness, wrapperProps: { config } });

  expect(getAutostart).not.toHaveBeenCalled();
  await waitFor(() => {
    expect(config.getMcpProxySettings).toHaveBeenCalled();
  });

  const proxy = within(screen.getByRole('tabpanel', { name: 'MCP Proxy' }));
  expect(proxy.getByRole('region', { name: 'MCP Proxy Gateway' })).toBeDefined();
  expect(proxy.getByRole('spinbutton', { name: 'Session Idle Timeout (seconds)' })).toBeDefined();
  expect(proxy.getByRole('spinbutton', { name: 'Maximum Concurrent Units' })).toBeDefined();
  const generalPanel = document.getElementById('settings-tabpanel-general') as HTMLElement;
  expect(generalPanel.hidden).toBe(true);
  expect(within(generalPanel).getByRole('spinbutton', { name: 'Port', hidden: true })).toBeDefined();
  expect(screen.queryByRole('spinbutton', { name: 'Port' })).toBeNull();
  expect(screen.queryByRole('checkbox', { name: 'Allow LAN' })).toBeNull();
});

test('General owns HTTP settings without proxy writes and stops polling on tab switch', async () => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
  const config = makeConfig();
  const { unmount } = render(SettingsPage, {}, { wrapper: Harness, wrapperProps: { config } });
  await settle();
  const generalPanel = screen.getByRole('tabpanel', { name: 'General' });
  const general = within(generalPanel);
  expect((document.getElementById('settings-tabpanel-mcp-proxy') as HTMLElement).hidden).toBe(true);
  expect(screen.queryByRole('checkbox', { name: 'Enable MCP Gateway' })).toBeNull();
  expect(screen.queryByRole('button', { name: 'Rotate Secret Token…' })).toBeNull();

  const port = general.getByRole('spinbutton', { name: 'Port' }) as HTMLInputElement;
  const save = general.getByRole('button', { name: 'Save HTTP Settings' }) as HTMLButtonElement;
  for (const value of ['', '0', '65536', '18765.5']) {
    await fireEvent.input(port, { target: { value } });
    expect(save.disabled).toBe(true);
  }
  expect(config.saveMcpProxySettings).not.toHaveBeenCalled();
  expect(config.setMomHttpSettings).not.toHaveBeenCalled();
  await fireEvent.input(port, { target: { value: '18765' } });
  await fireEvent.click(general.getByRole('checkbox', { name: 'Allow LAN' }));
  expect(general.getByRole('alert').textContent).toContain('Enable only on trusted networks.');
  await fireEvent.click(save);
  await settle();
  expect(config.setMomHttpSettings).toHaveBeenCalledExactlyOnceWith({ port: 18765, allowLan: true });
  expect(config.saveMcpProxySettings).not.toHaveBeenCalled();

  await fireEvent.click(screen.getByRole('button', { name: 'Refresh' }));
  await settle();
  expect(config.getMomHttpSettings).toHaveBeenCalledTimes(3);
  expect(config.getMcpProxySettings).not.toHaveBeenCalled();
  await fireEvent.click(screen.getByRole('tab', { name: 'MCP Proxy' }));
  await settle();
  expect(generalPanel.hidden).toBe(true);
  expect(general.getByRole('spinbutton', { name: 'Port', hidden: true })).toBe(port);
  const proxy = within(screen.getByRole('tabpanel', { name: 'MCP Proxy' }));
  expect(proxy.getByRole('spinbutton', { name: 'Session Idle Timeout (seconds)' })).toBeDefined();
  await vi.advanceTimersByTimeAsync(2000);
  expect(config.getMomHttpSettings).toHaveBeenCalledTimes(3);
  expect(config.getMcpProxySettings).toHaveBeenCalledTimes(2);
  unmount();
  await vi.advanceTimersByTimeAsync(4000);
  expect(config.getMomHttpSettings).toHaveBeenCalledTimes(3);
  expect(config.getMcpProxySettings).toHaveBeenCalledTimes(2);
});

test('HTTP settings displays error banner after a failed read', async () => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
  const getMomHttpSettings = vi
    .fn()
    .mockResolvedValueOnce(defaultHttpSettings)
    .mockRejectedValue(new Error('Read failed'));
  const config = makeConfig({ getMomHttpSettings });
  render(McpProxySettings, { http: true }, { wrapper: Harness, wrapperProps: { config } });
  await settle();
  expect(screen.queryByRole('alert')).toBeNull();
  await vi.advanceTimersByTimeAsync(2000);
  expect(screen.getByRole('alert')).toBeDefined();
  expect(screen.getByRole('alert').textContent).toContain('Read failed');
});

test('McpProxySettings saves gateway settings with expected revision', async () => {
  page.url = new URL('http://localhost/settings?tab=mcp-proxy');
  const saveMcpProxySettings = vi.fn().mockResolvedValue(undefined);
  const config = makeConfig({ saveMcpProxySettings });

  render(SettingsPage, {}, { wrapper: Harness, wrapperProps: { config } });

  await waitFor(() => {
    expect(config.getMcpProxySettings).toHaveBeenCalled();
  });

  const idleTimeoutInput = screen.getByLabelText('Session Idle Timeout (seconds)');
  await fireEvent.input(idleTimeoutInput, { target: { value: '450' } });
  const saveBtn = screen.getByRole('button', { name: 'Save Gateway Settings' });
  await fireEvent.click(saveBtn);

  await waitFor(() => {
    expect(saveMcpProxySettings).toHaveBeenCalledWith(
      expect.objectContaining({
        idleTimeout: 450,
        revision: 5,
      }),
      5,
    );
  });
});

test('RotateKeyDialog confirms and displays rotated secret token once', async () => {
  page.url = new URL('http://localhost/settings?tab=mcp-proxy');
  const rotateMcpProxyKey = vi.fn().mockResolvedValue('secret-key-super-safe');
  const config = makeConfig({ rotateMcpProxyKey });

  render(SettingsPage, {}, { wrapper: Harness, wrapperProps: { config } });

  await waitFor(() => {
    expect(config.getMcpProxySettings).toHaveBeenCalled();
  });

  const rotateBtn = screen.getByRole('button', { name: /Rotate Secret Token/i });
  await fireEvent.click(rotateBtn);

  // Dialog opens
  const confirmBtn = screen.getByRole('button', { name: 'Generate New Token' });
  await fireEvent.click(confirmBtn);

  await waitFor(() => {
    expect(rotateMcpProxyKey).toHaveBeenCalled();
  });

  // Displays key input
  const keyInput = (await screen.findByLabelText('New Secret Token Generated')) as HTMLInputElement;
  expect(keyInput.value).toBe('secret-key-super-safe');

  // Close dialog
  const closeBtn = screen.getByRole('button', { name: 'Close' });
  await fireEvent.click(closeBtn);
});
