import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { tick } from 'svelte';
import { beforeEach, expect, test, vi } from 'vitest';
import type { CommandAdapter } from '$src/config/adapter.js';
import { type ConfigStore, createConfigStore } from '$src/config/store.svelte.js';
import Harness from '$src/test-utils/harness.svelte';
import type { IslandState } from '$src/types/island.js';
import MiniIsland from '../MiniIsland.svelte';

const native = vi.hoisted(() => ({ isTauri: vi.fn(() => true) }));
vi.mock('@tauri-apps/api/core', () => ({ isTauri: native.isTauri }));
const initial: IslandState = {
  supported: true,
  enabled: false,
  connection: 'disabled',
  connectionGeneration: 0,
  sessions: [],
  permissions: [],
  questions: [],
  preferences: { theme: 'dark', locale: 'en' },
  notchWidth: 180,
  topInset: 32,
};
function mount(overrides: Record<string, unknown> = {}) {
  const config = {
    preferences: { theme: 'dark', locale: 'en' },
    getAppInfo: vi.fn().mockResolvedValue({ version: '2.0.0', platform: 'macos' }),
    getIslandState: vi.fn().mockResolvedValue(initial),
    setIslandEnabled: vi.fn().mockResolvedValue({ ...initial, enabled: true, connection: 'connecting' }),
    ...overrides,
  };
  const view = render(MiniIsland, {}, { wrapper: Harness, wrapperProps: { config: config as unknown as ConfigStore } });
  return { ...view, config };
}
function mountTarget(overrides: Record<string, unknown> = {}) {
  const adapter = {
    getAppInfo: vi.fn().mockResolvedValue({ version: '2.0.0', platform: 'macos' }),
    getIslandState: vi.fn().mockResolvedValue(initial),
    setIslandEnabled: vi.fn().mockResolvedValue(initial),
    refreshIsland: vi.fn(),
    setOpenCodeServiceTarget: vi.fn().mockResolvedValue({ connected: false }),
    ...overrides,
  };
  const config = createConfigStore(adapter as unknown as CommandAdapter, false, false);
  const view = render(MiniIsland, {}, { wrapper: Harness, wrapperProps: { config } });
  return { ...view, config, adapter };
}
beforeEach(() => native.isTauri.mockReturnValue(true));

test('macOS switch reads independently, defaults off, guards pending and uses native readback immediately', async () => {
  let finish!: (value: IslandState) => void;
  const setIslandEnabled = vi.fn().mockImplementationOnce(
    () =>
      new Promise<IslandState>((resolve) => {
        finish = resolve;
      }),
  );
  const view = mount({ setIslandEnabled });
  const control = (await screen.findByRole('checkbox', { name: 'OpenCode mini island' })) as HTMLInputElement;
  await waitFor(() => expect(control.checked).toBe(false));
  await waitFor(() => expect(control.hasAttribute('disabled')).toBe(false));
  expect(screen.getByText(/Shows local OpenCode V2 sessions and pending requests/).textContent).toContain(
    'enabling may start that service',
  );
  await fireEvent.click(control);
  await fireEvent.click(control);
  expect(setIslandEnabled).toHaveBeenCalledExactlyOnceWith(true);
  expect(control.hasAttribute('disabled')).toBe(true);
  finish({ ...initial, enabled: false });
  await waitFor(() => expect(control.hasAttribute('disabled')).toBe(false));
  expect(control.checked).toBe(false);
  expect(view.config.getIslandState).toHaveBeenCalledOnce();
});

test.each(['windows', 'linux', 'other'])('native %s hides island and does not fetch state', async (platform) => {
  const view = mount({ getAppInfo: vi.fn().mockResolvedValue({ version: '2.0.0', platform }) });
  await tick();
  expect(screen.queryByRole('checkbox')).toBeNull();
  expect(view.config.getIslandState).not.toHaveBeenCalled();
});

test('browser and unsupported macOS do not show the switch', async () => {
  native.isTauri.mockReturnValue(false);
  const browser = mount();
  await tick();
  expect(screen.queryByRole('checkbox')).toBeNull();
  expect(browser.config.getAppInfo).not.toHaveBeenCalled();
  expect(browser.config.getIslandState).not.toHaveBeenCalled();
  browser.unmount();
  native.isTauri.mockReturnValue(true);
  const unsupported = mount({ getIslandState: vi.fn().mockResolvedValue({ ...initial, supported: false }) });
  await waitFor(() => expect(unsupported.config.getIslandState).toHaveBeenCalledOnce());
  await tick();
  expect(screen.queryByRole('checkbox')).toBeNull();
});

test('disabling only writes the Island setting, never replies to pending requests or interrupts work', async () => {
  const islandReplyPermission = vi.fn();
  const islandReplyQuestion = vi.fn();
  const islandCancelQuestion = vi.fn();
  const islandInterrupt = vi.fn();
  const setIslandEnabled = vi.fn().mockResolvedValue(initial);
  const view = mount({
    getIslandState: vi.fn().mockResolvedValue({ ...initial, enabled: true, connection: 'connected' }),
    setIslandEnabled,
    islandReplyPermission,
    islandReplyQuestion,
    islandCancelQuestion,
    islandInterrupt,
  });
  const control = (await screen.findByRole('checkbox', { name: 'OpenCode mini island' })) as HTMLInputElement;
  await waitFor(() => expect(control.checked).toBe(true));
  await fireEvent.click(control);
  expect(setIslandEnabled).toHaveBeenCalledExactlyOnceWith(false);
  await waitFor(() => expect(control.disabled).toBe(false));
  expect(control.checked).toBe(false);
  view.unmount();
  for (const operation of [islandReplyPermission, islandReplyQuestion, islandCancelQuestion, islandInterrupt])
    expect(operation).not.toHaveBeenCalled();
});

