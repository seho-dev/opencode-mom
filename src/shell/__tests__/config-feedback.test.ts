import { render } from '@testing-library/svelte';
import { tick } from 'svelte';
import { SvelteMap } from 'svelte/reactivity';
import { expect, test, vi } from 'vitest';
import type { ConfigStore } from '$src/config/store.svelte.js';
import Harness from '$src/test-utils/harness.svelte';
import ConfigFeedback from '../ConfigFeedback.svelte';
import type { ToastOptions } from '../toast.svelte.js';

const { toast } = vi.hoisted(() => ({ toast: vi.fn() }));
vi.mock('../toast.svelte.js', () => ({ toast }));

test('offers the correct recovery action for conflict, busy, and blocked references', async () => {
  toast.mockClear();
  const state = new SvelteMap<string, { code: string; message: string } | null>([['error', null]]);
  const reloadKeepingDraft = vi.fn();
  const continueEditing = vi.fn();
  const discardDraftAndRefresh = vi.fn();
  const config = {
    preferences: { theme: 'dark', locale: 'en' },
    get error() {
      return state.get('error');
    },
    reloadKeepingDraft,
    continueEditing,
    discardDraftAndRefresh,
  } as unknown as ConfigStore;
  render(ConfigFeedback, {}, { wrapper: Harness, wrapperProps: { config } });

  state.set('error', { code: 'conflict', message: 'Outdated draft' });
  await tick();
  const conflict = toast.mock.lastCall?.[0] as ToastOptions;
  expect(conflict).toMatchObject({
    variant: 'error',
    title: 'Outdated draft',
    actions: [
      { label: 'Keep draft and reload' },
      { label: 'Review and merge manually' },
      { label: 'Discard draft and refresh' },
    ],
  });
  conflict.actions?.[0].onclick();
  conflict.actions?.[1].onclick();
  conflict.actions?.[2].onclick();
  expect(reloadKeepingDraft).toHaveBeenCalledOnce();
  expect(continueEditing).toHaveBeenCalledOnce();
  expect(discardDraftAndRefresh).toHaveBeenCalledOnce();

  state.set('error', { code: 'busy', message: 'Try later' });
  await tick();
  const busy = toast.mock.lastCall?.[0] as ToastOptions;
  expect(busy).toMatchObject({ variant: 'error', action: { label: 'Continue editing' } });
  expect(busy.actions).toBeUndefined();
  busy.action?.onclick();

  state.set('error', { code: 'references_blocked', message: 'Still in use' });
  await tick();
  const blocked = toast.mock.lastCall?.[0] as ToastOptions;
  expect(blocked).toMatchObject({ variant: 'error', action: { label: 'Back to draft' } });
  blocked.action?.onclick();
  expect(continueEditing).toHaveBeenCalledTimes(3);
  expect(toast).toHaveBeenCalledTimes(3);
});
