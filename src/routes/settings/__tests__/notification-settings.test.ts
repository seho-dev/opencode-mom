import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { tick } from 'svelte';
import { beforeEach, expect, test, vi } from 'vitest';
import type { ConfigStore } from '$src/config/store.svelte.js';
import { en } from '$src/i18n/dictionaries/en.js';
import Harness from '$src/test-utils/harness.svelte';
import type { NotificationSettings as NotificationPreferences } from '$src/types/notifications.js';
import NotificationSettings from '../NotificationSettings.svelte';

const native = vi.hoisted(() => ({ isTauri: vi.fn(() => true) }));
vi.mock('@tauri-apps/api/core', () => ({ isTauri: native.isTauri }));

const defaults: NotificationPreferences = {
  enabled: false,
  agentCompleted: true,
  approvalRequired: true,
  agentError: true,
};

const copy = {
  master: en['settings.notifications'],
  completed: en['settings.notificationsAgentCompleted'],
  authorize: en['settings.notificationsRequestPermission'],
  open: en['settings.notificationsOpenSettings'],
  refresh: en['settings.notificationsRefresh'],
  granted: en['settings.notificationsPermissionGranted'],
  denied: en['settings.notificationsPermissionDenied'],
  prompt: en['settings.notificationsPermissionPrompt'],
  promptWindows: en['settings.notificationsPermissionPromptWindows'],
  failed: en['settings.notificationsPermissionFailed'],
  saveFailed: en['settings.notificationsSaveFailed'],
};

function mount(overrides: Record<string, unknown> = {}, active = true) {
  const config = {
    preferences: { theme: 'dark', locale: 'en' },
    getAppInfo: vi.fn().mockResolvedValue({ version: '2.0.0', platform: 'macos' }),
    getNotificationSettings: vi.fn().mockResolvedValue(defaults),
    setNotificationSettings: vi.fn().mockResolvedValue(defaults),
    getNotificationPermission: vi.fn().mockResolvedValue('prompt'),
    requestNotificationPermission: vi.fn().mockResolvedValue('granted'),
    openNotificationSettings: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  };
  const view = render(
    NotificationSettings,
    { active },
    { wrapper: Harness, wrapperProps: { config: config as unknown as ConfigStore } },
  );
  return { ...view, config };
}

function spy(config: Record<string, unknown>, key: string) {
  return config[key] as ReturnType<typeof vi.fn>;
}

async function loaded(config: ReturnType<typeof mount>['config']) {
  await waitFor(() => expect(config.getNotificationSettings).toHaveBeenCalled());
  await waitFor(() => expect(config.getNotificationPermission).toHaveBeenCalled());
}

function master() {
  return screen.getByRole('checkbox', { name: copy.master }) as HTMLInputElement;
}

function status() {
  return screen.getByRole('status').textContent ?? '';
}

// Backend errors must never reach the DOM: the panel renders fixed localized copy only.
const SENTINEL = 'sk-SENTINEL-SECRET prompt=leak-this-command password=hunter2';

