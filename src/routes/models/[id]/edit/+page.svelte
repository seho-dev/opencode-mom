<script lang="ts">
import { goto } from '$app/navigation';
import { page } from '$app/state';
import PageHead from '$src/components/PageHead.svelte';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import type { ModelDef } from '$src/types/models.js';
import ModelForm from '../../ModelForm.svelte';

const config = getConfig();
const i18n = getI18n();
const ref = $derived(decodeURIComponent(page.params.id ?? ''));
const found = $derived(config.models().find((model) => model.ref === ref));
async function update(providerId: string, value: ModelDef) {
  await config.updateModel(providerId, value);
  await goto('/models');
}
</script>

<svelte:head><title>{i18n.t('models.editMetaTitle')}</title></svelte:head>
<PageHead eyebrow={i18n.t('models.eyebrow')} title={i18n.t('models.editTitle')} />
{#if found}
  <ModelForm
    mode="edit"
    initial={found}
    defaultProviderId={found.providerId}
    saving={config.saving}
    submitLabel={i18n.t('modelForm.saveChanges')}
    onSave={update}
  />
{:else if !config.loading}
  <div class="state-banner error" role="alert">
    {i18n.t('models.notFound')}
  </div>
{/if}
