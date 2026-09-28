<script lang="ts">
import { goto } from '$app/navigation';
import PageHead from '$src/components/PageHead.svelte';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import type { ModelDef } from '$src/types/models.js';
import ModelForm from '../ModelForm.svelte';

const config = getConfig();
const i18n = getI18n();
async function create(providerId: string, value: ModelDef) {
  await config.createModel(providerId, value);
  await goto('/models');
}
</script>

<svelte:head><title>{i18n.t('models.newMetaTitle')}</title></svelte:head>
<PageHead eyebrow={i18n.t('models.eyebrow')} title={i18n.t('models.newTitle')} />
{#if !config.loading && config.providers.length === 0}
  <div class="state-banner error" role="alert">
    {i18n.t('models.noProvider')}
  </div>
{:else}
  <ModelForm mode="new" providers={config.providers} saving={config.saving} onSave={create} />
{/if}