test('read/save errors preserve confirmed state and retry only reads; teardown guards late result', async () => {
  const getIslandState = vi.fn().mockRejectedValueOnce(new Error('Read failed')).mockResolvedValueOnce(initial);
  const setIslandEnabled = vi.fn().mockRejectedValueOnce(new Error('Write failed'));
  const view = mount({ getIslandState, setIslandEnabled });
  expect((await screen.findByRole('alert')).textContent).toContain('Read failed');
  expect(screen.getByRole('checkbox').hasAttribute('disabled')).toBe(true);
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh status' }));
  await waitFor(() => expect(screen.getByRole('checkbox').hasAttribute('disabled')).toBe(false));
  await fireEvent.click(screen.getByRole('checkbox'));
  expect((await screen.findByRole('alert')).textContent).toContain('Write failed');
  expect((screen.getByRole('checkbox') as HTMLInputElement).checked).toBe(false);
  expect(setIslandEnabled).toHaveBeenCalledOnce();
  view.unmount();
  let finish!: (value: IslandState) => void;
  const pending = mount({
    getIslandState: vi.fn().mockImplementation(
      () =>
        new Promise<IslandState>((resolve) => {
          finish = resolve;
        }),
    ),
  });
  await waitFor(() => expect(pending.config.getIslandState).toHaveBeenCalledOnce());
  pending.unmount();
  mount();
  finish({ ...initial, enabled: true });
  await waitFor(() => expect(screen.getByRole('checkbox').hasAttribute('disabled')).toBe(false));
  expect((screen.getByRole('checkbox') as HTMLInputElement).checked).toBe(false);
});

test.each([false, true])(
  'successful target saves reread without changing Island enabled=%s; failed saves do nothing',
  async (enabled) => {
    const value = { ...initial, enabled, connection: enabled ? ('connected' as const) : ('disabled' as const) };
    const view = mountTarget({ getIslandState: vi.fn().mockResolvedValue(value) });
    const control = (await screen.findByRole('checkbox')) as HTMLInputElement;
    await waitFor(() => expect(control.disabled).toBe(false));
    expect(control.checked).toBe(enabled);
    await view.config.setOpenCodeServiceTarget('http://127.0.0.1:5096');
    await waitFor(() => expect(view.adapter.getIslandState).toHaveBeenCalledTimes(2));
    expect(screen.getByRole('checkbox')).toBe(control);
    expect(control.checked).toBe(enabled);
    view.adapter.setOpenCodeServiceTarget.mockRejectedValueOnce(new Error('Save failed'));
    await expect(view.config.setOpenCodeServiceTarget(null)).rejects.toThrow('Save failed');
    await tick();
    expect(view.config.serviceTargetVersion).toBe(1);
    expect(view.adapter.getIslandState).toHaveBeenCalledTimes(2);
    expect(view.adapter.setIslandEnabled).not.toHaveBeenCalled();
    expect(view.adapter.refreshIsland).not.toHaveBeenCalled();
  },
);

test('target changes during loading reject old readback and coalesce one latest read', async () => {
  let finish!: (value: IslandState) => void;
  let finishLatest!: (value: IslandState) => void;
  const getIslandState = vi
    .fn()
    .mockImplementationOnce(
      () =>
        new Promise<IslandState>((resolve) => {
          finish = resolve;
        }),
    )
    .mockImplementationOnce(
      () =>
        new Promise<IslandState>((resolve) => {
          finishLatest = resolve;
        }),
    );
  const view = mountTarget({ getIslandState });
  await waitFor(() => expect(getIslandState).toHaveBeenCalledOnce());
  await view.config.setOpenCodeServiceTarget('http://127.0.0.1:5096');
  await tick();
  await view.config.setOpenCodeServiceTarget(null);
  await tick();
  expect(getIslandState).toHaveBeenCalledOnce();
  finish({ ...initial, enabled: true, error: 'Old target' });
  await waitFor(() => expect(getIslandState).toHaveBeenCalledTimes(2));
  const control = screen.getByRole('checkbox') as HTMLInputElement;
  expect(control.checked).toBe(false);
  expect(screen.queryByText(/Old target/)).toBeNull();
  finishLatest(initial);
  await waitFor(() => expect(control.disabled).toBe(false));
  expect(control.checked).toBe(false);
  expect(view.adapter.setIslandEnabled).not.toHaveBeenCalled();
  expect(view.adapter.refreshIsland).not.toHaveBeenCalled();
});

test('target change during a preference save defers readback, ignores the old save result and never retries the write', async () => {
  let finish!: (value: IslandState) => void;
  const setIslandEnabled = vi.fn().mockImplementationOnce(
    () =>
      new Promise<IslandState>((resolve) => {
        finish = resolve;
      }),
  );
  const view = mountTarget({ setIslandEnabled });
  const control = (await screen.findByRole('checkbox')) as HTMLInputElement;
  await waitFor(() => expect(control.disabled).toBe(false));
  await fireEvent.click(control);
  await view.config.setOpenCodeServiceTarget(null);
  await tick();
  expect(view.adapter.getIslandState).toHaveBeenCalledOnce();
  finish({ ...initial, enabled: true, error: 'Old save result' });
  await waitFor(() => expect(view.adapter.getIslandState).toHaveBeenCalledTimes(2));
  await waitFor(() => expect(control.disabled).toBe(false));
  expect(control.checked).toBe(false);
  expect(screen.queryByText(/Old save result/)).toBeNull();
  expect(setIslandEnabled).toHaveBeenCalledExactlyOnceWith(true);
  expect(view.adapter.refreshIsland).not.toHaveBeenCalled();
});
