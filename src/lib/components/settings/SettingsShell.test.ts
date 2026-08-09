import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

vi.mock('tabbable', async (importOriginal) => {
  const actual = await importOriginal<typeof import('tabbable')>();
  const withJsdomDisplayCheck = <T extends Record<string, unknown> | undefined>(options: T) => ({
    ...options,
    displayCheck: 'none' as const
  });

  return {
    ...actual,
    tabbable: (node: Parameters<typeof actual.tabbable>[0], options?: Parameters<typeof actual.tabbable>[1]) => actual.tabbable(node, withJsdomDisplayCheck(options)),
    focusable: (node: Parameters<typeof actual.focusable>[0], options?: Parameters<typeof actual.focusable>[1]) => actual.focusable(node, withJsdomDisplayCheck(options)),
    isFocusable: (node: Parameters<typeof actual.isFocusable>[0], options?: Parameters<typeof actual.isFocusable>[1]) => actual.isFocusable(node, withJsdomDisplayCheck(options))
  };
});

import { createSettingsStore } from '../../settingsStore';
import { degradedSettingsFixtureClient, settingsFixtureClient } from '../../settingsFixture';
import type { SettingsCommandClient } from '../../contracts';
import SettingsShell from './SettingsShell.svelte';

const isCheckboxChecked = (checkbox: HTMLElement): boolean => checkbox instanceof HTMLInputElement
  ? checkbox.checked
  : checkbox.getAttribute('aria-checked') === 'true';

const pressSpace = async (checkbox: HTMLElement): Promise<void> => {
  const wasChecked = isCheckboxChecked(checkbox);
  checkbox.focus();
  await fireEvent.keyDown(checkbox, { key: ' ', code: 'Space' });
  await fireEvent.keyUp(checkbox, { key: ' ', code: 'Space' });

  if (isCheckboxChecked(checkbox) === wasChecked && checkbox instanceof HTMLInputElement) {
    await fireEvent.click(checkbox);
  }

  await waitFor(() => expect(isCheckboxChecked(checkbox)).toBe(!wasChecked));
};

