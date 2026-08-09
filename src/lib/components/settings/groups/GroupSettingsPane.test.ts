import { cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import { createSettingsStore } from '../../../settingsStore';
import { degradedSettingsFixtureClient, settingsFixtureClient } from '../../../settingsFixture';
import GroupSettingsPane from './GroupSettingsPane.svelte';
import DeleteGroupDialog from './DeleteGroupDialog.svelte';
import SettingsShell from '../SettingsShell.svelte';

const renderPane = async (client: typeof settingsFixtureClient) => {
  const store = createSettingsStore(client);
  let state!: import('../../../settingsStore').SettingsState;
  store.subscribe((next) => { state = next; });
  await store.load();
  return { store, getState: () => state };
};

describe('Groups settings components', () => {
  afterEach(() => cleanup());

  it('keeps list actions, selection, badges, metadata, mappings, and replace behavior', async () => {
    render(SettingsShell, { props: { store: createSettingsStore(settingsFixtureClient) } });

    expect(await screen.findByRole('heading', { name: 'Group Settings' })).toBeTruthy();
    expect(screen.getByText('Active')).toBeTruthy();
    expect(screen.getByLabelText('Name')).toBeTruthy();
    expect(screen.getByText('Category Mappings')).toBeTruthy();
    expect(screen.getByText('Agent Overrides')).toBeTruthy();
    expect(document.querySelector('[data-group-region="metadata"]')).toBeTruthy();
    expect(document.querySelector('[data-group-region="exact-model-replacement"]')).toBeTruthy();
    expect(document.querySelector('[data-group-region="standard-mappings"]')).toBeTruthy();
    expect(document.querySelector('[data-group-region="opencode-overrides"]')).toBeTruthy();

    const find = screen.getByLabelText('Find exact model') as HTMLInputElement;
    find.value = 'anthropic/claude-sonnet-4';
    await fireEvent.input(find);
    const replace = screen.getByLabelText('Replace with') as HTMLInputElement;
    replace.value = 'openai/gpt-5.2';
    await fireEvent.input(replace);
    await fireEvent.click(screen.getByRole('button', { name: 'Replace All Exact Matches' }));
    await waitFor(() => expect((screen.getByLabelText('Category model 1') as HTMLInputElement).value).toBe('openai/gpt-5.2'));

    expect(screen.getAllByRole('button', { name: 'New Group' }).length).toBeGreaterThan(0);
    expect(screen.getAllByRole('button', { name: 'Copy group' }).length).toBeGreaterThan(0);
  });

  it('uses the controlled Enabled checkbox and preserves degraded OpenCode read-only state', async () => {
    const { store, getState } = await renderPane(degradedSettingsFixtureClient);
    render(GroupSettingsPane, { props: { store, state: getState() } });

    const enabled = await screen.findByRole('checkbox', { name: 'Enabled' });
    const initial = enabled.getAttribute('aria-checked');
    enabled.focus();
    await fireEvent.keyDown(enabled, { key: ' ', code: 'Space' });
    await fireEvent.keyUp(enabled, { key: ' ', code: 'Space' });
    await waitFor(() => expect(enabled.getAttribute('aria-checked')).not.toBe(initial));

    expect(screen.getByRole('alert').textContent).toContain('OpenCode agent discovery warning');
    expect(screen.getByLabelText('OpenCode override count').textContent).toBe('1');
    expect(screen.queryByLabelText('OpenCode model reviewer')).toBeNull();
    const preservedModel = screen.getByDisplayValue('anthropic/claude-opus-4') as HTMLInputElement;
    expect(preservedModel.readOnly).toBe(true);
    expect(preservedModel.disabled).toBe(false);
    expect(document.querySelector('[data-opencode-mode="degraded"]')).toBeTruthy();
  });

  it('closes and restores trigger focus before a deferred delete settles', async () => {
    let settle!: () => void;
    const pending = new Promise<void>((resolve) => { settle = resolve; });
    render(DeleteGroupDialog, { props: { groupName: 'Default', disabled: false, onDelete: () => pending } });
    const trigger = screen.getByRole('button', { name: /^Delete$/ });
    trigger.focus();
    await fireEvent.click(trigger);
    const dialog = await screen.findByRole('dialog', { name: /Delete Default/ });
    expect(dialog).toBeTruthy();
    await fireEvent.pointerDown(document.body);
    expect(screen.getByRole('dialog', { name: /Delete Default/ })).toBe(dialog);

    await fireEvent.click(within(dialog).getByRole('button', { name: 'Delete Group' }));
    expect(screen.queryByRole('dialog')).toBeNull();
    await waitFor(() => expect(document.activeElement).toBe(trigger));
    settle();
  });

  it('keeps duplicate validation and disabled unsaved switching semantics', async () => {
    render(SettingsShell, { props: { store: createSettingsStore(settingsFixtureClient) } });
    const name = await screen.findByLabelText('Name') as HTMLInputElement;
    name.value = 'Research';
    await fireEvent.input(name);
    expect(await screen.findByText('A group named "Research" already exists.')).toBeTruthy();
    expect(screen.getByRole('button', { name: /^Save$/ }).hasAttribute('disabled')).toBe(true);
  });

});
