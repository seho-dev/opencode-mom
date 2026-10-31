import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { beforeEach, expect, test, vi } from 'vitest';
import { goto } from '$app/navigation';
import { page as routePage } from '$app/state';
import { makeScopedConfig } from '$src/components/__tests__/scope-config.js';
import { en } from '$src/i18n/dictionaries/en.js';
import Harness from '$src/test-utils/harness.svelte';
import type { SlashCommandEntry } from '$src/types/commands.js';
import Commands from '../+page.svelte';
import Detail from '../[id]/+page.svelte';
import NewCommand from '../new/+page.svelte';

vi.mock('$app/navigation', () => ({ goto: vi.fn().mockResolvedValue(undefined) }));
vi.mock('$app/state', async () => {
  const { createSubscriber } = await import('svelte/reactivity');
  let notify = () => {};
  const track = createSubscriber((update) => {
    notify = update;
  });
  let params = { id: 'team/review' };
  let url = new URL('http://localhost/commands');
  return {
    page: {
      get url() {
        track();
        return url;
      },
      set url(value) {
        url = value;
        notify();
      },
      get params() {
        track();
        return params;
      },
      set params(value) {
        params = value;
        notify();
      },
    },
  };
});
const page: { params: { id?: string }; url: URL } = routePage;
const command: SlashCommandEntry = {
  id: 'team/review',
  name: 'team/review',
  description: 'Review changes',
  agent: 'build',
  model: 'provider/model',
  subagent: false,
  sourcePath: '/global/commands/team/review.md',
  resolvedPath: '/resolved/commands/team/review.md',
  content: '---\ndescription: Review\n---\n\n<script>unsafe()</script>\n$ARGUMENTS\n',
};
function config(overrides: Record<string, unknown> = {}, selected: string | null = 'project-a') {
  return makeScopedConfig(
    {
      listSlashCommands: vi.fn().mockResolvedValue({ data: [command], diagnostics: [] }),
      getSlashCommand: vi.fn().mockResolvedValue(command),
      createSlashCommand: vi.fn().mockResolvedValue(command),
      updateSlashCommand: vi.fn().mockResolvedValue(command),
      deleteSlashCommand: vi.fn().mockResolvedValue(undefined),
      reloadOpencode: vi.fn(),
      ...overrides,
    },
    selected,
  );
}
beforeEach(() => {
  page.params = { id: command.id };
  page.url = new URL('http://localhost/commands');
  vi.mocked(goto).mockClear();
});

test('project list is writable and same-name Global comparison remains independently readonly', async () => {
  const store = config();
  render(Commands, {}, { wrapper: Harness, wrapperProps: { config: store } });
  await screen.findByRole('link', { name: `/${command.name}` });
  expect(store.listSlashCommands).toHaveBeenCalledExactlyOnceWith('project-a');
  expect(screen.getByText(en['scope.projectWritable'])).toBeTruthy();
  expect(screen.queryByText(/Manage Markdown command files/)).toBeNull();
  expect(screen.getByRole('link', { name: 'New command' }).getAttribute('href')).toBe(
    '/commands/new?project=project-a',
  );
  expect(screen.getByRole('link', { name: `Edit ${command.name}` }).getAttribute('href')).toBe(
    '/commands/team%2Freview?project=project-a&edit=1',
  );
  await fireEvent.click(screen.getByRole('checkbox', { name: 'Show global configuration' }));
  await waitFor(() => expect(screen.getAllByRole('link', { name: `/${command.name}` })).toHaveLength(2));
  expect(screen.getAllByRole('link', { name: `/${command.name}` }).map((link) => link.getAttribute('href'))).toEqual([
    '/commands/team%2Freview?project=project-a',
    '/commands/team%2Freview?readonly=1',
  ]);
  expect(screen.getAllByRole('link', { name: `Edit ${command.name}` })).toHaveLength(1);
  expect(screen.getByText(en['commands.sameName'])).toBeTruthy();
  expect(screen.queryByText(/Global comparison is read-only/)).toBeNull();
});

