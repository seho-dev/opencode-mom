import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { expect, test, vi } from 'vitest';
import type { ConfigStore } from '$src/config/store.svelte.js';
import { getToasts } from '$src/shell/toast.svelte.js';
import Harness from '$src/test-utils/harness.svelte';
import type { ModelCatalogEntry } from '$src/types/models.js';
import ModelList from '../+page.svelte';

function mount(catalog: ModelCatalogEntry[] = []) {
  const config = {
    preferences: { locale: 'en', theme: 'light' },
    catalog,
    catalogLoading: false,
    catalogError: null,
    loading: false,
    error: null,
    refreshCatalog: vi.fn(),
    models: vi.fn(() => []),
  } as unknown as ConfigStore;

  render(ModelList, {}, { wrapper: Harness, wrapperProps: { config } });

  return { table: screen.getByRole('table') as HTMLTableElement };
}

const custom = (ref: string, name: string): ModelCatalogEntry => ({
  ref: ref as `${string}/${string}`,
  providerId: 'example',
  name,
  isCustom: true,
});

test('model list query trims leading and trailing whitespace', async () => {
  const { table } = mount([custom('example/alpha', 'Alpha Model'), custom('example/beta', 'Beta Model')]);

  expect(table.tBodies[0].rows.length).toBe(2);

  await fireEvent.input(screen.getByRole('textbox', { name: 'Search models' }), {
    target: { value: '   alpha   ' },
  });
  expect(table.tBodies[0].rows.length).toBe(1);
  expect(table.tBodies[0].rows[0].cells[1].textContent?.trim()).toBe('Alpha Model');
});

test('model delete failure reports the readable message through the toast', async () => {
  const deleteModel = vi.fn().mockRejectedValue(new Error('model is referenced'));
  const config = {
    preferences: { locale: 'en', theme: 'light' },
    catalog: [custom('example/alpha', 'Alpha Model')],
    catalogLoading: false,
    catalogError: null,
    loading: false,
    error: null,
    refreshCatalog: vi.fn(),
    models: vi.fn(() => []),
    deleteModel,
  } as unknown as ConfigStore;

  render(ModelList, {}, { wrapper: Harness, wrapperProps: { config } });

  await fireEvent.click(screen.getByRole('button', { name: 'Delete example/alpha' }));
  await fireEvent.click(screen.getByRole('button', { name: 'Delete' }));

  await waitFor(() => expect(deleteModel).toHaveBeenCalledExactlyOnceWith('example/alpha'));
  await waitFor(() =>
    expect(getToasts().some((entry) => entry.variant === 'error' && entry.description === 'model is referenced')).toBe(
      true,
    ),
  );
});
