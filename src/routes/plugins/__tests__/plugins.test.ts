import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { tick } from 'svelte';
import { afterAll, beforeAll, expect, test, vi } from 'vitest';
import { makeScopedConfig } from '$src/components/__tests__/scope-config.js';
import { en } from '$src/i18n/dictionaries/en.js';
import Harness from '$src/test-utils/harness.svelte';
import type { PluginConfigPreview, PluginRemovalPlan, PluginRuntime, PluginRuntimeItem } from '$src/types/plugins.js';
import Plugins from '../+page.svelte';

const originalScroll = HTMLElement.prototype.scrollIntoView;
beforeAll(() => {
  HTMLElement.prototype.scrollIntoView = vi.fn();
});
afterAll(() => {
  if (originalScroll) HTMLElement.prototype.scrollIntoView = originalScroll;
  else Reflect.deleteProperty(HTMLElement.prototype, 'scrollIntoView');
});

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: Error) => void;
  const promise = new Promise<T>((done, fail) => {
    resolve = done;
    reject = fail;
  });
  return { promise, resolve, reject };
}
function packageItem(target = 'example-plugin', canUpdate = true): PluginRuntimeItem {
  return {
    source: { type: 'package', target, version: '1.0.0', outdated: true },
    features: { server: true },
    state: { status: 'active' },
    canUpdate,
    canRemove: true,
  };
}
function runtime(data: PluginRuntimeItem[] = [packageItem()], directory = '/service/location'): PluginRuntime {
  return { directory, data };
}
function config(overrides: Record<string, unknown> = {}, selected: string | null = null) {
  const obsolete = {
    listPlugins: vi.fn(),
    getPlugin: vi.fn(),
    createPlugin: vi.fn(),
    updatePlugin: vi.fn(),
    deletePlugin: vi.fn(),
  };
  const store = makeScopedConfig(
    {
      ...obsolete,
      serviceTargetVersion: 0,
      listPluginRuntime: vi.fn().mockResolvedValue(runtime()),
      getPluginConfigSources: vi.fn().mockResolvedValue({ directory: '/service/location', data: [] }),
      updatePluginPackage: vi.fn().mockResolvedValue(undefined),
      preparePluginRemoval: vi.fn(),
      removePluginDeclaration: vi.fn().mockResolvedValue(undefined),
      reloadOpencode: vi.fn(),
      ...overrides,
    },
    selected,
  );
  return { store, obsolete };
}
function show(store: ReturnType<typeof config>['store']) {
  return render(Plugins, {}, { wrapper: Harness, wrapperProps: { config: store } });
}

function removalPlan(): PluginRemovalPlan {
  return {
    target: { type: 'package', target: 'example-plugin' },
    sources: [
      {
        path: '/global/opencode.jsonc',
        expectedPath: '/resolved/opencode.jsonc',
        expectedHash: 'private-snapshot-hash',
      },
    ],
    serviceKey: 'original-service-key',
  };
}

