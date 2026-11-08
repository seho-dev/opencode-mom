import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { tick } from 'svelte';
import { afterEach, expect, test, vi } from 'vitest';
import type { CommandAdapter } from '$src/config/adapter.js';
import { createConfigStore } from '$src/config/store.svelte.js';
import { en } from '$src/i18n/dictionaries/en.js';
import { zh } from '$src/i18n/dictionaries/zh.js';
import Harness from '$src/test-utils/harness.svelte';
import type { LidState } from '$src/types/power.js';
import LidProtection from '../LidProtection.svelte';

const disabled: LidState = { enabled: false, phase: 'disabled' };
const enabled: LidState = { enabled: true, phase: 'protected', activeSessions: 2 };
const privateError = 'Enable lid protection again. <b>private native error</b>';

function mount(overrides: Record<string, unknown> = {}, active = true) {
  const adapter = {
    savePreferences: vi.fn().mockResolvedValue(undefined),
    getLidProtection: vi.fn().mockResolvedValue(disabled),
    setLidProtection: vi.fn().mockResolvedValue({ enabled: true, phase: 'idle' }),
    setOpenCodeServiceTarget: vi.fn().mockResolvedValue({ connected: false }),
    ...overrides,
  };
  const config = createConfigStore(adapter as unknown as CommandAdapter, false, false);
  const view = render(LidProtection, { active }, { wrapper: Harness, wrapperProps: { config } });
  const toggle = screen.getByRole('checkbox') as HTMLInputElement;

  return { ...view, config, adapter, toggle };
}

async function settle() {
  for (let step = 0; step < 4; step++) {
    await Promise.resolve();
    await tick();
  }
}

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

test('loads the preference once, keeps the native switch pending, and waits for authoritative toggle results', async () => {
  const read = Promise.withResolvers<LidState>();
  const save = Promise.withResolvers<LidState>();
  const { adapter, toggle } = mount({
    getLidProtection: vi.fn().mockReturnValue(read.promise),
    setLidProtection: vi.fn().mockReturnValue(save.promise),
  });

  expect(toggle.tagName).toBe('INPUT');
  expect(toggle.type).toBe('checkbox');
  expect(toggle.name).toBe('lid-closed');
  expect(toggle.disabled).toBe(true);
  expect(toggle.checked).toBe(false);
  expect(toggle.getAttribute('aria-busy')).toBe('true');
  await fireEvent.click(toggle);
  expect(adapter.setLidProtection).not.toHaveBeenCalled();
  read.resolve(disabled);
  await settle();
  expect(adapter.getLidProtection).toHaveBeenCalledExactlyOnceWith();
  expect(toggle.disabled).toBe(false);

  for (const id of toggle.getAttribute('aria-describedby')?.split(' ') ?? []) {
    expect(document.getElementById(id)).toBeTruthy();
  }

  toggle.focus();
  await fireEvent.click(toggle);
  await fireEvent.click(toggle);
  expect(adapter.setLidProtection).toHaveBeenCalledExactlyOnceWith(true);
  expect(toggle.checked).toBe(false);
  expect(toggle.disabled).toBe(true);
  save.resolve({ enabled: true, phase: 'checking' });
  await settle();
  expect(toggle.checked).toBe(true);
  expect(toggle.disabled).toBe(false);
  expect(toggle.getAttribute('aria-busy')).toBe('false');
  expect(document.activeElement).toBe(toggle);
  expect(adapter.getLidProtection).toHaveBeenCalledOnce();
  expect(screen.queryByRole('alert')).toBeNull();
});

