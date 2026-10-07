import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { tick } from 'svelte';
import { beforeEach, expect, test, vi } from 'vitest';
import type { ConfigStore } from '$src/config/store.svelte.js';
import Harness from '$src/test-utils/harness.svelte';
import AppHeader from '../AppHeader.svelte';
import { toast } from '../toast.svelte.js';

vi.mock('$app/state', () => ({ page: { url: new URL('http://localhost/settings') } }));
vi.mock('$app/navigation', () => ({ goto: vi.fn().mockResolvedValue(undefined) }));
vi.mock('../toast.svelte.js', () => ({ toast: vi.fn() }));

beforeEach(() => vi.mocked(toast).mockClear());

function mount(overrides: Record<string, unknown> = {}) {
  const config = {
    preferences: { locale: 'en', theme: 'dark' },
    loading: false,
    splashLoading: false,
    openProjectPage: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  };
  const view = render(AppHeader, {}, { wrapper: Harness, wrapperProps: { config: config as unknown as ConfigStore } });
  return { config, ...view };
}

test('translated GitHub icon has a title, opens through config and guards repeated pending clicks', async () => {
  let finish: () => void = () => {};
  const openProjectPage = vi.fn().mockImplementationOnce(
    () =>
      new Promise<void>((resolve) => {
        finish = resolve;
      }),
  );
  mount({ preferences: { locale: 'zh', theme: 'dark' }, openProjectPage });
  const button = screen.getByRole('button', { name: '在默认浏览器中打开 GitHub 项目' }) as HTMLButtonElement;
  expect(button.title).toBe('在默认浏览器中打开 GitHub 项目');
  expect(screen.getByRole('group', { name: '页眉控件' })).toBeTruthy();
  expect(button.querySelector('svg')?.getAttribute('aria-hidden')).toBe('true');
  expect(openProjectPage).not.toHaveBeenCalled();
  await fireEvent.click(button);
  expect(openProjectPage).toHaveBeenCalledExactlyOnceWith('repository');
  expect(button.disabled).toBe(true);
  expect(button.getAttribute('aria-busy')).toBe('true');
  await fireEvent.click(button);
  expect(openProjectPage).toHaveBeenCalledOnce();
  finish();
  await waitFor(() => expect(button.disabled).toBe(false));
  expect(toast).not.toHaveBeenCalled();
});

test('opener failure gives translated error feedback and leaves the GitHub icon usable for retry', async () => {
  const openProjectPage = vi
    .fn()
    .mockRejectedValueOnce(new Error('No default browser'))
    .mockResolvedValueOnce(undefined);
  mount({ openProjectPage });
  const button = screen.getByRole('button', {
    name: 'Open project on GitHub in your default browser',
  }) as HTMLButtonElement;
  await fireEvent.click(button);
  await waitFor(() =>
    expect(toast).toHaveBeenCalledExactlyOnceWith({
      variant: 'error',
      description: 'Could not open GitHub in your default browser: No default browser',
    }),
  );
  expect(button.disabled).toBe(false);
  await fireEvent.click(button);
  await waitFor(() => expect(button.disabled).toBe(false));
  expect(openProjectPage).toHaveBeenCalledTimes(2);
});

test('late opener failure after header teardown does not create a stale toast', async () => {
  let reject: (cause: Error) => void = () => {};
  const openProjectPage = vi.fn().mockImplementationOnce(
    () =>
      new Promise<void>((_resolve, fail) => {
        reject = fail;
      }),
  );
  const view = mount({ openProjectPage });
  await fireEvent.click(screen.getByRole('button', { name: 'Open project on GitHub in your default browser' }));
  view.unmount();
  reject(new Error('Late browser error'));
  await tick();
  expect(toast).not.toHaveBeenCalled();
});
