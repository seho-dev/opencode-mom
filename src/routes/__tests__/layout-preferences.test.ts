import { listen } from '@tauri-apps/api/event';
import { render, waitFor } from '@testing-library/svelte';
import { createRawSnippet } from 'svelte';
import { beforeEach, expect, test, vi } from 'vitest';
import { goto } from '$app/navigation';
import { page } from '$app/state';
import type { ThemePreference } from '$src/types/app.js';
import Layout from '../+layout.svelte';

type Handler = (event: { payload: unknown }) => void;
const native = vi.hoisted(() => ({
  invoke: vi.fn(),
  isTauri: vi.fn(() => true),
  listen: vi.fn(),
  listeners: new Map<string, Handler>(),
  unlisten: vi.fn(),
}));
vi.mock('@tauri-apps/api/core', () => ({ invoke: native.invoke, isTauri: native.isTauri }));
vi.mock('@tauri-apps/api/event', () => ({ listen: native.listen }));
vi.mock('$src/config/adapter.js', async (importOriginal) => {
  const adapter = await importOriginal<typeof import('$src/config/adapter.js')>();
  return { ...adapter, createCommandAdapter: () => adapter.createTauriAdapter(native.invoke) };
});
vi.mock('$app/navigation', () => ({ goto: vi.fn().mockResolvedValue(undefined) }));
vi.mock('$app/state', () => ({ page: { url: new URL('http://localhost/tray') } }));

let persistedTheme: ThemePreference;
const calls = (command: string) => native.invoke.mock.calls.filter(([name]) => name === command);
const mount = () =>
  render(Layout, {
    children: createRawSnippet(() => ({ render: () => '<span>Layout child</span>' })),
  });

beforeEach(() => {
  vi.restoreAllMocks();
  vi.mocked(goto).mockClear();
  native.invoke.mockReset();
  native.isTauri.mockReturnValue(true);
  native.listen.mockReset();
  native.listeners.clear();
  native.unlisten.mockClear();
  page.url = new URL('http://localhost/tray');
  persistedTheme = 'dark';
  document.documentElement.dataset.theme = 'dark';
  native.invoke.mockImplementation(async (command: string) => {
    if (command === 'load_app_state') {
      return {
        providers: [],
        agents: [],
        groups: [],
        selectedGroupId: null,
        preferences: { locale: 'en', theme: persistedTheme },
      };
    }
    if (command === 'opencode_list_models') return [];
    throw new Error(`Unexpected command: ${command}`);
  });
  native.listen.mockImplementation(async (name: string, handler: Handler) => {
    native.listeners.set(name, handler);
    return () => {
      native.unlisten(name);
      native.listeners.delete(name);
    };
  });
});

test('main keeps its navigation/config listeners and does not subscribe to tray preference updates', async () => {
  page.url = new URL('http://localhost/');
  const view = mount();
  await waitFor(() => expect(native.listeners.size).toBe(2));
  await waitFor(() => expect(calls('load_app_state')).toHaveLength(1));
  expect(listen).toHaveBeenCalledWith('tray:navigate', expect.any(Function), { target: 'main' });
  expect(listen).toHaveBeenCalledWith('tray:config-changed', expect.any(Function), { target: 'main' });
  expect(native.listeners.has('app:preferences-changed')).toBe(false);
  expect(calls('load_app_state')).toHaveLength(1);
  expect(calls('opencode_list_models')).toHaveLength(1);

  const navigate = native.listeners.get('tray:navigate');
  const changed = native.listeners.get('tray:config-changed');
  navigate?.({ payload: '/groups' });
  expect(goto).not.toHaveBeenCalled();
  navigate?.({ payload: '/settings' });
  await waitFor(() => expect(goto).toHaveBeenCalledExactlyOnceWith('/settings'));
  persistedTheme = 'light';
  changed?.({ payload: null });
  await waitFor(() => expect(document.documentElement.dataset.theme).toBe('light'));
  expect(calls('load_app_state')).toHaveLength(2);
  expect(calls('opencode_list_models')).toHaveLength(2);

  view.unmount();
  expect(native.unlisten.mock.calls.map(([name]) => name).sort()).toEqual(['tray:config-changed', 'tray:navigate']);
  navigate?.({ payload: '/settings' });
  changed?.({ payload: null });
  expect(goto).toHaveBeenCalledOnce();
  expect(calls('load_app_state')).toHaveLength(2);
});

