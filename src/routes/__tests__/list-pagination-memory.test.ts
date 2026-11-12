import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { createSubscriber } from 'svelte/reactivity';
import { expect, test, vi } from 'vitest';
import { makeScopedConfig } from '$src/components/__tests__/scope-config.js';
import type { ConfigStore } from '$src/config/store.svelte.js';
import { en } from '$src/i18n/dictionaries/en.js';
import Harness from '$src/test-utils/harness.svelte';
import type { SlashCommandEntry } from '$src/types/commands.js';
import type { ProviderDef } from '$src/types/providers.js';
import Commands from '../commands/+page.svelte';
import Providers from '../providers/+page.svelte';

vi.mock('$app/state', () => ({
  page: { url: new URL('http://localhost/commands') },
}));

// Minimal reactive signal so a test can mutate store-like inputs after mount.
function signal<T>(initial: T) {
  let value = initial;
  let notify = () => {};
  const track = createSubscriber((update) => {
    notify = update;
  });

  return {
    get value() {
      track();
      return value;
    },
    set next(next: T) {
      value = next;
      notify();
    },
  };
}

function providerConfig(names: string[]) {
  const providers = signal<ProviderDef[]>(names.map((name) => ({ name, models: {} })));

  return {
    preferences: { locale: 'en', theme: 'light' },
    get providers() {
      return providers.value;
    },
    loading: false,
    saving: false,
    error: null,
    deleteProvider: vi.fn(async (name: string) => {
      providers.next = providers.value.filter((provider) => provider.name !== name);
    }),
    addProvider(name: string) {
      providers.next = [...providers.value, { name, models: {} }];
    },
    refresh: vi.fn(async () => {}),
  };
}

function commandRows(prefix: string, count: number): SlashCommandEntry[] {
  return Array.from({ length: count }, (_, index) => {
    const name = `${prefix}-${String(index + 1).padStart(2, '0')}`;
    const path = `/tmp/commands/${name}.md`;

    return {
      id: name,
      name,
      description: '',
      sourcePath: path,
      resolvedPath: path,
      content: '',
    };
  });
}

function mountProviders(config: object) {
  return render(Providers, {}, { wrapper: Harness, wrapperProps: { config: config as ConfigStore } });
}

function mountCommands(config: object) {
  return render(Commands, {}, { wrapper: Harness, wrapperProps: { config: config as ConfigStore } });
}

test('leaving the list and returning restores the remembered page', async () => {
  const config = providerConfig(Array.from({ length: 12 }, (_, index) => `prov-${String(index + 1).padStart(2, '0')}`));
  const first = mountProviders(config);

  await fireEvent.click(screen.getByRole('button', { name: en['common.nextPage'] }));
  expect(screen.getByText(/Page 2 of 3/)).toBeTruthy();
  expect(screen.getByText('prov-06')).toBeTruthy();

  first.unmount();
  mountProviders(config);

  await waitFor(() => expect(screen.getByText('prov-06')).toBeTruthy());
  expect(screen.getByText(/Page 2 of 3/)).toBeTruthy();
});

test('search changes reset to the first page while the remembered page stays writable', async () => {
  const config = providerConfig(Array.from({ length: 25 }, (_, index) => `prov-${String(index + 1).padStart(2, '0')}`));
  mountProviders(config);

  await fireEvent.click(screen.getByRole('button', { name: en['common.nextPage'] }));
  expect(screen.getByText(/Page 2 of 5/)).toBeTruthy();

  const search = screen.getByRole('textbox', { name: en['providers.searchLabel'] });

  await fireEvent.input(search, { target: { value: 'prov' } });
  await waitFor(() => expect(screen.getByText(/Page 1 of 5/)).toBeTruthy());
  expect(screen.getByText('prov-01')).toBeTruthy();

  await fireEvent.input(search, { target: { value: '' } });
  await waitFor(() => expect(screen.getByText(/Page 1 of 5/)).toBeTruthy());
  expect(screen.queryByText(/Page 2 of 5/)).toBeNull();
});

