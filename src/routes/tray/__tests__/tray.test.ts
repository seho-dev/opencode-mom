import { listen } from '@tauri-apps/api/event';
import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { beforeEach, expect, test, vi } from 'vitest';
import { createTauriAdapter } from '$src/config/adapter.js';
import { createConfigStore } from '$src/config/store.svelte.js';
import { en } from '$src/i18n/dictionaries/en.js';
import { zh } from '$src/i18n/dictionaries/zh.js';
import { createI18n } from '$src/i18n/i18n.svelte.js';
import Harness from '$src/test-utils/harness.svelte';
import type { LocalePreference, ThemePreference } from '$src/types/app.js';
import type { Group } from '$src/types/groups.js';
import Tray from '../+page.svelte';

const native = vi.hoisted(() => ({
  invoke: vi.fn(),
  isTauri: vi.fn(() => true),
  listeners: new Map<string, (event: { payload: unknown }) => void>(),
  unlisten: vi.fn(),
}));
vi.mock('@tauri-apps/api/core', () => ({ invoke: native.invoke, isTauri: native.isTauri }));
vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (name: string, handler: (event: { payload: unknown }) => void) => {
    native.listeners.set(name, handler);
    return () => {
      native.unlisten(name);
      native.listeners.delete(name);
    };
  }),
}));

const group = (id: string, name: string, type: Group['type'], isEnabled = true): Group => ({
  id,
  name,
  type,
  isEnabled,
  description: '',
  updatedAt: '',
  openCodeAgentOverrides: [],
  slimAgentOverrides: null,
  omoAgentOverrides: null,
  omoCategoryMappings: null,
});
const groups = [
  group('native-id', 'My local group', 'native'),
  group('slim-id', 'My plugin group', 'slim'),
  group('omo-id', 'My agent group', 'omo'),
  group('disabled-id', 'Not enabled', 'native', false),
];

beforeEach(() => {
  vi.restoreAllMocks();
  native.invoke.mockReset();
  native.isTauri.mockReturnValue(true);
  native.listeners.clear();
  native.unlisten.mockClear();
  document.documentElement.dataset.theme = 'dark';
  vi.spyOn(document, 'hasFocus').mockReturnValue(false);
});

function mount(
  options: {
    groups?: Group[];
    stateError?: Error;
    healthError?: Error;
    locale?: LocalePreference;
    sessions?: number;
  } = {},
) {
  let selectedGroupId = 'native-id';
  let persistedTheme: ThemePreference = 'light';
  let persistedLocale: LocalePreference = options.locale ?? 'en';
  native.invoke.mockImplementation(async (command: string, args?: Record<string, unknown>) => {
    if (command === 'load_app_state') {
      if (options.stateError) throw options.stateError;
      return {
        providers: [],
        agents: [],
        groups: options.groups ?? groups,
        selectedGroupId,
        preferences: { locale: persistedLocale, theme: persistedTheme },
      };
    }
    if (command === 'switch_group') selectedGroupId = args?.id as string;
    if (command === 'opencode_active_sessions') {
      if (options.healthError) throw options.healthError;
      return options.sessions ?? 0;
    }
    if (command === 'opencode_list_models') throw new Error('Tray must not prefetch the model catalog');
    return undefined;
  });
  const config = createConfigStore(createTauriAdapter(native.invoke), false);
  const view = render(Tray, {}, { wrapper: Harness, wrapperProps: { config } });
  return {
    config,
    setPersistedTheme: (theme: ThemePreference) => {
      persistedTheme = theme;
    },
    setPersistedLocale: (locale: LocalePreference) => {
      persistedLocale = locale;
    },
    ...view,
  };
}

async function emit(name: string, payload: unknown = null) {
  await waitFor(() => expect(native.listeners.has(name)).toBe(true));
  native.listeners.get(name)?.({ payload });
}
const calls = (command: string) => native.invoke.mock.calls.filter(([name]) => name === command);

