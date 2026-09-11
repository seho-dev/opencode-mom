<script lang="ts">
  import { goto } from '$app/navigation';
  import { Eye, EyeOff } from '@lucide/svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import FormActions from '$lib/components/app/FormActions.svelte';
  import NpmAdapterInput from '$lib/components/app/NpmAdapterInput.svelte';
  import PageHead from '$lib/components/app/PageHead.svelte';
  import { getConfig } from '$lib/features/config/context.js';
  import type { ProviderDef } from '$lib/features/config/types.js';
  import { toast } from '$lib/components/app/toast.svelte.js';

  const config = getConfig();
  let name = $state('');
  let npm = $state('');
  let baseURL = $state('');
  let apiKey = $state('');
  let headers = $state('');
  let apiKeyVisible = $state(false);

  const parseHeaders = (): Record<string, string> | undefined => {
    if (!headers.trim()) return undefined;
    const parsed: unknown = JSON.parse(headers);
    if (!parsed || Array.isArray(parsed) || typeof parsed !== 'object')
      throw new Error('headers must be a JSON object');
    return parsed as Record<string, string>;
  };
  async function submit() {
    if (!name.trim() || !npm.trim()) return;
    try {
      const value: ProviderDef = {
        name: name.trim(),
        npm: npm.trim(),
        options: {
          apiKey: apiKey || undefined,
          baseURL: baseURL || undefined,
          headers: parseHeaders(),
        },
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
      <label for="provider-name">Name</label>
      <input id="provider-name" bind:value={name} required placeholder="e.g. my-provider" />
    </div>
    <div class="field">
      <label for="provider-npm">NPM adapter</label>
      <NpmAdapterInput bind:value={npm} />
    </div>
    <div class="field full">
      <label for="provider-base-url">Base URL</label>
      <input id="provider-base-url" bind:value={baseURL} placeholder="https://api.example.com/v1" />
    </div>
    <div class="field full">
      <label for="provider-api-key">API key</label>
      <div class="flex gap-2">
        <input
          id="provider-api-key"
          class="min-w-0 flex-1"
          type={apiKeyVisible ? 'text' : 'password'}
          bind:value={apiKey}
          autocomplete="off"
        />
        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          aria-label={apiKeyVisible ? 'Hide API key' : 'Show API key'}
          onclick={() => (apiKeyVisible = !apiKeyVisible)}
        >
          {#if apiKeyVisible}<EyeOff size={14} />{:else}<Eye size={14} />{/if}
        </Button>
      </div>
    </div>
    <div class="field full">
      <label for="provider-headers">Headers</label>
      <textarea id="provider-headers" bind:value={headers} placeholder={`{ "Authorization": "..." }`}></textarea>
      <small class="muted">Optional JSON object sent with every request.</small>
    </div>
  </div>
  <FormActions>
    <Button href="/providers" variant="outline">Cancel</Button>
    <Button type="submit" disabled={config.saving}>Save provider</Button>
  </FormActions>
</form>
