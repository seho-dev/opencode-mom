import { get } from 'svelte/store';
import { describe, expect, it } from 'vitest';

import { createQuickSwitchStore } from './appStore';
import type { QuickSwitchCommandClient } from './contracts';
import { currentGroupId, fakeClient, loadedState, nextGroupId, switchedState } from './appStoreTestSupport';

describe('createQuickSwitchStore', () => {
  it('Given a client When loading Then groups, current group, warning, and discovery are derived', async () => {
    const client = fakeClient({ loadResponse: loadedState });
    const store = createQuickSwitchStore(client);

    await store.load();

    const state = get(store);
    expect(state.status).toBe('ready');
    expect(state.view.currentGroup?.id).toBe(currentGroupId);
    expect(state.error).toBeNull();
  });

  it('Given switch success with warning When switching Then the store refreshes and exposes the warning', async () => {
    const client = fakeClient({ loadResponse: loadedState, switchResponse: switchedState });
    const store = createQuickSwitchStore(client);

    await store.load();
    await store.switchTo(nextGroupId);

    const state = get(store);
    expect(state.switchingGroupId).toBeNull();
    expect(state.view.currentGroup?.id).toBe(nextGroupId);
    expect(state.view.warning).toBe('OpenCode config was skipped.');
  });

  it('Given a no-op response When switching Then the current group and status remain stable', async () => {
    const client = fakeClient({
      loadResponse: loadedState,
      switchResponse: {
        outcome: 'noOp',
        warnings: ['Already using this group.'],
        appState: loadedState.appState
      }
    });
    const store = createQuickSwitchStore(client);

    await store.load();
    await store.switchTo(currentGroupId);

    const state = get(store);
    expect(state.status).toBe('ready');
    expect(state.view.currentGroup?.id).toBe(currentGroupId);
    expect(state.view.warning).toBe('Already using this group.');
    expect(state.error).toBeNull();
  });

  it('Given a clean switch success When a prior warning existed Then the warning state is cleared', async () => {
    const client = fakeClient({
      loadResponse: {
        ...loadedState,
        appState: {
          ...loadedState.appState,
          lastWarningSummary: { message: 'Previous warning.', count: 1 }
        }
      },
      switchResponse: {
        outcome: 'success',
        warnings: [],
        appState: {
          ...loadedState.appState,
          selectedGroupID: nextGroupId,
          selectedGroupName: 'Research',
          lastWarningSummary: null
        }
      }
    });
    const store = createQuickSwitchStore(client);

    await store.load();
    await store.switchTo(nextGroupId);

    const state = get(store);
    expect(state.view.currentGroup?.id).toBe(nextGroupId);
    expect(state.view.warning).toBeNull();
  });

  it('Given a rejected switch command When switching Then the group list remains and the error is rendered', async () => {
    const client = fakeClient({ loadResponse: loadedState, switchError: 'Group is disabled.' });
    const store = createQuickSwitchStore(client);

    await store.load();
    await store.switchTo(nextGroupId);

    const state = get(store);
    expect(state.view.currentGroup?.id).toBe(currentGroupId);
    expect(state.error).toBe('Group is disabled.');
  });

});
