import type { UpdateCheck } from '$src/types/app.js';

// Reactive stand-in for the parts of ConfigStore that the shell update flow reads.
// Lives in a `.svelte.ts` module so `$state` fields stay reactive inside plain component tests.
export function createShellConfig(updateCheck: UpdateCheck | null = null) {
  const config = $state({
    preferences: { locale: 'en', theme: 'dark' },
    loading: false,
    saving: false,
    switching: false,
    reloading: false,
    error: null,
    groups: [],
    providers: [],
    agents: [],
    selectedGroupId: null,
    updateCheck,
  });

  return config;
}
