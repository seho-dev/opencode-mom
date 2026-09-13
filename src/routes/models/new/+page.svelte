<script lang="ts">
  import { goto } from '$app/navigation';
  import PageHead from '$lib/components/app/PageHead.svelte';
  import ModelForm from '$lib/components/app/ModelForm.svelte';
  import { getConfig } from '$lib/features/config/context.js';
  import type { ModelDef } from '$lib/features/config/types.js';
  const config = getConfig();
  async function create(providerId: string, value: ModelDef) {
    await config.createModel(providerId, value);
    await goto('/models');
  }
</script>

<svelte:head><title>New model · opencode-mom</title></svelte:head><PageHead
  eyebrow="CONFIG / MODELS"
  title="New model"
/>
<p class="text-xs text-muted-foreground mb-3">
  Models can only be created under custom providers (those defined in the opencode config file). Builtin models are
  read-only and provided by opencode via CLI.
</p>
{#if !config.loading && config.providers.length === 0}<div class="state-banner error" role="alert">
    No provider available. Create a provider first.
  </div>{:else}<ModelForm mode="new" providers={config.providers} saving={config.saving} onSave={create} />{/if}
