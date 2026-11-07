import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { tick } from 'svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { goto } from '$app/navigation';
import { page } from '$app/state';
import type { CommandAdapter } from '$src/config/adapter.js';
import { createConfigStore } from '$src/config/store.svelte.js';
import { en } from '$src/i18n/dictionaries/en.js';
import { zh } from '$src/i18n/dictionaries/zh.js';
import Settings from '$src/routes/settings/+page.svelte';
import LoggingSettings from '$src/routes/settings/LoggingSettings.svelte';
import SettingsSearch from '$src/routes/settings/SettingsSearch.svelte';
import AppSidebar from '$src/shell/AppSidebar.svelte';
import MobileNavigation from '$src/shell/MobileNavigation.svelte';
import { isNavigationItemActive, navigationItems } from '$src/shell/navigation.js';
import Harness from '$src/test-utils/harness.svelte';
import { LOG_TYPES, type LoggingState, type LogPage, type LogQuery, type LogRecord } from '$src/types/logging.js';
import Logs from '../+page.svelte';
import LogDetails from '../LogDetails.svelte';

vi.mock('$app/state', async () => {
  const { createSubscriber } = await import('svelte/reactivity');
  let notify: () => void = () => {};
  const track = createSubscriber((update) => {
    notify = update;
  });
  let url = new URL('http://localhost/logs');

  return {
    page: {
      get url() {
        track();
        return url;
      },
      set url(value: URL) {
        url = value;
        notify();
      },
      params: {},
    },
  };
});
vi.mock('$app/navigation', () => ({ goto: vi.fn().mockResolvedValue(undefined), beforeNavigate: vi.fn() }));

const disabled: LoggingState = {
  enabled: false,
  recording: false,
  directory: '/Users/test/.config/opencode-mom/logs',
  error: null,
  droppedRecords: 0,
};
const enabled: LoggingState = { ...disabled, enabled: true, recording: true };
const record: LogRecord = {
  schemaVersion: 1,
  timestamp: '2026-11-01T12:34:56.789Z',
  sessionId: 'session-a',
  sequence: 42,
  operationId: 'operation-a',
  type: 'group',
  status: 'success',
  event: 'group.projected',
  message: 'Selected group projected.',
  durationMs: 0,
  errorCode: null,
  context: { step: 'complete', count: 0, targetWritten: false },
};
const firstPage: LogPage = { records: [record], nextCursor: 'opaque-next', invalidLines: 0 };

function setPageUrl(url: string | URL) {
  page.url = new URL(url) as typeof page.url;
}

function makeConfig(overrides: Record<string, unknown> = {}) {
  const adapter = {
    savePreferences: vi.fn().mockResolvedValue(undefined),
    getAppInfo: vi.fn().mockResolvedValue({ version: '2.0.0', platform: 'linux' }),
    getLoggingState: vi.fn().mockResolvedValue(disabled),
    setLoggingEnabled: vi.fn().mockResolvedValue(enabled),
    queryLogs: vi.fn<(query: LogQuery) => Promise<LogPage>>().mockResolvedValue(firstPage),
    openLogsDirectory: vi.fn().mockResolvedValue(undefined),
    clearLogs: vi.fn().mockResolvedValue(disabled),
    ...overrides,
  };
  const config = createConfigStore(adapter as unknown as CommandAdapter, false, false);

  return { adapter, config };
}

function mountLogs(overrides: Record<string, unknown> = {}) {
  const context = makeConfig(overrides);

  return { ...context, ...render(Logs, {}, { wrapper: Harness, wrapperProps: { config: context.config } }) };
}

function mountSettings(overrides: Record<string, unknown> = {}, active = true) {
  const context = makeConfig(overrides);

  return {
    ...context,
    ...render(LoggingSettings, { active }, { wrapper: Harness, wrapperProps: { config: context.config } }),
  };
}

async function settle() {
  for (let step = 0; step < 6; step++) {
    await Promise.resolve();
    await tick();
  }
}

async function select(label: string, option: string) {
  await fireEvent.click(screen.getByRole('button', { name: label }));
  await fireEvent.click(screen.getByRole('option', { name: option }));
  await settle();
}

async function expandDetails(details: HTMLDetailsElement, open = true) {
  details.open = open;
  await fireEvent(details, new Event('toggle'));
  await settle();
}

beforeEach(() => {
  Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', { configurable: true, value: vi.fn() });
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
  Reflect.deleteProperty(HTMLElement.prototype, 'scrollIntoView');
});

test('Settings waits for authoritative toggle results, guards duplicates, preserves failures and displays recording failures', async () => {
  const save = Promise.withResolvers<LoggingState>();
  const { adapter, config } = mountSettings({
    setLoggingEnabled: vi
      .fn()
      .mockReturnValueOnce(save.promise)
      .mockRejectedValueOnce(new Error('<b>private error</b>')),
  });
  const toggle = screen.getByRole('checkbox', { name: en['settings.logging'] }) as HTMLInputElement;

  expect(toggle.disabled).toBe(true);
  await settle();
  expect(adapter.getLoggingState).toHaveBeenCalledOnce();
  expect(toggle.checked).toBe(false);
  expect(toggle.name).toBe('diagnostic-logging');
  expect(screen.getByRole('link', { name: en['settings.loggingView'] }).getAttribute('href')).toBe('/logs');
  toggle.focus();
  await fireEvent.click(toggle);
  await fireEvent.click(toggle);
  expect(adapter.setLoggingEnabled).toHaveBeenCalledExactlyOnceWith(true);
  expect(toggle.checked).toBe(false);
  expect(toggle.disabled).toBe(true);
  save.resolve({ ...enabled, recording: false, error: 'disk full', droppedRecords: 3 });
  await settle();
  expect(toggle.checked).toBe(true);
  expect(toggle.disabled).toBe(false);
  expect(document.activeElement).toBe(toggle);
  expect(screen.getByRole('alert').textContent).toContain(en['logs.recordingFailedHint']);
  expect(screen.getByText(en['logs.droppedRecords'].replace('{count}', '3'))).toBeTruthy();
  await fireEvent.click(toggle);
  await settle();
  expect(toggle.checked).toBe(true);
  expect(screen.getByRole('alert').textContent).toContain(en['settings.loggingSaveFailed']);
  expect(document.body.textContent).not.toContain('private error');
  await config.setLocale('zh');
  await settle();
  expect(screen.getByRole('checkbox', { name: zh['settings.logging'] })).toBe(toggle);
  expect(screen.getByRole('alert').textContent).toContain(zh['settings.loggingSaveFailed']);
});

