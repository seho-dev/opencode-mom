<script lang="ts">
  import '../app.css';
  import AppShell from '$lib/components/app/AppShell.svelte';
  import SplashScreen from '$lib/components/app/SplashScreen.svelte';
  import { createCommandAdapter } from '$lib/features/config/adapter.js';
  import { createConfigStore } from '$lib/features/config/store.svelte.js';
  import { getConfig, setConfig } from '$lib/features/config/context.js';
  import { createI18n } from '$lib/features/i18n/i18n.svelte.js';
  import { setI18n } from '$lib/features/i18n/context.js';
  setConfig(createConfigStore(createCommandAdapter()));
  const config = getConfig()!;
  setI18n(createI18n(() => config.preferences.locale));
  void config.loadCatalog().catch(() => {});
</script>

<AppShell><slot /></AppShell>
<SplashScreen visible={config.loading} />
