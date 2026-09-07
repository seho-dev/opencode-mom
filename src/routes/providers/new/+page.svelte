<script lang="ts">
  import { goto } from '$app/navigation';
  import { Plus, Trash2 } from '@lucide/svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import FormActions from '$lib/components/app/FormActions.svelte';
  import PageHead from '$lib/components/app/PageHead.svelte';
  import { getConfig } from '$lib/features/config/context.js';
  import type { ProviderDef } from '$lib/features/config/types.js';
  import { toast } from '$lib/components/app/toast.svelte.js';

  const config = getConfig();
  let id = $state('');
  let name = $state('');
  let api = $state('');
  let npm = $state('');
  let env = $state('');
  let whitelist = $state('');
  let blacklist = $state('');
  let options = $state<{ key: string; value: string }[]>([]);

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
  function optionRecord() {
    const result: Record<string, unknown> = {};
    for (const option of options) {
      if (!option.key.trim()) continue;
      try {
        result[option.key.trim()] = JSON.parse(option.value);
      } catch {
        result[option.key.trim()] = option.value;
      }
    }
    return Object.keys(result).length ? result : undefined;
  }
  async function submit() {
    if (!id.trim()) return;
    try {
      const value: ProviderDef = {
        id: id.trim(),
        name: name || undefined,
        api: api || undefined,
        npm: npm || undefined,
        env: lines(env),
        whitelist: lines(whitelist),
        blacklist: lines(blacklist),
        options: optionRecord(),
        models: {},
      };
      await config.createProvider(value);
      goto('/providers');
    } catch (error) {
      toast({
        variant: 'error',
        description: error instanceof Error ? error.message : 'Save failed. Check the input and configuration state.',
      });
    }
  }
</script>

<svelte:head><title>New provider · opencode-mom</title></svelte:head>
<PageHead eyebrow="CONFIG / PROVIDERS" title="New provider" />

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
      <input id="provider-id" bind:value={id} required placeholder="e.g. anthropic" />
    </div>
    <div class="field">
      <label for="provider-name">Display name</label>
      <input id="provider-name" bind:value={name} />
    </div>
    <div class="field full">
      <label for="provider-api">API URL</label>
      <input id="provider-api" bind:value={api} placeholder="https://api.example.com/v1" />
    </div>
    <div class="field full">
      <label for="provider-npm">NPM adapter</label>
      <input id="provider-npm" bind:value={npm} />
    </div>
    <div class="field">
      <label for="provider-env">Environment variables</label>
      <textarea id="provider-env" bind:value={env} placeholder={'API_KEY\nBASE_URL'}></textarea>
      <small class="muted">One variable name per line.</small>
    </div>
    <div class="field">
      <label for="provider-whitelist">Model whitelist</label>
      <textarea id="provider-whitelist" bind:value={whitelist} placeholder="model-id"></textarea>
      <small class="muted">One model ID per line.</small>
    </div>
    <div class="field full">
      <label for="provider-blacklist">Model blacklist</label>
      <textarea id="provider-blacklist" bind:value={blacklist} placeholder="deprecated-model"></textarea>
    </div>
    <fieldset class="field full" style="border:0;margin:0;padding:0;min-inline-size:0">
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
            placeholder="Key"
          />
          <input
            aria-label={`Option value ${index + 1}`}
            value={option.value}
            oninput={(event) => updateOption(index, 'value', event.currentTarget.value)}
            placeholder="Value (JSON supported)"
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
      {#if !options.length}
        <p class="muted">No provider options set.</p>
      {/if}
    </fieldset>
  </div>
  <FormActions>
    <Button href="/providers" variant="outline">Cancel</Button>
    <Button type="submit" disabled={config.saving}>Save provider</Button>
  </FormActions>
</form>