test('Settings starts native work only on active entry, manual retry or toggle and never polls', async () => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });

  const { adapter, rerender, unmount } = mountSettings({}, false);
  const toggle = screen.getByRole('checkbox') as HTMLInputElement;

  await settle();
  await fireEvent.click(toggle);
  await vi.advanceTimersByTimeAsync(60_000);
  expect(adapter.getLoggingState).not.toHaveBeenCalled();
  expect(adapter.setLoggingEnabled).not.toHaveBeenCalled();
  await rerender({ active: true });
  await settle();
  await vi.advanceTimersByTimeAsync(60_000);
  expect(adapter.getLoggingState).toHaveBeenCalledOnce();
  await fireEvent.click(toggle);
  await settle();
  await vi.advanceTimersByTimeAsync(60_000);
  expect(adapter.getLoggingState).toHaveBeenCalledOnce();
  await rerender({ active: false });
  await fireEvent.click(toggle);
  expect(adapter.setLoggingEnabled).toHaveBeenCalledOnce();
  await rerender({ active: true });
  await settle();
  expect(adapter.getLoggingState).toHaveBeenCalledTimes(2);
  unmount();
  await vi.advanceTimersByTimeAsync(60_000);
  expect(adapter.getLoggingState).toHaveBeenCalledTimes(2);
});

test.each(['resolve', 'reject'] as const)(
  'Settings rejects stale %s results across inactive round trips and retains the pending write guard',
  async (completion) => {
    const save = Promise.withResolvers<LoggingState>();
    const { adapter, rerender, unmount } = mountSettings({ setLoggingEnabled: vi.fn().mockReturnValue(save.promise) });

    await settle();

    const toggle = screen.getByRole('checkbox') as HTMLInputElement;

    await fireEvent.click(toggle);
    await rerender({ active: false });
    await rerender({ active: true });
    await fireEvent.click(toggle);
    expect(adapter.setLoggingEnabled).toHaveBeenCalledOnce();
    expect(adapter.getLoggingState).toHaveBeenCalledOnce();
    await rerender({ active: false });

    if (completion === 'resolve') save.resolve(enabled);
    else save.reject(new Error('stale failure'));

    await settle();
    expect(adapter.getLoggingState).toHaveBeenCalledOnce();
    expect(toggle.checked).toBe(false);
    expect(screen.queryByRole('alert')).toBeNull();
    await rerender({ active: true });
    await settle();
    expect(adapter.getLoggingState).toHaveBeenCalledTimes(2);
    expect(screen.getByRole('checkbox')).toBe(toggle);
    unmount();
  },
);

test('Settings keeps the previous confirmed state after a read failure and retries explicitly', async () => {
  const { adapter, rerender } = mountSettings({
    getLoggingState: vi
      .fn()
      .mockResolvedValueOnce(enabled)
      .mockRejectedValueOnce(new Error('private'))
      .mockResolvedValue(disabled),
  });

  await settle();

  const toggle = screen.getByRole('checkbox') as HTMLInputElement;

  await rerender({ active: false });
  await rerender({ active: true });
  await settle();
  expect(toggle.checked).toBe(true);
  expect(screen.getByRole('alert').textContent).toContain(en['settings.loggingLoadFailed']);
  await fireEvent.click(screen.getByRole('button', { name: en['settings.loggingRefresh'] }));
  await settle();
  expect(adapter.getLoggingState).toHaveBeenCalledTimes(3);
  expect(toggle.checked).toBe(false);
});

test('Settings retries an enabled failed writer with true only, preserves preference and guards pending/inactive work', async () => {
  const retry = Promise.withResolvers<LoggingState>();
  const failed = { ...enabled, recording: false, error: 'writer_failed' };
  const { adapter, rerender } = mountSettings({
    getLoggingState: vi.fn().mockResolvedValue(failed),
    setLoggingEnabled: vi.fn().mockReturnValueOnce(retry.promise).mockResolvedValueOnce(enabled),
  });

  await settle();

  const toggle = screen.getByRole('checkbox', { name: en['settings.logging'] }) as HTMLInputElement;
  const button = screen.getByRole('button', { name: en['settings.loggingRetryRecording'] });

  expect(toggle.checked).toBe(true);
  await fireEvent.click(screen.getByRole('button', { name: en['settings.loggingRefresh'] }));
  await settle();
  expect(adapter.setLoggingEnabled).not.toHaveBeenCalled();
  button.focus();
  await fireEvent.click(button);
  await fireEvent.click(button);
  expect(adapter.setLoggingEnabled).toHaveBeenCalledExactlyOnceWith(true);
  expect(toggle.checked).toBe(true);
  expect(toggle.disabled).toBe(true);
  retry.reject(new Error('private retry error'));
  await settle();
  expect(toggle.checked).toBe(true);
  expect(screen.getByRole('alert').textContent).toContain(en['settings.loggingSaveFailed']);
  expect(document.body.textContent).not.toContain('private retry error');
  await rerender({ active: false });
  await fireEvent.click(button);
  expect(adapter.setLoggingEnabled).toHaveBeenCalledOnce();
  await rerender({ active: true });
  await settle();
  await fireEvent.click(screen.getByRole('button', { name: en['settings.loggingRetryRecording'] }));
  await settle();
  expect(adapter.setLoggingEnabled.mock.calls.map(([value]) => value)).toEqual([true, true]);
  expect(toggle.checked).toBe(true);
  expect(screen.queryByRole('alert')).toBeNull();
  expect(screen.queryByRole('button', { name: en['settings.loggingRetryRecording'] })).toBeNull();
});

