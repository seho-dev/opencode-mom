<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import PageHead from '$lib/components/app/PageHead.svelte';
  import ModelForm from '$lib/components/app/ModelForm.svelte';
  import { getConfig } from '$lib/features/config/context.js';
  import type { ModelDef } from '$lib/features/config/types.js';
  const config = getConfig();
  const ref = $derived(decodeURIComponent(page.params.id ?? ''));
  const found = $derived(config.models().find((model) => model.ref === ref));
  async function update(providerId: string, value: ModelDef) {
    await config.updateModel(providerId, value);
    await goto('/models');
  }
</script>

<svelte:head><title>Edit model · opencode-mom</title></svelte:head><PageHead
  eyebrow="CONFIG / MODELS"
  title="Edit model"
/>
{#if found}<ModelForm
    mode="edit"
    initial={found}
    defaultProviderId={found.providerId}
    saving={config.saving}
    submitLabel="Save changes"
    onSave={update}
  />{:else if !config.loading}<div class="state-banner error" role="alert">
    The model does not exist or has not been loaded yet.
  </div>{/if}