test.each(['disabled', 'idle', 'checking', 'protected', 'unknown', 'error'] as const)(
  'native %s readback displays only the enabled preference, never phase/session warnings or backend errors',
  async (phase) => {
    const { container, toggle } = mount({
      getLidProtection: vi.fn().mockResolvedValue({ ...enabled, phase, error: privateError }),
    });

    await settle();
    expect(toggle.checked).toBe(true);
    expect(toggle.disabled).toBe(false);
    expect(screen.queryByRole('status')).toBeNull();
    expect(screen.queryByRole('alert')).toBeNull();
    expect(screen.queryByRole('button')).toBeNull();
    expect(container.querySelector('details')).toBeNull();
    expect(
      screen.queryByText(/protected|protection not confirmed|status unknown|keep the lid open|working sessions/i),
    ).toBeNull();
    expect(container.textContent).not.toContain(privateError);
    expect(screen.getByText(en['settings.lidHint'])).toBeTruthy();
    expect(container.textContent).toContain(en['settings.lidAuthorization']);
    expect(container.textContent).toContain(en['settings.lidSafety']);
  },
);

test.each(['en', 'zh'] as const)(
  'failed explicit toggles keep the preference and show only a concise %s error',
  async (locale) => {
    const messages = locale === 'zh' ? zh : en;
    const { adapter, config, container, toggle } = mount({
      getLidProtection: vi.fn().mockResolvedValue(enabled),
      setLidProtection: vi
        .fn()
        .mockRejectedValueOnce({ code: 'configuration_failed', message: privateError })
        .mockResolvedValueOnce(disabled),
    });

    await settle();
    await config.setLocale(locale);
    await settle();
    expect(container.textContent).toContain(messages['settings.lidAuthorization']);
    expect(container.textContent).toContain(messages['settings.lidSafety']);
    await fireEvent.click(toggle);
    await settle();
    expect(screen.getByRole('alert').textContent?.trim()).toBe(messages['settings.lidSaveFailed']);
    expect(container.textContent).not.toContain(privateError);
    expect(toggle.checked).toBe(true);
    expect(toggle.disabled).toBe(false);
    expect(toggle.getAttribute('aria-describedby')).toContain('lid-error');
    await fireEvent.click(toggle);
    await settle();
    expect(toggle.checked).toBe(false);
    expect(screen.queryByRole('alert')).toBeNull();
    expect(adapter.setLidProtection).toHaveBeenNthCalledWith(1, false);
    expect(adapter.setLidProtection).toHaveBeenNthCalledWith(2, false);
    expect(adapter.getLidProtection).toHaveBeenCalledOnce();
  },
);

test.each([
  { enabled: false, phase: 'error', error: privateError },
  { enabled: true, phase: 'error', error: privateError },
  { enabled: true, phase: 'idle', error: privateError },
  { enabled: false, phase: 'idle' },
] satisfies LidState[])(
  'failed toggle readback owns enabled=$enabled without exposing its backend payload',
  async (result) => {
    const { container, toggle } = mount({ setLidProtection: vi.fn().mockResolvedValue(result) });

    await settle();
    await fireEvent.click(toggle);
    await settle();
    expect(toggle.checked).toBe(result.enabled);
    expect(toggle.disabled).toBe(false);
    expect(screen.getByRole('alert').textContent?.trim()).toBe(en['settings.lidSaveFailed']);
    expect(container.textContent).not.toContain(privateError);
    expect(screen.queryByRole('status')).toBeNull();
  },
);

test('initial read failure is concise, keeps an unknown preference disabled, and retries only on active entry', async () => {
  const { adapter, container, rerender, toggle } = mount({
    getLidProtection: vi.fn().mockRejectedValueOnce(new Error(privateError)).mockResolvedValueOnce(enabled),
  });

  await settle();
  expect(screen.getByRole('alert').textContent?.trim()).toBe(en['settings.lidLoadFailed']);
  expect(container.textContent).not.toContain(privateError);
  expect(toggle.checked).toBe(false);
  expect(toggle.disabled).toBe(true);
  await fireEvent.click(toggle);
  expect(adapter.setLidProtection).not.toHaveBeenCalled();
  await rerender({ active: false });
  await rerender({ active: true });
  await settle();
  expect(adapter.getLidProtection).toHaveBeenCalledTimes(2);
  expect(toggle.checked).toBe(true);
  expect(toggle.disabled).toBe(false);
  expect(screen.queryByRole('alert')).toBeNull();
});