test('Settings shell refreshes logging only in General, and bilingual search jumps to the retained non-action anchor', async () => {
  const { config, adapter } = makeConfig({
    getAutostart: vi.fn().mockResolvedValue(false),
    getLidProtection: vi.fn().mockResolvedValue({ enabled: false, phase: 'disabled' }),
    getMomHttpSettings: vi.fn().mockResolvedValue({ port: 18765, allowLan: false }),
    getMcpProxySettings: vi.fn().mockResolvedValue({ enabled: false, idleTimeout: 300, maxUnits: 16, revision: 1 }),
    getMcpProxyState: vi.fn().mockResolvedValue({
      actualState: 'stopped',
      securityErrors: [],
      activeUnits: 0,
      activeSessions: 0,
      maxUnits: 16,
      revision: 1,
      generation: 1,
    }),
  });

  setPageUrl('http://localhost/settings?tab=mcp-proxy');
  render(Settings, {}, { wrapper: Harness, wrapperProps: { config } });
  await settle();
  expect(adapter.getLoggingState).not.toHaveBeenCalled();

  const refresh = within(
    screen.getByRole('heading', { name: en['settings.title'], level: 1 }).closest('.page-head') as HTMLElement,
  ).getByRole('button', { name: en['configuration.refresh'] });

  await fireEvent.click(refresh);
  await settle();
  expect(adapter.getLoggingState).not.toHaveBeenCalled();

  const search = screen.getByRole('searchbox', { name: en['settings.searchLabel'] });

  await fireEvent.input(search, { target: { value: '日志' } });
  expect(adapter.getLoggingState).not.toHaveBeenCalled();
  await fireEvent.click(screen.getByRole('button', { name: new RegExp(en['settings.logging']) }));
  await settle();
  expect(document.activeElement?.id).toBe('settings-logging');
  expect(screen.getByRole('tab', { name: en['settings.tabGeneral'] }).getAttribute('aria-selected')).toBe('true');
  expect(adapter.getLoggingState).toHaveBeenCalledOnce();

  const toggle = screen.getByRole('checkbox', { name: en['settings.logging'] });

  await fireEvent.keyDown(search, { key: 'Escape' });
  await fireEvent.click(refresh);
  await settle();
  expect(adapter.getLoggingState).toHaveBeenCalledTimes(2);
  expect(screen.getByRole('checkbox', { name: en['settings.logging'] })).toBe(toggle);
  expect(adapter.setLoggingEnabled).not.toHaveBeenCalled();
});

test('logging deep link selects General and focuses its setting without toggling', async () => {
  const { config, adapter } = makeConfig({
    getAutostart: vi.fn().mockResolvedValue(false),
    getLidProtection: vi.fn().mockResolvedValue({ enabled: false, phase: 'disabled' }),
    getMomHttpSettings: vi.fn().mockResolvedValue({ port: 18765, allowLan: false }),
  });

  setPageUrl('http://localhost/settings?tab=behavior#settings-logging');
  render(Settings, {}, { wrapper: Harness, wrapperProps: { config } });
  await settle();
  expect(screen.getByRole('tab', { name: en['settings.tabGeneral'] }).getAttribute('aria-selected')).toBe('true');
  expect(document.activeElement?.id).toBe('settings-logging');
  expect(adapter.getLoggingState).toHaveBeenCalledOnce();
  expect(adapter.setLoggingEnabled).not.toHaveBeenCalled();
});

test.each(['behavior', 'mcp-proxy'] as const)(
  'logging deep link activates General in a retained %s page before lazy reads settle',
  async (tab) => {
    const read = Promise.withResolvers<LoggingState>();
    const { config, adapter } = makeConfig({
      getAutostart: vi.fn().mockResolvedValue(false),
      getLidProtection: vi.fn().mockResolvedValue({ enabled: false, phase: 'disabled' }),
      getMomHttpSettings: vi.fn().mockResolvedValue({ port: 18765, allowLan: false }),
      getMcpProxySettings: vi.fn().mockResolvedValue({ enabled: false, idleTimeout: 300, maxUnits: 16, revision: 1 }),
      getMcpProxyState: vi.fn().mockResolvedValue({
        actualState: 'stopped',
        securityErrors: [],
        activeUnits: 0,
        activeSessions: 0,
        maxUnits: 16,
        revision: 1,
        generation: 1,
      }),
      getGlobalBehavior: vi
        .fn()
        .mockResolvedValue({ path: '/AGENTS.md', resolvedPath: '/AGENTS.md', content: 'retained draft', exists: true }),
      getLoggingState: vi.fn().mockReturnValue(read.promise),
    });

    setPageUrl(`http://localhost/settings?tab=${tab}`);
    render(Settings, {}, { wrapper: Harness, wrapperProps: { config } });
    await settle();
    expect(adapter.getLoggingState).not.toHaveBeenCalled();

    const anchor = document.getElementById('settings-logging');

    setPageUrl('http://localhost/settings?tab=general#settings-logging');
    await settle();
    expect(screen.getByRole('tab', { name: en['settings.tabGeneral'] }).getAttribute('aria-selected')).toBe('true');
    expect(document.getElementById('settings-logging')).toBe(anchor);
    expect(document.activeElement).toBe(anchor);
    expect((screen.getByRole('checkbox', { name: en['settings.logging'] }) as HTMLInputElement).disabled).toBe(true);
    read.resolve(disabled);
    await settle();
    expect(document.activeElement).toBe(anchor);
    expect(adapter.getLoggingState).toHaveBeenCalledOnce();
    expect(adapter.setLoggingEnabled).not.toHaveBeenCalled();
    expect(screen.queryByText(/TypeError/)).toBeNull();
    await fireEvent.click(
      screen.getByRole('tab', { name: tab === 'behavior' ? en['settings.behaviorTab'] : en['settings.tabMcpProxy'] }),
    );

    const lastUrl = new URL(String(vi.mocked(goto).mock.calls.at(-1)?.[0]));

    expect(lastUrl.hash).toBe('');
    setPageUrl(lastUrl);
    await settle();
    expect(
      screen
        .getByRole('tab', { name: tab === 'behavior' ? en['settings.behaviorTab'] : en['settings.tabMcpProxy'] })
        .getAttribute('aria-selected'),
    ).toBe('true');
  },
);

