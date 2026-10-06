import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterAll, beforeAll, expect, test, vi } from 'vitest';
import type { ConfigStore } from '$src/config/store.svelte.js';
import Harness from '$src/test-utils/harness.svelte';
import AgentList from '../+page.svelte';

const originalScrollIntoView = HTMLElement.prototype.scrollIntoView;
beforeAll(() => {
  HTMLElement.prototype.scrollIntoView = vi.fn();
});
afterAll(() => {
  if (originalScrollIntoView) HTMLElement.prototype.scrollIntoView = originalScrollIntoView;
  else delete (HTMLElement.prototype as Partial<HTMLElement>).scrollIntoView;
});

test('cancel keeps both sources, while confirming removes only the selected storage', async () => {
  const deleteAgent = vi.fn().mockResolvedValue(undefined);
  const config = {
    preferences: { locale: 'en', theme: 'light' },
    agents: [{ id: 'helper', source: 'both', description: 'Custom helper' }],
    loading: false,
    deleteAgent,
  } as unknown as ConfigStore;
  render(AgentList, {}, { wrapper: Harness, wrapperProps: { config } });

  await fireEvent.click(screen.getByRole('button', { name: 'Delete helper' }));
  const dialog = screen.getByRole('dialog');
  expect(within(dialog).getByText('Source to delete')).toBeTruthy();
  await fireEvent.click(within(dialog).getByRole('button', { name: 'Cancel' }));
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect(deleteAgent).not.toHaveBeenCalled();

  await fireEvent.click(screen.getByRole('button', { name: 'Delete helper' }));
  const confirmation = screen.getByRole('dialog');
  const source = within(confirmation).getByRole('button', { name: 'Source to delete' });
  await fireEvent.click(source);
  const option = await screen.findByRole('option', { name: 'Global Markdown' });
  expect(option.closest('[role="listbox"]')?.id).toBe(source.getAttribute('aria-controls'));
  expect(confirmation.contains(option)).toBe(false);
  await fireEvent.click(option);
  await fireEvent.click(within(confirmation).getByRole('button', { name: 'Delete' }));
  await waitFor(() => expect(deleteAgent).toHaveBeenCalledOnce());
  expect(deleteAgent).toHaveBeenCalledWith('helper', 'global_markdown');
});
