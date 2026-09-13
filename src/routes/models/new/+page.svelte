<script lang="ts">
  import { goto } from '$app/navigation';
  import PageHead from '$lib/components/app/PageHead.svelte';
  import ModelForm from '$lib/components/app/ModelForm.svelte';
  import { getConfig } from '$lib/features/config/context.js';
  import { getI18n } from '$lib/features/i18n/context.js';
  import type { ModelDef } from '$lib/features/config/types.js';
  const config = getConfig();
  const i18n = getI18n();
  async function create(providerId: string, value: ModelDef) {
    await config.createModel(providerId, value);
    await goto('/models');
  }
</script>

<svelte:head><title>{i18n.t('models.newMetaTitle')}</title></svelte:head><PageHead
  eyebrow={i18n.t('models.eyebrow')}
  title={i18n.t('models.newTitle')}
/>
<p class="text-xs text-muted-foreground mb-3">
  {i18n.t('models.newNotice')}
</p>
{#if !config.loading && config.providers.length === 0}<div class="state-banner error" role="alert">
    {i18n.t('models.noProvider')}
  </div>{:else}<ModelForm mode="new" providers={config.providers} saving={config.saving} onSave={create} />{/if}