test.each(['en', 'zh'] as const)(
  'logging read failures remain visible and localized without raw TypeError text in %s',
  async (locale) => {
    const { config, adapter } = makeConfig({
      getLoggingState: vi
        .fn()
        .mockRejectedValueOnce(new TypeError('adapter.getLoggingState is not a function'))
        .mockResolvedValueOnce(disabled),
    });

    await config.setLocale(locale);
    render(LoggingSettings, {}, { wrapper: Harness, wrapperProps: { config } });
    await settle();

    const messages = locale === 'zh' ? zh : en;

    expect(screen.getByRole('alert').textContent).toContain(messages['settings.loggingLoadFailed']);
    expect(screen.getByRole('alert').textContent).not.toMatch(/TypeError|adapter|getLoggingState/);
    expect((screen.getByRole('checkbox', { name: messages['settings.logging'] }) as HTMLInputElement).disabled).toBe(
      true,
    );
    await fireEvent.click(screen.getByRole('button', { name: messages['settings.loggingRefresh'] }));
    await settle();
    expect(adapter.getLoggingState).toHaveBeenCalledTimes(2);
    expect(screen.queryByRole('alert')).toBeNull();
  },
);

test('viewer keeps disabled history readable with absolute local time, bounded details, partial counts and no polling', async () => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });

  const literal = '<img src=x onerror=alert(1)> plain text';
  const { adapter, container, config } = mountLogs({
    getLoggingState: vi.fn().mockResolvedValue({ ...disabled, droppedRecords: 4 }),
    queryLogs: vi.fn().mockResolvedValue({ ...firstPage, invalidLines: 2, records: [{ ...record, message: literal }] }),
  });

  await settle();
  expect(adapter.queryLogs).toHaveBeenCalledOnce();

  const query = adapter.queryLogs.mock.calls[0]?.[0] as LogQuery;

  expect(query.limit).toBe(50);
  expect(new Date(query.to as string).getTime() - new Date(query.from as string).getTime()).toBe(86_400_000);
  expect(screen.getByText(record.event, { selector: 'p' })).toBeTruthy();
  expect(screen.getByText(en['logs.disabledHistory'])).toBeTruthy();
  expect(screen.getByText(en['logs.invalidLines'].replace('{count}', '2'))).toBeTruthy();
  expect(screen.getByText(en['logs.droppedRecords'].replace('{count}', '4'))).toBeTruthy();
  expect(screen.getByRole('link', { name: en['logs.settings'] }).getAttribute('href')).toBe(
    '/settings?tab=general#settings-logging',
  );
  expect(container.querySelector('time')?.getAttribute('datetime')).toBe(record.timestamp);
  expect(container.querySelector('img')).toBeNull();

  const details = container.querySelector('details') as HTMLDetailsElement;

  adapter.queryLogs.mockResolvedValueOnce({ records: [], nextCursor: null, invalidLines: 0 });
  await expandDetails(details);
  expect(within(details).getByText(record.operationId)).toBeTruthy();
  expect(within(details).getByText(record.sessionId)).toBeTruthy();
  expect(within(details).getByText(record.timestamp)).toBeTruthy();
  expect(within(details).getByText('0 ms')).toBeTruthy();
  expect(within(details).getByText('No')).toBeTruthy();
  expect(within(details).getByText(literal)).toBeTruthy();
  await vi.advanceTimersByTimeAsync(60_000);
  expect(adapter.queryLogs).toHaveBeenCalledTimes(2);
  expect(adapter.getLoggingState).toHaveBeenCalledOnce();
  await config.setLocale('zh');
  await settle();
  expect(screen.getByRole('heading', { name: zh['logs.title'], level: 1 })).toBeTruthy();
  expect(screen.getByText(zh['logs.disabledHistory'])).toBeTruthy();
  expect(adapter.queryLogs).toHaveBeenCalledTimes(2);
});

test('filters send only valid values, older pages preserve the query window, and refresh starts from newest', async () => {
  const { adapter } = mountLogs();

  await settle();
  await select(en['logs.type'], en['logs.type.mcp_proxy']);
  await select(en['logs.status'], en['logs.error']);
  await select(en['logs.period'], en['logs.recent7d']);

  const filtered = adapter.queryLogs.mock.calls.at(-1)?.[0] as LogQuery;

  expect(filtered).toEqual({
    type: 'mcp_proxy',
    status: 'error',
    from: expect.any(String),
    to: expect.any(String),
    limit: 50,
  });
  expect(new Date(filtered.to as string).getTime() - new Date(filtered.from as string).getTime()).toBe(7 * 86_400_000);
  adapter.queryLogs.mockResolvedValueOnce({
    records: [{ ...record, sequence: 41 }],
    nextCursor: null,
    invalidLines: 0,
  });
  await fireEvent.click(screen.getByRole('button', { name: en['logs.loadMore'] }));
  await settle();
  expect(adapter.queryLogs).toHaveBeenLastCalledWith({ ...filtered, cursor: 'opaque-next' });
  expect(screen.getByText(en['logs.recordCount'].replace('{count}', '1'))).toBeTruthy();
  await select(en['logs.period'], en['logs.allTime']);
  expect(adapter.queryLogs).toHaveBeenLastCalledWith({ type: 'mcp_proxy', status: 'error', limit: 50 });
  await fireEvent.click(screen.getByRole('button', { name: en['configuration.refresh'] }));
  await settle();
  expect(adapter.queryLogs).toHaveBeenLastCalledWith({ type: 'mcp_proxy', status: 'error', limit: 50 });
  expect(adapter.getLoggingState).toHaveBeenCalledTimes(2);
  expect(adapter.setLoggingEnabled).not.toHaveBeenCalled();
});

