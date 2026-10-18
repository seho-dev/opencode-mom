import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterAll, beforeAll, expect, test, vi } from 'vitest';
import { makeScopedConfig } from '$src/components/__tests__/scope-config.js';
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

test('project agents are independent of Global state, readonly, and builtin tabs keep their static catalog', async () => {
  const globalAgents = [{ id: 'global-helper', source: 'inline', description: 'Global only' }];
  const listAgents = vi
    .fn()
    .mockResolvedValue([{ id: 'project-helper', source: 'markdown', description: 'Project only' }]);
  const config = makeScopedConfig({ agents: globalAgents, listAgents });
  render(AgentList, {}, { wrapper: Harness, wrapperProps: { config } });
  const link = await screen.findByRole('link', { name: 'project-helper' });
  expect(link.getAttribute('href')).toBe('/agents/project-helper?project=project-a');
  expect(listAgents).toHaveBeenCalledExactlyOnceWith('project-a');
  expect(config.agents).toEqual(globalAgents);
  expect(screen.queryByRole('button', { name: 'Delete project-helper' })).toBeNull();
  expect(screen.queryByRole('link', { name: 'Edit project-helper' })).toBeNull();
  expect(screen.queryByRole('link', { name: 'New agent' })).toBeNull();
  await fireEvent.click(screen.getByRole('tab', { name: /^Native/ }));
  const builtin = screen.getByRole('tabpanel').textContent;
  expect(builtin).not.toContain('project-helper');
  await fireEvent.click(screen.getByRole('button', { name: 'Return to Global' }));
  await waitFor(() => expect(config.selectedProjectId).toBeNull());
  expect(screen.getByRole('tabpanel').textContent).toBe(builtin);
  expect(listAgents).toHaveBeenCalledTimes(1);
  await fireEvent.click(screen.getByRole('tab', { name: /^Custom/ }));
  expect(screen.getByText('global-helper')).toBeTruthy();
  expect(screen.getByRole('link', { name: 'Edit global-helper' })).toBeTruthy();
});

test('same-name project and Global agents remain distinct readonly entries', async () => {
  const config = makeScopedConfig({
    agents: [{ id: 'helper', source: 'inline' }],
    listAgents: vi.fn().mockResolvedValue([{ id: 'helper', source: 'markdown' }]),
  });
  render(AgentList, {}, { wrapper: Harness, wrapperProps: { config } });
  await screen.findByRole('link', { name: 'helper' });
  await fireEvent.click(screen.getByRole('checkbox', { name: 'Show global configuration' }));
  await waitFor(() => expect(screen.getAllByRole('link', { name: 'helper' })).toHaveLength(2));
  expect(screen.getAllByRole('link', { name: 'helper' }).map((link) => link.getAttribute('href'))).toEqual([
    '/agents/helper?project=project-a',
    '/agents/helper',
  ]);
  expect(screen.queryByRole('link', { name: 'Edit helper' })).toBeNull();
  expect(screen.queryByRole('button', { name: 'Delete helper' })).toBeNull();
});

