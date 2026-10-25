import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { expect, test, vi } from 'vitest';
import type { ConfigStore } from '$src/config/store.svelte.js';
import Harness from '$src/test-utils/harness.svelte';
import RotateKeyDialog from '../RotateKeyDialog.svelte';

function mount(rotateMcpProxyKey = vi.fn().mockResolvedValue('private-rotated-token')) {
  const config = { preferences: { locale: 'en' }, rotateMcpProxyKey } as unknown as ConfigStore;
  return render(RotateKeyDialog, { open: true }, { wrapper: Harness, wrapperProps: { config } });
}

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

test('leaving and re-entering the proxy tab discards a pending one-time token', async () => {
  let resolve!: (value: string) => void;
  const rotateMcpProxyKey = vi.fn().mockReturnValue(
    new Promise<string>((done) => {
      resolve = done;
    }),
  );
  const { rerender } = mount(rotateMcpProxyKey);
  await fireEvent.click(await screen.findByRole('button', { name: 'Generate New Token' }));

  await rerender({ open: true, active: false });
  await rerender({ open: true, active: true });
  resolve('late-private-token');
  await waitFor(() =>
    expect((screen.getByRole('button', { name: 'Generate New Token' }) as HTMLButtonElement).disabled).toBe(false),
  );
  expect(screen.queryByLabelText('New Secret Token Generated')).toBeNull();
  expect(rotateMcpProxyKey).toHaveBeenCalledOnce();
});