test('Delete prepares Global paths, Cancel writes nothing, and confirmation echoes the complete fresh plan', async () => {
  const cancelled = removalPlan();
  const confirmed = { ...removalPlan(), serviceKey: 'fresh-service-key' };
  const { store } = config({
    preparePluginRemoval: vi.fn().mockResolvedValueOnce(cancelled).mockResolvedValueOnce(confirmed),
    listPluginRuntime: vi.fn().mockResolvedValueOnce(runtime()).mockResolvedValue(runtime([])),
  });
  show(store);
  await screen.findByText('example-plugin');
  await fireEvent.click(screen.getByRole('button', { name: en['plugins.configPreview'] }));
  await screen.findByText(en['plugins.configEmpty']);
  await fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: en['common.close'] }));
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  await fireEvent.click(screen.getByRole('button', { name: en['common.delete'] }));
  let dialog = await screen.findByRole('dialog');
  expect(within(dialog).getByText('example-plugin')).toBeTruthy();
  expect(within(dialog).getByText('/global/opencode.jsonc')).toBeTruthy();
  expect(within(dialog).getByText(en['plugins.removalConfirm'])).toBeTruthy();
  expect(dialog.textContent).not.toContain('private-snapshot-hash');
  expect(dialog.textContent).not.toContain('/resolved/opencode.jsonc');
  expect(store.preparePluginRemoval).toHaveBeenCalledExactlyOnceWith(cancelled.target);
  expect(store.removePluginDeclaration).not.toHaveBeenCalled();
  await fireEvent.click(within(dialog).getByRole('button', { name: en['common.cancel'] }));
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect(store.removePluginDeclaration).not.toHaveBeenCalled();
  await fireEvent.click(screen.getByRole('button', { name: en['common.delete'] }));
  dialog = await screen.findByRole('dialog');
  await fireEvent.click(within(dialog).getByRole('button', { name: en['common.delete'] }));
  await screen.findByText(en['plugins.declarationRemoved']);
  await screen.findByText(en['plugins.runtimeEmpty']);
  expect(store.preparePluginRemoval).toHaveBeenCalledTimes(2);
  expect(store.removePluginDeclaration).toHaveBeenCalledExactlyOnceWith(confirmed);
  expect(vi.mocked(store.removePluginDeclaration).mock.calls[0]?.[0]).toBe(confirmed);
  expect(store.listPluginRuntime).toHaveBeenCalledTimes(2);
  expect(store.updatePluginPackage).not.toHaveBeenCalled();
  expect(store.reloadOpencode).not.toHaveBeenCalled();
  expect(screen.queryByText(en['plugins.packageUpdated'])).toBeNull();
  await fireEvent.click(screen.getByRole('button', { name: en['plugins.configPreview'] }));
  await screen.findByText(en['plugins.configEmpty']);
  expect(store.getPluginConfigSources).toHaveBeenCalledTimes(2);
});

test('Delete obeys canRemove for local files and prepares again after prepare or stale snapshot failure without retries', async () => {
  const target = { type: 'local', path: '/global/declared.ts' } as const;
  const failed = { ...removalPlan(), target };
  const fresh = { ...failed, serviceKey: 'fresh-plan' };
  const items: PluginRuntimeItem[] = [
    { source: target, state: { status: 'active' }, features: {}, canUpdate: false, canRemove: true },
    {
      source: { type: 'local', path: '/global/auto-loaded.ts' },
      state: { status: 'active' },
      features: {},
      canUpdate: false,
      canRemove: false,
    },
  ];
  const { store } = config({
    listPluginRuntime: vi.fn().mockResolvedValue(runtime(items)),
    preparePluginRemoval: vi
      .fn()
      .mockRejectedValueOnce(new Error('private prepare error'))
      .mockResolvedValueOnce(failed)
      .mockResolvedValueOnce(fresh),
    removePluginDeclaration: vi
      .fn()
      .mockRejectedValueOnce(new Error('external snapshot changed'))
      .mockResolvedValue(undefined),
  });
  show(store);
  await screen.findByText(target.path);
  const disabled = within(screen.getByText('/global/auto-loaded.ts').closest('tr') as HTMLTableRowElement).getByRole(
    'button',
    { name: en['common.delete'] },
  ) as HTMLButtonElement;
  expect(disabled.disabled).toBe(true);
  await fireEvent.click(disabled);
  expect(store.preparePluginRemoval).not.toHaveBeenCalled();
  const button = within(screen.getByText(target.path).closest('tr') as HTMLTableRowElement).getByRole('button', {
    name: en['common.delete'],
  });
  await fireEvent.click(button);
  await screen.findByText(en['plugins.removalPrepareFailed']);
  expect(screen.queryByRole('dialog')).toBeNull();
  expect(store.preparePluginRemoval).toHaveBeenCalledExactlyOnceWith(target);
  expect(store.removePluginDeclaration).not.toHaveBeenCalled();
  expect(screen.queryByText(/private prepare error/)).toBeNull();
  await fireEvent.click(button);
  let dialog = await screen.findByRole('dialog');
  await fireEvent.click(within(dialog).getByRole('button', { name: en['common.delete'] }));
  await screen.findByText(en['plugins.removalFailed']);
  expect(screen.queryByRole('dialog')).toBeNull();
  expect(store.preparePluginRemoval).toHaveBeenCalledTimes(2);
  expect(store.removePluginDeclaration).toHaveBeenCalledExactlyOnceWith(failed);
  expect(store.listPluginRuntime).toHaveBeenCalledOnce();
  expect(screen.queryByText(/external snapshot changed/)).toBeNull();
  await fireEvent.click(button);
  dialog = await screen.findByRole('dialog');
  expect(store.preparePluginRemoval).toHaveBeenCalledTimes(3);
  expect(store.removePluginDeclaration).toHaveBeenCalledOnce();
  await fireEvent.click(within(dialog).getByRole('button', { name: en['common.delete'] }));
  await screen.findByText(en['plugins.declarationRemoved']);
  expect(store.removePluginDeclaration).toHaveBeenLastCalledWith(fresh);
  expect(store.removePluginDeclaration).toHaveBeenCalledTimes(2);
  expect(store.reloadOpencode).not.toHaveBeenCalled();
});

