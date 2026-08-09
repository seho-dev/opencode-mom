import { writable } from 'svelte/store';

import type { AppStateResponse, QuickSwitchCommandClient, Uuid } from './contracts';
import { commandErrorMessage, tauriQuickSwitchCommandClient } from './tauriClient';
import { emptyQuickSwitchView, toQuickSwitchView, type QuickSwitchViewData } from './quickSwitchView';

export type QuickSwitchStatus = 'idle' | 'loading' | 'ready' | 'error';

export type QuickSwitchState = Readonly<{
  status: QuickSwitchStatus;
  response: AppStateResponse | null;
  view: QuickSwitchViewData;
  error: string | null;
  switchingGroupId: Uuid | null;
}>;

export type QuickSwitchStore = Readonly<{
  subscribe: ReturnType<typeof writable<QuickSwitchState>>['subscribe'];
  load: () => Promise<void>;
  switchTo: (id: Uuid) => Promise<void>;
}>;

const initialState: QuickSwitchState = {
  status: 'idle',
  response: null,
  view: emptyQuickSwitchView,
  error: null,
  switchingGroupId: null,
};

export const createQuickSwitchStore = (client: QuickSwitchCommandClient): QuickSwitchStore => {
  const store = writable<QuickSwitchState>(initialState);

  const load = async (): Promise<void> => {
    store.update((state) => ({ ...state, status: 'loading', error: null }));
    try {
      const response = await client.loadAppState();
      store.set({
        status: 'ready',
        response,
        view: toQuickSwitchView(response),
        error: null,
        switchingGroupId: null,
      });
    } catch (error) {
      store.update((state) => ({
        ...state,
        status: 'error',
        error: commandErrorMessage(error),
        switchingGroupId: null
      }));
    }
  };

  const switchTo = async (id: Uuid): Promise<void> => {
    store.update((state) => ({ ...state, error: null, switchingGroupId: id }));
    try {
      const switched = await client.switchGroup(id);
      store.update((state) => {
        if (!state.response) {
          return { ...state, switchingGroupId: null };
        }
        const response: AppStateResponse = {
          ...state.response,
          appState: {
            ...switched.appState,
            lastWarningSummary: warningSummary(switched.warnings)
          }
        };
        return {
          ...state,
          status: 'ready',
          response,
          view: toQuickSwitchView(response),
          error: null,
          switchingGroupId: null
        };
      });
    } catch (error) {
      store.update((state) => ({
        ...state,
        status: state.response ? 'ready' : 'error',
        error: commandErrorMessage(error),
        switchingGroupId: null
      }));
    }
  };

  return {
    subscribe: store.subscribe,
    load,
    switchTo
  };
};


export const quickSwitchStore = createQuickSwitchStore(tauriQuickSwitchCommandClient);
const warningSummary = (warnings: readonly string[]): AppStateResponse['appState']['lastWarningSummary'] => {
  if (warnings.length === 0) {
    return null;
  }
  return {
    message: warnings.join('; '),
    count: warnings.length
  };
};
