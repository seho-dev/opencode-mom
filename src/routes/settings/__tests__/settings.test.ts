import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { expect, test, vi } from 'vitest';
import type { ConfigStore } from '$src/config/store.svelte.js';
import { toast } from '$src/shell/toast.svelte.js';
import Harness from '$src/test-utils/harness.svelte';
import Settings from '../+page.svelte';

vi.mock('$src/shell/toast.svelte.js', () => ({ toast: vi.fn() }));
function mount(overrides: Record<string, unknown> = {}) {
  const config = {
    preferences: { locale: 'en', theme: 'dark' },
    getAutostart: vi.fn().mockResolvedValue(false),
    setAutostart: vi.fn().mockResolvedValue(true),
    getLidProtection: vi.fn().mockResolvedValue({ enabled: false, phase: 'disabled' }),
    setLidProtection: vi.fn().mockResolvedValue({ enabled: true, phase: 'idle' }),
    getAppInfo: vi.fn().mockResolvedValue({ version: '0.1.0', platform: 'other' }),
    checkForUpdates: vi.fn(),
    openProjectPage: vi.fn(),
    ...overrides,
  } as unknown as ConfigStore;
  render(Settings, {}, { wrapper: Harness, wrapperProps: { config } });
  return config;
}

test('reads OS status before enabling interaction and updates only after readback; lid state is independent', async () => {
  let read: (value: boolean) => void = () => {};
  let save: (value: boolean) => void = () => {};
  const getAutostart = vi.fn().mockImplementationOnce(
    () =>
      new Promise<boolean>((done) => {
        read = done;
      }),
  );
  const setAutostart = vi.fn().mockImplementationOnce(
    () =>
      new Promise<boolean>((done) => {
        save = done;
      }),
  );
  const config = mount({ getAutostart, setAutostart });
  const autostart = screen.getByRole('checkbox', { name: 'Launch at login' }) as HTMLInputElement;
  const lid = screen.getByRole('checkbox', { name: 'Stay awake with lid closed' }) as HTMLInputElement;
  expect(autostart.disabled).toBe(true);
  expect(lid.disabled).toBe(true);
  expect(lid.checked).toBe(false);
  await waitFor(() => expect(getAutostart).toHaveBeenCalledOnce());
  expect(setAutostart).not.toHaveBeenCalled();
  read(false);
  await waitFor(() => expect(autostart.disabled).toBe(false));
  await fireEvent.click(autostart);
  expect(setAutostart).toHaveBeenCalledExactlyOnceWith(true);
  expect(autostart.checked).toBe(false);
  expect(autostart.disabled).toBe(true);
  save(true);
  await waitFor(() => expect(autostart.checked).toBe(true));
  expect(vi.mocked(toast)).toHaveBeenCalledWith({
    variant: 'success',
    description: 'Launch-at-login setting updated.',
  });
  expect(lid.checked).toBe(false);
  expect(lid.disabled).toBe(false);
  expect(config.getLidProtection).toHaveBeenCalledOnce();
  expect(config.setLidProtection).not.toHaveBeenCalled();
  expect(setAutostart).toHaveBeenCalledOnce();
});

test('failed save preserves confirmed OS status and allows retry without a startup enable call', async () => {
  const setAutostart = vi.fn().mockRejectedValueOnce(new Error('permission denied')).mockResolvedValueOnce(false);
  const config = mount({ getAutostart: vi.fn().mockResolvedValue(true), setAutostart });
  const autostart = screen.getByRole('checkbox', { name: 'Launch at login' }) as HTMLInputElement;
  await waitFor(() => expect(autostart.checked).toBe(true));
  expect(setAutostart).not.toHaveBeenCalled();
  await fireEvent.click(autostart);
  const alert = await screen.findByRole('alert');
  expect(alert.textContent).toContain('permission denied');
  expect(autostart.checked).toBe(true);
  expect(autostart.disabled).toBe(false);
  await fireEvent.click(autostart);
  await waitFor(() => expect(autostart.checked).toBe(false));
  expect(setAutostart).toHaveBeenNthCalledWith(2, false);
  expect(config.getAutostart).toHaveBeenCalledOnce();
});

test('native unavailable is an honest disabled error state with read retry', async () => {
  const getAutostart = vi
    .fn()
    .mockRejectedValueOnce(new Error('Desktop application required'))
    .mockResolvedValueOnce(false);
  const config = mount({ getAutostart });
  const alert = await screen.findByRole('alert');
  const autostart = screen.getByRole('checkbox', { name: 'Launch at login' }) as HTMLInputElement;
  expect(alert.textContent).toContain('Desktop application required');
  expect(autostart.disabled).toBe(true);
  expect(config.setAutostart).not.toHaveBeenCalled();
  await fireEvent.click(within(alert).getByRole('button', { name: 'Retry' }));
  await waitFor(() => expect(autostart.disabled).toBe(false));
  expect(getAutostart).toHaveBeenCalledTimes(2);
});

test('uses OS readback instead of requested value and refresh only reads status', async () => {
  const getAutostart = vi.fn().mockResolvedValueOnce(false).mockResolvedValueOnce(true);
  const setAutostart = vi.fn().mockResolvedValue(false);
  mount({ getAutostart, setAutostart });
  const autostart = screen.getByRole('checkbox', { name: 'Launch at login' }) as HTMLInputElement;
  await waitFor(() => expect(autostart.disabled).toBe(false));
  await fireEvent.click(autostart);
  await waitFor(() => expect(autostart.disabled).toBe(false));
  expect(setAutostart).toHaveBeenCalledExactlyOnceWith(true);
  expect(autostart.checked).toBe(false);
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh' }));
  await waitFor(() => expect(autostart.checked).toBe(true));
  expect(getAutostart).toHaveBeenCalledTimes(2);
  expect(setAutostart).toHaveBeenCalledOnce();
});

test('unavailable autostart does not block lid interaction or its independent refresh', async () => {
  const getAutostart = vi.fn().mockRejectedValue(new Error('Login unavailable'));
  const config = mount({ getAutostart });
  const lid = screen.getByRole('checkbox', { name: 'Stay awake with lid closed' }) as HTMLInputElement;
  await waitFor(() => expect(lid.disabled).toBe(false));
  await fireEvent.click(lid);
  await waitFor(() => expect(lid.checked).toBe(true));
  expect(config.setLidProtection).toHaveBeenCalledExactlyOnceWith(true);
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh status' }));
  await waitFor(() => expect(config.getLidProtection).toHaveBeenCalledTimes(2));
  expect(getAutostart).toHaveBeenCalledOnce();
  expect(config.setAutostart).not.toHaveBeenCalled();
});

test('mount and system refresh read local app details without checking updates', async () => {
  const config = mount();
  await screen.findByText('Installed version: 0.1.0');
  expect(config.getAppInfo).toHaveBeenCalledOnce();
  expect(config.checkForUpdates).not.toHaveBeenCalled();
  await waitFor(() =>
    expect((screen.getByRole('button', { name: 'Refresh' }) as HTMLButtonElement).disabled).toBe(false),
  );
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh' }));
  expect(config.checkForUpdates).not.toHaveBeenCalled();
  expect(config.getAppInfo).toHaveBeenCalledOnce();
});
