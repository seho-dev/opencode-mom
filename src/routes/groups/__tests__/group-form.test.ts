import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterAll, beforeAll, expect, test, vi } from 'vitest';
import type { ConfigStore } from '$src/config/store.svelte.js';
import Harness from '$src/test-utils/harness.svelte';
import type { Group } from '$src/types/groups.js';
import GroupForm from '../GroupForm.svelte';

vi.mock('$app/navigation', () => ({ goto: vi.fn() }));

const scrollIntoView = Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'scrollIntoView');
beforeAll(() => {
  Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', { configurable: true, value: vi.fn() });
});
afterAll(() => {
  if (scrollIntoView) Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', scrollIntoView);
  else Reflect.deleteProperty(HTMLElement.prototype, 'scrollIntoView');
});

const catalog = [
  {
    ref: 'example/existing',
    providerId: 'example',
    modelId: 'existing',
    name: 'Existing',
    isCustom: true,
    variants: [{ id: 'fast' }],
  },
  {
    ref: 'example/other',
    providerId: 'example',
    modelId: 'other',
    name: 'Other',
    isCustom: true,
    variants: [{ id: 'slow' }],
  },
];
const config = (overrides: Record<string, unknown> = {}) =>
  ({
    preferences: { locale: 'en' },
    providers: [{ name: 'example', models: {} }],
    agents: [
      { id: 'build', source: 'inline' },
      { id: 'plan', source: 'inline' },
    ],
    saving: false,
    formResetVersion: 0,
    catalogLoading: false,
    catalogModels: () => catalog,
    models: () => [],
    ...overrides,
  }) as ConfigStore;

async function select(label: string, option: string) {
  await fireEvent.click(screen.getByRole('button', { name: label }));
  await fireEvent.click(within(screen.getByRole('listbox')).getByRole('option', { name: option }));
}

test('a group requires a nonblank name', async () => {
  const saveGroup = vi.fn();
  render(GroupForm, {}, { wrapper: Harness, wrapperProps: { config: config({ saveGroup }) } });
  await fireEvent.input(screen.getByRole('textbox', { name: 'Group name' }), { target: { value: '   ' } });
  await fireEvent.click(screen.getByRole('button', { name: 'Save group' }));

  expect(saveGroup).not.toHaveBeenCalled();
  expect(screen.getByRole('alert').textContent).toContain('Group name is required.');
});

test('a new group trims its name and saves empty mappings', async () => {
  const saveGroup = vi.fn().mockResolvedValue(undefined);
  render(GroupForm, {}, { wrapper: Harness, wrapperProps: { config: config({ saveGroup }) } });
  await fireEvent.input(screen.getByRole('textbox', { name: 'Group name' }), { target: { value: '  My group  ' } });
  await fireEvent.click(screen.getByRole('button', { name: 'Save group' }));

  await waitFor(() => expect(saveGroup).toHaveBeenCalledOnce());
  expect(saveGroup).toHaveBeenCalledWith({
    id: expect.any(String),
    name: 'My group',
    description: '',
    type: 'native',
    openCodeAgentOverrides: [],
    slimAgentOverrides: null,
    omoAgentOverrides: null,
    omoCategoryMappings: null,
    isEnabled: false,
    updatedAt: expect.any(String),
  });
});

test('editing a group preserves its identity while changing native agent, model, and variant mappings', async () => {
  const saveGroup = vi.fn().mockResolvedValue(undefined);
  const group: Group = {
    id: 'group-1',
    name: 'Old name',
    description: 'Existing description',
    type: 'native',
    openCodeAgentOverrides: [{ agentName: 'build', modelRef: 'example/existing', variant: 'fast' }],
    slimAgentOverrides: null,
    omoAgentOverrides: null,
    omoCategoryMappings: null,
    isEnabled: true,
    updatedAt: '2025-01-01T00:00:00.000Z',
  };
  render(GroupForm, { group }, { wrapper: Harness, wrapperProps: { config: config({ saveGroup }) } });

  await waitFor(() =>
    expect((screen.getByRole('textbox', { name: 'Group name' }) as HTMLInputElement).value).toBe('Old name'),
  );
  expect((screen.getByRole('combobox', { name: 'Mapping name' }) as HTMLInputElement).value).toBe('build');
  expect(screen.getByRole('button', { name: 'Mapping variant' }).textContent).toContain('fast');
  await fireEvent.click(screen.getByRole('button', { name: 'Add mapping' }));
  expect(screen.getAllByRole('button', { name: 'Remove mapping' })).toHaveLength(2);
  await fireEvent.click(screen.getAllByRole('button', { name: 'Remove mapping' })[0]);
  expect(screen.getAllByRole('button', { name: 'Remove mapping' })).toHaveLength(1);
  await fireEvent.focus(screen.getByRole('combobox', { name: 'Mapping name' }));
  await fireEvent.click(screen.getByRole('option', { name: /plan/ }));
  await select('Model reference', 'example/other');
  await select('Mapping variant', 'slow');
  await fireEvent.input(screen.getByRole('textbox', { name: 'Group name' }), { target: { value: '  Renamed  ' } });
  await fireEvent.click(screen.getByRole('button', { name: 'Save group' }));

  await waitFor(() => expect(saveGroup).toHaveBeenCalledOnce());
  expect(saveGroup).toHaveBeenCalledWith({
    ...group,
    name: 'Renamed',
    openCodeAgentOverrides: [{ agentName: 'plan', modelRef: 'example/other', variant: 'slow' }],
  });
});