test('late cursor results cannot replace new filters; query failures retain the old page and disable stale paging', async () => {
  const old = Promise.withResolvers<LogPage>();
  const newer = { ...record, sequence: 43, event: 'resource.saved' };
  const { adapter } = mountLogs();

  await settle();
  adapter.queryLogs
    .mockReturnValueOnce(old.promise)
    .mockResolvedValueOnce({ records: [newer], nextCursor: 'other-cursor', invalidLines: 0 });
  await fireEvent.click(screen.getByRole('button', { name: en['logs.loadMore'] }));
  await select(en['logs.status'], en['logs.error']);
  expect(screen.getByText(newer.event, { selector: 'p' })).toBeTruthy();
  old.resolve({ records: [{ ...record, event: 'stale-event' }], nextCursor: 'stale-cursor', invalidLines: 99 });
  await settle();
  expect(screen.queryByText('stale-event')).toBeNull();
  adapter.queryLogs.mockRejectedValueOnce(new Error('<b>private failure</b>'));
  await select(en['logs.type'], en['logs.type.http']);
  expect(screen.getByText(newer.event, { selector: 'p' })).toBeTruthy();
  expect(screen.getByRole('alert').textContent).toContain(en['logs.loadFailed']);
  expect(screen.getByText(en['logs.retained'])).toBeTruthy();
  expect((screen.getByRole('button', { name: en['logs.loadMore'] }) as HTMLButtonElement).disabled).toBe(true);
  expect(document.body.textContent).not.toContain('private failure');
});

test('older pages remain accessible beyond 200 records while each page keeps 50 rows and the same query window', async () => {
  const records = Array.from({ length: 50 }, (_, index) => ({
    ...record,
    sequence: 1000 - index,
    event: `event-${1000 - index}`,
  }));
  const { adapter } = mountLogs({
    queryLogs: vi.fn().mockResolvedValue({ records, nextCursor: 'next-1', invalidLines: 0 }),
  });

  await settle();

  const window = adapter.queryLogs.mock.calls[0]?.[0] as LogQuery;

  for (let page = 1; page <= 5; page++) {
    adapter.queryLogs.mockResolvedValueOnce({
      records: records.map((item) => ({
        ...item,
        sequence: item.sequence - page * 50,
        event: `event-${item.sequence - page * 50}`,
      })),
      nextCursor: page === 5 ? null : `next-${page + 1}`,
      invalidLines: 0,
    });
    await fireEvent.click(screen.getByRole('button', { name: en['logs.loadMore'] }));
    await settle();
    expect(adapter.queryLogs).toHaveBeenLastCalledWith({ ...window, cursor: `next-${page}` });
    expect(screen.getByText(en['logs.recordCount'].replace('{count}', '50'))).toBeTruthy();

    const rows = within(screen.getByRole('table')).getAllByRole('row');

    expect(rows).toHaveLength(51);
    expect(rows[1]?.textContent).toContain(`event-${1000 - page * 50}`);
    expect(rows.at(-1)?.textContent).toContain(`event-${951 - page * 50}`);
    expect(screen.queryByText(`event-${1000 - (page - 1) * 50}`, { selector: 'p' })).toBeNull();
    expect((screen.getByRole('button', { name: en['logs.loadMore'] }) as HTMLButtonElement).disabled).toBe(page === 5);
  }

  expect(adapter.queryLogs).toHaveBeenCalledTimes(6);
  await fireEvent.click(screen.getByRole('button', { name: en['configuration.refresh'] }));
  await settle();
  expect(adapter.queryLogs.mock.calls.at(-1)?.[0]?.cursor).toBeUndefined();
  expect(within(screen.getByRole('table')).getAllByRole('row')[1]?.textContent).toContain('event-1000');
});

test('expanding record details lazily queries only its operationId and shows bounded chronological partial history', async () => {
  const { adapter, container } = mountLogs();

  await settle();
  expect(adapter.queryLogs).toHaveBeenCalledOnce();

  const details = container.querySelector('details') as HTMLDetailsElement;

  adapter.queryLogs.mockResolvedValueOnce({
    records: [
      { ...record, sequence: 43, event: 'trace-finished' },
      { ...record, sequence: 41, event: 'trace-started' },
    ],
    nextCursor: 'more-trace',
    invalidLines: 2,
  });
  await expandDetails(details);

  const trace = within(details).getByRole('region', { name: en['logs.traceTitle'] });

  expect(adapter.queryLogs).toHaveBeenLastCalledWith({ operationId: record.operationId, limit: 100 });

  const items = within(trace).getAllByRole('listitem');

  expect(items[0]?.textContent).toContain('trace-started');
  expect(items[1]?.textContent).toContain('trace-finished');
  expect(within(trace).getByText(en['logs.invalidLines'].replace('{count}', '2'))).toBeTruthy();
  expect(within(trace).getByText(en['logs.traceMore'].replace('{count}', '100'))).toBeTruthy();
  await expandDetails(details, false);
  await expandDetails(details);
  expect(adapter.queryLogs).toHaveBeenCalledTimes(2);
  await fireEvent.click(screen.getByRole('button', { name: en['configuration.refresh'] }));
  await settle();
  adapter.queryLogs.mockResolvedValueOnce({ records: [], nextCursor: null, invalidLines: 0 });
  await expandDetails(details);
  expect(screen.getByText(en['logs.traceEmpty'])).toBeTruthy();
});

test('late operation trace failures are ignored after collapse and start no trace readback', async () => {
  const traceRead = Promise.withResolvers<LogPage>();
  const { adapter, unmount, container } = mountLogs();

  await settle();
  adapter.queryLogs.mockReturnValueOnce(traceRead.promise);

  const details = container.querySelector('details') as HTMLDetailsElement;

  await expandDetails(details);
  await expandDetails(details, false);
  traceRead.reject(new Error('private late trace'));
  await settle();
  expect(details.open).toBe(false);
  expect(screen.queryByRole('alert')).toBeNull();
  unmount();
  expect(adapter.queryLogs).toHaveBeenCalledTimes(2);
});

test('trace failures use localized bounded feedback, retry uses the same operation, and empty partial results keep warnings', async () => {
  const { adapter, container } = mountLogs();

  await settle();
  adapter.queryLogs
    .mockRejectedValueOnce(new Error('<b>private trace</b>'))
    .mockResolvedValueOnce({ records: [], nextCursor: null, invalidLines: 3 });

  const details = container.querySelector('details') as HTMLDetailsElement;

  await expandDetails(details);

  const trace = within(details).getByRole('region', { name: en['logs.traceTitle'] });

  expect(within(trace).getByRole('alert').textContent).toBe(en['logs.traceFailed']);
  expect(trace.textContent).not.toContain('private trace');
  await fireEvent.click(within(trace).getByRole('button', { name: en['configuration.retry'] }));
  await settle();
  expect(adapter.queryLogs).toHaveBeenLastCalledWith({ operationId: record.operationId, limit: 100 });
  expect(within(trace).getByText(en['logs.traceEmpty'])).toBeTruthy();
  expect(within(trace).getByText(en['logs.invalidLines'].replace('{count}', '3'))).toBeTruthy();
});

