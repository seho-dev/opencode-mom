import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { expect, test, vi } from 'vitest';
import type { ConfigStore } from '$src/config/store.svelte.js';
import Harness from '$src/test-utils/harness.svelte';
import { GROUP_TYPE_NATIVE } from '$src/utils/constants.js';
import AgentList from '../agents/+page.svelte';
import GroupList from '../groups/+page.svelte';
import ModelList from '../models/+page.svelte';
import ProviderList from '../providers/+page.svelte';

const lists = [
  { name: 'Providers', component: ProviderList, title: 'Provider Registry', row: 'example', path: 'providers' },
  { name: 'Models', component: ModelList, title: 'Models', row: 'Custom model', path: 'models' },
  { name: 'Agents', component: AgentList, title: 'Agents', row: 'helper', path: 'agents' },
  { name: 'Groups', component: GroupList, title: 'Groups', row: 'Example group', path: 'groups' },
];

function mount(list: (typeof lists)[number], flags: { loading?: boolean; saving?: boolean } = {}) {
  const unrelated = Object.fromEntries(
    [
      'reloadOpencode',
      'refreshAll',
      'discardDraftAndRefresh',
      'continueEditing',
      'createProvider',
      'updateProvider',
      'deleteProvider',
      'createModel',
      'updateModel',
      'deleteModel',
      'createAgent',
      'updateAgent',
      'deleteAgent',
      'saveGroup',
      'deleteGroup',
      'switchGroup',
      'loadCatalog',
      'loadTokenUsageRecords',
      'setTheme',
      'setLocale',
      'setAutostart',
    ].map((name) => [name, vi.fn()]),
  );
  const error = { code: 'conflict' as const, message: 'Keep the existing draft error' };
  const draftRecovery = { operation: 'updateProvider', payload: { name: 'draft' }, error, conflict: true };
  const config = {
    ...unrelated,
    preferences: { locale: 'en', theme: 'light' },
    get providers() {
      return [{ name: 'example', models: {} }];
    },
    get catalog() {
      return [{ ref: 'example/custom', providerId: 'example', name: 'Custom model', isCustom: true }];
    },
    get agents() {
      return [{ id: 'helper', source: 'inline', description: 'Custom helper' }];
    },
    get groups() {
      return [
        {
          id: 'example',
          name: 'Example group',
          description: '',
          type: GROUP_TYPE_NATIVE,
          openCodeAgentOverrides: [],
          isEnabled: false,
        },
      ];
    },
    loading: flags.loading ?? false,
    saving: flags.saving ?? false,
    catalogLoading: false,
    catalogError: null,
    switching: false,
    reloading: false,
    splashLoading: false,
    error,
    draftRecovery,
    formResetVersion: 7,
    refresh: vi.fn().mockResolvedValue(undefined),
  };
  render(list.component, {}, { wrapper: Harness, wrapperProps: { config: config as unknown as ConfigStore } });
  return { config, unrelated, button: screen.getByRole('button', { name: 'Refresh' }) as HTMLButtonElement };
}

test.each(lists)('$name refresh reads the central store and stays disabled for the full request', async (list) => {
  const { config, unrelated, button } = mount(list);
  let finish!: () => void;
  config.refresh.mockReturnValueOnce(
    new Promise<void>((resolve) => {
      finish = resolve;
    }),
  );
  const draft = config.draftRecovery;
  const error = config.error;

  expect(screen.getByRole('heading', { level: 1, name: list.title })).toBeTruthy();
  expect(screen.getByRole('table')).toBeTruthy();
  expect(screen.getByText(list.row)).toBeTruthy();
  expect(screen.getByRole('link', { name: /^New/ }).getAttribute('href')).toBe(`/${list.path}/new`);
  expect(button.disabled).toBe(false);
  expect(button.tabIndex).toBe(0);
  expect(button.querySelector('svg')?.getAttribute('aria-hidden')).toBe('true');

  await fireEvent.click(button);
  expect(config.refresh).toHaveBeenCalledExactlyOnceWith(true);
  expect(button.disabled).toBe(true);
  expect(button.getAttribute('aria-busy')).toBe('true');
  await fireEvent.click(button);
  expect(config.refresh).toHaveBeenCalledOnce();

  finish();
  await waitFor(() => expect(button.disabled).toBe(false));
  expect(button.getAttribute('aria-busy')).toBe('false');
  expect(config.draftRecovery).toBe(draft);
  expect(config.error).toBe(error);
  expect(config.formResetVersion).toBe(7);
  expect(config.splashLoading).toBe(false);
  expect(screen.getByText(list.row)).toBeTruthy();
  for (const action of Object.values(unrelated)) expect(action).not.toHaveBeenCalled();
});

test.each(lists.flatMap((list) => ['loading', 'saving'].map((flag) => ({ ...list, flag }))))(
  '$name refresh cannot run while $flag',
  async (list) => {
    const { config, button } = mount(list, { [list.flag]: true });
    expect(button.disabled).toBe(true);
    await fireEvent.click(button);
    expect(config.refresh).not.toHaveBeenCalled();
  },
);