test('renders real group names, canonical type labels and eligibility; switches by ID and reads back the active group', async () => {
  const { config } = mount();
  await waitFor(() => expect(config.loading).toBe(false));
  expect(screen.getByRole('button', { name: 'My local group, OpenCode' }).getAttribute('aria-pressed')).toBe('true');
  expect(screen.getByRole('button', { name: 'My plugin group, Slim' }).getAttribute('aria-pressed')).toBe('false');
  expect(screen.getByText('Slim')).toBeTruthy();
  expect(screen.getByText('OMO')).toBeTruthy();
  expect((screen.getByRole('button', { name: 'Not enabled, OpenCode, disabled' }) as HTMLButtonElement).disabled).toBe(
    true,
  );
  expect(screen.queryByText('Cluster-Alpha-Prod')).toBeNull();
  expect(screen.queryByText('online')).toBeNull();
  expect(calls('opencode_active_sessions')).toHaveLength(0);
  expect(calls('opencode_list_models')).toHaveLength(0);
  await fireEvent.click(screen.getByRole('button', { name: 'My plugin group, Slim' }));
  await waitFor(() => expect(config.selectedGroupId).toBe('slim-id'));
  expect(calls('switch_group')).toEqual([['switch_group', { id: 'slim-id' }]]);
  expect(screen.getByRole('button', { name: 'My plugin group, Slim' }).getAttribute('aria-pressed')).toBe('true');
  expect(screen.getByRole('button', { name: 'My local group, OpenCode' }).getAttribute('aria-pressed')).toBe('false');
  expect(await screen.findByText('Group switched. Reload to apply plugin changes.')).toBeTruthy();
  await fireEvent.click(screen.getByRole('button', { name: 'My plugin group, Slim' }));
  expect(calls('switch_group')).toHaveLength(1);
});

test('Open, Settings and Quit invoke the exact native commands; Reload is serialized and checks health after completion', async () => {
  const { config } = mount();
  await waitFor(() => expect(config.loading).toBe(false));
  await emit('tray:shown');
  await screen.findByText('online');
  await waitFor(() => expect(config.loading).toBe(false));
  await fireEvent.click(screen.getByRole('button', { name: 'Open' }));
  await fireEvent.click(screen.getByRole('button', { name: 'Settings' }));
  await fireEvent.click(screen.getByRole('button', { name: 'Quit MOM' }));
  expect(calls('open_main_window')).toEqual([
    ['open_main_window', { section: null }],
    ['open_main_window', { section: 'settings' }],
  ]);
  expect(calls('quit_app')).toEqual([['quit_app']]);
  let finish = () => {};
  const fallback = native.invoke.getMockImplementation();
  native.invoke.mockImplementation((command: string, args?: Record<string, unknown>) =>
    command === 'opencode_reload'
      ? new Promise<void>((resolve) => {
          finish = resolve;
        })
      : fallback?.(command, args),
  );
  const reload = screen.getByRole('button', { name: 'Reload' });
  reload.dispatchEvent(new MouseEvent('click', { bubbles: true }));
  reload.dispatchEvent(new MouseEvent('click', { bubbles: true }));
  await screen.findByText('Reloading OpenCode…');
  expect(calls('opencode_reload')).toHaveLength(1);
  expect((screen.getByRole('button', { name: 'Settings' }) as HTMLButtonElement).disabled).toBe(true);
  expect(calls('opencode_active_sessions')).toHaveLength(1);
  finish();
  await screen.findByText('online');
  expect(calls('opencode_active_sessions')).toHaveLength(2);
  expect(calls('opencode_list_models')).toHaveLength(0);
});