test.each(['filter', 'clear', 'unmount'] as const)(
  'pending expanded trace rejects stale results after %s',
  async (action) => {
    const pending = Promise.withResolvers<LogPage>();
    const { adapter, container, unmount } = mountLogs();

    await settle();
    adapter.queryLogs.mockReturnValueOnce(pending.promise);

    const details = container.querySelector('details') as HTMLDetailsElement;

    await expandDetails(details);
    expect(adapter.queryLogs).toHaveBeenLastCalledWith({ operationId: record.operationId, limit: 100 });

    if (action === 'filter') await select(en['logs.status'], en['logs.error']);
    else if (action === 'clear') {
      await fireEvent.click(screen.getByRole('button', { name: en['logs.clear'] }));
      await fireEvent.click(await screen.findByRole('button', { name: en['logs.clearConfirm'] }));
      await settle();
    } else unmount();

    const calls = adapter.queryLogs.mock.calls.length;

    pending.resolve({
      records: [{ ...record, event: 'stale-trace-checkpoint' }],
      nextCursor: 'stale',
      invalidLines: 99,
    });
    await settle();
    expect(screen.queryByText('stale-trace-checkpoint')).toBeNull();
    expect(adapter.queryLogs).toHaveBeenCalledTimes(calls);
    if (action === 'filter') expect(details.open).toBe(false);
  },
);

test('detail epoch invalidates a pending operation request without eagerly reading the next epoch', async () => {
  const pending = Promise.withResolvers<LogPage>();
  const { config, adapter } = makeConfig({
    queryLogs: vi
      .fn()
      .mockReturnValueOnce(pending.promise)
      .mockResolvedValue({ records: [], nextCursor: null, invalidLines: 0 }),
  });
  const { container, rerender } = render(
    LogDetails,
    { record, trace: true, epoch: 1 },
    { wrapper: Harness, wrapperProps: { config } },
  );

  await settle();
  expect(adapter.queryLogs).not.toHaveBeenCalled();

  const details = container.querySelector('details') as HTMLDetailsElement;

  await expandDetails(details);
  await rerender({ epoch: 2 });
  pending.reject(new Error('stale epoch failure'));
  await settle();
  expect(details.open).toBe(false);
  expect(screen.queryByRole('alert')).toBeNull();
  expect(adapter.queryLogs).toHaveBeenCalledOnce();
  await expandDetails(details);
  expect(adapter.queryLogs).toHaveBeenCalledTimes(2);
  expect(screen.getByText(en['logs.traceEmpty'])).toBeTruthy();
});

test('viewer never presents a stale recording status as green while reading or after a failed status refresh', async () => {
  const initial = Promise.withResolvers<LoggingState>();
  const refresh = Promise.withResolvers<LoggingState>();

  mountLogs({ getLoggingState: vi.fn().mockReturnValueOnce(initial.promise).mockReturnValueOnce(refresh.promise) });
  await settle();

  const stateRegion = screen.getByRole('region', { name: en['logs.settings'] });

  expect(within(stateRegion).getByRole('status').textContent).toContain(en['settings.loading']);
  expect(within(stateRegion).queryByText(en['logs.recording'], { exact: true })).toBeNull();
  initial.resolve(enabled);
  await settle();
  expect(within(stateRegion).getByRole('status').textContent).toContain(en['logs.recording']);
  await fireEvent.click(screen.getByRole('button', { name: en['configuration.refresh'] }));
  await settle();

  const dot = within(stateRegion).getByRole('status').querySelector('[aria-hidden="true"]');

  expect(dot?.className).not.toContain('--status-success');
  expect(within(stateRegion).getByRole('status').textContent).toContain(en['settings.loading']);
  refresh.reject(new Error('private status error'));
  await settle();
  expect(within(stateRegion).getByRole('status').textContent).toContain(en['common.unknownStatus']);
  expect(within(stateRegion).getByRole('alert').textContent).toContain(en['logs.stateFailed']);
  expect(within(stateRegion).queryByText(en['logs.recording'], { exact: true })).toBeNull();
  expect(dot?.className).not.toContain('--status-success');
});

test('a failed clear cannot restore a previously green recording badge without status readback', async () => {
  mountLogs({
    getLoggingState: vi.fn().mockResolvedValue(enabled),
    clearLogs: vi.fn().mockRejectedValue(new Error('clear failed')),
  });
  await settle();

  const region = screen.getByRole('region', { name: en['logs.settings'] });

  expect(within(region).getByRole('status').textContent).toContain(en['logs.recording']);
  await fireEvent.click(screen.getByRole('button', { name: en['logs.clear'] }));
  await fireEvent.click(await screen.findByRole('button', { name: en['logs.clearConfirm'] }));
  await settle();
  await fireEvent.click(screen.getByRole('button', { name: en['common.cancel'] }));
  await settle();
  expect(within(region).getByRole('status').textContent).toContain(en['common.unknownStatus']);
  expect(within(region).getByRole('status').querySelector('[aria-hidden="true"]')?.className).not.toContain(
    '--status-success',
  );
});

