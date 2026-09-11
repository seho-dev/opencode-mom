<script lang="ts">
  import { page } from '$app/state';
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
  const source = $derived(config.providers.find((provider) => provider.name === page.params.id));
  let npm = $state('');
  let baseURL = $state('');
  let apiKey = $state('');
  let headers = $state('');
  let loaded = $state('');
  let apiKeyVisible = $state(false);

  $effect(() => {
    if (source && loaded !== source.name) {
      loaded = source.name;
      npm = source.npm ?? '';
      baseURL = source.options?.baseURL ?? '';
      apiKey = source.options?.apiKey ?? '';
      headers = source.options?.headers ? JSON.stringify(source.options.headers, null, 2) : '';
    }
  });

  const parseHeaders = (): Record<string, string> | undefined => {
    if (!headers.trim()) return undefined;
    const parsed: unknown = JSON.parse(headers);
    if (!parsed || Array.isArray(parsed) || typeof parsed !== 'object')
      throw new Error('headers must be a JSON object');
    return parsed as Record<string, string>;
  };
  async function submit() {
    if (!source) return;
    try {
      await config.updateProvider({
        ...source,
        npm: npm.trim() || undefined,
        options: {
          apiKey,
          baseURL: baseURL || undefined,
          headers: parseHeaders(),
        },
      } as ProviderDef);
      goto('/providers');
    } catch (error) {
      toast({
        variant: 'error',
        description: error instanceof Error ? error.message : 'Save failed. Check the input and configuration state.',
      });
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
        <label for="provider-name">Name</label>
        <input id="provider-name" value={source.name} disabled />
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
        <small class="muted">Leave empty to keep the currently configured key.</small>
      </div>
      <div class="field full">
        <label for="provider-headers">Headers</label>
        <textarea id="provider-headers" bind:value={headers} placeholder={`{ "Authorization": "..." }`}></textarea>
        <small class="muted">Optional JSON object sent with every request.</small>
      </div>
    </div>
    <FormActions>
      <Button href="/providers" variant="outline">Cancel</Button>
      <Button type="submit" disabled={config.saving}>Save changes</Button>
    </FormActions>
  </form>
{:else if !config.loading}
  <div class="state-banner error" role="alert">The provider does not exist or has not been loaded yet.</div>
{/if}