test('service drift clears prepared plans and ignores late prepare or submit replies; unmount ignores pending prepare', async () => {
  const { createSubscriber } = await import('svelte/reactivity');
  let notify = () => {};
  const track = createSubscriber((update) => {
    notify = update;
  });
  let generation = 0;
  const preparing = deferred<PluginRemovalPlan>();
  const submitting = deferred<void>();
  const unmounted = deferred<PluginRemovalPlan>();
  const { store } = config({
    preparePluginRemoval: vi
      .fn()
      .mockReturnValueOnce(preparing.promise)
      .mockResolvedValueOnce(removalPlan())
      .mockResolvedValueOnce(removalPlan())
      .mockReturnValueOnce(unmounted.promise),
    removePluginDeclaration: vi.fn().mockReturnValue(submitting.promise),
  });
  Object.defineProperty(store, 'serviceTargetVersion', {
    get() {
      track();
      return generation;
    },
  });
  const view = show(store);
  await screen.findByText('example-plugin');
  await fireEvent.click(screen.getByRole('button', { name: en['common.delete'] }));
  generation += 1;
  notify();
  await waitFor(() => expect(store.listPluginRuntime).toHaveBeenCalledTimes(2));
  await screen.findByText('example-plugin');
  preparing.resolve(removalPlan());
  await preparing.promise;
  await tick();
  expect(screen.queryByRole('dialog')).toBeNull();
  expect(store.removePluginDeclaration).not.toHaveBeenCalled();
  await fireEvent.click(screen.getByRole('button', { name: en['common.delete'] }));
  await screen.findByRole('dialog');
  generation += 1;
  notify();
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  await screen.findByText('example-plugin');
  expect(store.removePluginDeclaration).not.toHaveBeenCalled();
  await fireEvent.click(screen.getByRole('button', { name: en['common.delete'] }));
  await fireEvent.click(within(await screen.findByRole('dialog')).getByRole('button', { name: en['common.delete'] }));
  generation += 1;
  notify();
  await waitFor(() => expect(store.listPluginRuntime).toHaveBeenCalledTimes(4));
  await screen.findByText('example-plugin');
  submitting.resolve();
  await submitting.promise;
  await tick();
  expect(screen.queryByRole('dialog')).toBeNull();
  expect(screen.queryByText(en['plugins.declarationRemoved'])).toBeNull();
  expect(store.listPluginRuntime).toHaveBeenCalledTimes(4);
  await fireEvent.click(screen.getByRole('button', { name: en['common.delete'] }));
  view.unmount();
  unmounted.resolve(removalPlan());
  await unmounted.promise;
  await tick();
  expect(screen.queryByRole('dialog')).toBeNull();
  expect(store.removePluginDeclaration).toHaveBeenCalledOnce();
});