test('hidden startup is quiet; focus/show notifications coalesce and reopening refreshes configuration and status', async () => {
  const { config } = mount();
  await waitFor(() => expect(native.listeners.has('tray:shown')).toBe(true));
  await waitFor(() => expect(config.loading).toBe(false));
  for (const name of ['tauri://focus', 'tauri://blur', 'tray:shown']) {
    expect(listen).toHaveBeenCalledWith(name, expect.any(Function), {
      target: { kind: 'WebviewWindow', label: 'tray' },
    });
  }
  expect(calls('load_app_state')).toHaveLength(1);
  expect(calls('opencode_active_sessions')).toHaveLength(0);
  await emit('tauri://focus', true);
  await emit('tray:shown');
  await screen.findByText('online');
  expect(calls('load_app_state')).toHaveLength(2);
  expect(calls('opencode_active_sessions')).toHaveLength(1);
  await emit('tauri://blur');
  expect(screen.queryByText('online')).toBeNull();
  await native.invoke('switch_group', { id: 'omo-id' });
  await emit('tauri://focus');
  await emit('tray:shown');
  await waitFor(() => expect(config.selectedGroupId).toBe('omo-id'));
  await screen.findByText('online');
  expect(calls('load_app_state')).toHaveLength(3);
  expect(calls('opencode_active_sessions')).toHaveLength(2);
});

test('initial focused mount catches a show that happened before listener registration', async () => {
  vi.mocked(document.hasFocus).mockReturnValue(true);
  mount();
  await screen.findByText('online');
  expect(calls('opencode_active_sessions')).toHaveLength(1);
  expect(calls('load_app_state')).toHaveLength(2);
});

test('startup and reopening read persisted light preferences without preference writes or catalog requests', async () => {
  const { config, setPersistedTheme } = mount();
  await waitFor(() => expect(config.loading).toBe(false));
  expect(document.documentElement.dataset.theme).toBe('light');
  await emit('tray:shown');
  await screen.findByText('online');
  await emit('tauri://blur');
  setPersistedTheme('dark');
  await emit('tray:shown');
  await waitFor(() => expect(document.documentElement.dataset.theme).toBe('dark'));
  await emit('tauri://blur');
  setPersistedTheme('light');
  await emit('tray:shown');
  await waitFor(() => expect(document.documentElement.dataset.theme).toBe('light'));
  expect(calls('save_preferences')).toHaveLength(0);
  expect(calls('opencode_list_models')).toHaveLength(0);
});

test('pending status is not fake online; failed status is unavailable with an honest tooltip and can recover on reopening', async () => {
  mount();
  let reject = (_cause: Error) => {};
  const fallback = native.invoke.getMockImplementation();
  native.invoke.mockImplementation((command: string, args?: Record<string, unknown>) =>
    command === 'opencode_active_sessions'
      ? new Promise<number>((_resolve, fail) => {
          reject = fail;
        })
      : fallback?.(command, args),
  );
  await emit('tray:shown');
  await screen.findByText('checking…');
  expect(screen.queryByText('online')).toBeNull();
  reject(new Error('CLI unavailable'));
  const unavailable = await screen.findByText('unavailable');
  expect(unavailable.getAttribute('title')).toContain('CLI unavailable');
  expect(unavailable.getAttribute('title')).toContain('does not confirm OpenCode is offline');
  expect(unavailable.getAttribute('title')).toContain('may start the local OpenCode service');
  native.invoke.mockImplementation(fallback);
  await emit('tauri://blur');
  await emit('tray:shown');
  await screen.findByText('online');
});

test('configuration loading/error and empty groups stay actionable without fabricated groups', async () => {
  const { config } = mount({ stateError: new Error('Configuration read failed') });
  expect(screen.getByText('Loading groups…')).toBeTruthy();
  const alert = await screen.findByRole('alert');
  expect(alert.textContent).toContain('Configuration read failed');
  expect(screen.queryByText('online')).toBeNull();
  expect(screen.getByText('Groups could not be loaded.')).toBeTruthy();
  expect(config.groups).toEqual([]);
  await fireEvent.click(screen.getByRole('button', { name: 'Open MOM to manage groups' }));
  expect(calls('open_main_window')).toEqual([['open_main_window', { section: null }]]);
});

test('empty configuration offers Open; browser-only rendering disables native controls and never checks runtime', async () => {
  native.isTauri.mockReturnValue(false);
  const { config } = mount({ groups: [] });
  await waitFor(() => expect(config.loading).toBe(false));
  expect(screen.getByText('No runtime groups yet.')).toBeTruthy();
  expect(screen.getByText('unavailable').getAttribute('title')).toContain('desktop application');
  expect((screen.getByRole('button', { name: 'Open' }) as HTMLButtonElement).disabled).toBe(true);
  expect((screen.getByRole('button', { name: 'Open MOM to manage groups' }) as HTMLButtonElement).disabled).toBe(true);
  expect(calls('opencode_active_sessions')).toHaveLength(0);
});