test('never polls after load, toggles or target readback, even as fake timers advance', async () => {
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });

  const { adapter, config, toggle, unmount } = mount();

  await settle();
  await vi.advanceTimersByTimeAsync(10000);
  expect(adapter.getLidProtection).toHaveBeenCalledOnce();
  await fireEvent.click(toggle);
  await settle();
  await vi.advanceTimersByTimeAsync(10000);
  expect(adapter.getLidProtection).toHaveBeenCalledOnce();
  await config.setOpenCodeServiceTarget(null);
  await settle();
  await vi.advanceTimersByTimeAsync(10000);
  expect(adapter.getLidProtection).toHaveBeenCalledTimes(2);
  expect(vi.getTimerCount()).toBe(0);
  unmount();
  await vi.advanceTimersByTimeAsync(10000);
  expect(adapter.getLidProtection).toHaveBeenCalledTimes(2);
});

test('inactive mount and target changes start no work; each active entry reads without toggling', async () => {
  const { adapter, config, rerender, toggle } = mount({}, false);

  await config.setOpenCodeServiceTarget(null);
  await settle();
  await fireEvent.click(toggle);
  expect(adapter.getLidProtection).not.toHaveBeenCalled();
  expect(adapter.setLidProtection).not.toHaveBeenCalled();
  await rerender({ active: true });
  await settle();
  expect(adapter.getLidProtection).toHaveBeenCalledOnce();
  await rerender({ active: false });
  await fireEvent.click(toggle);
  await rerender({ active: true });
  await settle();
  expect(adapter.getLidProtection).toHaveBeenCalledTimes(2);
  expect(adapter.setLidProtection).not.toHaveBeenCalled();
});

test.each([disabled, enabled])(
  'only successful target saves reread the preference without remounting (enabled=$enabled)',
  async (value) => {
    const { adapter, config, toggle } = mount({ getLidProtection: vi.fn().mockResolvedValue(value) });

    await settle();
    await config.setOpenCodeServiceTarget('http://127.0.0.1:5096');
    await settle();
    expect(adapter.getLidProtection).toHaveBeenCalledTimes(2);
    expect(screen.getByRole('checkbox')).toBe(toggle);
    expect(toggle.checked).toBe(value.enabled);
    adapter.setOpenCodeServiceTarget.mockRejectedValueOnce(new Error('Save failed'));
    await expect(config.setOpenCodeServiceTarget(null)).rejects.toThrow('Save failed');
    await settle();
    expect(config.serviceTargetVersion).toBe(1);
    expect(adapter.getLidProtection).toHaveBeenCalledTimes(2);
    expect(adapter.setLidProtection).not.toHaveBeenCalled();
  },
);

test.each(['read', 'save'] as const)(
  'target changes during a pending %s defer one latest read and reject stale success or failure',
  async (operation) => {
    for (const completion of ['resolve', 'reject'] as const) {
      const pending = Promise.withResolvers<LidState>();
      const latest = Promise.withResolvers<LidState>();
      const getLidProtection = vi.fn();
      if (operation === 'save') getLidProtection.mockResolvedValueOnce(disabled);
      getLidProtection.mockReturnValueOnce(operation === 'read' ? pending.promise : latest.promise);
      if (operation === 'read') getLidProtection.mockReturnValueOnce(latest.promise);
      const view = mount({ getLidProtection, setLidProtection: vi.fn().mockReturnValue(pending.promise) });
      await settle();
      if (operation === 'save') await fireEvent.click(view.toggle);
      await view.config.setOpenCodeServiceTarget('http://127.0.0.1:5096');
      await settle();
      await view.config.setOpenCodeServiceTarget(null);
      await settle();
      expect(getLidProtection).toHaveBeenCalledOnce();
      expect(view.toggle.disabled).toBe(true);
      if (completion === 'resolve') pending.resolve(enabled);
      else pending.reject(new Error(privateError));
      await settle();
      expect(getLidProtection).toHaveBeenCalledTimes(2);
      expect(view.toggle.checked).toBe(false);
      expect(view.toggle.disabled).toBe(true);
      expect(screen.queryByRole('alert')).toBeNull();
      latest.resolve({ enabled: true, phase: 'unknown', error: privateError });
      await settle();
      expect(view.toggle.checked).toBe(true);
      expect(view.toggle.disabled).toBe(false);
      expect(screen.queryByRole('alert')).toBeNull();
      expect(view.adapter.setLidProtection).toHaveBeenCalledTimes(operation === 'save' ? 1 : 0);
      view.unmount();
    }
  },
);

