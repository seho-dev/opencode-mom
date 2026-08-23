<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { Plus, Trash2 } from '@lucide/svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import FormActions from '$lib/components/app/FormActions.svelte';
  import PageHead from '$lib/components/app/PageHead.svelte';
  import { getConfig } from '$lib/features/config/context.js';
  import type { ProviderDef } from '$lib/features/config/types.js';

  const config = getConfig();
  const source = $derived(config.providers.find((provider) => provider.id === page.params.id));
  let name = $state('');
  let api = $state('');
  let npm = $state('');
  let env = $state('');
  let whitelist = $state('');
  let blacklist = $state('');
  let options = $state<{ key: string; value: string }[]>([]);
  let rawOptions = $state('{}');
  let loaded = $state('');
  let message = $state('');

  const stringify = (value: unknown) => (typeof value === 'string' ? value : JSON.stringify(value));

  $effect(() => {
    if (source && loaded !== source.id) {
      loaded = source.id;
      name = source.name ?? '';
      api = source.api ?? '';
      npm = source.npm ?? '';
      env = (source.env ?? []).join('\n');
      whitelist = (source.whitelist ?? []).join('\n');
      blacklist = (source.blacklist ?? []).join('\n');
      options = Object.entries(source.options ?? {}).map(([key, value]) => ({ key, value: stringify(value) }));
      rawOptions = JSON.stringify(source.options ?? {}, null, 2);
    }
  });

  const lines = (value: string) =>
    value
      .split('\n')
      .map((line) => line.trim())
      .filter(Boolean);
  function addOption() {
    options = [...options, { key: '', value: '' }];
  }
  function removeOption(index: number) {
    options = options.filter((_, i) => i !== index);
  }
  function updateOption(index: number, key: 'key' | 'value', value: string) {
    options = options.map((entry, i) => (i === index ? { ...entry, [key]: value } : entry));
  }
  async function submit() {
    if (!source) return;
    try {
      const raw = JSON.parse(rawOptions);
      if (!raw || Array.isArray(raw) || typeof raw !== 'object')
        throw new Error('Provider options must be a JSON object');
      await config.updateProvider({
        ...source,
        name: name || undefined,
        api: api || undefined,
        npm: npm || undefined,
        env: lines(env),
        whitelist: lines(whitelist),
        blacklist: lines(blacklist),
        options: raw as Record<string, unknown>,
      } as ProviderDef);
      goto('/providers');
    } catch (error) {
      message = error instanceof Error ? error.message : 'Save failed. Check the input and configuration state.';
    }
  }
</script>

<svelte:head><title>Edit provider · opencode-mom</title></svelte:head>
<PageHead eyebrow="CONFIG / PROVIDERS" title={`Edit ${page.params.id}`} />
{#if source}
  <form
    class="panel form-panel"
    onsubmit={(event) => {
      event.preventDefault();
      submit();
    }}
  >
    <div class="form-grid">
      <div class="field">
        <label for="provider-id">Provider ID</label>
        <input id="provider-id" value={source.id} disabled />
      </div>
      <div class="field">
        <label for="provider-name">Display name</label>
        <input id="provider-name" bind:value={name} />
      </div>
      <div class="field full">
        <label for="provider-api">API URL</label>
        <input id="provider-api" bind:value={api} />
      </div>
      <div class="field full">
        <label for="provider-npm">NPM adapter</label>
        <input id="provider-npm" bind:value={npm} />
      </div>
      <div class="field">
        <label for="provider-env">Environment variables</label>
        <textarea id="provider-env" bind:value={env}></textarea>
      </div>
      <div class="field">
        <label for="provider-whitelist">Model whitelist</label>
        <textarea id="provider-whitelist" bind:value={whitelist}></textarea>
      </div>
      <div class="field full">
        <label for="provider-blacklist">Model blacklist</label>
        <textarea id="provider-blacklist" bind:value={blacklist}></textarea>
      </div>
      <fieldset class="field full">
        <legend class="field-label-row">
          <span>Provider options</span>
          <Button type="button" size="sm" variant="outline" onclick={addOption}>
            <Plus size={13} /> Add option
          </Button>
        </legend>
        {#each options as option, index}
          <div class="key-value-row">
            <input
              aria-label={`Option key ${index + 1}`}
              value={option.key}
              oninput={(event) => updateOption(index, 'key', event.currentTarget.value)}
            />
            <input
              aria-label={`Option value ${index + 1}`}
              value={option.value}
              oninput={(event) => updateOption(index, 'value', event.currentTarget.value)}
            />
            <Button
              type="button"
              size="icon-sm"
              variant="ghost"
              aria-label="Remove option"
              onclick={() => removeOption(index)}
            >
              <Trash2 size={14} />
            </Button>
          </div>
        {/each}
      </fieldset>
      <div class="field full">
        <label for="provider-options-json">Options JSON (advanced)</label>
        <textarea id="provider-options-json" bind:value={rawOptions}></textarea>
        <small class="muted">This JSON is the final saved value; use it to edit nested options.</small>
      </div>
      {#if message}
        <div class="state-banner error full" role="alert">{message}</div>
      {/if}
    </div>
    <FormActions>
      <Button href="/providers" variant="outline">Cancel</Button>
      <Button type="submit" disabled={config.saving}>Save changes</Button>
    </FormActions>
  </form>
{:else if !config.loading}
  <div class="state-banner error" role="alert">The provider does not exist or has not been loaded yet.</div>
{/if}
