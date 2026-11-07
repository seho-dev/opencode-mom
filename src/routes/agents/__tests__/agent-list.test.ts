import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { tick } from 'svelte';
import { afterAll, beforeAll, expect, test, vi } from 'vitest';
import { makeScopedConfig } from '$src/components/__tests__/scope-config.js';
import type { CommandAdapter } from '$src/config/adapter.js';
import { type ConfigStore, createConfigStore } from '$src/config/store.svelte.js';
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

test('agent list query trims leading and trailing whitespace', async () => {
  const config = {
    preferences: { locale: 'en', theme: 'light' },
    agents: [
      { id: 'helper', source: 'inline', description: 'project helper' },
      { id: 'other', source: 'inline', description: 'unrelated' },
    ],
    loading: false,
    selectedProjectId: null,
  } as unknown as ConfigStore;

  render(AgentList, {}, { wrapper: Harness, wrapperProps: { config } });

  const searchInput = screen.getByRole('textbox', { name: 'Search agents' });

  await fireEvent.input(searchInput, { target: { value: '   helper   ' } });
  expect(screen.getByRole('link', { name: 'Edit helper' })).toBeTruthy();
  expect(screen.queryByRole('link', { name: 'Edit other' })).toBeNull();
});
test('tablist arrows and Home/End follow the runtime tab order and keep refs on the same tabs', async () => {
  const config = createConfigStore({} as unknown as CommandAdapter, false, false);

  config.applyPreferences({ locale: 'en', theme: 'light', defaultPlugin: 'native' });
  render(AgentList, {}, { wrapper: Harness, wrapperProps: { config } });

  const custom = screen.getByRole('tab', { name: /Custom/ });
  const native = screen.getByRole('tab', { name: /Native/ });
  const slim = screen.getByRole('tab', { name: /Slim/ });
  const omo = screen.getByRole('tab', { name: /OMO/ });

  custom.focus();

  // Initial order has Custom first, followed by defaultPlugin ('native') order: [custom, native, slim, omo].
  expect(screen.getAllByRole('tab')).toEqual([custom, native, slim, omo]);
  expect(custom.getAttribute('tabindex')).toBe('0');
  expect(native.getAttribute('tabindex')).toBe('-1');
  expect(custom.getAttribute('aria-selected')).toBe('true');
  expect(screen.getByRole('tabpanel').id).toBe('agents-tabpanel-custom');

  // ArrowRight moves from custom to native.
  await fireEvent.keyDown(custom, { key: 'ArrowRight' });
  expect(document.activeElement).toBe(native);
  expect(native.getAttribute('aria-selected')).toBe('true');
  expect(native.getAttribute('tabindex')).toBe('0');
  expect(custom.getAttribute('aria-selected')).toBe('false');
  expect(custom.getAttribute('tabindex')).toBe('-1');
  expect(screen.getByRole('tabpanel').getAttribute('aria-labelledby')).toBe('agents-tab-native');

  // End moves to omo (last).
  await fireEvent.keyDown(native, { key: 'End' });
  expect(document.activeElement).toBe(omo);
  expect(omo.getAttribute('aria-selected')).toBe('true');
  expect(screen.getByRole('tabpanel').getAttribute('aria-labelledby')).toBe('agents-tab-omo');

  // Home moves back to custom (first).
  await fireEvent.keyDown(omo, { key: 'Home' });
  expect(document.activeElement).toBe(custom);
  expect(custom.getAttribute('aria-selected')).toBe('true');
  expect(screen.getByRole('tabpanel').id).toBe('agents-tabpanel-custom');

  // ArrowLeft from the first entry (custom) wraps to the last one (omo).
  await fireEvent.keyDown(custom, { key: 'ArrowLeft' });
  expect(document.activeElement).toBe(omo);
  expect(omo.getAttribute('aria-selected')).toBe('true');
  expect(screen.getByRole('tabpanel').getAttribute('aria-labelledby')).toBe('agents-tab-omo');

  // ArrowRight from the last entry (omo) wraps back to the first one (custom).
  await fireEvent.keyDown(omo, { key: 'ArrowRight' });
  expect(document.activeElement).toBe(custom);
  expect(custom.getAttribute('aria-selected')).toBe('true');
  expect(screen.getByRole('tabpanel').id).toBe('agents-tabpanel-custom');

  // Changing runtime preference reorders builtin tabs while custom stays first: [custom, omo, native, slim].
  config.applyPreferences({ ...config.preferences, defaultPlugin: 'omo' });
  await tick();
  expect(screen.getAllByRole('tab')).toEqual([custom, omo, native, slim]);
  expect(screen.getByRole('tab', { name: /Custom/ })).toBe(custom);
  expect(screen.getByRole('tab', { name: /Native/ })).toBe(native);
  expect(screen.getByRole('tab', { name: /Slim/ })).toBe(slim);
  expect(screen.getByRole('tab', { name: /OMO/ })).toBe(omo);

  // Keyboard navigation on reordered tabs: ArrowRight from custom walks into omo.
  await fireEvent.keyDown(custom, { key: 'ArrowRight' });
  expect(document.activeElement).toBe(omo);
  expect(omo.getAttribute('aria-selected')).toBe('true');
  expect(screen.getByRole('tabpanel').getAttribute('aria-labelledby')).toBe('agents-tab-omo');

  await fireEvent.keyDown(omo, { key: 'ArrowRight' });
  expect(document.activeElement).toBe(native);
  expect(native.getAttribute('aria-selected')).toBe('true');
  expect(screen.getByRole('tabpanel').getAttribute('aria-labelledby')).toBe('agents-tab-native');

  // End moves to slim (last).
  await fireEvent.keyDown(native, { key: 'End' });
  expect(document.activeElement).toBe(slim);
  expect(slim.getAttribute('aria-selected')).toBe('true');
  expect(screen.getByRole('tabpanel').getAttribute('aria-labelledby')).toBe('agents-tab-slim');

  // ArrowRight wraps from slim to custom.
  await fireEvent.keyDown(slim, { key: 'ArrowRight' });
  expect(document.activeElement).toBe(custom);
  expect(custom.getAttribute('aria-selected')).toBe('true');
  expect(screen.getByRole('tabpanel').id).toBe('agents-tabpanel-custom');

  // ArrowLeft wraps from custom to slim.
  await fireEvent.keyDown(custom, { key: 'ArrowLeft' });
  expect(document.activeElement).toBe(slim);
  expect(slim.getAttribute('aria-selected')).toBe('true');
  expect(screen.getByRole('tabpanel').getAttribute('aria-labelledby')).toBe('agents-tab-slim');
});