test('two backend Global matches from a 90-item service runtime show slim on the first page without project controls', async () => {
  const read = deferred<PluginRuntime>();
  const { store, obsolete } = config({ listPluginRuntime: vi.fn().mockReturnValue(read.promise) });
  show(store);
  await screen.findByText(en['plugins.runtimeLoading']);
  expect(screen.getByRole('heading', { name: en['plugins.title'] })).toBeTruthy();
  expect(screen.queryByRole('heading', { name: en['plugins.runtimeTitle'] })).toBeNull();
  expect(screen.getByText(en['plugins.globalDeclarations'])).toBeTruthy();
  expect(screen.queryByRole('button', { name: en['scope.select'] })).toBeNull();
  expect(screen.queryByRole('button', { name: en['scope.add'] })).toBeNull();
  expect(screen.queryByRole('button', { name: en['scope.manage'] })).toBeNull();
  expect(screen.queryByRole('button', { name: /check updates/i })).toBeNull();
  expect(screen.queryByText(en['plugins.optionsHint'])).toBeNull();
  expect(screen.queryByRole('checkbox')).toBeNull();
  expect(screen.queryByRole('link', { name: /new|edit|delete/i })).toBeNull();
  expect(screen.queryByRole('button', { name: /^(new plugin|edit|delete|save)$/i })).toBeNull();
  read.resolve(
    runtime([
      packageItem('oh-my-opencode-slim'),
      {
        source: { type: 'local', path: '/plugins/local.ts' },
        features: { tui: true },
        state: { status: 'failed', error: 'Plugin failed to load' },
        canUpdate: false,
        canRemove: false,
      },
    ]),
  );
  const slim = await screen.findByText('oh-my-opencode-slim');
  expect(within(slim.closest('tr') as HTMLTableRowElement).getByText(en['plugins.runtimeActive'])).toBeTruthy();
  expect(screen.getByText('/plugins/local.ts')).toBeTruthy();
  expect(screen.getByText('Plugin failed to load')).toBeTruthy();
  expect(screen.getAllByRole('row')).toHaveLength(3);
  expect(screen.queryByRole('navigation', { name: en['common.pagination'] })).toBeNull();
  expect(store.listPluginRuntime).toHaveBeenCalledExactlyOnceWith();
  expect(store.getPluginConfigSources).not.toHaveBeenCalled();
  expect(store.updatePluginPackage).not.toHaveBeenCalled();
  for (const method of Object.values(obsolete)) expect(method).not.toHaveBeenCalled();
});

test('built-in and SDK plugins display their API IDs and can be searched individually', async () => {
  const data = (
    [
      ['builtin-alpha', 'builtin'],
      ['builtin-beta', 'builtin'],
      ['sdk-alpha', 'sdk'],
      ['sdk-beta', 'sdk'],
    ] as const
  ).map<PluginRuntimeItem>(([id, type]) => ({
    id,
    source: { type },
    features: {},
    state: { status: 'active' },
    canUpdate: false,
    canRemove: false,
  }));
  const { store } = config({ listPluginRuntime: vi.fn().mockResolvedValue(runtime(data)) });
  show(store);
  await screen.findByText('builtin-alpha');
  for (const item of data) expect(screen.getByText(item.id ?? '')).toBeTruthy();
  const search = screen.getByRole('searchbox', { name: en['plugins.search'] });
  await fireEvent.input(search, { target: { value: 'sdk-beta' } });
  expect(screen.getByText('sdk-beta')).toBeTruthy();
  expect(screen.queryByText('sdk-alpha')).toBeNull();
  expect(screen.queryByText('builtin-alpha')).toBeNull();
  expect(screen.getAllByRole('row')).toHaveLength(2);
  await fireEvent.input(search, { target: { value: 'builtin-beta' } });
  expect(screen.getByText('builtin-beta')).toBeTruthy();
  expect(screen.queryByText('builtin-alpha')).toBeNull();
  expect(screen.queryByText('sdk-beta')).toBeNull();
  expect(screen.getAllByRole('row')).toHaveLength(2);
});

test('Plugins stays Global when the shared project selection changes', async () => {
  const { store } = config({}, 'project-a');
  show(store);
  await screen.findByText('example-plugin');
  expect(store.listPluginRuntime).toHaveBeenCalledExactlyOnceWith();
  expect(screen.queryByRole('checkbox')).toBeNull();
  expect(screen.queryByText(en['scope.projectWritable'])).toBeNull();
  expect(screen.queryByText(en['scope.projectReadOnly'])).toBeNull();
  expect(screen.queryByText('/work/alpha')).toBeNull();
  await store.selectProject('project-b');
  await tick();
  expect(store.listPluginRuntime).toHaveBeenCalledExactlyOnceWith();
  expect(screen.getByText('example-plugin')).toBeTruthy();
  expect(screen.getByText(en['plugins.globalDeclarations'])).toBeTruthy();
});

