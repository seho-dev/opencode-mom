<script lang="ts">
  import '../app.css';
  import AppShell from '$lib/components/app/AppShell.svelte';
  import { createBrowserMockAdapter, createCommandAdapter } from '$lib/features/config/adapter.js';
  import { createConfigStore } from '$lib/features/config/store.svelte.js';
  import { setConfig } from '$lib/features/config/context.js';
  const inTauri =
    typeof window !== 'undefined' &&
    Boolean((window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);
  setConfig(createConfigStore(inTauri ? createCommandAdapter() : createBrowserMockAdapter()));
</script>

<AppShell><slot /></AppShell>