test('Escape hides without quitting; stale health completions cannot restore online after hiding and listeners are cleaned up', async () => {
  const view = mount();
  let finish = (_sessions: number) => {};
  const fallback = native.invoke.getMockImplementation();
  native.invoke.mockImplementation((command: string, args?: Record<string, unknown>) =>
    command === 'opencode_active_sessions'
      ? new Promise<number>((resolve) => {
          finish = resolve;
        })
      : fallback?.(command, args),
  );
  await emit('tray:shown');
  await screen.findByText('checking…');
  await fireEvent.keyDown(window, { key: 'Escape' });
  expect(calls('hide_tray_window')).toEqual([['hide_tray_window']]);
  expect(calls('quit_app')).toHaveLength(0);
  finish(3);
  await screen.findByText('not checked');
  expect(screen.queryByText('online')).toBeNull();
  view.unmount();
  expect(native.unlisten.mock.calls.map(([name]) => name).sort()).toEqual([
    'tauri://blur',
    'tauri://focus',
    'tray:shown',
  ]);
});

test('failed switch preserves selection and reports the native error; retry clears the error', async () => {
  const { config } = mount();
  await waitFor(() => expect(config.loading).toBe(false));
  native.invoke.mockRejectedValueOnce(new Error('Switch failed'));
  await fireEvent.click(screen.getByRole('button', { name: 'My agent group, OMO' }));
  expect((await screen.findByRole('alert')).textContent).toContain('Switch failed');
  expect(config.selectedGroupId).toBe('native-id');
  await fireEvent.click(screen.getByRole('button', { name: 'My agent group, OMO' }));
  await waitFor(() => expect(config.selectedGroupId).toBe('omo-id'));
  expect(screen.queryByRole('alert')).toBeNull();
});

test.each([1, 3])(
  'renders Chinese tray controls, accessible group labels and status for %i active sessions',
  async (sessions) => {
    const { config } = mount({ locale: 'zh', sessions });
    const i18n = createI18n(() => 'zh');
    await waitFor(() => expect(config.loading).toBe(false));
    expect(screen.getByRole('button', { name: zh['tray.reload'] }).title).toBe(zh['tray.reloadTitle']);
    expect(screen.getByRole('button', { name: zh['tray.open'] }).title).toBe(zh['tray.openTitle']);
    expect(screen.getByRole('button', { name: zh['nav.settings'] })).toBeTruthy();
    expect(screen.getByRole('button', { name: zh['tray.quit'] })).toBeTruthy();
    expect(screen.getByRole('heading', { name: zh['tray.runtimeGroup'] })).toBeTruthy();
    expect(screen.getByText(zh['tray.active'])).toBeTruthy();
    expect(screen.getByText(zh['groups.disabled'])).toBeTruthy();
    const disabled = screen.getByRole('button', {
      name: i18n.t('tray.disabledGroupLabel', { name: 'Not enabled', type: zh['tray.typeNative'] }),
    }) as HTMLButtonElement;
    expect(disabled.disabled).toBe(true);
    expect(disabled.title).toBe(i18n.t('tray.disabledGroupTitle', { name: 'Not enabled' }));
    const unchecked = screen.getByText(zh['tray.status.unchecked']);
    expect(unchecked.title).toBe(zh['tray.statusHelp']);
    expect(unchecked.getAttribute('aria-label')).toBe(
      i18n.t('tray.runtimeStatus', { status: zh['tray.status.unchecked'], detail: zh['tray.statusHelp'] }),
    );
    await emit('tray:shown');
    const online = await screen.findByText(zh['tray.status.online']);
    const detail = i18n.t(sessions === 1 ? 'tray.statusOnlineOne' : 'tray.statusOnlineMany', {
      count: sessions,
      help: zh['tray.statusHelp'],
    });
    expect(online.title).toBe(detail);
    expect(online.getAttribute('aria-label')).toBe(
      i18n.t('tray.runtimeStatus', { status: zh['tray.status.online'], detail }),
    );
    await fireEvent.keyDown(window, { key: 'Escape' });
    expect(calls('hide_tray_window')).toEqual([['hide_tray_window']]);
    expect(calls('quit_app')).toHaveLength(0);
  },
);