test('runtime search and pagination keep the displayed list bounded', async () => {
  const { store } = config({
    listPluginRuntime: vi
      .fn()
      .mockResolvedValue(runtime(Array.from({ length: 12 }, (_, i) => packageItem(`pkg-${i}`, false)))),
  });
  show(store);
  await screen.findByText('pkg-0');
  expect(screen.getAllByRole('row')).toHaveLength(11);
  expect(screen.queryByText('pkg-11')).toBeNull();
  await fireEvent.click(screen.getByRole('button', { name: en['common.nextPage'] }));
  await screen.findByText('pkg-11');
  expect(screen.getAllByRole('row')).toHaveLength(3);
  await fireEvent.input(screen.getByRole('searchbox', { name: en['plugins.search'] }), { target: { value: 'pkg-11' } });
  expect(screen.getAllByRole('row')).toHaveLength(2);
  await fireEvent.input(screen.getByRole('searchbox', { name: en['plugins.search'] }), {
    target: { value: 'no match' },
  });
  expect(screen.getByText(en['common.noMatches'])).toBeTruthy();
});

test('API failures are not empty runtime results and have no local fallback', async () => {
  const { store, obsolete } = config({
    listPluginRuntime: vi
      .fn()
      .mockRejectedValueOnce(new Error('private connection detail'))
      .mockResolvedValue(runtime([])),
  });
  show(store);
  await screen.findByRole('alert');
  expect(screen.getByText(en['plugins.runtimeUnavailable'])).toBeTruthy();
  expect(screen.queryByText(en['plugins.runtimeEmpty'])).toBeNull();
  expect(screen.queryByText(/private connection detail/)).toBeNull();
  expect(screen.queryByRole('button', { name: en['plugins.updatePackage'] })).toBeNull();
  await fireEvent.click(screen.getByRole('button', { name: en['plugins.runtimeRefresh'] }));
  await screen.findByText(en['plugins.runtimeEmpty']);
  for (const method of Object.values(obsolete)) expect(method).not.toHaveBeenCalled();
});

test('eligible package updates require confirmation before a shared-cache mutation', async () => {
  const update = deferred<void>();
  const { store } = config({
    listPluginRuntime: vi
      .fn()
      .mockResolvedValue(runtime([packageItem('available-package'), packageItem('not-approved', false)])),
    updatePluginPackage: vi.fn().mockReturnValue(update.promise),
  });
  show(store);
  const available = await screen.findByText('available-package');
  const buttons = screen.getAllByRole('button', { name: en['plugins.updatePackage'] }) as HTMLButtonElement[];
  expect(buttons).toHaveLength(2);
  expect(buttons.filter((button) => button.disabled)).toHaveLength(1);
  await fireEvent.click(
    within(available.closest('tr') as HTMLTableRowElement).getByRole('button', { name: en['plugins.updatePackage'] }),
  );
  const dialog = screen.getByRole('dialog');
  expect(dialog.textContent).toContain(en['plugins.updateConfirm'].replace('{name}', 'available-package'));
  expect(dialog.textContent).toContain('⚠️');
  expect(store.updatePluginPackage).not.toHaveBeenCalled();
  await fireEvent.click(within(dialog).getByRole('button', { name: en['plugins.updatePackage'] }));
  expect((within(dialog).getByRole('button', { name: en['common.cancel'] }) as HTMLButtonElement).disabled).toBe(true);
  update.resolve();
  await waitFor(() => expect(store.listPluginRuntime).toHaveBeenCalledTimes(2));
  await screen.findByText(en['plugins.packageUpdated']);
  expect(store.updatePluginPackage).toHaveBeenCalledExactlyOnceWith('available-package');
  expect(store.getPluginConfigSources).not.toHaveBeenCalled();
  expect(store.reloadOpencode).not.toHaveBeenCalled();
});

