<script lang="ts">
import '../app.css';
import { createCommandAdapter } from '$src/config/adapter.js';
import { getConfig, setConfig } from '$src/config/context.js';
import { createConfigStore } from '$src/config/store.svelte.js';
import { setI18n } from '$src/i18n/context.js';
import { createI18n } from '$src/i18n/i18n.svelte.js';
import AppShell from '$src/shell/AppShell.svelte';
import SplashScreen from '$src/shell/SplashScreen.svelte';

setConfig(createConfigStore(createCommandAdapter()));
const config = getConfig()!;
setI18n(createI18n(() => config.preferences.locale));
</script>

<AppShell><slot /></AppShell>
<SplashScreen visible={config.splashLoading} />