test('persisted locale refresh updates visible status details and switch notices without health requests or preference writes', async () => {
  const { config, setPersistedLocale } = mount({ locale: 'zh', sessions: 1 });
  await waitFor(() => expect(config.loading).toBe(false));
  await emit('tray:shown');
  await screen.findByText(zh['tray.status.online']);
  await waitFor(() => expect(config.loading).toBe(false));
  const i18n = createI18n(() => 'zh');
  await fireEvent.click(
    screen.getByRole('button', {
      name: i18n.t('tray.groupLabel', { name: 'My plugin group', type: zh['groupForm.typeSlim'] }),
    }),
  );
  await screen.findByText(zh['tray.groupSwitched']);
  setPersistedLocale('en');
  await config.reloadKeepingDraft();
  await screen.findByText(en['tray.groupSwitched']);
  const online = screen.getByText(en['tray.status.online']);
  expect(online.title).toBe(
    createI18n(() => 'en').t('tray.statusOnlineOne', { count: 1, help: en['tray.statusHelp'] }),
  );
  expect(screen.getByRole('button', { name: en['tray.reload'] }).title).toBe(en['tray.reloadTitle']);
  expect(screen.getByRole('button', { name: 'Not enabled, OpenCode, disabled' }).title).toContain('enable this group');
  expect(screen.getByRole('button', { name: 'My plugin group, Slim' }).getAttribute('aria-pressed')).toBe('true');
  expect(screen.queryByText(zh['tray.groupSwitched'])).toBeNull();
  expect(calls('opencode_active_sessions')).toHaveLength(1);
  expect(calls('save_preferences')).toHaveLength(0);
  expect(calls('opencode_list_models')).toHaveLength(0);
});

test('Chinese empty and browser-only states use translated messages and disabled native controls', async () => {
  native.isTauri.mockReturnValue(false);
  const { config } = mount({ locale: 'zh', groups: [] });
  await waitFor(() => expect(config.loading).toBe(false));
  expect(screen.getByText(zh['tray.noGroups'])).toBeTruthy();
  expect(screen.getByText(zh['tray.status.unavailable']).title).toBe(zh['tray.desktopOnly']);
  expect((screen.getByRole('button', { name: zh['tray.manageGroups'] }) as HTMLButtonElement).disabled).toBe(true);
  expect(calls('opencode_active_sessions')).toHaveLength(0);
});

test('Chinese unavailable details and fallback errors follow locale changes while preserving native error messages', async () => {
  const { config, setPersistedLocale } = mount({ locale: 'zh', healthError: new Error('CLI unavailable') });
  await emit('tray:shown');
  const unavailable = await screen.findByText(zh['tray.status.unavailable']);
  expect(unavailable.title).toBe(
    createI18n(() => 'zh').t('tray.statusUnavailable', {
      message: 'CLI unavailable',
      help: zh['tray.statusHelp'],
    }),
  );
  await waitFor(() => expect(config.loading).toBe(false));
  native.invoke.mockRejectedValueOnce(null);
  await fireEvent.click(screen.getByRole('button', { name: zh['tray.open'] }));
  expect((await screen.findByRole('alert')).textContent).toContain(zh['tray.operationFailed']);
  setPersistedLocale('en');
  await config.reloadKeepingDraft();
  await waitFor(() => expect(screen.getByRole('alert').textContent).toContain(en['tray.operationFailed']));
  expect(screen.getByText(en['tray.status.unavailable']).title).toContain('does not confirm OpenCode is offline');
  expect(calls('opencode_active_sessions')).toHaveLength(1);
});
