import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { expect, test, vi } from 'vitest';
import type { ConfigStore } from '$src/config/store.svelte.js';
import Harness from '$src/test-utils/harness.svelte';
import type { ProviderDef } from '$src/types/providers.js';
import ProviderList from '../+page.svelte';

function mount(providers: ProviderDef[] = [], loading = false, error: ConfigStore['error'] = null) {
  const deleteProvider = vi.fn().mockResolvedValue(undefined);
  const config = {
    preferences: { locale: 'en', theme: 'light' },
    providers,
    loading,
    error,
    deleteProvider,
  } as unknown as ConfigStore;
  render(ProviderList, {}, { wrapper: Harness, wrapperProps: { config } });
  return { table: screen.getByRole('table') as HTMLTableElement, deleteProvider };
}

test('provider headers and rows have five columns with working actions in the final column', async () => {
  const providers: ProviderDef[] = [
    {
      name: 'example',
      package: 'aisdk:@ai-sdk/openai',
      settings: { baseURL: 'https://api.example.com/v1' },
      models: { custom: { id: 'custom', name: 'Custom' } },
    },
    { name: 'minimal', models: {} },
  ];
  const { table, deleteProvider } = mount(providers);
  expect(Array.from(table.rows[0].cells, (cell) => cell.textContent?.trim())).toEqual([
    'Name',
    'NPM',
    'Base URL',
    'Models',
    'Actions',
  ]);
  expect(table.rows[0].cells[4].classList.contains('th-actions')).toBe(true);
  Array.from(table.tBodies[0].rows).forEach((row, index) => {
    expect(row.cells.length).toBe(5);
    expect(row.cells[4].classList.contains('row-actions')).toBe(true);
    expect(
      within(row.cells[4])
        .getByRole('link', { name: `Edit ${providers[index].name}` })
        .getAttribute('href'),
    ).toBe(`/providers/${providers[index].name}/edit`);
    expect(within(row.cells[4]).getByRole('button', { name: `Delete ${providers[index].name}` })).toBeTruthy();
  });

  await fireEvent.click(screen.getByRole('button', { name: 'Delete example' }));
  await fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'Cancel' }));
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect(deleteProvider).not.toHaveBeenCalled();
  await fireEvent.click(screen.getByRole('button', { name: 'Delete example' }));
  await fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'Delete' }));
  await waitFor(() => expect(deleteProvider).toHaveBeenCalledExactlyOnceWith('example'));
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
});

test.each([
  { state: 'loading', loading: true, error: null },
  { state: 'empty', loading: false, error: null },
  { state: 'load error', loading: false, error: { code: 'ipc_error' as const, message: 'Load failed' } },
])('$state row spans all five provider columns', ({ loading, error }) => {
  const { table } = mount([], loading, error);
  expect(table.rows[0].cells.length).toBe(5);
  expect(table.tBodies[0].rows.length).toBe(1);
  const cells = table.tBodies[0].rows[0].cells;
  expect(cells.length).toBe(1);
  expect(cells[0].colSpan).toBe(5);
  expect(cells[0].textContent?.trim()).toBe(loading ? 'Loading configuration...' : 'No providers.');
});
