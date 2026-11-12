import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { expect, test, vi } from 'vitest';
import type { ConfigStore } from '$src/config/store.svelte.js';
import { zh } from '$src/i18n/dictionaries/zh.js';
import Harness from '$src/test-utils/harness.svelte';
import AppearanceControls from '../AppearanceControls.svelte';

function mount(overrides: Record<string, unknown> = {}) {
  const config = {
    preferences: { theme: 'dark', locale: 'zh' },
    loading: false,
    saving: false,
    splashLoading: false,
    switching: false,
    reloading: false,
    refreshAll: vi.fn().mockResolvedValue(undefined),
    setTheme: vi.fn(),
    setLocale: vi.fn(),
    reloadOpencode: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  } as unknown as ConfigStore;

  return { config, ...render(AppearanceControls, {}, { wrapper: Harness, wrapperProps: { config } }) };
}

function restartButton() {
  const group = screen.getByRole('group', { name: zh['appearance.group'] });

  return within(group).getAllByRole('button')[0] as HTMLButtonElement;
}

async function openConfirmation() {
  await fireEvent.click(restartButton());

  return await screen.findByRole('dialog', { name: zh['header.reloadTitle'] });
}

test('provides config and translated labels to a child', () => {
  const { config } = mount({ preferences: { theme: 'dark', locale: 'zh' } });

  expect(screen.getByRole('group', { name: '页眉控件' })).toBeTruthy();
  expect(screen.getByRole('button', { name: '主题：深色。切换到浅色。' })).toBeTruthy();
  expect(config.reloadOpencode).not.toHaveBeenCalled();
});

test('restart asks for confirmation and only reloads after the confirm button', async () => {
  const reloadOpencode = vi.fn().mockResolvedValue(undefined);
  const { config } = mount({ reloadOpencode });
  const restart = restartButton();

  expect(restart.getAttribute('aria-busy')).toBe('false');
  expect(reloadOpencode).not.toHaveBeenCalled();

  const dialog = await openConfirmation();

  expect(within(dialog).getByText(zh['header.reloadDescription'])).toBeTruthy();
  expect(reloadOpencode).not.toHaveBeenCalled();
  await fireEvent.click(within(dialog).getByRole('button', { name: zh['header.reloadOpencode'] }));
  await waitFor(() => expect(reloadOpencode).toHaveBeenCalledExactlyOnceWith());
  expect(config.reloadOpencode).toHaveBeenCalledOnce();
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
});

test('canceling the confirmation through its button or Escape leaves OpenCode untouched', async () => {
  const reloadOpencode = vi.fn().mockResolvedValue(undefined);
  mount({ reloadOpencode });

  let dialog = await openConfirmation();

  await fireEvent.click(within(dialog).getByRole('button', { name: zh['common.cancel'] }));
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect(reloadOpencode).not.toHaveBeenCalled();

  dialog = await openConfirmation();
  await fireEvent.keyDown(dialog, { key: 'Escape' });
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect(reloadOpencode).not.toHaveBeenCalled();
});

test('restart stays disabled and cannot confirm while any busy state is active', async () => {
  for (const busy of ['reloading', 'switching', 'loading', 'saving', 'splashLoading'] as const) {
    const reloadOpencode = vi.fn().mockResolvedValue(undefined);
    const view = mount({ [busy]: true, reloadOpencode });
    const restart = restartButton();

    expect(restart.disabled).toBe(true);
    await fireEvent.click(restart);
    expect(reloadOpencode).not.toHaveBeenCalled();
    expect(screen.queryByRole('dialog')).toBeNull();
    view.unmount();
  }
});

test('confirming twice while the restart is pending reloads only once', async () => {
  let finish: () => void = () => {};
  const reloadOpencode = vi.fn(
    () =>
      new Promise<void>((resolve) => {
        finish = resolve;
      }),
  );
  mount({ reloadOpencode });

  const dialog = await openConfirmation();
  const confirm = within(dialog).getByRole('button', { name: zh['header.reloadOpencode'] });

  confirm.dispatchEvent(new MouseEvent('click', { bubbles: true }));
  confirm.dispatchEvent(new MouseEvent('click', { bubbles: true }));
  await waitFor(() => expect(reloadOpencode).toHaveBeenCalledOnce());
  expect(within(dialog).getByRole('status').textContent).toBe(zh['header.reloading']);
  await fireEvent.keyDown(dialog, { key: 'Escape' });
  expect(screen.getByRole('dialog', { name: zh['header.reloadTitle'] })).toBeTruthy();
  finish();
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect(reloadOpencode).toHaveBeenCalledOnce();
});