test('uncertain update never retries or reads automatically; only a successful explicit GET unlocks mutation', async () => {
  const { store } = config({ updatePluginPackage: vi.fn().mockRejectedValue(new Error('sensitive outcome')) });
  show(store);
  await screen.findByText('example-plugin');
  await fireEvent.click(screen.getByRole('button', { name: en['plugins.updatePackage'] }));
  await fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: en['plugins.updatePackage'] }));
  await screen.findByText(en['plugins.updatePackageFailed']);
  expect(store.updatePluginPackage).toHaveBeenCalledExactlyOnceWith('example-plugin');
  expect(store.listPluginRuntime).toHaveBeenCalledOnce();
  expect(screen.queryByText(/sensitive outcome/)).toBeNull();
  expect((screen.getByRole('button', { name: en['plugins.updatePackage'] }) as HTMLButtonElement).disabled).toBe(true);
  vi.mocked(store.listPluginRuntime).mockRejectedValueOnce(new Error('still unavailable'));
  await fireEvent.click(screen.getByRole('button', { name: en['plugins.runtimeRefresh'] }));
  await screen.findByText(en['plugins.runtimeUnavailable']);
  expect(screen.queryByRole('button', { name: en['plugins.updatePackage'] })).toBeNull();
  await fireEvent.click(screen.getByRole('button', { name: en['plugins.runtimeRefresh'] }));
  await screen.findByText('example-plugin');
  expect((screen.getByRole('button', { name: en['plugins.updatePackage'] }) as HTMLButtonElement).disabled).toBe(false);
  expect(store.updatePluginPackage).toHaveBeenCalledOnce();
  expect(store.reloadOpencode).not.toHaveBeenCalled();
});

test('each row keeps an Update action and refresh consumes fresh eligibility from the list response', async () => {
  const current: PluginRuntimeItem = {
    ...packageItem('current-package'),
    source: { type: 'package', target: 'current-package', version: '1.0.0', outdated: false },
  };
  const items: PluginRuntimeItem[] = [
    packageItem('available-package'),
    packageItem('pinned@1.2.3', false),
    current,
    packageItem('ambiguous-package', false),
    {
      ...packageItem('updating-package'),
      source: { type: 'package', target: 'updating-package', outdated: true, updating: true },
    },
    {
      ...packageItem('unknown-status-package'),
      source: { type: 'package', target: 'unknown-status-package', version: '1.0.0' },
    },
    {
      source: { type: 'local', path: '/local/plugin.ts' },
      state: { status: 'active' },
      features: {},
      canUpdate: true,
      canRemove: false,
    },
  ];
  const fresh: PluginRuntimeItem = {
    ...packageItem('current-package'),
    source: { type: 'package', target: 'current-package', version: '2.0.0', outdated: true },
  };
  const { store } = config({
    listPluginRuntime: vi
      .fn()
      .mockResolvedValueOnce(runtime(items))
      .mockResolvedValue(runtime([fresh])),
  });
  show(store);
  await screen.findByText('current-package');
  const buttons = screen.getAllByRole('button', { name: en['plugins.updatePackage'] }) as HTMLButtonElement[];
  expect(buttons).toHaveLength(items.length);
  expect(buttons.filter((button) => !button.disabled)).toHaveLength(1);
  for (const target of [
    'pinned@1.2.3',
    'current-package',
    'ambiguous-package',
    'updating-package',
    'unknown-status-package',
    '/local/plugin.ts',
  ]) {
    const row = within(screen.getByText(target).closest('tr') as HTMLTableRowElement);
    const button = row.getByRole('button', { name: en['plugins.updatePackage'] }) as HTMLButtonElement;
    expect(button.disabled).toBe(true);
    expect(button.title).toBe(en['plugins.updateUnavailable']);
    await fireEvent.click(button);
  }
  expect(screen.queryByRole('dialog')).toBeNull();
  expect(store.updatePluginPackage).not.toHaveBeenCalled();
  expect(
    within(screen.getByText('current-package').closest('tr') as HTMLTableRowElement).queryByText(
      en['plugins.outdated'],
    ),
  ).toBeNull();
  expect(
    within(screen.getByText('pinned@1.2.3').closest('tr') as HTMLTableRowElement).getByText(en['plugins.outdated']),
  ).toBeTruthy();
  await fireEvent.click(screen.getByRole('button', { name: en['plugins.runtimeRefresh'] }));
  await screen.findByText('2.0.0');
  expect(store.listPluginRuntime).toHaveBeenCalledTimes(2);
  expect((screen.getByRole('button', { name: en['plugins.updatePackage'] }) as HTMLButtonElement).disabled).toBe(false);
  expect(screen.queryByRole('button', { name: /check updates/i })).toBeNull();
  expect(store.updatePluginPackage).not.toHaveBeenCalled();
});