test('tray reloads persisted dark/light/dark preferences without saving or requesting the model catalog', async () => {
  const view = mount();
  await waitFor(() => expect(listen).toHaveBeenCalledTimes(1));
  await waitFor(() => expect(calls('load_app_state')).toHaveLength(2));
  expect(listen).toHaveBeenCalledExactlyOnceWith('app:preferences-changed', expect.any(Function), {
    target: 'tray',
  });
  expect(document.documentElement.dataset.theme).toBe('dark');
  const changed = native.listeners.get('app:preferences-changed');
  for (const theme of ['light', 'dark'] as const) {
    persistedTheme = theme;
    changed?.({ payload: null });
    await waitFor(() => expect(document.documentElement.dataset.theme).toBe(theme));
  }
  expect(calls('load_app_state')).toHaveLength(4);
  expect(calls('save_preferences')).toHaveLength(0);
  expect(calls('opencode_list_models')).toHaveLength(0);

  view.unmount();
  expect(native.unlisten).toHaveBeenCalledExactlyOnceWith('app:preferences-changed');
  changed?.({ payload: null });
  expect(calls('load_app_state')).toHaveLength(4);
});

test('tray catches a persisted light theme changed before listener registration finishes', async () => {
  let register!: (unlisten: () => void) => void;
  native.listen.mockReturnValueOnce(
    new Promise<() => void>((resolve) => {
      register = resolve;
    }),
  );
  mount();
  await waitFor(() => expect(calls('load_app_state')).toHaveLength(1));
  expect(document.documentElement.dataset.theme).toBe('dark');
  persistedTheme = 'light';
  register(() => native.unlisten('app:preferences-changed'));
  await waitFor(() => expect(document.documentElement.dataset.theme).toBe('light'));
  expect(calls('load_app_state')).toHaveLength(2);
  expect(calls('save_preferences')).toHaveLength(0);
  expect(calls('opencode_list_models')).toHaveLength(0);
});

test.each(['/tray', '/'])('late listener registration after unmount cleans up without reloading (%s)', async (path) => {
  page.url = new URL(`http://localhost${path}`);
  const registrations: (() => void)[] = [];
  native.listen.mockImplementation(
    (name: string) =>
      new Promise<() => void>((resolve) => {
        registrations.push(() => resolve(() => native.unlisten(name)));
      }),
  );
  const view = mount();
  await waitFor(() => expect(listen).toHaveBeenCalledTimes(path === '/tray' ? 1 : 2));
  await waitFor(() => expect(calls('load_app_state')).toHaveLength(1));
  view.unmount();
  for (const register of registrations) register();
  await waitFor(() => expect(native.unlisten).toHaveBeenCalledTimes(registrations.length));
  expect(calls('load_app_state')).toHaveLength(1);
});

test('tray listener registration failures are reported without changing persisted preferences', async () => {
  const error = new Error('Listener denied');
  const report = vi.spyOn(console, 'error').mockImplementation(() => {});
  native.listen.mockRejectedValueOnce(error);
  persistedTheme = 'light';
  mount();
  await waitFor(() => expect(report).toHaveBeenCalledWith('Could not listen for preference updates:', error));
  expect(document.documentElement.dataset.theme).toBe('light');
  expect(calls('load_app_state')).toHaveLength(1);
  expect(calls('save_preferences')).toHaveLength(0);
});

test('browser-only tray still loads persisted light preferences without native listeners', async () => {
  native.isTauri.mockReturnValue(false);
  persistedTheme = 'light';
  mount();
  await waitFor(() => expect(document.documentElement.dataset.theme).toBe('light'));
  expect(listen).not.toHaveBeenCalled();
  expect(calls('opencode_list_models')).toHaveLength(0);
});