test('switching type with native mappings requires confirmation and keeps mappings on cancel', async () => {
  const saveGroup = vi.fn().mockResolvedValue(undefined);
  render(GroupForm, {}, { wrapper: Harness, wrapperProps: { config: config({ saveGroup }) } });
  await fireEvent.input(screen.getByRole('textbox', { name: 'Group name' }), { target: { value: 'Switchable' } });
  await fireEvent.click(screen.getByRole('button', { name: 'Add mapping' }));
  await fireEvent.click(screen.getByRole('radio', { name: 'OMO' }));

  const dialog = await screen.findByRole('dialog', { name: 'Switch group type' });
  expect((screen.getByRole('radio', { name: 'Native' }) as HTMLInputElement).checked).toBe(true);
  await fireEvent.click(within(dialog).getByRole('button', { name: 'Cancel' }));
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect(screen.getByRole('button', { name: 'Remove mapping' })).toBeTruthy();
  await fireEvent.click(screen.getByRole('radio', { name: 'OMO' }));
  await fireEvent.click(within(await screen.findByRole('dialog')).getByRole('button', { name: 'Confirm' }));
  await waitFor(() => expect((screen.getByRole('radio', { name: 'OMO' }) as HTMLInputElement).checked).toBe(true));
  await fireEvent.click(screen.getByRole('button', { name: 'Save group' }));

  await waitFor(() => expect(saveGroup).toHaveBeenCalledOnce());
  expect(saveGroup).toHaveBeenCalledWith(
    expect.objectContaining({
      type: 'omo',
      openCodeAgentOverrides: [{ agentName: '', modelRef: 'example/existing' }],
      omoAgentOverrides: null,
      omoCategoryMappings: null,
    }),
  );
});

test('OMO agent and category mappings support add, remove, model and variant selection', async () => {
  const saveGroup = vi.fn().mockResolvedValue(undefined);
  render(GroupForm, {}, { wrapper: Harness, wrapperProps: { config: config({ saveGroup }) } });
  await fireEvent.input(screen.getByRole('textbox', { name: 'Group name' }), { target: { value: 'OMO group' } });
  await fireEvent.click(screen.getByRole('radio', { name: 'OMO' }));
  await fireEvent.click(screen.getByRole('button', { name: 'Add mapping' }));
  await fireEvent.focus(screen.getByRole('combobox', { name: 'Mapping name' }));
  await fireEvent.click(screen.getAllByRole('option', { name: /sisyphus/ })[0]);
  await select('Mapping variant', 'fast');

  await fireEvent.click(screen.getByRole('tab', { name: 'OMO Categories' }));
  await fireEvent.click(screen.getByRole('button', { name: 'Add mapping' }));
  await fireEvent.click(screen.getByRole('button', { name: 'Remove mapping' }));
  expect(screen.queryByRole('button', { name: 'Remove mapping' })).toBeNull();
  await fireEvent.click(screen.getByRole('button', { name: 'Add mapping' }));
  await fireEvent.focus(screen.getByRole('combobox', { name: 'Mapping name' }));
  await fireEvent.click(screen.getByRole('option', { name: 'quick' }));
  await select('Model reference', 'example/other');
  await select('Mapping variant', 'slow');
  await fireEvent.click(screen.getByRole('button', { name: 'Save group' }));

  await waitFor(() => expect(saveGroup).toHaveBeenCalledOnce());
  expect(saveGroup).toHaveBeenCalledWith(
    expect.objectContaining({
      name: 'OMO group',
      type: 'omo',
      openCodeAgentOverrides: [],
      slimAgentOverrides: null,
      omoAgentOverrides: [{ agentName: 'sisyphus', modelRef: 'example/existing', variant: 'fast' }],
      omoCategoryMappings: [{ categoryName: 'quick', modelRef: 'example/other', variant: 'slow' }],
    }),
  );
});
