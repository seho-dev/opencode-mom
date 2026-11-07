import { fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { tick } from 'svelte';
import { expect, test, vi } from 'vitest';
import type { CommandAdapter } from '$src/config/adapter.js';
import { type ConfigStore, createConfigStore } from '$src/config/store.svelte.js';
import Harness from '$src/test-utils/harness.svelte';
import type { AppState } from '$src/types/app.js';
import type { Group } from '$src/types/groups.js';
import GroupList from '../+page.svelte';

function group(type: Group['type'], isEnabled = false): Group {
  return {
    id: `${type}-id`,
    name: `${type} lane`,
    description: 'Keep this description',
    type,
    isEnabled,
    updatedAt: '2026-01-01T00:00:00Z',
    openCodeAgentOverrides: [{ agentName: 'build', modelRef: 'example/native', variant: 'fast' }],
    slimAgentOverrides: [{ agentName: 'oracle', modelRef: 'example/slim', variant: 'slow' }],
    omoAgentOverrides: [{ agentName: 'sisyphus', modelRef: 'example/omo' }],
    omoCategoryMappings: [{ categoryName: 'visual-engineering', modelRef: 'example/category' }],
  };
}

async function mount(
  options: {
    groups?: Group[];
    selectedGroupId?: string | null;
    locale?: 'en' | 'zh';
    defaultPlugin?: Group['type'];
  } = {},
) {
  const state: AppState = {
    providers: [],
    agents: [],
    groups: options.groups ?? [group('native'), group('slim'), group('omo')],
    selectedGroupId: options.selectedGroupId ?? null,

    preferences: {
      theme: 'dark',
      locale: options.locale ?? 'en',
      defaultPlugin: options.defaultPlugin ?? 'native',
    },
  };
  const adapter = {
    loadAppState: vi.fn(async () => structuredClone(state)),
    saveGroup: vi.fn(async (value: Group) => {
      const saved = { ...value, updatedAt: '2026-10-22T00:00:00Z' };

      state.groups = state.groups.map((entry) => (entry.id === saved.id ? saved : entry));

      return saved;
    }),
    switchGroup: vi.fn(async (id: string) => {
      state.selectedGroupId = id;
    }),
    deleteGroup: vi.fn(async (id: string) => {
      state.groups = state.groups.filter((entry) => entry.id !== id);
    }),
    opencodeReload: vi.fn().mockResolvedValue(undefined),
  };
  const config = createConfigStore(adapter as unknown as CommandAdapter, false, false);

  await config.refresh(true);

  const view = render(GroupList, {}, { wrapper: Harness, wrapperProps: { config } });

  return { ...view, adapter, config, state };
}

test.each(['native', 'slim', 'omo'] as const)(
  '%s enables and disables on the list without changing selection or mappings',
  async (type) => {
    const selected = { ...group('native', true), id: 'selected', name: 'Selected lane' };
    const initial = group(type);
    const view = await mount({ groups: [selected, initial], selectedGroupId: selected.id });

    if (type !== 'native')
      await fireEvent.click(screen.getByRole('tab', { name: `${type === 'slim' ? 'Slim' : 'OMO'} (1)` }));

    const control = screen.getByRole('checkbox', { name: `Enable ${initial.name}` }) as HTMLInputElement;

    expect(control.type).toBe('checkbox');
    expect(control.checked).toBe(false);
    expect(control.disabled).toBe(false);
    control.focus();
    expect(document.activeElement).toBe(control);

    await fireEvent.click(control);
    await waitFor(() => expect(control.disabled).toBe(false));
    expect(view.adapter.saveGroup).toHaveBeenCalledExactlyOnceWith({ ...initial, isEnabled: true });
    expect(screen.getByRole('checkbox', { name: `Disable ${initial.name}` })).toBe(control);
    expect(control.checked).toBe(true);

    await fireEvent.click(control);
    await waitFor(() => expect(control.disabled).toBe(false));
    expect(view.adapter.saveGroup).toHaveBeenNthCalledWith(2, {
      ...initial,
      isEnabled: false,
      updatedAt: '2026-10-22T00:00:00Z',
    });
    expect(screen.getByRole('checkbox', { name: `Enable ${initial.name}` })).toBe(control);
    expect(control.checked).toBe(false);
    expect(view.config.selectedGroupId).toBe(selected.id);
    expect(view.adapter.switchGroup).not.toHaveBeenCalled();
    expect(view.adapter.opencodeReload).not.toHaveBeenCalled();
    expect(view.adapter.deleteGroup).not.toHaveBeenCalled();
    expect(screen.queryByRole('dialog')).toBeNull();
  },
);

test('pending toggle retains confirmed status and blocks duplicate saves and conflicting actions through readback', async () => {
  const view = await mount({
    groups: [group('native'), { ...group('native', true), id: 'other', name: 'Other lane' }],
  });
  const save = Promise.withResolvers<Group>();
  const readback = Promise.withResolvers<AppState>();

  view.adapter.saveGroup.mockReturnValueOnce(save.promise);
  view.adapter.loadAppState.mockReturnValueOnce(readback.promise);

  const control = screen.getByRole('checkbox', { name: 'Enable native lane' }) as HTMLInputElement;

  await fireEvent.click(control);
  expect(control.checked).toBe(false);
  expect(control.disabled).toBe(true);
  expect(control.getAttribute('aria-busy')).toBe('true');
  expect(screen.getByRole('button', { name: 'Refresh' }).hasAttribute('disabled')).toBe(true);
  expect(screen.getByRole('link', { name: 'Edit native lane' }).getAttribute('aria-disabled')).toBe('true');
  expect(screen.getByRole('link', { name: 'New group' }).getAttribute('aria-disabled')).toBe('true');
  expect(screen.getByRole('button', { name: 'Delete Other lane' }).hasAttribute('disabled')).toBe(true);
  expect((screen.getByRole('checkbox', { name: 'Disable Other lane' }) as HTMLInputElement).disabled).toBe(true);
  await fireEvent.click(control);
  await fireEvent.click(screen.getByRole('button', { name: 'Switch to Other lane' }));
  await fireEvent.click(screen.getByRole('button', { name: 'Delete Other lane' }));
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh' }));
  expect(view.adapter.saveGroup).toHaveBeenCalledOnce();
  expect(view.adapter.switchGroup).not.toHaveBeenCalled();
  expect(view.adapter.loadAppState).toHaveBeenCalledOnce();
  expect(screen.queryByRole('dialog')).toBeNull();

  save.resolve({ ...group('native'), isEnabled: true });
  await waitFor(() => expect(view.adapter.loadAppState).toHaveBeenCalledTimes(2));
  expect(view.config.loading).toBe(true);
  expect(screen.getByRole('checkbox', { name: 'Enable native lane' })).toBe(control);
  expect(control.checked).toBe(false);
  expect(control.disabled).toBe(true);
  readback.resolve({ ...view.state, groups: [{ ...group('native'), isEnabled: true }] });
  await waitFor(() => expect(control.disabled).toBe(false));
  expect(control.checked).toBe(true);
  expect(control.getAttribute('aria-busy')).toBe('false');
});

test.each([false, true])(
  'failed toggle keeps confirmed enabled=%s and retries only on another click',
  async (isEnabled) => {
    const initial = group('native', isEnabled);
    const view = await mount({ groups: [initial] });
    const save = Promise.withResolvers<Group>();

    view.adapter.saveGroup.mockReturnValueOnce(save.promise);

    const control = screen.getByRole('checkbox', {
      name: `${isEnabled ? 'Disable' : 'Enable'} native lane`,
    }) as HTMLInputElement;

    await fireEvent.click(control);
    save.reject({ code: 'validation_failed', message: 'Save rejected' });
    await waitFor(() => expect(control.disabled).toBe(false));
    expect(control.checked).toBe(isEnabled);
    expect(view.config.groups[0]).toEqual(initial);
    expect(view.config.error?.code).toBe('validation_failed');
    expect(view.config.draftRecovery?.payload).toEqual({ ...initial, isEnabled: !isEnabled });
    expect(view.adapter.loadAppState).toHaveBeenCalledOnce();
    expect(view.adapter.saveGroup).toHaveBeenCalledOnce();

    await fireEvent.click(control);
    await waitFor(() => expect(control.disabled).toBe(false));
    expect(view.adapter.saveGroup).toHaveBeenCalledTimes(2);
    expect(control.checked).toBe(!isEnabled);
    expect(view.config.error).toBeNull();
  },
);

test.each([true, false])(
  'native click cancellation preserves confirmed checked state after save succeeds=%s',
  async (succeeds) => {
    const view = await mount({ groups: [group('native')] });
    const save = Promise.withResolvers<Group>();

    view.adapter.saveGroup.mockReturnValueOnce(save.promise);

    const control = screen.getByRole('checkbox', { name: 'Enable native lane' }) as HTMLInputElement;

    control.click();
    await tick();
    expect(control.checked).toBe(false);
    expect(control.disabled).toBe(true);

    if (succeeds) {
      const saved = { ...group('native'), isEnabled: true };

      view.state.groups = [saved];
      save.resolve(saved);
    } else {
      save.reject({ code: 'validation_failed', message: 'Save rejected' });
    }

    await waitFor(() => expect(control.disabled).toBe(false));
    await tick();
    expect(control.checked).toBe(succeeds);
    expect(view.config.groups[0]?.isEnabled).toBe(succeeds);
    expect(
      screen.getByRole('checkbox', { name: `${succeeds ? 'Disable' : 'Enable'} ${view.config.groups[0]?.name}` }),
    ).toBe(control);
    expect(view.adapter.saveGroup).toHaveBeenCalledOnce();
  },
);

test('selected group cannot be disabled while another enabled group remains independently editable', async () => {
  const selected = group('native', true);
  const other = { ...selected, id: 'other', name: 'Other lane' };
  const view = await mount({ groups: [selected, other], selectedGroupId: selected.id });
  const selectedControl = screen.getByRole('checkbox', { name: 'Disable native lane' }) as HTMLInputElement;

  expect(selectedControl.checked).toBe(true);
  expect(selectedControl.disabled).toBe(true);
  expect(selectedControl.title).toBe('Selected');
  await fireEvent.click(selectedControl);
  expect(view.adapter.saveGroup).not.toHaveBeenCalled();
  await fireEvent.click(screen.getByRole('checkbox', { name: 'Disable Other lane' }));
  await waitFor(() => expect(view.adapter.saveGroup).toHaveBeenCalledOnce());
  expect(view.adapter.saveGroup).toHaveBeenCalledWith({ ...other, isEnabled: false });
  expect(view.config.selectedGroupId).toBe(selected.id);
});

test('manual refresh blocks toggle until latest group fields arrive', async () => {
  const view = await mount();
  const pending = Promise.withResolvers<AppState>();

  view.adapter.loadAppState.mockReturnValueOnce(pending.promise);
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh' }));

  const control = screen.getByRole('checkbox', { name: 'Enable native lane' }) as HTMLInputElement;

  expect(control.disabled).toBe(true);
  await fireEvent.click(control);
  expect(view.adapter.saveGroup).not.toHaveBeenCalled();

  const latest = { ...group('native'), description: 'Latest external edit', updatedAt: '2026-10-21T00:00:00Z' };

  pending.resolve({ ...view.state, groups: [latest] });
  await waitFor(() => expect(control.disabled).toBe(false));
  await fireEvent.click(control);
  expect(view.adapter.saveGroup).toHaveBeenCalledExactlyOnceWith({ ...latest, isEnabled: true });
});

test.each(['loading', 'saving', 'switching', 'reloading'])('%s blocks toggle and list writes', async (flag) => {
  const config = {
    groups: [group('native')],
    preferences: { locale: 'en' },
    loading: false,
    saving: false,
    switching: false,
    reloading: false,
    selectedGroupId: null,
    saveGroup: vi.fn(),
    switchGroup: vi.fn(),
    refresh: vi.fn(),
    [flag]: true,
  };

  render(GroupList, {}, { wrapper: Harness, wrapperProps: { config: config as unknown as ConfigStore } });

  const control = screen.getByRole('checkbox', { name: 'Enable native lane' }) as HTMLInputElement;

  expect(control.disabled).toBe(true);
  await fireEvent.click(control);
  await fireEvent.click(screen.getByRole('button', { name: 'Refresh' }));
  await tick();
  expect(config.saveGroup).not.toHaveBeenCalled();
  expect(config.refresh).not.toHaveBeenCalled();
});

test('Chinese accessible names identify the group and follow confirmed enabled state', async () => {
  await mount({ groups: [group('native')], locale: 'zh' });

  const control = (await screen.findByRole('checkbox', { name: '启用 native lane' })) as HTMLInputElement;

  await fireEvent.click(control);
  await waitFor(() => expect(control.disabled).toBe(false));
  expect(screen.getByRole('checkbox', { name: '停用 native lane' })).toBe(control);
  expect(control.checked).toBe(true);
});

test('tablist arrows and Home/End follow the runtime tab order and keep refs on the same tabs', async () => {
  const view = await mount({ defaultPlugin: 'slim' });
  const slim = await screen.findByRole('tab', { name: /Slim/ });
  const native = screen.getByRole('tab', { name: /Native/ });
  const omo = screen.getByRole('tab', { name: /OMO/ });

  await waitFor(() => expect(slim.getAttribute('aria-selected')).toBe('true'));
  slim.focus();

  // Initial order is [slim, native, omo] derived from runtime defaultPlugin.
  expect(screen.getAllByRole('tab')).toEqual([slim, native, omo]);
  expect(slim.getAttribute('tabindex')).toBe('0');
  expect(native.getAttribute('tabindex')).toBe('-1');
  expect(omo.getAttribute('tabindex')).toBe('-1');

  // ArrowRight moves from slim to native.
  await fireEvent.keyDown(slim, { key: 'ArrowRight' });
  expect(document.activeElement).toBe(native);
  expect(native.getAttribute('aria-selected')).toBe('true');
  expect(native.getAttribute('tabindex')).toBe('0');
  expect(slim.getAttribute('aria-selected')).toBe('false');
  expect(slim.getAttribute('tabindex')).toBe('-1');
  expect(screen.getByRole('tabpanel').id).toBe('groups-tabpanel-native');
  expect(screen.getByRole('tabpanel').getAttribute('aria-labelledby')).toBe('groups-tab-native');

  // End moves to omo (last).
  await fireEvent.keyDown(native, { key: 'End' });
  expect(document.activeElement).toBe(omo);
  expect(omo.getAttribute('aria-selected')).toBe('true');
  expect(screen.getByRole('tabpanel').id).toBe('groups-tabpanel-omo');

  // Home moves to slim (first).
  await fireEvent.keyDown(omo, { key: 'Home' });
  expect(document.activeElement).toBe(slim);
  expect(slim.getAttribute('aria-selected')).toBe('true');
  expect(screen.getByRole('tabpanel').id).toBe('groups-tabpanel-slim');

  // ArrowLeft from the first entry wraps to the last one (omo).
  await fireEvent.keyDown(slim, { key: 'ArrowLeft' });
  expect(document.activeElement).toBe(omo);
  expect(omo.getAttribute('aria-selected')).toBe('true');
  expect(screen.getByRole('tabpanel').id).toBe('groups-tabpanel-omo');

  // ArrowRight from the last entry wraps back to the first one (slim).
  await fireEvent.keyDown(omo, { key: 'ArrowRight' });
  expect(document.activeElement).toBe(slim);
  expect(slim.getAttribute('aria-selected')).toBe('true');

  // Changing runtime preference reorders tabOrder to [omo, native, slim] and preserves DOM node identity.
  view.config.applyPreferences({ ...view.config.preferences, defaultPlugin: 'omo' });
  await tick();
  expect(screen.getAllByRole('tab')).toEqual([omo, native, slim]);
  expect(screen.getByRole('tab', { name: /Slim/ })).toBe(slim);
  expect(screen.getByRole('tab', { name: /Native/ })).toBe(native);
  expect(screen.getByRole('tab', { name: /OMO/ })).toBe(omo);

  // Keyboard navigation on reordered tabs: Home moves to omo, ArrowRight walks to native, End moves to slim.
  slim.focus();
  await fireEvent.keyDown(slim, { key: 'Home' });
  expect(document.activeElement).toBe(omo);
  expect(omo.getAttribute('aria-selected')).toBe('true');
  expect(screen.getByRole('tabpanel').id).toBe('groups-tabpanel-omo');

  await fireEvent.keyDown(omo, { key: 'ArrowRight' });
  expect(document.activeElement).toBe(native);
  expect(native.getAttribute('aria-selected')).toBe('true');
  expect(screen.getByRole('tabpanel').id).toBe('groups-tabpanel-native');

  await fireEvent.keyDown(native, { key: 'End' });
  expect(document.activeElement).toBe(slim);
  expect(slim.getAttribute('aria-selected')).toBe('true');
  expect(screen.getByRole('tabpanel').id).toBe('groups-tabpanel-slim');

  // ArrowRight wraps from slim back to omo.
  await fireEvent.keyDown(slim, { key: 'ArrowRight' });
  expect(document.activeElement).toBe(omo);
  expect(omo.getAttribute('aria-selected')).toBe('true');

  // ArrowLeft wraps from omo back to slim.
  await fireEvent.keyDown(omo, { key: 'ArrowLeft' });
  expect(document.activeElement).toBe(slim);
  expect(slim.getAttribute('aria-selected')).toBe('true');
});

test('prioritizes tab selection matching default plugin preference', async () => {
  mount({ defaultPlugin: 'slim' });
  await waitFor(() => {
    expect(screen.getByRole('tab', { name: /Slim/ }).getAttribute('aria-selected')).toBe('true');
    expect(screen.getByRole('tab', { name: /Native/ }).getAttribute('aria-selected')).toBe('false');
  });
});

test('group list query trims leading and trailing whitespace', async () => {
  const { config } = await mount({
    groups: [
      { ...group('native'), id: 'native-alpha', name: 'Alpha Lane' },
      { ...group('native'), id: 'native-beta', name: 'Beta Lane' },
    ],
  });

  expect(config.groups).toHaveLength(2);

  const searchInput = await screen.findByRole('textbox', { name: 'Search groups' });

  await fireEvent.input(searchInput, { target: { value: '   alpha   ' } });
  expect(screen.getByText('Alpha Lane')).toBeTruthy();
  expect(screen.queryByText('Beta Lane')).toBeNull();
});
