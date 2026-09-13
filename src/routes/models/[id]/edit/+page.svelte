<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import PageHead from '$lib/components/app/PageHead.svelte';
  import ModelForm from '$lib/components/app/ModelForm.svelte';
  import { getConfig } from '$lib/features/config/context.js';
  import { getI18n } from '$lib/features/i18n/context.js';
  import type { ModelDef } from '$lib/features/config/types.js';
  const config = getConfig();
  const i18n = getI18n();
  const ref = $derived(decodeURIComponent(page.params.id ?? ''));
  const found = $derived(config.models().find((model) => model.ref === ref));
  async function update(providerId: string, value: ModelDef) {
    await config.updateModel(providerId, value);
    await goto('/models');
  }
</script>

<svelte:head><title>{i18n.t('models.editMetaTitle')}</title></svelte:head><PageHead
  eyebrow={i18n.t('models.eyebrow')}
  title={i18n.t('models.editTitle')}
/>
{#if found}<ModelForm
    mode="edit"
    initial={found}
    defaultProviderId={found.providerId}
    saving={config.saving}
    submitLabel={i18n.t('modelForm.saveChanges')}
    onSave={update}
  />{:else if !config.loading}<div class="state-banner error" role="alert">
    {i18n.t('models.notFound')}
  </div>{/if}
