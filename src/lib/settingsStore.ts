import { derived, get, writable } from 'svelte/store';

import type { ModelGroup, SettingsCommandClient, Uuid } from './contracts';
import { createSettingsGroupActions } from './settingsGroupActions';
import { cloneGroup, discoverPresentation, emptyOpenCodePresentation } from './settingsGroupDraft';
import { createSettingsMatchActions } from './settingsMatchActions';
import {
  deriveSettingsState,
  errorMessage,
  initialSettingsFacts,
  type SettingsFacts,
  type SettingsState,
  type StatusMessage
} from './settingsState';
import { tauriSettingsCommandClient } from './tauriClient';

export type { SettingsState, StatusMessage } from './settingsState';

export type SettingsStore = Readonly<{
  subscribe: ReturnType<typeof writable<SettingsState>>['subscribe'];
  load: () => Promise<void>;
  selectGroup: (id: Uuid) => Promise<void>;
  createGroup: () => Promise<void>;
  copySelectedGroup: () => Promise<void>;
  deleteSelectedGroup: () => Promise<void>;
  saveDraftGroup: () => Promise<void>;
  cancelDraftGroup: () => Promise<void>;
  switchToSelectedGroup: () => Promise<void>;
  updateDraftGroup: (patch: Partial<ModelGroup>) => Promise<void>;
  setMatchSearch: (value: string) => Promise<void>;
  setMatchReplace: (value: string) => void;
  replaceExactMatches: () => Promise<void>;
}>;

export const createSettingsStore = (client: SettingsCommandClient): SettingsStore => {
  const factsStore = writable<SettingsFacts>(initialSettingsFacts);
  const publicStore = derived(factsStore, deriveSettingsState);
  const readFacts = (): SettingsFacts => get(factsStore);
  const readState = (): SettingsState => deriveSettingsState(readFacts());
  const updateFacts = (updater: (facts: SettingsFacts) => SettingsFacts): void => factsStore.update(updater);
  let groupContextRevision = 0;

  const load = async (): Promise<void> => {
    updateFacts((facts) => ({ ...facts, status: 'loading', message: null }));
    try {
      const response = await client.loadAppState();
      const selected = response.groups.find((group) => group.id === response.appState.selectedGroupID) ?? response.groups[0] ?? null;
      const [openCodePresentationResult] = await Promise.allSettled([
        selected ? discoverPresentation(client, selected) : Promise.resolve(emptyOpenCodePresentation)
      ] as const);
      factsStore.set({
        ...initialSettingsFacts,
        status: 'ready',
        persistedGroups: response.groups,
        appState: response.appState,
        draftGroup: selected ? cloneGroup(selected) : null,
        openCodePresentation: openCodePresentationResult.status === 'fulfilled' ? openCodePresentationResult.value : emptyOpenCodePresentation
      });
    } catch (error) {
      updateFacts((facts) => ({ ...facts, status: 'error', message: errorMessage(error) }));
    }
  };

  const advanceGroupContext = (): void => { groupContextRevision += 1; };
  const matchActions = createSettingsMatchActions({ readFacts, updateFacts, advanceContext: advanceGroupContext });
  const groupActions = createSettingsGroupActions({
    client,
    readFacts,
    readState,
    updateFacts,
    advanceContext: advanceGroupContext,
    currentContext: () => groupContextRevision,
    matchActions
  });
  return {
    subscribe: publicStore.subscribe,
    load,
    ...groupActions,
    setMatchSearch: matchActions.setMatchSearch,
    setMatchReplace: matchActions.setMatchReplace,
    replaceExactMatches: matchActions.replaceExactMatches
  };
};

export const settingsStore = createSettingsStore(tauriSettingsCommandClient);
