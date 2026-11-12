import { render, screen } from '@testing-library/svelte';
import { createRawSnippet, flushSync } from 'svelte';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import Harness from '$src/test-utils/harness.svelte';
import { createShellConfig } from '$src/test-utils/shell-config.svelte.js';
import type { AppInfo, UpdateCheck } from '$src/types/app.js';
import AppShell from '../AppShell.svelte';
import { toast } from '../toast.svelte.js';

const tauri = vi.hoisted(() => ({ isTauri: vi.fn(() => true) }));

vi.mock('@tauri-apps/api/core', () => ({ isTauri: tauri.isTauri }));
vi.mock('$app/state', () => ({ page: { url: new URL('http://localhost/') } }));
vi.mock('$app/navigation', () => ({ goto: vi.fn().mockResolvedValue(undefined) }));
vi.mock('../toast.svelte.js', () => ({
  toast: vi.fn(),
  dismissToast: vi.fn(),
  getToasts: () => [],
}));

const latest: UpdateCheck = { currentVersion: '2.0.0', latestVersion: '2.1.0', updateAvailable: true };
const upToDate: UpdateCheck = { currentVersion: '2.0.0', latestVersion: '2.0.0', updateAvailable: false };
const interval = 2 * 60 * 60 * 1_000;

const children = createRawSnippet(() => ({ render: () => '<span>shell child</span>' }));

type ShellConfig = ReturnType<typeof createShellConfig>;

// The mock mirrors the real store: a resolved check publishes its result through `config.updateCheck`.
function mount(outcomes: Array<UpdateCheck | Error> = [upToDate]) {
  const config = createShellConfig() as ShellConfig & Record<string, unknown>;
  let index = 0;
  const checkForUpdates = vi.fn(async () => {
    const outcome = outcomes[Math.min(index, outcomes.length - 1)];
    index += 1;
    if (outcome instanceof Error) throw outcome;
    (config as { updateCheck: UpdateCheck | null }).updateCheck = outcome;
    return outcome;
  });
  config.checkForUpdates = checkForUpdates;
  config.getAppInfo = vi.fn<() => Promise<AppInfo>>().mockResolvedValue({ version: '2.0.0', platform: 'macos' });
  config.openProjectPage = vi.fn().mockResolvedValue(undefined);
  config.reloadOpencode = vi.fn().mockResolvedValue(undefined);

  const view = render(AppShell, { children }, { wrapper: Harness, wrapperProps: { config } });
  return { config, checkForUpdates, view };
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout', 'Date'] });
  vi.setSystemTime(new Date(2026, 10, 11, 12));
});
afterEach(() => {
  vi.useRealTimers();
  vi.clearAllMocks();
});

async function advance(ms: number) {
  vi.advanceTimersByTime(ms);
  for (let i = 0; i < 5; i += 1) await Promise.resolve();
  flushSync();
}

test('desktop shell checks updates shortly after launch, not on first render', async () => {
  const { checkForUpdates } = mount();

  await Promise.resolve();
  expect(checkForUpdates).not.toHaveBeenCalled();

  await advance(5_000);
  expect(checkForUpdates).toHaveBeenCalledOnce();
});

test('recurring update checks repeat on the periodic interval', async () => {
  const { checkForUpdates } = mount();

  await advance(5_000);
  expect(checkForUpdates).toHaveBeenCalledTimes(1);

  await advance(interval);
  expect(checkForUpdates).toHaveBeenCalledTimes(2);

  await advance(interval);
  expect(checkForUpdates).toHaveBeenCalledTimes(3);
});

test('a rejected automatic check stays silent and does not stop the next scheduled check', async () => {
  const { checkForUpdates } = mount([new Error('GitHub rate limit'), upToDate]);

  await advance(5_000);
  expect(checkForUpdates).toHaveBeenCalledOnce();
  expect(screen.queryByRole('alert')).toBeNull();
  expect(toast).not.toHaveBeenCalled();

  await advance(interval);
  expect(checkForUpdates).toHaveBeenCalledTimes(2);
  expect(toast).not.toHaveBeenCalled();
});

test('non-desktop runtimes never poll for updates', async () => {
  tauri.isTauri.mockReturnValue(false);
  try {
    const { checkForUpdates, config } = mount();
    await advance(5_000);
    await advance(interval);
    expect(checkForUpdates).not.toHaveBeenCalled();
    expect(config.getAppInfo).not.toHaveBeenCalled();
  } finally {
    tauri.isTauri.mockReturnValue(true);
  }
});

test('unmounting before launch clears the pending check so nothing fires afterwards', async () => {
  const { checkForUpdates, view } = mount();
  view.unmount();

  await advance(5_000);
  await advance(interval);
  expect(checkForUpdates).not.toHaveBeenCalled();
});

test('in-flight check does not reschedule after unmount', async () => {
  let finish: (value: UpdateCheck) => void = () => {};
  const config = createShellConfig() as ShellConfig & Record<string, unknown>;
  const checkForUpdates = vi.fn(
    () =>
      new Promise<UpdateCheck>((resolve) => {
        finish = resolve;
      }),
  );
  config.checkForUpdates = checkForUpdates;
  config.getAppInfo = vi.fn<() => Promise<AppInfo>>().mockResolvedValue({ version: '2.0.0', platform: 'macos' });
  config.openProjectPage = vi.fn().mockResolvedValue(undefined);
  config.reloadOpencode = vi.fn().mockResolvedValue(undefined);
  const view = render(AppShell, { children }, { wrapper: Harness, wrapperProps: { config } });

  await advance(5_000);
  expect(checkForUpdates).toHaveBeenCalledOnce();
  view.unmount();
  finish(latest);
  await advance(interval);
  expect(checkForUpdates).toHaveBeenCalledOnce();
});

test('a resolved newer release surfaces the hint in the sidebar without any alert', async () => {
  mount([latest]);

  await advance(5_000);

  const hint = screen.getByRole('button', {
    name: 'Version 2.1.0 available — open the release page in your browser',
  });
  expect(hint.textContent).toContain('Update 2.1.0');
  expect(screen.queryByRole('alert')).toBeNull();
});

test('a check that reports no newer release never renders a hint', async () => {
  mount([upToDate]);

  await advance(5_000);

  expect(screen.queryByRole('button', { name: /available — open the release page/ })).toBeNull();
});
