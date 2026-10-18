<script lang="ts">
import GlobalDestination from '$src/components/GlobalDestination.svelte';
import PageHead from '$src/components/PageHead.svelte';
import { getI18n } from '$src/i18n/context.js';
import McpForm from '../McpForm.svelte';

const i18n = getI18n();
</script>

<svelte:head><title>{i18n.t('mcp.newMetaTitle')}</title></svelte:head>
<PageHead eyebrow={i18n.t('mcp.eyebrow')} title={i18n.t('mcp.newTitle')} />
<GlobalDestination />
<p class="page-description">{i18n.t('mcp.newDescription')}</p>
<McpForm />
