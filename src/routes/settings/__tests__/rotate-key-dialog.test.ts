import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, expect, test, vi } from 'vitest';
import type { ConfigStore } from '$src/config/store.svelte.js';
import Harness from '$src/test-utils/harness.svelte';
import RotateKeyDialog from '../RotateKeyDialog.svelte';

function mount(rotateMomHttpKey = vi.fn().mockResolvedValue('private-rotated-token')) {
  const config = { preferences: { locale: 'en' }, rotateMomHttpKey } as unknown as ConfigStore;

  return render(RotateKeyDialog, { open: true }, { wrapper: Harness, wrapperProps: { config } });
}

afterEach(() => {
  vi.unstubAllGlobals();
});

test('externally closing the token dialog clears its one-time secret before reopening', async () => {
  const { rerender } = mount();

  await fireEvent.click(await screen.findByRole('button', { name: 'Generate New Token' }));
  expect(((await screen.findByLabelText('New Secret Token Generated')) as HTMLInputElement).value).toBe(
    'private-rotated-token',
  );

  await rerender({ open: false });
  await rerender({ open: true });
  expect(screen.queryByLabelText('New Secret Token Generated')).toBeNull();
  expect(await screen.findByRole('button', { name: 'Generate New Token' })).toBeTruthy();
});

test.each(['close', 'inactive'] as const)('%s discards late one-time token results before re-entry', async (action) => {
  let resolve!: (value: string) => void;
  const rotateMomHttpKey = vi.fn().mockReturnValue(
    new Promise<string>((done) => {
      resolve = done;
    }),
  );
  const { rerender } = mount(rotateMomHttpKey);
  const confirm = (await screen.findByRole('button', { name: 'Generate New Token' })) as HTMLButtonElement;

  await fireEvent.click(confirm);
  expect(confirm.disabled).toBe(true);
  expect(screen.getByRole('dialog').getAttribute('aria-busy')).toBe('true');
  await fireEvent.click(confirm);
  expect(rotateMomHttpKey).toHaveBeenCalledOnce();

  await rerender(action === 'close' ? { open: false } : { open: true, active: false });
  await rerender({ open: true, active: true });

  const pendingConfirm = screen.getByRole('button', { name: 'Updating…' }) as HTMLButtonElement;

  expect(pendingConfirm.disabled).toBe(true);
  await fireEvent.click(pendingConfirm);
  expect(rotateMomHttpKey).toHaveBeenCalledOnce();
  expect(screen.queryByLabelText('New Secret Token Generated')).toBeNull();
  resolve('late-private-token');
  await waitFor(() =>
    expect((screen.getByRole('button', { name: 'Generate New Token' }) as HTMLButtonElement).disabled).toBe(false),
  );
  expect(screen.queryByLabelText('New Secret Token Generated')).toBeNull();
  expect(rotateMomHttpKey).toHaveBeenCalledOnce();
});

test('leaving General clears the displayed token and copied state before returning', async () => {
  const writeText = vi.fn().mockResolvedValue(undefined);

  vi.stubGlobal('navigator', { clipboard: { writeText } });

  const { rerender } = mount();

  await fireEvent.click(await screen.findByRole('button', { name: 'Generate New Token' }));
  await fireEvent.click(await screen.findByRole('button', { name: 'Copy Token' }));
  expect(writeText).toHaveBeenCalledExactlyOnceWith('private-rotated-token');
  expect(screen.getByRole('button', { name: 'Copied!' })).toBeTruthy();
  await rerender({ open: true, active: false });
  expect(screen.queryByLabelText('New Secret Token Generated')).toBeNull();
  await rerender({ open: true, active: true });
  expect(screen.queryByLabelText('New Secret Token Generated')).toBeNull();
  await fireEvent.click(screen.getByRole('button', { name: 'Generate New Token' }));
  expect(await screen.findByRole('button', { name: 'Copy Token' })).toBeTruthy();
});

test('late clipboard completion cannot restore copied state after close and reopening', async () => {
  let finishCopy!: () => void;
  const writeText = vi.fn().mockReturnValue(
    new Promise<void>((resolve) => {
      finishCopy = resolve;
    }),
  );

  vi.stubGlobal('navigator', { clipboard: { writeText } });

  const { rerender } = mount();

  await fireEvent.click(await screen.findByRole('button', { name: 'Generate New Token' }));
  await fireEvent.click(await screen.findByRole('button', { name: 'Copy Token' }));
  await rerender({ open: false });
  await rerender({ open: true });
  await fireEvent.click(screen.getByRole('button', { name: 'Generate New Token' }));
  finishCopy();
  await waitFor(() => expect(screen.queryByRole('button', { name: 'Copied!' })).toBeNull());
  expect(screen.getByRole('button', { name: 'Copy Token' })).toBeTruthy();
});

test('unmount discards a pending HTTP rotation without feedback or readback', async () => {
  let finish!: (value: string) => void;
  const pending = new Promise<string>((resolve) => {
    finish = resolve;
  });
  const rotateMomHttpKey = vi.fn().mockReturnValue(pending);
  const onRotated = vi.fn();
  const view = mount(rotateMomHttpKey);

  await view.rerender({ open: true, onRotated });
  await fireEvent.click(await screen.findByRole('button', { name: 'Generate New Token' }));
  view.unmount();
  finish('late-unmounted-token');
  await pending;
  expect(onRotated).not.toHaveBeenCalled();
  expect(screen.queryByLabelText('New Secret Token Generated')).toBeNull();
});