test('configuration preview is on demand, preserves API order and distinguishes absent options from an empty object', async () => {
  const source = deferred<PluginConfigPreview>();
  const { store } = config({ getPluginConfigSources: vi.fn().mockReturnValue(source.promise) }, 'project-b');
  show(store);
  await screen.findByText('example-plugin');
  expect(store.getPluginConfigSources).not.toHaveBeenCalled();
  await fireEvent.click(screen.getByRole('button', { name: en['plugins.configPreview'] }));
  const dialog = screen.getByRole('dialog');
  expect(within(dialog).getByText(en['plugins.configLoading'])).toBeTruthy();
  source.resolve({
    directory: '/project/location',
    data: [
      {
        type: 'document',
        plugins: [
          'string-entry',
          { package: 'no-options' },
          { package: 'empty-options', options: {} },
          { package: 'sensitive', options: { token: '<script>unsafe()</script>' } },
        ],
      },
      { type: 'directory', path: '/plugins/candidate-directory' },
      { type: 'document', path: '/config/last.jsonc', plugins: [] },
    ],
  });
  await within(dialog).findByText('string-entry');
  expect(store.getPluginConfigSources).toHaveBeenCalledExactlyOnceWith();
  expect(within(dialog).getByText(en['plugins.configPreviewHint'])).toBeTruthy();
  expect(within(dialog).getByText(en['plugins.optionsHint'])).toBeTruthy();
  expect(within(dialog).getByText(en['plugins.pathNotProvided'])).toBeTruthy();
  expect(within(dialog).getAllByText(en['plugins.optionsNotProvided'])).toHaveLength(2);
  expect([...dialog.querySelectorAll('pre')].map((pre) => pre.textContent)).toEqual([
    '{}',
    JSON.stringify({ token: '<script>unsafe()</script>' }, null, 2),
  ]);
  expect(dialog.querySelector('script')).toBeNull();
  expect(dialog.querySelector('textarea, input')).toBeNull();
  const sources = [...(dialog.querySelector('ol')?.children ?? [])];
  expect(sources).toHaveLength(3);
  expect(sources[0]?.textContent).toContain('string-entry');
  expect(sources[1]?.textContent).toContain('/plugins/candidate-directory');
  expect(sources[1]?.textContent).not.toContain(en['plugins.options']);
  expect(sources[2]?.textContent).toContain('/config/last.jsonc');
  expect(store.updatePluginPackage).not.toHaveBeenCalled();
  await fireEvent.click(within(dialog).getByRole('button', { name: en['common.close'] }));
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
});

test('configuration failure is independent from runtime upgrade eligibility and can retry its GET', async () => {
  const { store } = config({
    getPluginConfigSources: vi
      .fn()
      .mockRejectedValueOnce(new Error('private preview detail'))
      .mockResolvedValue({ directory: '/service/location', data: [] }),
  });
  show(store);
  await screen.findByText('example-plugin');
  await fireEvent.click(screen.getByRole('button', { name: en['plugins.configPreview'] }));
  const dialog = screen.getByRole('dialog');
  await within(dialog).findByText(en['plugins.configUnavailable']);
  expect(screen.queryByText(/private preview detail/)).toBeNull();
  await fireEvent.click(within(dialog).getByRole('button', { name: en['configuration.retry'] }));
  await within(dialog).findByText(en['plugins.configEmpty']);
  await fireEvent.click(within(dialog).getByRole('button', { name: en['common.close'] }));
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect((screen.getByRole('button', { name: en['plugins.updatePackage'] }) as HTMLButtonElement).disabled).toBe(false);
  expect(store.listPluginRuntime).toHaveBeenCalledOnce();
  await fireEvent.click(screen.getByRole('button', { name: en['plugins.updatePackage'] }));
  expect(screen.getByRole('dialog').textContent).toContain(en['plugins.updateConfirmTitle']);
});

