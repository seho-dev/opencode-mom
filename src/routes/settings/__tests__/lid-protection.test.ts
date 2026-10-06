import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { tick } from 'svelte';
import { afterEach, expect, test, vi } from 'vitest';
import type { ConfigStore } from '$src/config/store.svelte.js';
import Harness from '$src/test-utils/harness.svelte';
import type { LidState } from '$src/types/power.js';
import LidProtection from '../LidProtection.svelte';

const disabled: LidState = { enabled: false, phase: 'disabled' };
const protectedState: LidState = { enabled: true, phase: 'protected', activeSessions: 2 };
function deferred() {
  let resolve!: (value: LidState) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<LidState>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}
function mount(overrides: Record<string, unknown> = {}) {
  const config = {
    preferences: { locale: 'en', theme: 'dark' },
    getLidProtection: vi.fn().mockResolvedValue(disabled),
    setLidProtection: vi.fn().mockResolvedValue({ enabled: true, phase: 'idle' }),
    ...overrides,
  } as unknown as ConfigStore;
  const view = render(LidProtection, {}, { wrapper: Harness, wrapperProps: { config } });
  const toggle = screen.getByRole('checkbox', { name: 'Stay awake with lid closed' }) as HTMLInputElement;
  return { config, toggle, ...view };
}
async function settle() {
  await tick();
  await Promise.resolve();
  await tick();
}
afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

test('loads without enabling, waits through authorization, and uses native phase rather than enabled', async () => {
  const read = deferred();
  const save = deferred();
  const getLidProtection = vi.fn().mockReturnValueOnce(read.promise).mockResolvedValue(protectedState);
  const setLidProtection = vi.fn().mockReturnValue(save.promise);
  const { toggle } = mount({ getLidProtection, setLidProtection });
  expect(toggle.disabled).toBe(true);
  expect(toggle.checked).toBe(false);
  expect(screen.getByRole('status').textContent).toBe('Reading lid protection status…');
  await settle();
  expect(getLidProtection).toHaveBeenCalledOnce();
  expect(setLidProtection).not.toHaveBeenCalled();
  read.resolve(disabled);
  await waitFor(() => expect(toggle.disabled).toBe(false));
  for (const id of toggle.getAttribute('aria-describedby')?.split(' ') ?? []) {
    expect(document.getElementById(id)).toBeTruthy();
  }
  toggle.focus();
  expect(document.activeElement).toBe(toggle);
  await fireEvent.click(toggle);
  expect(setLidProtection).toHaveBeenCalledExactlyOnceWith(true);
  expect(toggle.checked).toBe(false);
  expect(toggle.disabled).toBe(true);
  expect(screen.getByRole('status').textContent).toContain('authorization may take a moment');
  expect((screen.getByRole('button', { name: 'Refresh status' }) as HTMLButtonElement).disabled).toBe(true);
  save.resolve({ enabled: true, phase: 'idle' });
  await waitFor(() => expect(toggle.checked).toBe(true));
  expect(toggle.disabled).toBe(false);
  expect(screen.getByRole('status').textContent).toBe('Watching — no working sessions');
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh status' }));
  await waitFor(() => expect(screen.getByRole('status').textContent).toBe('Protected — 2 working sessions'));
  expect(setLidProtection).toHaveBeenCalledOnce();
});

test.each([
  ['disabled', 'Disabled'],
  ['idle', 'Watching — no working sessions'],
  ['checking', 'Checking administrator authorization — keep the lid open'],
  ['protected', 'Protected — working sessions detected'],
  ['unknown', 'Status unknown — keep the lid open'],
  ['error', 'Protection not confirmed — keep the lid open'],
] as const)('renders native %s without claiming protection from the enabled flag', async (phase, text) => {
  const { toggle } = mount({ getLidProtection: vi.fn().mockResolvedValue({ enabled: true, phase }) });
  await waitFor(() => expect(screen.getByRole('status').textContent).toBe(text));
  expect(toggle.checked).toBe(true);
  expect(toggle.disabled).toBe(false);
  if (phase === 'unknown' || phase === 'error') {
    expect(screen.getByRole('button', { name: 'Turn off and restore' })).toBeTruthy();
  }
});