test('comparison error preserves project rows and late project responses cannot replace Global scope', async () => {
  let resolve!: (value: { data: SlashCommandEntry[]; diagnostics: string[] }) => void;
  const listSlashCommands = vi
    .fn()
    .mockImplementationOnce(
      () =>
        new Promise((done) => {
          resolve = done;
        }),
    )
    .mockResolvedValueOnce({ data: [{ ...command, name: 'global' }], diagnostics: [] });
  const store = config({ listSlashCommands });
  render(Commands, {}, { wrapper: Harness, wrapperProps: { config: store } });
  await waitFor(() => expect(listSlashCommands).toHaveBeenCalledWith('project-a'));
  await fireEvent.click(screen.getByRole('button', { name: 'Return to Global' }));
  await screen.findByRole('link', { name: '/global' });
  resolve({ data: [command], diagnostics: ['late'] });
  await waitFor(() => expect(screen.queryByRole('link', { name: `/${command.name}` })).toBeNull());
  expect(screen.queryByText('late')).toBeNull();
});

test('new routes write explicit Global null even when the shared selector points at a project', async () => {
  page.url = new URL('http://localhost/commands/new');
  const store = config();
  render(NewCommand, {}, { wrapper: Harness, wrapperProps: { config: store } });
  const content = '---\ndescription: A\nagent: plan\nmodel: x/y\nsubagent: true\n---\n\n$1 $ARGUMENTS';
  await fireEvent.input(screen.getByLabelText('Command name'), { target: { value: 'team/review' } });
  await fireEvent.input(screen.getByLabelText('Full Markdown document'), { target: { value: content } });
  await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
  await waitFor(() =>
    expect(store.createSlashCommand).toHaveBeenCalledExactlyOnceWith({ name: 'team/review', content }, null),
  );
  expect(store.selectProject).not.toHaveBeenCalled();
  expect(store.reloadOpencode).not.toHaveBeenCalled();
});

test('detail previews raw Markdown safely, edit freezes snapshot and conflict keeps the complete draft', async () => {
  page.url = new URL('http://localhost/commands/team%2Freview?project=project-b');
  const updateSlashCommand = vi.fn().mockRejectedValue(new Error('changed externally'));
  const store = config({ updateSlashCommand });
  const { container } = render(Detail, {}, { wrapper: Harness, wrapperProps: { config: store } });
  await screen.findByRole('heading', { name: `/${command.name}` });
  expect(container.querySelector('pre')?.textContent).toBe(command.content);
  expect(container.querySelector('script')).toBeNull();
  expect(store.getSlashCommand).toHaveBeenCalledExactlyOnceWith(command.id, 'project-b');
  await fireEvent.click(screen.getByRole('button', { name: 'Edit' }));
  expect(screen.getAllByText(en['scope.projectWritable'])).toHaveLength(1);
  expect(screen.getByText(en['commands.contentHint'], { exact: false })).toBeTruthy();
  expect(screen.queryByText(/Referenced agents and models must/)).toBeNull();
  const input = screen.getByLabelText('Full Markdown document') as HTMLTextAreaElement;
  await fireEvent.input(input, { target: { value: '' } });
  await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
  await screen.findByText(/changed externally/);
  expect(input.value).toBe('');
  expect(updateSlashCommand).toHaveBeenCalledExactlyOnceWith(
    { id: command.id, content: '', expectedPath: command.resolvedPath, expectedContent: command.content },
    'project-b',
  );
});

