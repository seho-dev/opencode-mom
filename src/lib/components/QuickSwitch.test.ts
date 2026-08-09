import { cleanup, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import QuickSwitch from './QuickSwitch.svelte';
import { createQuickSwitchStore } from '../appStore';
import type { QuickSwitchCommandClient } from '../contracts';
import { appState, client, currentGroupId, nextGroupId, noEnabledGroupsState } from './QuickSwitchTestSupport';

describe('QuickSwitch', () => {
  afterEach(() => {
    cleanup();
  });

  it('Given the Quick Switch surface When rendered Then the root exposes the stable full-surface hook', () => {
    const store = createQuickSwitchStore(client({ loadResponse: new Promise(() => undefined) }));

    const { container } = render(QuickSwitch, { props: { store } });

    expect(container.querySelector('main[data-surface="quick-switch"]')).toBeInstanceOf(HTMLElement);
  });

  it('Given loaded state When rendered Then current group, mappings, and targets are visible', async () => {
    const store = createQuickSwitchStore(client());

    render(QuickSwitch, { props: { store } });

    expect(await screen.findByRole('heading', { name: 'Default' })).toBeInstanceOf(HTMLElement);
    expect(screen.getByText('anthropic/claude-sonnet-4')).toBeInstanceOf(HTMLElement);
    expect(screen.getByText('anthropic/claude-opus-4')).toBeInstanceOf(HTMLElement);
    expect(screen.getByRole('button', { name: 'Switch to Research' }).hasAttribute('disabled')).toBe(false);
    expect(screen.getByText('Active')).toBeInstanceOf(HTMLElement);

    expect(screen.queryByRole('button', { name: 'Settings' })).toBeNull();
  });

  it('Given a slow load When rendered Then the loading state reserves the quick switch surface', () => {
    const store = createQuickSwitchStore(client({ loadResponse: new Promise(() => undefined) }));

    render(QuickSwitch, { props: { store } });

    expect(screen.getByText('Loading groups...')).toBeInstanceOf(HTMLElement);
  });

  it('Given a slow load When loading is announced Then the surface exposes polite busy semantics', () => {
    const store = createQuickSwitchStore(client({ loadResponse: new Promise(() => undefined) }));

    const { container } = render(QuickSwitch, { props: { store } });

    const loadingRegion = container.querySelector('[aria-live="polite"][aria-busy="true"]');
    expect(loadingRegion).toBeInstanceOf(HTMLElement);
    expect(loadingRegion?.textContent).toContain('Loading groups...');
  });

  it('Given a never-resolving load When rendered Then loading has no interactive footer controls', () => {
    const store = createQuickSwitchStore(client({ loadResponse: new Promise(() => undefined) }));

    render(QuickSwitch, { props: { store } });

    expect(screen.queryByRole('button', { name: 'Settings' })).toBeNull();
    expect(screen.queryByRole('contentinfo')).toBeNull();
  });

  it('Given a rejected load When rendered Then the load failure is visible without leaking Settings', async () => {
    const store = createQuickSwitchStore(client({
      loadResponse: Promise.reject(new Error('load failed'))
    }));

    render(QuickSwitch, { props: { store } });

    expect(await screen.findByRole('heading', { name: 'Unable to load groups' })).toBeInstanceOf(HTMLElement);
    expect(screen.getByText('load failed')).toBeInstanceOf(HTMLElement);
    expect(screen.getByRole('button', { name: 'Retry' })).toBeInstanceOf(HTMLButtonElement);
    expect(screen.queryByText('Loading groups...')).toBeNull();
    expect(screen.queryByRole('button', { name: 'Settings' })).toBeNull();
  });

  it('Given persisted warnings and an OpenCode discovery error When loaded Then both status messages remain visible', async () => {
    const store = createQuickSwitchStore(client({
      state: {
        ...appState,
        openCodeAgentDiscoveryError: 'OpenCode config is malformed.'
      }
    }));

    render(QuickSwitch, { props: { store } });

    expect(await screen.findByText('OpenCode config was skipped.')).toBeInstanceOf(HTMLElement);
    expect(screen.getByText('OpenCode agent discovery warning: OpenCode config is malformed.')).toBeInstanceOf(HTMLElement);
    expect(screen.getAllByRole('status')).toHaveLength(2);
  });

  it('Given loaded state When initial loading completes Then focus moves to the current group summary', async () => {
    const store = createQuickSwitchStore(client());

    render(QuickSwitch, { props: { store } });

    const summary = await screen.findByLabelText('Current group summary');
    await waitFor(() => expect(document.activeElement).toBe(summary));
  });

  it('Given loaded state When root controls render Then the surface has no Settings control', async () => {
    const store = createQuickSwitchStore(client());

    render(QuickSwitch, { props: { store } });

    await screen.findByRole('heading', { name: 'Default' });

    expect(screen.queryByRole('button', { name: 'Settings' })).toBeNull();
  });

  it('Given no enabled groups When rendered Then the no enabled groups empty state is visible', async () => {
    const store = createQuickSwitchStore(client({ state: noEnabledGroupsState }));

    render(QuickSwitch, { props: { store } });

    expect(await screen.findByText('No enabled groups')).toBeInstanceOf(HTMLElement);
    expect(screen.queryByRole('button', { name: /Switch to/ })).toBeNull();
  });

  it('Given a switchable group When clicked Then success updates the current marker', async () => {
    const store = createQuickSwitchStore(client({
      switchResponse: {
        outcome: 'success',
        warnings: [],
        appState: {
          ...appState.appState,
          selectedGroupID: nextGroupId,
          selectedGroupName: 'Research',
          lastWarningSummary: null
        }
      }
    }));

    render(QuickSwitch, { props: { store } });

    (await screen.findByRole('button', { name: 'Switch to Research' })).click();

    expect(await screen.findByRole('heading', { name: 'Research' })).toBeInstanceOf(HTMLElement);
    expect(screen.queryByRole('button', { name: 'Switch to Research' })).toBeNull();
  });

  it('Given the active group When its no-op state renders Then it has an active marker and no action', async () => {
    const store = createQuickSwitchStore(client());

    render(QuickSwitch, { props: { store } });

    expect(await screen.findByText('Active')).toBeInstanceOf(HTMLElement);
    expect(screen.queryByRole('button', { name: 'Switch to Default' })).toBeNull();
  });

  it('Given switch rejection When clicked Then the failure is visible and targets stay enabled for a retry', async () => {
    const store = createQuickSwitchStore(client({ switchError: 'Group is disabled.' }));

    render(QuickSwitch, { props: { store } });

    (await screen.findByRole('button', { name: 'Switch to Research' })).click();

    expect(await screen.findByText('Group is disabled.')).toBeInstanceOf(HTMLElement);
    await waitFor(() => expect(screen.getByRole('button', { name: 'Switch to Research' }).hasAttribute('disabled')).toBe(false));
  });

  it('Given the loaded quick switch When tab order is inspected Then switch controls follow visual order', async () => {
    const store = createQuickSwitchStore(client());
    render(QuickSwitch, { props: { store } });
    await screen.findByRole('heading', { name: 'Default' });

    const focusOrder = Array.from(document.querySelectorAll<HTMLElement>('button:not([disabled])'))
      .map((element) => element.getAttribute('aria-label') ?? element.textContent?.trim());
    expect(focusOrder).toEqual(['Switch to Research']);
  });

  it('Given a focused switch action When switching succeeds Then focus remains on the corresponding active row', async () => {
    const store = createQuickSwitchStore(client({
      switchResponse: {
        outcome: 'success',
        warnings: [],
        appState: {
          ...appState.appState,
          selectedGroupID: nextGroupId,
          selectedGroupName: 'Research',
          lastWarningSummary: null
        }
      }
    }));

    render(QuickSwitch, { props: { store } });

    const switchButton = await screen.findByRole('button', { name: 'Switch to Research' });
    switchButton.focus();
    switchButton.click();

    const activeTarget = await screen.findByLabelText('Research, active group');
    await waitFor(() => expect(document.activeElement).toBe(activeTarget));
  });
});
