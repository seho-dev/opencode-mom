<script lang="ts">
  import '../app.css';
  import AppShell from '$lib/components/app/AppShell.svelte';
  import { createCommandAdapter } from '$lib/features/config/adapter.js';
  import { createConfigStore } from '$lib/features/config/store.svelte.js';
  import { getConfig, setConfig } from '$lib/features/config/context.js';
  setConfig(createConfigStore(createCommandAdapter()));
  const config = getConfig()!;
  // Prefetch the opencode model catalog at startup so list pages can render from the shared store directly.
  void config.loadCatalog().catch(() => {});
</script>

<AppShell><slot /></AppShell>
