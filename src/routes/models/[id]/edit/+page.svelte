<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { Button } from '$lib/components/ui/button/index.js';
  import PageHead from '$lib/components/app/PageHead.svelte';
  import FormActions from '$lib/components/app/FormActions.svelte';
  import { getConfig } from '$lib/features/config/context.js';
  import type { ModelDef } from '$lib/features/config/types.js';
  const config = getConfig();
  const ref = $derived(decodeURIComponent(page.params.id ?? ''));
  const found = $derived(config.models().find((model) => model.ref === ref));
  let initialized = $state('');
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
  let message = $state('');
  const obj = (value: string, label: string) => {
    const parsed = JSON.parse(value);
    if (!parsed || Array.isArray(parsed) || typeof parsed !== 'object')
      throw new Error(`${label} must be a JSON object`);
    return parsed;
  };
  const pretty = (value: unknown) => JSON.stringify(value ?? {}, null, 2);
  $effect(() => {
    if (found && initialized !== found.ref) {
      initialized = found.ref;
      name = found.name ?? '';
      family = found.family ?? '';
      releaseDate = found.release_date ?? '';
      status = found.status ?? 'active';
      temperature = found.temperature?.toString() ?? '';
      reasoning = found.reasoning ?? false;
      toolCall = found.tool_call ?? false;
      attachment = found.attachment ?? false;
      interleaved = found.interleaved ?? false;
      experimental = found.experimental ?? false;
      cost = pretty(found.cost);
      limit = pretty(found.limit);
      modalities = pretty(found.modalities);
      options = pretty(found.options);
      headers = pretty(found.headers);
      variants = pretty(found.variants);
      const { id, ...extra } = found;
      delete (extra as Record<string, unknown>)['ref'];
      delete (extra as Record<string, unknown>)['providerId'];
      raw = pretty(extra);
    }
  });
  async function submit() {
    if (!found) return;
    try {
      const extra = obj(raw, 'advanced fields');
      await config.updateModel(found.providerId, {
        ...extra,
        id: found.id,
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
      message = e instanceof Error ? e.message : 'Invalid JSON or save failed';
    }
  }
</script>

<svelte:head><title>Edit model · opencode-mom</title></svelte:head><PageHead
  eyebrow="CONFIG / MODELS"
  title="Edit model"
/>{#if found}<form
    class="panel form-panel"
    onsubmit={(e) => {
      e.preventDefault();
      submit();
    }}
  >
    <div class="form-grid">
      <div class="field full">
        <label for="model-ref">Full reference</label><input id="model-ref" value={found.ref} disabled />
      </div>
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
      <fieldset class="group-fieldset full">
        <legend>Capability flags</legend><label class="check-row"
          ><input type="checkbox" bind:checked={reasoning} /> reasoning</label
        ><label class="check-row"><input type="checkbox" bind:checked={toolCall} /> tool_call</label><label
          class="check-row"><input type="checkbox" bind:checked={attachment} /> attachment</label
        ><label class="check-row"><input type="checkbox" bind:checked={interleaved} /> interleaved</label><label
          class="check-row"><input type="checkbox" bind:checked={experimental} /> experimental</label
        >
      </fieldset>
      <div class="field"><label for="cost">cost JSON</label><textarea id="cost" bind:value={cost}></textarea></div>
      <div class="field"><label for="limit">limit JSON</label><textarea id="limit" bind:value={limit}></textarea></div>
      <div class="field">
        <label for="modalities">modalities JSON</label><textarea id="modalities" bind:value={modalities}></textarea>
      </div>
      <div class="field">
        <label for="options">options JSON</label><textarea id="options" bind:value={options}></textarea>
      </div>
      <div class="field">
        <label for="headers">headers JSON</label><textarea id="headers" bind:value={headers}></textarea>
      </div>
      <div class="field">
        <label for="variants">variants JSON</label><textarea id="variants" bind:value={variants}></textarea>
      </div>
      <div class="field full">
        <label for="model-raw">Advanced extra JSON</label><textarea id="model-raw" bind:value={raw}></textarea>
      </div>
      {#if message}<div class="state-banner error full" role="alert">{message}</div>{/if}
    </div>
    <FormActions
      ><Button href="/models" variant="outline">Cancel</Button><Button type="submit" disabled={config.saving}
        >Save changes</Button
      ></FormActions
    >
  </form>{:else if !config.loading}<div class="state-banner error" role="alert">
    The model does not exist or has not been loaded yet.
  </div>{/if}
