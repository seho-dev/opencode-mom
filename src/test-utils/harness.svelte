<script lang="ts">
import type { Snippet } from 'svelte';
import { setConfig } from '$src/config/context.js';
import type { ConfigStore } from '$src/config/store.svelte.js';
import { setI18n } from '$src/i18n/context.js';
import { createI18n } from '$src/i18n/i18n.svelte.js';

let { config, children }: { config: ConfigStore; children: Snippet } = $props();

// svelte-ignore state_referenced_locally
setConfig(config);
setI18n(createI18n(() => config.preferences.locale));
</script>

{@render children()}