test.each(['en', 'zh'] as const)(
  'fixed backend summaries localize in the list, fields and trace while technical/unknown data stays literal in %s',
  async (locale) => {
    const summaries = [
      ['Operation completed', 'logs.summary.operationCompleted'],
      ['Operation failed', 'logs.summary.operationFailed'],
      ['Checkpoint reached', 'logs.summary.checkpointReached'],
      ['Event completed', 'logs.summary.eventCompleted'],
      ['Event failed', 'logs.summary.eventFailed'],
    ] as const;
    const records: LogRecord[] = summaries.map(([message], index) => ({
      ...record,
      sequence: 50 - index,
      event: `operation.checkpoint.${index}`,
      message,
    }));

    records.push({ ...record, sequence: 45, event: 'unknown.message', message: 'toString' });

    const { config, adapter } = makeConfig({
      queryLogs: vi.fn().mockResolvedValue({ records, nextCursor: null, invalidLines: 0 }),
    });

    await config.setLocale(locale);

    const { container } = render(Logs, {}, { wrapper: Harness, wrapperProps: { config } });

    await settle();

    const messages = locale === 'zh' ? zh : en;
    const rows = within(screen.getByRole('table')).getAllByRole('row').slice(1);

    for (const [index, [, key]] of summaries.entries()) {
      const row = rows[index] as HTMLElement;

      expect(within(row).getByText(messages[key], { selector: 'p' })).toBeTruthy();
      expect(within(row).getByText(`operation.checkpoint.${index}`, { selector: 'p' })).toBeTruthy();

      const details = row.querySelector('details') as HTMLDetailsElement;

      expect(details.querySelector('dl')).toBeNull();
      await expandDetails(details);
      expect(within(row).getByText(messages[key], { selector: 'dd' })).toBeTruthy();
      await expandDetails(details, false);
    }

    expect(within(rows[5] as HTMLElement).getByText('toString', { selector: 'p' })).toBeTruthy();

    const details = container.querySelector('details') as HTMLDetailsElement;

    await expandDetails(details);

    const trace = within(details).getByRole('region', { name: messages['logs.traceTitle'] });

    for (const [, key] of summaries) expect(within(trace).getByText(messages[key], { selector: 'p' })).toBeTruthy();
    expect(within(trace).getByText(messages['logs.traceHint'].replace('{count}', '100'))).toBeTruthy();
    expect(adapter.queryLogs).toHaveBeenLastCalledWith({ operationId: record.operationId, limit: 100 });
  },
);

test.each(['en', 'zh'] as const)(
  'recording may remain active when preference saving needs attention in %s',
  async (locale) => {
    const messages = locale === 'zh' ? zh : en;
    const attention = { ...enabled, error: 'preference_save_failed' };
    const settingsContext = makeConfig({ getLoggingState: vi.fn().mockResolvedValue(attention) });

    await settingsContext.config.setLocale(locale);

    const settings = render(
      LoggingSettings,
      {},
      { wrapper: Harness, wrapperProps: { config: settingsContext.config } },
    );

    await settle();
    expect((screen.getByRole('checkbox', { name: messages['settings.logging'] }) as HTMLInputElement).checked).toBe(
      true,
    );
    expect(screen.getByRole('status').textContent).toContain(messages['logs.recording']);
    expect(screen.getByRole('status').textContent).toContain(messages['logs.recordingFailed']);
    expect(screen.queryByRole('button', { name: messages['settings.loggingRetryRecording'] })).toBeNull();
    expect(screen.getByRole('alert').textContent).toContain(messages['logs.recordingFailedHint']);
    settings.unmount();

    const viewerContext = makeConfig({ getLoggingState: vi.fn().mockResolvedValue(attention) });

    await viewerContext.config.setLocale(locale);
    render(Logs, {}, { wrapper: Harness, wrapperProps: { config: viewerContext.config } });
    await settle();

    const region = screen.getByRole('region', { name: messages['logs.settings'] });
    const status = within(region).getByRole('status');

    expect(status.textContent).toContain(messages['logs.recording']);
    expect(status.textContent).toContain(messages['logs.recordingFailed']);
    expect(status.textContent).not.toContain(messages['logs.recordingOff']);
    expect(status.querySelector('[aria-hidden="true"]')?.className).not.toContain('--status-success');
  },
);

test('initial errors are not empty or disabled history; refresh retries without confirmation and recording failure is explicit', async () => {
  const confirm = vi.spyOn(window, 'confirm');
  const { adapter } = mountLogs({
    getLoggingState: vi
      .fn()
      .mockRejectedValueOnce(new Error('private'))
      .mockResolvedValueOnce({ ...enabled, recording: false, error: 'disk failure' }),
    queryLogs: vi
      .fn()
      .mockRejectedValueOnce(new Error('private'))
      .mockResolvedValueOnce({ records: [], nextCursor: null, invalidLines: 0 }),
  });

  await settle();
  expect(screen.getByText(en['logs.unavailable'])).toBeTruthy();
  expect(screen.queryByText(en['logs.empty'])).toBeNull();
  expect(screen.queryByText(en['logs.disabledHistory'])).toBeNull();
  await fireEvent.click(screen.getByRole('button', { name: en['configuration.refresh'] }));
  await settle();
  expect(screen.getByText(en['logs.empty'])).toBeTruthy();
  expect(within(screen.getByRole('region', { name: en['logs.settings'] })).getByRole('status').textContent).toContain(
    en['logs.recordingFailed'],
  );
  expect(screen.getByRole('alert').textContent).toContain(en['logs.recordingFailedHint']);
  expect(adapter.queryLogs).toHaveBeenCalledTimes(2);
  expect(confirm).not.toHaveBeenCalled();
});

test('clear confirmation is cancel-first, works with recording disabled, rejects late queries and returns focus', async () => {
  const stale = Promise.withResolvers<LogPage>();
  const clearedState = Promise.withResolvers<LoggingState>();
  const { adapter } = mountLogs({ clearLogs: vi.fn().mockReturnValue(clearedState.promise) });

  await settle();

  const clear = screen.getByRole('button', { name: en['logs.clear'] });

  clear.focus();
  await fireEvent.click(clear);

  const dialog = await screen.findByRole('dialog', { name: en['logs.clearTitle'] });
  const cancel = within(dialog).getByRole('button', { name: en['common.cancel'] });

  await waitFor(() => expect(document.activeElement).toBe(cancel));
  expect(adapter.clearLogs).not.toHaveBeenCalled();
  await fireEvent.click(cancel);
  await waitFor(() => expect(document.activeElement).toBe(clear));
  adapter.queryLogs.mockReturnValueOnce(stale.promise);
  await fireEvent.click(screen.getByRole('button', { name: en['configuration.refresh'] }));
  await fireEvent.click(clear);
  await fireEvent.click(await screen.findByRole('button', { name: en['logs.clearConfirm'] }));
  expect(adapter.clearLogs).toHaveBeenCalledOnce();
  stale.resolve({ records: [{ ...record, event: 'stale-cleared-event' }], nextCursor: 'bad', invalidLines: 9 });
  await settle();
  expect(screen.queryByText('stale-cleared-event')).toBeNull();
  expect((screen.getByRole('button', { name: en['logs.clearing'] }) as HTMLButtonElement).disabled).toBe(true);
  clearedState.resolve(disabled);
  await settle();
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  await waitFor(() => expect(document.activeElement).toBe(clear));
  expect(screen.queryByText(record.event)).toBeNull();
  expect(screen.getByText(en['logs.cleared'])).toBeTruthy();
  expect(adapter.queryLogs).toHaveBeenCalledTimes(2);
  expect(adapter.setLoggingEnabled).not.toHaveBeenCalled();
});