test('deleting the last row of the last page steps back and does not jump forward again when a row is added', async () => {
  const config = providerConfig(Array.from({ length: 11 }, (_, index) => `prov-${String(index + 1).padStart(2, '0')}`));
  mountProviders(config);

  await fireEvent.click(screen.getByRole('button', { name: en['common.nextPage'] }));
  await fireEvent.click(screen.getByRole('button', { name: en['common.nextPage'] }));
  expect(screen.getByText(/Page 3 of 3/)).toBeTruthy();
  expect(screen.getByText('prov-11')).toBeTruthy();

  await fireEvent.click(screen.getByRole('button', { name: en['providers.deleteAria'].replace('{name}', 'prov-11') }));
  await fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: en['common.delete'] }));

  await waitFor(() => expect(screen.queryByText('prov-11')).toBeNull());
  await waitFor(() => expect(screen.getByText(/Page 2 of 2/)).toBeTruthy());
  expect(screen.getByText('prov-10')).toBeTruthy();

  config.addProvider('prov-12');
  await waitFor(() => expect(screen.getByText(/Page 2 of 3/)).toBeTruthy());
  expect(screen.getByText('prov-06')).toBeTruthy();
  expect(screen.queryByText('prov-12')).toBeNull();
});

test('an async load keeps the remembered page instead of resetting when rows arrive', async () => {
  const rows = commandRows('cmd', 15);
  const listSlashCommands = vi.fn();
  const config = makeScopedConfig({ listSlashCommands }, 'project-async');

  listSlashCommands.mockResolvedValueOnce({ data: rows, diagnostics: [] });
  const first = mountCommands(config);
  await screen.findByRole('link', { name: '/cmd-01' });
  await fireEvent.click(screen.getByRole('button', { name: en['common.nextPage'] }));
  expect(screen.getByText(/Page 2 of 3/)).toBeTruthy();

  const pending = Promise.withResolvers<{ data: SlashCommandEntry[]; diagnostics: string[] }>();

  listSlashCommands.mockReturnValueOnce(pending.promise);
  first.unmount();
  mountCommands(config);

  pending.resolve({ data: rows, diagnostics: [] });
  await screen.findByRole('link', { name: '/cmd-06' });
  expect(screen.getByText(/Page 2 of 3/)).toBeTruthy();
});

test('different lists sharing one store keep independent remembered pages', async () => {
  const rows = commandRows('cmd', 15);
  const config = {
    ...makeScopedConfig({ listSlashCommands: vi.fn(async () => ({ data: rows, diagnostics: [] })) }, null),
    providers: Array.from({ length: 15 }, (_, index) => ({
      name: `prov-${String(index + 1).padStart(2, '0')}`,
      models: {},
    })),
  };

  const providers = mountProviders(config);
  await fireEvent.click(screen.getByRole('button', { name: en['common.nextPage'] }));
  expect(screen.getByText(/Page 2 of 3/)).toBeTruthy();
  providers.unmount();

  mountCommands(config);

  await screen.findByRole('link', { name: '/cmd-01' });
  expect(screen.getAllByText(/Page 1 of 3/)).toHaveLength(1);
  expect(screen.queryByText(/Page 2 of 3/)).toBeNull();
});

test('different project scopes keep independent remembered pages', async () => {
  const projectRows = commandRows('alpha', 15);
  const otherRows = commandRows('beta', 15);
  const listSlashCommands = vi.fn(async (scope: string | null) => ({
    data: scope === 'project-b' ? otherRows : projectRows,
    diagnostics: [],
  }));
  const config = makeScopedConfig({ listSlashCommands }, 'project-a');
  mountCommands(config);

  await screen.findByRole('link', { name: '/alpha-01' });
  await fireEvent.click(screen.getByRole('button', { name: en['common.nextPage'] }));
  expect(screen.getByText(/Page 2 of 3/)).toBeTruthy();

  await config.selectProject('project-b');
  await screen.findByRole('link', { name: '/beta-01' });
  expect(screen.getByText(/Page 1 of 3/)).toBeTruthy();

  await fireEvent.click(screen.getByRole('button', { name: en['common.nextPage'] }));
  expect(screen.getByText(/Page 2 of 3/)).toBeTruthy();

  await config.selectProject('project-a');
  await screen.findByRole('link', { name: '/alpha-06' });
  expect(screen.getByText(/Page 2 of 3/)).toBeTruthy();
});