test('readonly comparison ignores edit query; delete uses the untouched expected source and route scope', async () => {
  page.url = new URL('http://localhost/commands/team%2Freview?readonly=1&edit=1');
  const store = config();
  const mounted = render(Detail, {}, { wrapper: Harness, wrapperProps: { config: store } });
  await screen.findByRole('heading', { name: `/${command.name}` });
  expect(screen.queryByRole('button', { name: 'Edit' })).toBeNull();
  expect(screen.queryByRole('button', { name: 'Delete' })).toBeNull();
  expect(screen.queryByRole('textbox')).toBeNull();
  mounted.unmount();
  page.url = new URL('http://localhost/commands/team%2Freview?project=project-b');
  render(Detail, {}, { wrapper: Harness, wrapperProps: { config: store } });
  await screen.findByRole('heading', { name: `/${command.name}` });
  await fireEvent.click(screen.getByRole('button', { name: 'Delete' }));
  await fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'Delete' }));
  await waitFor(() =>
    expect(store.deleteSlashCommand).toHaveBeenCalledExactlyOnceWith(
      { id: command.id, expectedPath: command.resolvedPath, expectedContent: command.content },
      'project-b',
    ),
  );
});

test('same ID scope changes ignore stale source completion', async () => {
  let resolve!: (value: SlashCommandEntry) => void;
  const getSlashCommand = vi
    .fn()
    .mockImplementationOnce(
      () =>
        new Promise((done) => {
          resolve = done;
        }),
    )
    .mockResolvedValueOnce({ ...command, content: 'Project B', sourcePath: '/b/review.md' });
  page.url = new URL('http://localhost/commands/team%2Freview?project=project-a');
  render(Detail, {}, { wrapper: Harness, wrapperProps: { config: config({ getSlashCommand }) } });
  await waitFor(() => expect(getSlashCommand).toHaveBeenCalledOnce());
  page.url = new URL('http://localhost/commands/team%2Freview?project=project-b');
  await screen.findByText('Project B');
  resolve(command);
  await waitFor(() => expect(screen.queryByText(command.sourcePath)).toBeNull());
  expect(getSlashCommand).toHaveBeenLastCalledWith(command.id, 'project-b');
});

test('Global comparison errors stay isolated from writable project list', async () => {
  const listSlashCommands = vi
    .fn()
    .mockResolvedValueOnce({ data: [command], diagnostics: [] })
    .mockRejectedValueOnce(new Error('Global denied'))
    .mockResolvedValueOnce({ data: [], diagnostics: [] });
  render(Commands, {}, { wrapper: Harness, wrapperProps: { config: config({ listSlashCommands }) } });
  await screen.findByRole('link', { name: `/${command.name}` });
  await fireEvent.click(screen.getByRole('checkbox', { name: 'Show global configuration' }));
  const alert = await screen.findByRole('alert');
  expect(alert.textContent).toContain('Global denied');
  expect(screen.getByRole('link', { name: `/${command.name}` })).toBeTruthy();
  expect(screen.getByRole('link', { name: `Edit ${command.name}` })).toBeTruthy();
  await fireEvent.click(within(alert).getByRole('button', { name: 'Retry' }));
  await waitFor(() => expect(screen.queryByRole('alert')).toBeNull());
  expect(listSlashCommands).toHaveBeenLastCalledWith(null);
});

test('route change destroys a pending editor so a late saved source cannot replace new detail', async () => {
  let resolve!: (value: SlashCommandEntry) => void;
  const updateSlashCommand = vi.fn().mockImplementation(
    () =>
      new Promise((done) => {
        resolve = done;
      }),
  );
  const getSlashCommand = vi
    .fn()
    .mockResolvedValueOnce(command)
    .mockResolvedValueOnce({ ...command, id: 'other', name: 'other', content: 'Other content' });
  render(Detail, {}, { wrapper: Harness, wrapperProps: { config: config({ getSlashCommand, updateSlashCommand }) } });
  await screen.findByRole('heading', { name: `/${command.name}` });
  await fireEvent.click(screen.getByRole('button', { name: 'Edit' }));
  await fireEvent.click(screen.getByRole('button', { name: 'Save' }));
  page.params = { id: 'other' };
  await screen.findByText('Other content');
  resolve(command);
  await waitFor(() => expect(screen.queryByRole('heading', { name: `/${command.name}` })).toBeNull());
  expect(goto).not.toHaveBeenCalled();
});