test('clear and open failures retain history, remain retryable and never display raw errors', async () => {
  const { adapter } = mountLogs({
    clearLogs: vi.fn().mockRejectedValueOnce(new Error('<b>private clear</b>')).mockResolvedValueOnce(disabled),
    openLogsDirectory: vi.fn().mockRejectedValueOnce(new Error('<b>private open</b>')).mockResolvedValueOnce(undefined),
  });

  await settle();
  await fireEvent.click(screen.getByRole('button', { name: en['logs.openDirectory'] }));
  await settle();
  expect(screen.getByRole('alert').textContent).toContain(en['logs.openFailed']);
  await fireEvent.click(screen.getByRole('button', { name: en['logs.openDirectory'] }));
  await settle();
  expect(adapter.openLogsDirectory).toHaveBeenCalledTimes(2);
  await fireEvent.click(screen.getByRole('button', { name: en['logs.clear'] }));
  await fireEvent.click(await screen.findByRole('button', { name: en['logs.clearConfirm'] }));
  await settle();
  expect(screen.getByRole('dialog')).toBeTruthy();
  expect(within(screen.getByRole('dialog')).getByRole('alert').textContent).toContain(en['logs.clearFailed']);
  expect(document.body.textContent).not.toContain('private clear');
  await fireEvent.click(screen.getByRole('button', { name: en['common.cancel'] }));
  await settle();
  expect(screen.getByText(record.event, { selector: 'p' })).toBeTruthy();
  expect((screen.getByRole('button', { name: en['logs.loadMore'] }) as HTMLButtonElement).disabled).toBe(true);
  expect(screen.getByText(en['logs.retained'])).toBeTruthy();
});

test('unmount drops pending native results and starts no follow-up reads', async () => {
  const read = Promise.withResolvers<LogPage>();
  const state = Promise.withResolvers<LoggingState>();
  const { adapter, unmount } = mountLogs({
    queryLogs: vi.fn().mockReturnValue(read.promise),
    getLoggingState: vi.fn().mockReturnValue(state.promise),
  });

  await settle();
  unmount();
  read.reject(new Error('late query'));
  state.reject(new Error('late state'));
  await settle();
  expect(adapter.queryLogs).toHaveBeenCalledOnce();
  expect(adapter.getLoggingState).toHaveBeenCalledOnce();
  expect(screen.queryByRole('alert')).toBeNull();
});

test.each(['en', 'zh'] as const)(
  'desktop and mobile expose independent logs/settings navigation in %s',
  async (locale) => {
    const { config, adapter } = makeConfig();

    await config.setLocale(locale);
    setPageUrl('http://localhost/logs');

    const messages = locale === 'zh' ? zh : en;
    const sidebar = render(AppSidebar, {}, { wrapper: Harness, wrapperProps: { config } });

    expect(screen.getByRole('link', { name: messages['nav.logs'] }).getAttribute('aria-current')).toBe('page');
    expect(screen.getByRole('link', { name: messages['nav.settings'] }).getAttribute('href')).toBe('/settings');
    sidebar.unmount();
    render(MobileNavigation, {}, { wrapper: Harness, wrapperProps: { config } });
    await fireEvent.click(screen.getByRole('button', { name: messages['header.openMenu'] }));

    const drawer = screen.getByRole('dialog', { name: messages['header.mobileNav'] });

    expect(within(drawer).getByRole('link', { name: messages['nav.logs'] }).getAttribute('aria-current')).toBe('page');
    expect(within(drawer).getByRole('link', { name: messages['nav.settings'] }).getAttribute('href')).toBe('/settings');
    await fireEvent.click(within(drawer).getByRole('link', { name: messages['nav.logs'] }));
    expect(goto).toHaveBeenCalledWith('/logs');
    expect(adapter.getLoggingState).not.toHaveBeenCalled();
    expect(navigationItems.filter((item) => item.href === '/logs')).toHaveLength(1);
    expect(isNavigationItemActive('/logs/details', '/logs')).toBe(true);
    expect(isNavigationItemActive('/logs-other', '/logs')).toBe(false);
  },
);

test('Settings search is bilingual, static, focuses the logging anchor and includes every log type translation', async () => {
  const { config, adapter } = makeConfig();
  const onselect = vi.fn();

  render(SettingsSearch, { onselect }, { wrapper: Harness, wrapperProps: { config } });

  const search = screen.getByRole('searchbox', { name: en['settings.searchLabel'] });

  for (const query of ['日志', 'diagnostic logging']) {
    await fireEvent.input(search, { target: { value: query } });
    await fireEvent.click(screen.getByRole('button', { name: new RegExp(en['settings.logging']) }));
    expect(onselect).toHaveBeenLastCalledWith(expect.objectContaining({ tab: 'general', target: 'settings-logging' }));
  }

  await config.setLocale('zh');
  await settle();
  expect(screen.getByRole('button', { name: new RegExp(zh['settings.logging']) })).toBeTruthy();
  expect(adapter.getLoggingState).not.toHaveBeenCalled();
  expect(adapter.queryLogs).not.toHaveBeenCalled();

  for (const type of LOG_TYPES) {
    expect(en[`logs.type.${type}`]).toBeTruthy();
    expect(zh[`logs.type.${type}`]).toBeTruthy();
  }
});