test('late project agent read cannot replace Global state or reappear after returning', async () => {
  let resolve: (value: unknown[]) => void = () => {};
  const listAgents = vi.fn().mockImplementation(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  const config = makeScopedConfig({ agents: [{ id: 'global-helper', source: 'inline' }], listAgents });
  render(AgentList, {}, { wrapper: Harness, wrapperProps: { config } });
  await waitFor(() => expect(listAgents).toHaveBeenCalledWith('project-a'));
  await fireEvent.click(screen.getByRole('button', { name: 'Return to Global' }));
  await screen.findByText('global-helper');
  resolve([{ id: 'late-helper', source: 'markdown' }]);
  await waitFor(() => expect(screen.queryByText('late-helper')).toBeNull());
  expect(config.agents.map((agent) => agent.id)).toEqual(['global-helper']);
});

test('project Refresh lazily rereads both scopes without replacing Global agents cache', async () => {
  const globalAgents = [{ id: 'cached-global', source: 'inline' }];
  const listAgents = vi
    .fn()
    .mockResolvedValueOnce([{ id: 'project-old', source: 'markdown' }])
    .mockResolvedValueOnce([{ id: 'global-old', source: 'inline' }])
    .mockResolvedValueOnce([{ id: 'project-new', source: 'markdown' }])
    .mockResolvedValueOnce([{ id: 'global-new', source: 'inline' }]);
  const config = makeScopedConfig({ agents: globalAgents, listAgents });
  render(AgentList, {}, { wrapper: Harness, wrapperProps: { config } });
  await screen.findByRole('link', { name: 'project-old' });
  expect(listAgents).toHaveBeenCalledTimes(1);
  await fireEvent.click(screen.getByRole('checkbox', { name: 'Show global configuration' }));
  await screen.findByRole('link', { name: 'global-old' });
  expect(listAgents).toHaveBeenNthCalledWith(2);
  expect(screen.queryByText('cached-global')).toBeNull();
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh' }));
  await screen.findByRole('link', { name: 'project-new' });
  await screen.findByRole('link', { name: 'global-new' });
  expect(listAgents).toHaveBeenNthCalledWith(3, 'project-a');
  expect(listAgents).toHaveBeenNthCalledWith(4);
  expect(config.agents).toEqual(globalAgents);
  expect(screen.queryByRole('link', { name: 'Edit global-new' })).toBeNull();
});

test('older Global comparison read cannot overwrite a newer toggle read or leak into Global', async () => {
  let resolve: (value: unknown[]) => void = () => {};
  const listAgents = vi
    .fn()
    .mockResolvedValueOnce([{ id: 'project-helper', source: 'markdown' }])
    .mockImplementationOnce(
      () =>
        new Promise((done) => {
          resolve = done;
        }),
    )
    .mockResolvedValueOnce([{ id: 'current-comparison', source: 'inline' }]);
  const config = makeScopedConfig({ agents: [{ id: 'cached-global', source: 'inline' }], listAgents });
  render(AgentList, {}, { wrapper: Harness, wrapperProps: { config } });
  await screen.findByRole('link', { name: 'project-helper' });
  const checkbox = screen.getByRole('checkbox', { name: 'Show global configuration' });
  await fireEvent.click(checkbox);
  await waitFor(() => expect(listAgents).toHaveBeenCalledTimes(2));
  await fireEvent.click(checkbox);
  await fireEvent.click(checkbox);
  await screen.findByRole('link', { name: 'current-comparison' });
  resolve([{ id: 'late-comparison', source: 'inline' }]);
  await waitFor(() => expect(screen.queryByText('late-comparison')).toBeNull());
  await fireEvent.click(screen.getByRole('button', { name: 'Return to Global' }));
  await screen.findByText('cached-global');
  expect(screen.queryByText('current-comparison')).toBeNull();
  expect(config.agents.map((agent) => agent.id)).toEqual(['cached-global']);
});

test('Agent Global comparison failure stays separate from project data and retries only Global', async () => {
  const listAgents = vi
    .fn()
    .mockResolvedValueOnce([{ id: 'project-helper', source: 'markdown' }])
    .mockRejectedValueOnce(new Error('Global read denied'))
    .mockResolvedValueOnce([{ id: 'global-helper', source: 'inline' }]);
  render(AgentList, {}, { wrapper: Harness, wrapperProps: { config: makeScopedConfig({ listAgents }) } });
  await screen.findByRole('link', { name: 'project-helper' });
  await fireEvent.click(screen.getByRole('checkbox', { name: 'Show global configuration' }));
  const alert = await screen.findByRole('alert');
  expect(alert.textContent).toContain('Global comparison could not be loaded');
  expect(screen.getByRole('link', { name: 'project-helper' })).toBeTruthy();
  await fireEvent.click(within(alert).getByRole('button', { name: 'Retry' }));
  await screen.findByRole('link', { name: 'global-helper' });
  expect(listAgents).toHaveBeenNthCalledWith(3);
  expect(screen.queryByRole('alert')).toBeNull();
});