test.each(['read', 'save'] as const)(
  'inactive completion of a pending %s keeps the latest target read queued until re-entry',
  async (operation) => {
    const pending = Promise.withResolvers<LidState>();
    const getLidProtection = vi.fn();

    if (operation === 'save') getLidProtection.mockResolvedValueOnce(disabled);
    else getLidProtection.mockReturnValueOnce(pending.promise);

    getLidProtection.mockResolvedValue(enabled);

    const view = mount({ getLidProtection, setLidProtection: vi.fn().mockReturnValue(pending.promise) });

    await settle();
    if (operation === 'save') await fireEvent.click(view.toggle);
    await view.rerender({ active: false });
    await view.config.setOpenCodeServiceTarget(null);
    await settle();
    pending.resolve(enabled);
    await settle();
    expect(getLidProtection).toHaveBeenCalledOnce();
    await fireEvent.click(view.toggle);
    expect(view.adapter.setLidProtection).toHaveBeenCalledTimes(operation === 'save' ? 1 : 0);
    await view.rerender({ active: true });
    await settle();
    expect(getLidProtection).toHaveBeenCalledTimes(2);
    expect(screen.getByRole('checkbox')).toBe(view.toggle);
    expect(view.toggle.checked).toBe(true);
    expect(view.toggle.disabled).toBe(false);
  },
);

test('fast active round trips retain the pending write guard and defer readback without another write', async () => {
  const save = Promise.withResolvers<LidState>();
  const { adapter, rerender, toggle } = mount({ setLidProtection: vi.fn().mockReturnValue(save.promise) });

  await settle();
  await fireEvent.click(toggle);
  await rerender({ active: false });
  await rerender({ active: true });
  await fireEvent.click(toggle);
  expect(adapter.setLidProtection).toHaveBeenCalledExactlyOnceWith(true);
  expect(adapter.getLidProtection).toHaveBeenCalledOnce();
  expect(toggle.disabled).toBe(true);
  save.resolve(enabled);
  await settle();
  expect(adapter.getLidProtection).toHaveBeenCalledTimes(2);
  expect(toggle.disabled).toBe(false);
});

test.each(['read', 'save'] as const)('unmount drops a queued target reload behind a pending %s', async (operation) => {
  const pending = Promise.withResolvers<LidState>();
  const { adapter, config, toggle, unmount } = mount({
    getLidProtection: vi.fn().mockReturnValueOnce(operation === 'read' ? pending.promise : Promise.resolve(disabled)),
    setLidProtection: vi.fn().mockReturnValue(pending.promise),
  });

  await settle();
  if (operation === 'save') await fireEvent.click(toggle);
  await config.setOpenCodeServiceTarget(null);
  await settle();
  unmount();
  pending.reject(new Error(privateError));
  await settle();
  expect(adapter.getLidProtection).toHaveBeenCalledOnce();
  expect(adapter.setLidProtection).toHaveBeenCalledTimes(operation === 'save' ? 1 : 0);
  expect(screen.queryByRole('checkbox')).toBeNull();
  expect(screen.queryByRole('alert')).toBeNull();
});