test('failed initial query allows OFF even while retry is pending; its late result cannot overwrite OFF', async () => {
  const retry = deferred();
  const getLidProtection = vi
    .fn()
    .mockRejectedValueOnce(new Error('Status unavailable'))
    .mockReturnValueOnce(retry.promise);
  const setLidProtection = vi.fn().mockResolvedValue(disabled);
  const { toggle } = mount({ getLidProtection, setLidProtection });
  expect((await screen.findByRole('alert')).textContent).toContain('Status unavailable');
  expect(toggle.disabled).toBe(true);
  await fireEvent.click(screen.getByRole('button', { name: 'Retry status' }));
  await fireEvent.click(screen.getByRole('button', { name: 'Turn off and restore' }));
  await waitFor(() => expect(toggle.disabled).toBe(false));
  expect(setLidProtection).toHaveBeenCalledExactlyOnceWith(false);
  retry.resolve(protectedState);
  await settle();
  expect(toggle.checked).toBe(false);
  expect(screen.getByRole('status').textContent).toBe('Disabled');
  expect(screen.queryByRole('alert')).toBeNull();
});

test('policy failure preserves confirmed switch state, never claims Protected, and permits OFF retry', async () => {
  const setLidProtection = vi
    .fn()
    .mockRejectedValueOnce(new Error('<b>restore denied</b>'))
    .mockResolvedValueOnce(disabled);
  const { toggle } = mount({ getLidProtection: vi.fn().mockResolvedValue(protectedState), setLidProtection });
  await waitFor(() => expect(toggle.checked).toBe(true));
  await fireEvent.click(toggle);
  const alert = await screen.findByRole('alert');
  expect(alert.textContent).toContain('<b>restore denied</b>');
  expect(alert.querySelector('b')).toBeNull();
  expect(toggle.checked).toBe(true);
  expect(toggle.disabled).toBe(false);
  expect(toggle.getAttribute('aria-describedby')).toContain('lid-error');
  expect(screen.getByRole('status').textContent).toBe('Protection not confirmed — keep the lid open');
  await fireEvent.click(screen.getByRole('button', { name: 'Turn off and restore' }));
  await waitFor(() => expect(toggle.checked).toBe(false));
  expect(setLidProtection).toHaveBeenNthCalledWith(2, false);
  expect(screen.getByRole('status').textContent).toBe('Disabled');
});

test('accepted command is not a fake success when native readback reports an error or a different enabled value', async () => {
  const { toggle, config } = mount({
    setLidProtection: vi.fn().mockResolvedValue({ enabled: false, phase: 'error', error: 'Authorization cancelled' }),
  });
  await waitFor(() => expect(toggle.disabled).toBe(false));
  await fireEvent.click(toggle);
  expect((await screen.findByRole('alert')).textContent).toContain('Authorization cancelled');
  expect(toggle.checked).toBe(false);
  expect(config.setLidProtection).toHaveBeenCalledExactlyOnceWith(true);
  expect(screen.getByRole('status').textContent).toBe('Protection not confirmed — keep the lid open');
});

test('a fresh poll clears a query failure, but a failed setting needs explicit readback or OFF retry', async () => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
  const getLidProtection = vi
    .fn()
    .mockResolvedValueOnce(protectedState)
    .mockRejectedValueOnce(new Error('Query failed'))
    .mockResolvedValue(protectedState);
  const setLidProtection = vi.fn().mockRejectedValue(new Error('Setting failed'));
  const { toggle } = mount({ getLidProtection, setLidProtection });
  await settle();
  await vi.advanceTimersByTimeAsync(2000);
  expect(screen.getByRole('alert').textContent).toContain('Query failed');
  expect(toggle.disabled).toBe(false);
  expect(screen.getByRole('status').textContent).toBe('Protection not confirmed — keep the lid open');
  await vi.advanceTimersByTimeAsync(2000);
  expect(screen.queryByRole('alert')).toBeNull();
  expect(screen.getByRole('status').textContent).toBe('Protected — 2 working sessions');
  await fireEvent.click(toggle);
  await settle();
  await vi.advanceTimersByTimeAsync(2000);
  expect(screen.getByRole('alert').textContent).toContain('Setting failed');
  expect(screen.getByRole('status').textContent).toBe('Protection not confirmed — keep the lid open');
  await fireEvent.click(screen.getByRole('button', { name: 'Retry status' }));
  await settle();
  expect(screen.queryByRole('alert')).toBeNull();
  expect(screen.getByRole('status').textContent).toBe('Protected — 2 working sessions');
});

