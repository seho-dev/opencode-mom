<script lang="ts">
import { page } from '$app/state';
import PageHead from '$src/components/PageHead.svelte';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import ProviderForm from '$src/routes/providers/ProviderForm.svelte';

const config = getConfig();
const i18n = getI18n();
const source = $derived(config.providers.find((provider) => provider.name === page.params.id));
</script>

<svelte:head><title>{i18n.t('providers.editMetaTitle')}</title></svelte:head>
<PageHead eyebrow={i18n.t('providers.eyebrow')} title={i18n.t('providers.editAria', { name: page.params.id ?? '' })} />
{#if source}
  <ProviderForm {source} />
{:else if !config.loading}
  <div class="state-banner error" role="alert">{i18n.t('providers.notFound')}</div>
{/if}
