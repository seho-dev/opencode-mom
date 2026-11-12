import type { ConfigStore } from '$src/config/store.svelte.js';

// Reactive stand-in for the config store: `$state` in `.svelte.ts` lets the Dashboard's entry
// refresh effect observe busy-state changes that a plain object cannot trigger.
export function createDashboardConfig(overrides: Partial<ConfigStore> = {}) {
  const config = $state({
    preferences: { locale: 'en', theme: 'dark' },
    providers: [],
    groups: [],
    selectedGroupId: null,
    loading: false,
    saving: false,
    switching: false,
    reloading: false,
    catalogLoading: false,
    error: null,
    switchGroup: async () => {},
    reloadOpencode: async () => {},
    refresh: async () => {},
    loadTokenUsageSnapshot: async () => ({
      records: [],
      updatedAt: null,
      refreshing: false,
      stale: false,
      error: null,
      resultRevision: 0,
      targetRevision: 0,
      cacheScope: 'process',
    }),
    ...overrides,
  }) as ConfigStore;

  return config;
}