test('service changes reread Global runtime, clear previews and reject stale reads and errors', async () => {
  const { createSubscriber } = await import('svelte/reactivity');
  let notify = () => {};
  const track = createSubscriber((update) => {
    notify = update;
  });
  let generation = 0;
  const oldRuntime = deferred<PluginRuntime>();
  const oldServiceRuntime = deferred<PluginRuntime>();
  const oldPreview = deferred<PluginConfigPreview>();
  const oldServicePreview = deferred<PluginConfigPreview>();
  const { store } = config(
    {
      listPluginRuntime: vi
        .fn()
        .mockReturnValueOnce(oldRuntime.promise)
        .mockReturnValueOnce(oldServiceRuntime.promise)
        .mockResolvedValue(runtime([packageItem('current-plugin')], '/current-location')),
      getPluginConfigSources: vi
        .fn()
        .mockReturnValueOnce(oldPreview.promise)
        .mockReturnValueOnce(oldServicePreview.promise)
        .mockResolvedValue({ directory: '/current-location', data: [{ type: 'directory', path: '/current-config' }] }),
    },
    'project-a',
  );
  Object.defineProperty(store, 'serviceTargetVersion', {
    get() {
      track();
      return generation;
    },
  });
  show(store);
  await waitFor(() => expect(store.listPluginRuntime).toHaveBeenCalledExactlyOnceWith());
  await fireEvent.click(screen.getByRole('button', { name: en['plugins.configPreview'] }));
  generation += 1;
  notify();
  await waitFor(() => expect(store.listPluginRuntime).toHaveBeenCalledTimes(2));
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  await fireEvent.click(screen.getByRole('button', { name: en['plugins.configPreview'] }));
  generation += 1;
  notify();
  await screen.findByText('current-plugin');
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  await fireEvent.click(screen.getByRole('button', { name: en['plugins.configPreview'] }));
  await screen.findByText('/current-config');
  oldRuntime.resolve(runtime([packageItem('stale-plugin')], '/stale-location'));
  oldServiceRuntime.reject(new Error('stale runtime error'));
  oldPreview.resolve({ directory: '/stale-location', data: [{ type: 'directory', path: '/stale-config' }] });
  oldServicePreview.reject(new Error('stale config error'));
  await Promise.allSettled([
    oldRuntime.promise,
    oldServiceRuntime.promise,
    oldPreview.promise,
    oldServicePreview.promise,
  ]);
  expect(screen.queryByText('stale-plugin')).toBeNull();
  expect(screen.queryByText('/stale-config')).toBeNull();
  expect(screen.queryByText(en['plugins.runtimeUnavailable'])).toBeNull();
  expect(screen.queryByText(en['plugins.configUnavailable'])).toBeNull();
  expect(screen.getByText('/current-config')).toBeTruthy();
  expect(store.listPluginRuntime).toHaveBeenCalledTimes(3);
  expect(store.getPluginConfigSources).toHaveBeenNthCalledWith(1);
  expect(store.getPluginConfigSources).toHaveBeenNthCalledWith(2);
  expect(store.getPluginConfigSources).toHaveBeenNthCalledWith(3);
});

test('a service switch closes an upgrade confirmation and ignores its late POST failure', async () => {
  const { createSubscriber } = await import('svelte/reactivity');
  let notify = () => {};
  const track = createSubscriber((update) => {
    notify = update;
  });
  let generation = 0;
  const update = deferred<void>();
  const { store } = config({
    listPluginRuntime: vi
      .fn()
      .mockResolvedValueOnce(runtime())
      .mockResolvedValue(runtime([packageItem('new-service-plugin')])),
    updatePluginPackage: vi.fn().mockReturnValue(update.promise),
  });
  Object.defineProperty(store, 'serviceTargetVersion', {
    get() {
      track();
      return generation;
    },
  });
  show(store);
  await screen.findByText('example-plugin');
  await fireEvent.click(screen.getByRole('button', { name: en['plugins.updatePackage'] }));
  await fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: en['plugins.updatePackage'] }));
  generation += 1;
  notify();
  await screen.findByText('new-service-plugin');
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  update.reject(new Error('old service failure'));
  await Promise.allSettled([update.promise]);
  expect(screen.queryByText(en['plugins.updatePackageFailed'])).toBeNull();
  expect((screen.getByRole('button', { name: en['plugins.updatePackage'] }) as HTMLButtonElement).disabled).toBe(false);
  expect(store.updatePluginPackage).toHaveBeenCalledOnce();
});