async function expectSafeError(copyKey: string) {
  const alert = await screen.findByRole('alert');
  expect(alert.textContent).toContain(copyKey);
  expect(alert.textContent).not.toContain(SENTINEL);
  expect(document.body.textContent ?? '').not.toContain('SENTINEL-SECRET');
  expect(document.body.textContent ?? '').not.toContain('hunter2');
  expect(document.body.textContent ?? '').not.toContain('leak-this-command');
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (cause: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

beforeEach(() => native.isTauri.mockReturnValue(true));

test('active entry reads preference and permission without requesting authorization', async () => {
  const { config } = mount();
  await loaded(config);
  await tick();
  expect(config.requestNotificationPermission).not.toHaveBeenCalled();
  expect(config.setNotificationSettings).not.toHaveBeenCalled();
  expect(config.openNotificationSettings).not.toHaveBeenCalled();
});

// The control shows the confirmed preference, never an optimistic local flip.
test('save pending keeps the control on the confirmed preference, and a rejection stays off', async () => {
  const { config } = mount();
  await loaded(config);
  expect(master().checked).toBe(false);
  const saving = deferred<NotificationPreferences>();
  spy(config, 'setNotificationSettings').mockReturnValueOnce(saving.promise);
  await fireEvent.click(master());
  await waitFor(() => expect(config.setNotificationSettings).toHaveBeenCalledTimes(1));
  expect(master().checked).toBe(false);
  saving.reject(new Error('write failed'));
  await tick();
  await waitFor(() => expect(master().checked).toBe(false));
  expect(config.requestNotificationPermission).not.toHaveBeenCalled();
});

test('enabling save success then requests permission exactly once after the preference is stored', async () => {
  const { config } = mount();
  await loaded(config);
  const order: string[] = [];
  spy(config, 'setNotificationSettings').mockImplementationOnce(async () => {
    order.push('save');
    return { ...defaults, enabled: true };
  });
  spy(config, 'requestNotificationPermission').mockImplementationOnce(async () => {
    order.push('request');
    return 'granted';
  });
  await fireEvent.click(master());
  await waitFor(() => expect(config.requestNotificationPermission).toHaveBeenCalledTimes(1));
  expect(order).toEqual(['save', 'request']);
  expect(config.requestNotificationPermission).toHaveBeenCalledWith();
  await waitFor(() => expect(master().checked).toBe(true));
  expect(master().disabled).toBe(false);
});

test('checkbox reflects the confirmed save while the permission request is pending', async () => {
  const { config } = mount();
  await loaded(config);
  const pendingRequest = deferred<string>();
  spy(config, 'setNotificationSettings').mockResolvedValueOnce({ ...defaults, enabled: true });
  spy(config, 'requestNotificationPermission').mockReturnValueOnce(pendingRequest.promise);
  await fireEvent.click(master());
  await waitFor(() => expect(config.requestNotificationPermission).toHaveBeenCalledTimes(1));
  const checkbox = master();
  expect(checkbox.checked).toBe(true);
  expect(checkbox.disabled).toBe(true);
  expect(checkbox.indeterminate).toBe(false);
  pendingRequest.resolve('granted');
  await waitFor(() => expect(master().disabled).toBe(false));
  expect(master().checked).toBe(true);
  expect(status()).toBe(copy.granted);
});

test('declined permission keeps the confirmed enabled preference and shows the denied state', async () => {
  const { config } = mount();
  await loaded(config);
  spy(config, 'setNotificationSettings').mockResolvedValueOnce({ ...defaults, enabled: true });
  spy(config, 'requestNotificationPermission').mockResolvedValueOnce('denied');
  await fireEvent.click(master());
  await waitFor(() => expect(config.requestNotificationPermission).toHaveBeenCalledTimes(1));
  await waitFor(() => expect(master().disabled).toBe(false));
  expect(config.setNotificationSettings).toHaveBeenCalledExactlyOnceWith({ ...defaults, enabled: true });
  expect(master().checked).toBe(true);
  expect(status()).toBe(copy.denied);
  expect(screen.queryByRole('alert')).toBeNull();
});

test('category toggle and disabling never request authorization', async () => {
  const on = { ...defaults, enabled: true };
  const { config } = mount({ getNotificationSettings: vi.fn().mockResolvedValue(on) });
  await loaded(config);
  spy(config, 'setNotificationSettings').mockResolvedValueOnce({ ...on, agentCompleted: false });
  await fireEvent.click(screen.getByRole('checkbox', { name: copy.completed }));
  await waitFor(() => expect(config.setNotificationSettings).toHaveBeenCalledTimes(1));
  await waitFor(() => expect(master().disabled).toBe(false));

  spy(config, 'setNotificationSettings').mockResolvedValueOnce({ ...on, agentCompleted: false, enabled: false });
  await fireEvent.click(master());
  await waitFor(() => expect(config.setNotificationSettings).toHaveBeenCalledTimes(2));
  await waitFor(() => expect(master().disabled).toBe(false));
  expect(config.requestNotificationPermission).not.toHaveBeenCalled();
  expect(config.openNotificationSettings).not.toHaveBeenCalled();
});

test('failed save never requests authorization and keeps no optimistic enablement', async () => {
  const { config } = mount();
  await loaded(config);
  spy(config, 'setNotificationSettings').mockRejectedValueOnce(new Error(SENTINEL));
  await fireEvent.click(master());
  await expectSafeError(copy.saveFailed);
  // The failed save must never be presented as enabled: the DOM follows the confirmed reread.
  await waitFor(() => expect(config.getNotificationSettings).toHaveBeenCalledTimes(2));
  const reread = spy(config, 'getNotificationSettings').mock.results.at(-1)?.value as Promise<NotificationPreferences>;
  await expect(reread).resolves.toMatchObject({ enabled: false });
  await waitFor(() => expect(master().checked).toBe(false));
  expect((master() as HTMLInputElement).disabled).toBe(false);
  expect(config.requestNotificationPermission).not.toHaveBeenCalled();
});

test('permission request failure keeps the saved preference, shows a safe permission alert, never save failure', async () => {
  const { config } = mount();
  await loaded(config);
  spy(config, 'setNotificationSettings').mockResolvedValueOnce({ ...defaults, enabled: true });
  spy(config, 'requestNotificationPermission').mockRejectedValueOnce(new Error(SENTINEL));
  await fireEvent.click(master());
  await expectSafeError(copy.failed);
  expect(document.body.textContent).not.toContain(copy.saveFailed);
  await waitFor(() => expect(master().disabled).toBe(false));
  expect(master().checked).toBe(true);
  expect(config.setNotificationSettings).toHaveBeenCalledExactlyOnceWith({ ...defaults, enabled: true });
  expect(config.requestNotificationPermission).toHaveBeenCalledTimes(1);
  // Recovery reads the authoritative state instead of retrying or rescuing the save.
  const reads = spy(config, 'getNotificationSettings').mock.calls.length;
  const permissions = spy(config, 'getNotificationPermission').mock.calls.length;
  await fireEvent.click(screen.getByRole('button', { name: copy.refresh }));
  await waitFor(() => expect(spy(config, 'getNotificationSettings').mock.calls.length).toBe(reads + 1));
  await waitFor(() => expect(spy(config, 'getNotificationPermission').mock.calls.length).toBe(permissions + 1));
  expect(config.requestNotificationPermission).toHaveBeenCalledTimes(1);
});

test('save completing after unmount never requests authorization', async () => {
  const { config, unmount } = mount();
  await loaded(config);
  const saving = deferred<NotificationPreferences>();
  spy(config, 'setNotificationSettings').mockReturnValueOnce(saving.promise);
  await fireEvent.click(master());
  await waitFor(() => expect(config.setNotificationSettings).toHaveBeenCalledTimes(1));
  unmount();
  saving.resolve({ ...defaults, enabled: true });
  await tick();
  await tick();
  expect(config.requestNotificationPermission).not.toHaveBeenCalled();
});

test('save completing after the panel goes inactive never requests authorization', async () => {
  const config = {
    preferences: { theme: 'dark', locale: 'en' },
    getAppInfo: vi.fn().mockResolvedValue({ version: '2.0.0', platform: 'macos' }),
    getNotificationSettings: vi.fn().mockResolvedValue(defaults),
    setNotificationSettings: vi.fn(),
    getNotificationPermission: vi.fn().mockResolvedValue('prompt'),
    requestNotificationPermission: vi.fn().mockResolvedValue('granted'),
    openNotificationSettings: vi.fn().mockResolvedValue(undefined),
  };
  const saving = deferred<NotificationPreferences>();
  spy(config, 'setNotificationSettings').mockReturnValueOnce(saving.promise);
  const view = render(
    NotificationSettings,
    { active: true },
    { wrapper: Harness, wrapperProps: { config: config as unknown as ConfigStore } },
  );
  await loaded(config);
  await fireEvent.click(master());
  await waitFor(() => expect(config.setNotificationSettings).toHaveBeenCalledTimes(1));
  await view.rerender({ active: false });
  saving.resolve({ ...defaults, enabled: true });
  await tick();
  await tick();
  expect(config.requestNotificationPermission).not.toHaveBeenCalled();
});

// Windows: the prompt-labelled action still issues the request IPC; the backend queries the real
// state and opens the OS settings page itself, so the panel must never call openNotificationSettings.
test('windows prompt action uses the request IPC and never opens settings from the panel', async () => {
  const { config } = mount({
    getAppInfo: vi.fn().mockResolvedValue({ version: '2.0.0', platform: 'windows' }),
    getNotificationSettings: vi.fn().mockResolvedValue({ ...defaults, enabled: true }),
    getNotificationPermission: vi.fn().mockResolvedValue('prompt'),
  });
  await loaded(config);
  expect(screen.queryByRole('button', { name: copy.authorize })).toBeNull();
  const labels = screen.getAllByRole('button').map((button) => button.textContent?.trim());
  expect(labels.filter((label) => label === copy.open)).toHaveLength(1);
  // Windows copy must not imply a macOS-style authorization dialog.
  expect(status()).toBe(copy.promptWindows);
  expect(document.body.textContent).not.toContain(copy.prompt);
  expect(config.openNotificationSettings).not.toHaveBeenCalled();

  await fireEvent.click(screen.getByRole('button', { name: copy.open }));
  await waitFor(() => expect(config.requestNotificationPermission).toHaveBeenCalledTimes(1));
  expect(config.requestNotificationPermission).toHaveBeenCalledWith();
  expect(config.openNotificationSettings).not.toHaveBeenCalled();
});

test('windows false to true saves then requests once and never fakes the permission result', async () => {
  const { config } = mount({
    getAppInfo: vi.fn().mockResolvedValue({ version: '2.0.0', platform: 'windows' }),
    getNotificationPermission: vi.fn().mockResolvedValue('denied'),
  });
  await loaded(config);
  const order: string[] = [];
  spy(config, 'setNotificationSettings').mockImplementationOnce(async () => {
    order.push('save');
    return { ...defaults, enabled: true };
  });
  spy(config, 'requestNotificationPermission').mockImplementationOnce(async () => {
    order.push('request');
    return 'denied';
  });
  await fireEvent.click(master());
  await waitFor(() => expect(config.requestNotificationPermission).toHaveBeenCalledTimes(1));
  expect(order).toEqual(['save', 'request']);
  await waitFor(() => expect(master().disabled).toBe(false));
  // denied is reported as denied on Windows too, never downgraded to an unknown state.
  expect(status()).toBe(copy.denied);
  expect(config.openNotificationSettings).not.toHaveBeenCalled();
});

test('windows denied exposes the explicit system settings action', async () => {
  const { config } = mount({
    getAppInfo: vi.fn().mockResolvedValue({ version: '2.0.0', platform: 'windows' }),
    getNotificationSettings: vi.fn().mockResolvedValue({ ...defaults, enabled: true }),
    getNotificationPermission: vi.fn().mockResolvedValue('denied'),
  });
  await loaded(config);
  expect(screen.queryByRole('button', { name: copy.authorize })).toBeNull();
  expect(status()).toBe(copy.denied);
  await fireEvent.click(screen.getByRole('button', { name: copy.open }));
  await waitFor(() => expect(config.openNotificationSettings).toHaveBeenCalledTimes(1));
  expect(config.requestNotificationPermission).not.toHaveBeenCalled();
});

test('windows granted reports granted instead of an unknown state', async () => {
  const { config } = mount({
    getAppInfo: vi.fn().mockResolvedValue({ version: '2.0.0', platform: 'windows' }),
    getNotificationSettings: vi.fn().mockResolvedValue({ ...defaults, enabled: true }),
    getNotificationPermission: vi.fn().mockResolvedValue('granted'),
  });
  await loaded(config);
  expect(status()).toBe(copy.granted);
  expect(screen.queryByRole('button', { name: copy.authorize })).toBeNull();
  expect(config.requestNotificationPermission).not.toHaveBeenCalled();
});

test('windows reading, toggle-off and category switches never request or open settings', async () => {
  const on = { ...defaults, enabled: true };
  const { config } = mount({
    getAppInfo: vi.fn().mockResolvedValue({ version: '2.0.0', platform: 'windows' }),
    getNotificationSettings: vi.fn().mockResolvedValue(on),
  });
  await loaded(config);
  expect(config.requestNotificationPermission).not.toHaveBeenCalled();
  expect(config.openNotificationSettings).not.toHaveBeenCalled();

  spy(config, 'setNotificationSettings').mockResolvedValueOnce({ ...on, agentCompleted: false });
  await fireEvent.click(screen.getByRole('checkbox', { name: copy.completed }));
  await waitFor(() => expect(config.setNotificationSettings).toHaveBeenCalledTimes(1));

  spy(config, 'setNotificationSettings').mockResolvedValueOnce({ ...on, agentCompleted: false, enabled: false });
  await fireEvent.click(master());
  await waitFor(() => expect(config.setNotificationSettings).toHaveBeenCalledTimes(2));
  await waitFor(() => expect(master().disabled).toBe(false));
  expect(config.requestNotificationPermission).not.toHaveBeenCalled();
  expect(config.openNotificationSettings).not.toHaveBeenCalled();
});

test('macos denied exposes only the system settings entry and reports the denied state', async () => {
  const { config } = mount({
    getNotificationSettings: vi.fn().mockResolvedValue({ ...defaults, enabled: true }),
    getNotificationPermission: vi.fn().mockResolvedValue('denied'),
  });
  await loaded(config);
  expect(screen.queryByRole('button', { name: copy.authorize })).toBeNull();
  expect(status()).toBe(copy.denied);
  await fireEvent.click(screen.getByRole('button', { name: copy.open }));
  await waitFor(() => expect(config.openNotificationSettings).toHaveBeenCalledTimes(1));
  expect(config.openNotificationSettings).toHaveBeenCalledWith();
  expect(config.requestNotificationPermission).not.toHaveBeenCalled();
});

test('macos prompt offers authorize and system settings as separate actions', async () => {
  const { config } = mount({
    getNotificationSettings: vi.fn().mockResolvedValue({ ...defaults, enabled: true }),
    getNotificationPermission: vi.fn().mockResolvedValue('prompt'),
  });
  await loaded(config);
  expect(status()).toBe(copy.prompt);
  await fireEvent.click(screen.getByRole('button', { name: copy.authorize }));
  await waitFor(() => expect(config.requestNotificationPermission).toHaveBeenCalledTimes(1));
  expect(config.openNotificationSettings).not.toHaveBeenCalled();
  await fireEvent.click(screen.getByRole('button', { name: copy.open }));
  await waitFor(() => expect(config.openNotificationSettings).toHaveBeenCalledTimes(1));
});

test('refresh only rereads preference and permission, never requests authorization', async () => {
  const { config } = mount({
    getNotificationSettings: vi.fn().mockResolvedValue({ ...defaults, enabled: true }),
    getNotificationPermission: vi.fn().mockResolvedValue('denied'),
  });
  await loaded(config);
  const reads = spy(config, 'getNotificationSettings').mock.calls.length;
  const permissions = spy(config, 'getNotificationPermission').mock.calls.length;
  await fireEvent.click(screen.getByRole('button', { name: copy.refresh }));
  await waitFor(() => expect(spy(config, 'getNotificationSettings').mock.calls.length).toBe(reads + 1));
  await waitFor(() => expect(spy(config, 'getNotificationPermission').mock.calls.length).toBe(permissions + 1));
  expect(config.requestNotificationPermission).not.toHaveBeenCalled();
  expect(config.openNotificationSettings).not.toHaveBeenCalled();
});

test('permission query failure is reported without blocking later explicit reads', async () => {
  const { config } = mount();
  spy(config, 'getNotificationPermission').mockRejectedValueOnce(new Error(SENTINEL));
  await waitFor(() => expect(config.getNotificationSettings).toHaveBeenCalled());
  await expectSafeError(copy.failed);
  await waitFor(() => expect(master().disabled).toBe(false));
  expect(config.requestNotificationPermission).not.toHaveBeenCalled();
  await fireEvent.click(screen.getByRole('button', { name: copy.refresh }));
  await waitFor(() => expect(spy(config, 'getNotificationPermission').mock.calls.length).toBe(2));
  await waitFor(() => expect(screen.queryByRole('alert')).toBeNull());
});

test('inactive panel performs no permission query or request', async () => {
  const { config } = mount({}, false);
  await tick();
  expect(config.getNotificationPermission).not.toHaveBeenCalled();
  expect(config.requestNotificationPermission).not.toHaveBeenCalled();
  expect(config.getNotificationSettings).not.toHaveBeenCalled();
  expect(config.openNotificationSettings).not.toHaveBeenCalled();
});

test('unsupported desktop platform skips permission reads', async () => {
  const config = {
    preferences: { theme: 'dark', locale: 'en' },
    getAppInfo: vi.fn().mockResolvedValue({ version: '2.0.0', platform: 'linux' }),
    getNotificationSettings: vi.fn().mockResolvedValue(defaults),
    setNotificationSettings: vi.fn().mockResolvedValue(defaults),
    getNotificationPermission: vi.fn().mockResolvedValue('unsupported'),
    requestNotificationPermission: vi.fn().mockResolvedValue('unsupported'),
    openNotificationSettings: vi.fn().mockResolvedValue(undefined),
  };
  render(
    NotificationSettings,
    { active: true },
    { wrapper: Harness, wrapperProps: { config: config as unknown as ConfigStore } },
  );
  expect(config.getNotificationPermission).not.toHaveBeenCalled();
});
