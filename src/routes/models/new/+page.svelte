<script lang="ts">
  import { goto } from '$app/navigation';
  import { Button } from '$lib/components/ui/button/index.js';
  import PageHead from '$lib/components/app/PageHead.svelte';
  import FormActions from '$lib/components/app/FormActions.svelte';
  import { getConfig } from '$lib/features/config/context.js';
  import type { ModelDef } from '$lib/features/config/types.js';
  import { toast } from '$lib/components/app/toast.svelte.js';
  const config = getConfig();
  let providerId = $state('');
  let id = $state('');
  let name = $state('');
  let family = $state('');
  let releaseDate = $state('');
  let status = $state<ModelDef['status']>('active');
  let temperature = $state('');
  let reasoning = $state(false);
  let toolCall = $state(false);
  let attachment = $state(false);
  let interleaved = $state(false);
  let experimental = $state(false);
  let cost = $state('{}');
  let limit = $state('{}');
  let modalities = $state('{}');
  let options = $state('{}');
  let headers = $state('{}');
  let variants = $state('{}');
  let raw = $state('{}');
  const obj = (value: string, label: string) => {
    const parsed = JSON.parse(value);
    if (!parsed || Array.isArray(parsed) || typeof parsed !== 'object')
      throw new Error(`${label} must be a JSON object`);
    return parsed;
  };
  async function submit() {
    try {
      const extra = obj(raw, 'advanced fields');
      await config.createModel(providerId, {
        ...extra,
        id: id.trim(),
        name: name || undefined,
        family: family || undefined,
        release_date: releaseDate || undefined,
        status,
        temperature: temperature ? Number(temperature) : undefined,
        reasoning,
        tool_call: toolCall,
        attachment,
        interleaved,
        experimental,
        cost: obj(cost, 'cost'),
        limit: obj(limit, 'limit'),
        modalities: obj(modalities, 'modalities'),
        options: obj(options, 'options'),
        headers: obj(headers, 'headers'),
        variants: obj(variants, 'variants'),
      });
      goto('/models');
    } catch (e) {
      toast({ variant: 'error', description: e instanceof Error ? e.message : 'Invalid JSON or save failed' });
    }
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
  </div>{:else}<form
    class="panel form-panel"
    onsubmit={(e) => {
      e.preventDefault();
      submit();
    }}
  >
    <div class="form-grid">
      <div class="field">
        <label for="provider">Provider</label><select id="provider" bind:value={providerId} required
          ><option value="" disabled>Select a provider</option>{#each config.providers as provider}<option
              value={provider.name}>{provider.name}</option
            >{/each}</select
        >
      </div>
      <div class="field"><label for="model-id">Model ID</label><input id="model-id" bind:value={id} required /></div>
      <div class="field"><label for="model-name">Name</label><input id="model-name" bind:value={name} /></div>
      <div class="field"><label for="family">Family</label><input id="family" bind:value={family} /></div>
      <div class="field">
        <label for="release-date">Release date</label><input id="release-date" type="date" bind:value={releaseDate} />
      </div>
      <div class="field">
        <label for="status">Status</label><select id="status" bind:value={status}
          ><option value="active">active</option><option value="alpha">alpha</option><option value="beta">beta</option
          ><option value="deprecated">deprecated</option></select
        >
      </div>
      <div class="field">
        <label for="temperature">Temperature</label><input
          id="temperature"
          type="number"
          step="0.1"
          bind:value={temperature}
        />
      </div>
      <div class="field full">
        <fieldset class="group-fieldset">
          <legend>Capability flags</legend><label class="check-row"
            ><input type="checkbox" bind:checked={reasoning} /> reasoning</label
          ><label class="check-row"><input type="checkbox" bind:checked={toolCall} /> tool_call</label><label
            class="check-row"><input type="checkbox" bind:checked={attachment} /> attachment</label
          ><label class="check-row"><input type="checkbox" bind:checked={interleaved} /> interleaved</label><label
            class="check-row"><input type="checkbox" bind:checked={experimental} /> experimental</label
          >
        </fieldset>
      </div>
      {#each [['cost', cost], ['limit', limit], ['modalities', modalities], ['options', options], ['headers', headers], ['variants', variants]] as field}<div
          class="field"
        >
          <label for={`model-${field[0]}`}>{field[0]} JSON</label><textarea
            id={`model-${field[0]}`}
            bind:value={field[1]}></textarea>
        </div>{/each}
      <div class="field full">
        <label for="model-raw">Advanced extra JSON</label><textarea id="model-raw" bind:value={raw}></textarea><small
          class="muted"
          >For fields that have no dedicated control yet; merged with the structured fields above when saving.</small
        >
      </div>
    </div>
    <FormActions
      ><Button href="/models" variant="outline">Cancel</Button><Button
        type="submit"
        disabled={config.saving || !providerId}>Save model</Button
      ></FormActions
    >
  </form>{/if}