test('polls status only, without overlap or unchanged live-region mutations, and pauses while saving', async () => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
  const poll = deferred();
  const save = deferred();
  const getLidProtection = vi
    .fn()
    .mockResolvedValueOnce(protectedState)
    .mockReturnValueOnce(poll.promise)
    .mockResolvedValue(protectedState);
  const setLidProtection = vi.fn().mockReturnValue(save.promise);
  const { toggle, unmount } = mount({ getLidProtection, setLidProtection });
  await settle();
  const status = screen.getByRole('status');
  const changed = vi.fn();
  const observer = new MutationObserver(changed);
  observer.observe(status, { childList: true, characterData: true, subtree: true });
  await vi.advanceTimersByTimeAsync(2000);
  expect(getLidProtection).toHaveBeenCalledTimes(2);
  await vi.advanceTimersByTimeAsync(6000);
  expect(getLidProtection).toHaveBeenCalledTimes(2);
  expect(status.textContent).toBe('Protected — 2 working sessions');
  poll.resolve({ ...protectedState });
  await settle();
  expect(changed).not.toHaveBeenCalled();
  expect(observer.takeRecords()).toHaveLength(0);
  observer.disconnect();
  await fireEvent.click(toggle);
  expect(status.textContent).toContain('Restoring your original power settings');
  await vi.advanceTimersByTimeAsync(6000);
  expect(getLidProtection).toHaveBeenCalledTimes(2);
  save.resolve(disabled);
  await settle();
  expect(toggle.checked).toBe(false);
  expect(status.textContent).toBe('Disabled');
  unmount();
  await vi.advanceTimersByTimeAsync(4000);
  expect(getLidProtection).toHaveBeenCalledTimes(2);
  expect(setLidProtection).toHaveBeenCalledExactlyOnceWith(false);
});

test.each(['resolve', 'reject'] as const)(
  'late poll %s cannot override a newer save; polling resumes after reconciliation',
  async (completion) => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    const poll = deferred();
    const getLidProtection = vi
      .fn()
      .mockResolvedValueOnce(protectedState)
      .mockReturnValueOnce(poll.promise)
      .mockResolvedValue(disabled);
    const setLidProtection = vi.fn().mockResolvedValue(disabled);
    const { toggle } = mount({ getLidProtection, setLidProtection });
    await settle();
    await vi.advanceTimersByTimeAsync(2000);
    await fireEvent.click(toggle);
    await settle();
    expect(toggle.checked).toBe(false);
    await vi.advanceTimersByTimeAsync(4000);
    expect(getLidProtection).toHaveBeenCalledTimes(2);
    if (completion === 'resolve') poll.resolve(protectedState);
    else poll.reject(new Error('stale read failure'));
    await settle();
    expect(screen.getByRole('status').textContent).toBe('Disabled');
    expect(screen.queryByRole('alert')).toBeNull();
    await vi.advanceTimersByTimeAsync(2000);
    expect(getLidProtection).toHaveBeenCalledTimes(3);
    expect(setLidProtection).toHaveBeenCalledExactlyOnceWith(false);
  },
);

test('a stale read finishing during authorization cannot restore the old protected status or restart polling', async () => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
  const poll = deferred();
  const save = deferred();
  const getLidProtection = vi
    .fn()
    .mockResolvedValueOnce(protectedState)
    .mockReturnValueOnce(poll.promise)
    .mockResolvedValue(disabled);
  const setLidProtection = vi.fn().mockReturnValue(save.promise);
  const { toggle } = mount({ getLidProtection, setLidProtection });
  await settle();
  await vi.advanceTimersByTimeAsync(2000);
  await fireEvent.click(toggle);
  poll.resolve(protectedState);
  await settle();
  expect(toggle.disabled).toBe(true);
  expect(screen.getByRole('status').textContent).toContain('Restoring your original power settings');
  await vi.advanceTimersByTimeAsync(4000);
  expect(getLidProtection).toHaveBeenCalledTimes(2);
  save.resolve(disabled);
  await settle();
  expect(toggle.checked).toBe(false);
  await vi.advanceTimersByTimeAsync(2000);
  expect(getLidProtection).toHaveBeenCalledTimes(3);
});

test.each(['read', 'save'] as const)(
  'unmount invalidates a pending %s and does not schedule another status read',
  async (operation) => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    const pending = deferred();
    const getLidProtection = vi
      .fn()
      .mockReturnValue(operation === 'read' ? pending.promise : Promise.resolve(disabled));
    const setLidProtection = vi.fn().mockReturnValue(pending.promise);
    const { toggle, unmount } = mount({ getLidProtection, setLidProtection });
    await settle();
    if (operation === 'save') await fireEvent.click(toggle);
    unmount();
    pending.reject(new Error('completed after unmount'));
    await settle();
    await vi.advanceTimersByTimeAsync(4000);
    expect(getLidProtection).toHaveBeenCalledOnce();
    expect(setLidProtection).toHaveBeenCalledTimes(operation === 'save' ? 1 : 0);
    expect(screen.queryByRole('status')).toBeNull();
  },
);
