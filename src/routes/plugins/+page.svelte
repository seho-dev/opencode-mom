<script lang="ts">
import { getI18n } from '$src/i18n/context.js';
import PluginRuntime from './PluginRuntime.svelte';

const i18n = getI18n();
</script>

<svelte:head><title>{i18n.t('plugins.metaTitle')}</title></svelte:head>
<PluginRuntime />