describe('SettingsShell', () => {
  afterEach(() => cleanup());

  const pendingLoadClient: SettingsCommandClient = {
    ...settingsFixtureClient,
    loadAppState: () => new Promise(() => undefined)
  };

  it('Given the Settings surface When rendered Then the root exposes the stable full-surface hook', () => {
    const { container } = render(SettingsShell, { props: { store: createSettingsStore(settingsFixtureClient) } });

    expect(container.querySelector('main[data-surface="settings"]')).toBeInstanceOf(HTMLElement);
  });

  it('Given a never-resolving load When mounted Then loading owns the detail region and panes do not leak', async () => {
    render(SettingsShell, { props: { store: createSettingsStore(pendingLoadClient) } });

    expect(await screen.findByText('Loading settings...')).toBeInstanceOf(HTMLElement);
    expect(screen.getByRole('main', { name: 'Model group settings' }).querySelector('.settings-detail')?.getAttribute('aria-live')).toBe('polite');
    expect(screen.queryByRole('heading', { name: 'Group Settings' })).toBeNull();
    expect(screen.getByText('Loading settings...').closest('[aria-busy="true"]')).toBeInstanceOf(HTMLElement);
  });

  it('Given an initial app-state load failure When mounted Then error and retry own the detail region', async () => {
    const loadAppState = vi.fn<SettingsCommandClient['loadAppState']>()
      .mockRejectedValueOnce({ code: 'loadAppStateFailed', message: 'Settings load failed.', detail: 'Settings data is unavailable.' })
      .mockImplementationOnce(settingsFixtureClient.loadAppState);
    render(SettingsShell, { props: { store: createSettingsStore({ ...settingsFixtureClient, loadAppState }) } });

    expect(await screen.findByRole('heading', { name: 'Unable to load settings' })).toBeInstanceOf(HTMLElement);
    expect(screen.getByText('Settings data is unavailable.')).toBeInstanceOf(HTMLElement);
    expect(screen.getByRole('button', { name: 'Retry' })).toBeInstanceOf(HTMLButtonElement);
    expect(screen.queryByRole('heading', { name: 'Group Settings' })).toBeNull();

    await fireEvent.click(screen.getByRole('button', { name: 'Retry' }));

    expect(await screen.findByRole('heading', { name: 'Group Settings' })).toBeInstanceOf(HTMLElement);
    expect(loadAppState).toHaveBeenCalledTimes(2);
  });

  it('Given loaded canonical facts and a dirty draft When a later load fails Then detail remains usable with the error message', async () => {
    const loadAppState = vi.fn<SettingsCommandClient['loadAppState']>()
      .mockImplementationOnce(settingsFixtureClient.loadAppState)
      .mockRejectedValueOnce({ code: 'loadAppStateFailed', message: 'Settings reload failed.', detail: 'Settings data is unavailable.' });
    const store = createSettingsStore({ ...settingsFixtureClient, loadAppState });
    render(SettingsShell, { props: { store } });

    const name = await screen.findByLabelText('Name');
    await fireEvent.input(name, { target: { value: 'Local draft' } });
    await store.load();

    expect(screen.queryByRole('heading', { name: 'Unable to load settings' })).toBeNull();
    expect(screen.getByText('Settings data is unavailable.')).toBeInstanceOf(HTMLElement);
    expect((screen.getByLabelText('Name') as HTMLInputElement).value).toBe('Local draft');
  });

  it('Given fixture data When rendered Then group list, active badge, metadata, mappings, overrides, and OpenCode editors are visible', async () => {
    render(SettingsShell, { props: { store: createSettingsStore(settingsFixtureClient) } });

    expect(await screen.findByRole('heading', { name: 'Group Settings' })).toBeInstanceOf(HTMLElement);
    expect(screen.getByText('Default')).toBeInstanceOf(HTMLElement);
    expect(screen.getByText('Active')).toBeInstanceOf(HTMLElement);
    expect(screen.getByLabelText('Name')).toBeInstanceOf(HTMLInputElement);
    expect(screen.getByText('Category Mappings')).toBeInstanceOf(HTMLElement);
    expect(screen.getByText('Agent Overrides')).toBeInstanceOf(HTMLElement);
    expect(screen.getByText('OpenCode Agent Overrides')).toBeInstanceOf(HTMLElement);
    expect(screen.getByLabelText('OpenCode model reviewer')).toBeInstanceOf(HTMLInputElement);
  });

  it('Given editable model groups When Enabled receives Space Then its checked state changes through the accessible control', async () => {
    render(SettingsShell, { props: { store: createSettingsStore(settingsFixtureClient) } });

    const enabled = await screen.findByRole('checkbox', { name: 'Enabled' });
    await pressSpace(enabled);
  });

  it('Given duplicate name edit When saving Then duplicate-name validation is visible', async () => {
    render(SettingsShell, { props: { store: createSettingsStore(settingsFixtureClient) } });

    const name = await screen.findByLabelText('Name');
    (name as HTMLInputElement).value = 'Research';
    await fireEvent.input(name);

    expect(await screen.findByText('A group named "Research" already exists.')).toBeInstanceOf(HTMLElement);
    expect(screen.getByRole('button', { name: /^Save$/ }).hasAttribute('disabled')).toBe(true);
  });

  it('Given a new group When created Then it appears in the group list before persistence', async () => {
    render(SettingsShell, { props: { store: createSettingsStore(settingsFixtureClient) } });

    const newGroupButton = (await screen.findAllByRole('button', { name: 'New Group' }))[0];
    if (!newGroupButton) expect.fail('Expected at least one New Group action.');
    await fireEvent.click(newGroupButton);

    expect(await screen.findByText('Untitled Group')).toBeInstanceOf(HTMLElement);
    expect(screen.getByRole('button', { name: /Switch To This Group/ }).hasAttribute('disabled')).toBe(true);
  });

  it('Given OpenCode discovery failure When rendered Then preserved overrides are read-only', async () => {
    render(SettingsShell, { props: { store: createSettingsStore(degradedSettingsFixtureClient) } });

    expect(await screen.findByText('OpenCode agent discovery warning: OpenCode config is malformed.')).toBeInstanceOf(HTMLElement);
    expect(screen.getByText('Editing disabled until OpenCode agent discovery succeeds.')).toBeInstanceOf(HTMLElement);
    expect(screen.getByLabelText('OpenCode override count').textContent).toBe('1');
    expect(screen.queryByLabelText('OpenCode model reviewer')).toBeNull();
  });

  it('Given discovered OpenCode override When edited Then the input remains bound to the draft value', async () => {
    render(SettingsShell, { props: { store: createSettingsStore(settingsFixtureClient) } });

    const input = await screen.findByLabelText('OpenCode model reviewer');
    (input as HTMLInputElement).value = 'openai/gpt-5.2';
    await fireEvent.input(input);

    expect((screen.getByLabelText('OpenCode model reviewer') as HTMLInputElement).value).toBe('openai/gpt-5.2');
  });

  it('Given delete confirmation When opened Then focus stays inside and Escape closes it', async () => {
    render(SettingsShell, { props: { store: createSettingsStore(settingsFixtureClient) } });

    const deleteTrigger = await screen.findByRole('button', { name: /^Delete$/ });
    deleteTrigger.focus();
    await fireEvent.click(deleteTrigger);
    const dialog = await screen.findByRole('dialog', { name: /Delete Default/ });
    const cancel = screen.getAllByRole('button', { name: /^Cancel$/ }).at(-1);
    await waitFor(() => expect(dialog.contains(document.activeElement)).toBe(true));
    expect(cancel).toBeInstanceOf(HTMLButtonElement);
    await fireEvent.keyDown(dialog, { key: 'Escape' });
    expect(screen.queryByRole('dialog', { name: /Delete Default/ })).toBeNull();
    await waitFor(() => expect(document.activeElement).toBe(deleteTrigger));
  });

  it('Given delete confirmation When focus reaches either boundary or interaction occurs outside Then focus cycles and the dialog persists', async () => {
    render(SettingsShell, { props: { store: createSettingsStore(settingsFixtureClient) } });

    const deleteTrigger = await screen.findByRole('button', { name: /^Delete$/ });
    await fireEvent.click(deleteTrigger);
    const dialog = await screen.findByRole('dialog', { name: /Delete Default/ });
    const cancel = within(dialog).getByRole('button', { name: /^Cancel$/ });
    const confirm = within(dialog).getByRole('button', { name: 'Delete Group' });

    await waitFor(() => expect(document.activeElement).toBe(cancel));
    confirm.focus();
    await fireEvent.keyDown(confirm, { key: 'Tab' });
    expect(document.activeElement).toBe(cancel);

    await fireEvent.keyDown(cancel, { key: 'Tab', shiftKey: true });
    expect(document.activeElement).toBe(confirm);

    await fireEvent.pointerDown(document.body);
    await fireEvent.mouseDown(document.body);
    await fireEvent.click(document.body);
    expect(screen.getByRole('dialog', { name: /Delete Default/ })).toBe(dialog);
  });

  it('Given batch replace controls When exact match is replaced Then the matching category model updates', async () => {
    render(SettingsShell, { props: { store: createSettingsStore(settingsFixtureClient) } });

    const find = await screen.findByLabelText('Find exact model');
    (find as HTMLInputElement).value = 'anthropic/claude-sonnet-4';
    await fireEvent.input(find);

    await waitFor(() => expect(screen.getByText('1 matches')).toBeInstanceOf(HTMLElement));
    const replace = screen.getByLabelText('Replace with');
    (replace as HTMLInputElement).value = 'openai/gpt-5.2';
    await fireEvent.input(replace);
    await fireEvent.click(screen.getByRole('button', { name: 'Replace All Exact Matches' }));

    expect((await screen.findByLabelText('Category model 1') as HTMLInputElement).value).toBe('openai/gpt-5.2');
  });
});
