import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterAll, beforeAll, expect, test, vi } from 'vitest';
import Harness from '$src/test-utils/harness.svelte';
import ResourceScope from '../ResourceScope.svelte';
import { makeScopedConfig } from './scope-config.js';

const originalScroll = HTMLElement.prototype.scrollIntoView;
beforeAll(() => {
  HTMLElement.prototype.scrollIntoView = vi.fn();
});
afterAll(() => {
  if (originalScroll) HTMLElement.prototype.scrollIntoView = originalScroll;
  else Reflect.deleteProperty(HTMLElement.prototype, 'scrollIntoView');
});

test('scope picker keeps Global first, searches paths, supports keyboard and retains confirmed selection after failure', async () => {
  const selectProject = vi.fn().mockRejectedValue(new Error('save denied'));
  const config = makeScopedConfig({ selectProject });
  render(ResourceScope, {}, { wrapper: Harness, wrapperProps: { config } });
  expect(screen.getByText('/work/alpha')).toBeTruthy();
  const trigger = screen.getByRole('button', { name: 'Select configuration scope' });
  await fireEvent.keyDown(trigger, { key: 'ArrowDown' });
  const options = await screen.findAllByRole('option');
  expect(options[0]?.textContent).toContain('Global');
  expect(options[1]?.textContent).toContain('/work/alpha');
  const search = screen.getByRole('combobox', { name: 'Search options' });
  await fireEvent.input(search, { target: { value: '/other' } });
  expect(screen.getAllByRole('option')).toHaveLength(2);
  await fireEvent.keyDown(search, { key: 'ArrowDown' });
  await fireEvent.keyDown(search, { key: 'Enter' });
  await screen.findByRole('alert');
  expect(selectProject).toHaveBeenCalledExactlyOnceWith('project-b');
  expect(config.selectedProjectId).toBe('project-a');
  expect(trigger.textContent).toContain('alpha');
  expect(screen.getByText('/work/alpha')).toBeTruthy();
  expect(screen.queryByText('/other/alpha')).toBeNull();
});

test('native picker cancellation makes no change; add selects returned folder and duplicate selects existing', async () => {
  const pickProjectFolder = vi
    .fn()
    .mockResolvedValueOnce(null)
    .mockResolvedValueOnce('/new/folder')
    .mockResolvedValueOnce('/work/alpha');
  const config = makeScopedConfig({ pickProjectFolder }, null);
  render(ResourceScope, {}, { wrapper: Harness, wrapperProps: { config } });
  await fireEvent.click(screen.getByRole('button', { name: 'Add folder' }));
  expect(config.addProject).not.toHaveBeenCalled();
  expect(config.selectedProjectId).toBeNull();
  await fireEvent.click(screen.getByRole('button', { name: 'Add folder' }));
  await screen.findByText('/new/folder');
  expect(config.addProject).toHaveBeenCalledWith('/new/folder');
  await fireEvent.click(screen.getByRole('button', { name: 'Add folder' }));
  await screen.findByText('/work/alpha');
  expect(config.projects.filter((project) => project.path === '/work/alpha')).toHaveLength(1);
  await fireEvent.click(screen.getByRole('button', { name: 'Return to Global' }));
  await waitFor(() => expect(config.selectedProjectId).toBeNull());
});

test('manage only forgets folders, prevents pending dismissal/removal, and returns Global when current folder is removed', async () => {
  let resolve = () => {};
  const config = makeScopedConfig();
  const original = config.removeProject;
  config.removeProject = vi.fn(async (id: string) => {
    await new Promise<void>((done) => {
      resolve = done;
    });
    await original(id);
  });
  render(ResourceScope, {}, { wrapper: Harness, wrapperProps: { config } });
  await fireEvent.click(screen.getByRole('button', { name: 'Manage folders' }));
  const dialog = await screen.findByRole('dialog');
  expect(within(dialog).getByText(/does not delete or change any files/)).toBeTruthy();
  await fireEvent.click(within(dialog).getByRole('button', { name: 'Remove folder /work/alpha from MOM' }));
  expect(
    (within(dialog).getByRole('button', { name: 'Remove folder /other/alpha from MOM' }) as HTMLButtonElement).disabled,
  ).toBe(true);
  await fireEvent.keyDown(dialog, { key: 'Escape' });
  expect(screen.getByRole('dialog')).toBeTruthy();
  resolve();
  await waitFor(() => expect(config.selectedProjectId).toBeNull());
  expect(config.removeProject).toHaveBeenCalledExactlyOnceWith('project-a');
  await fireEvent.click(within(dialog).getByRole('button', { name: 'Close' }));
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
});
