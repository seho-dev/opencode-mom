import { fireEvent, render, screen } from '@testing-library/svelte';
import { expect, test, vi } from 'vitest';
import type { ConfigStore } from '$src/config/store.svelte.js';
import Harness from '$src/test-utils/harness.svelte';
import type { Group } from '$src/types/groups.js';
import RuntimeCard from '../RuntimeCard.svelte';

const group = (type: Group['type'], name: string, isEnabled: boolean): Group => ({
  id: `${type}-${name}`,
  type,
  name,
  description: '',
  isEnabled,
  updatedAt: '2026-01-01T00:00:00Z',
  openCodeAgentOverrides: [],
  slimAgentOverrides: [],
  omoAgentOverrides: [],
  omoCategoryMappings: [],
});

function mount(
  variant: 'tile' | 'featured',
  groups: Group[],
  { selectedGroupId = groups[0]?.id ?? null, loading = false, busy = false } = {},
) {
  const config = {
    groups,
    selectedGroupId,
    loading,
    error: null,
    preferences: { locale: 'en', theme: 'light' },
  } as unknown as ConfigStore;
  const onswitch = vi.fn();

  render(RuntimeCard, { type: 'native', busy, variant, onswitch }, { wrapper: Harness, wrapperProps: { config } });

  return { onswitch };
}

test('tile variant exposes the selected runtime and forwards the switch trigger', async () => {
  const { onswitch } = mount('tile', [
    group('native', 'Alpha', true),
    group('native', 'Beta', false),
    group('slim', 'Other', true),
  ]);

  const trigger = screen.getByRole('button', { name: 'Switch Native group' });

  expect(screen.getByText('Alpha')).toBeTruthy();
  expect(screen.getByText('Current selection')).toBeTruthy();

  await fireEvent.click(trigger);
  expect(onswitch).toHaveBeenCalledExactlyOnceWith('native', trigger);
});

test('tile variant reports group counts when no group is selected', () => {
  mount('tile', [group('native', 'Alpha', true), group('native', 'Beta', false)], { selectedGroupId: null });

  expect(screen.getByText('No group selected')).toBeTruthy();
  expect(screen.getByText('1 enabled / 2 groups')).toBeTruthy();
});

test('featured variant shows the mapping roster without the tile trigger', () => {
  mount('featured', [group('native', 'Alpha', true)]);

  expect(screen.queryByRole('button', { name: 'Switch Native group' })).toBeNull();
  expect(screen.getByText('Runtime workbench')).toBeTruthy();
  expect(screen.getByRole('heading', { name: 'Native runtime' })).toBeTruthy();

  const editLinks = screen.getAllByRole('link', { name: 'Edit Alpha' });

  expect(editLinks.map((link) => link.getAttribute('href'))).toEqual([
    '/groups/native-Alpha/edit',
    '/groups/native-Alpha/edit',
  ]);
});

test('tile variant marks the switch trigger unavailable while busy', () => {
  mount('tile', [group('native', 'Alpha', true)], { busy: true });

  const trigger = screen.getByRole('button', { name: 'Switch Native group' }) as HTMLButtonElement;

  expect(trigger.disabled).toBe(true);
});
