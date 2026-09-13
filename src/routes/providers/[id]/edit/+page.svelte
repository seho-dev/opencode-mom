<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { Eye, EyeOff } from '@lucide/svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import FormActions from '$lib/components/app/FormActions.svelte';
  import NpmAdapterInput from '$lib/components/app/NpmAdapterInput.svelte';
  import PageHead from '$lib/components/app/PageHead.svelte';
  import { getConfig } from '$lib/features/config/context.js';
  import { getI18n } from '$lib/features/i18n/context.js';
  import type { ProviderDef } from '$lib/features/config/types.js';
  import { toast } from '$lib/components/app/toast.svelte.js';

  const config = getConfig();
  const i18n = getI18n();
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
      throw new Error(i18n.t('validation.mustBeJsonObject', { label: i18n.t('providers.headersLabel') }));
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
        description: error instanceof Error ? error.message : i18n.t('toast.saveFailedProvider'),
      });
    }
  }
</script>

<svelte:head><title>{i18n.t('providers.editMetaTitle')}</title></svelte:head>
<PageHead eyebrow={i18n.t('providers.eyebrow')} title={i18n.t('providers.editAria', { name: page.params.id ?? '' })} />
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
        <label for="provider-name">{i18n.t('providers.nameLabel')}</label>
        <input id="provider-name" value={source.name} disabled />
      </div>
      <div class="field">
        <label for="provider-npm">{i18n.t('providers.npmLabel')}</label>
        <NpmAdapterInput bind:value={npm} />
      </div>
      <div class="field full">
        <label for="provider-base-url">{i18n.t('providers.baseUrlLabel')}</label>
        <input id="provider-base-url" bind:value={baseURL} placeholder={i18n.t('providers.baseUrlPlaceholder')} />
      </div>
      <div class="field full">
        <label for="provider-api-key">{i18n.t('providers.apiKeyLabel')}</label>
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
            aria-label={apiKeyVisible ? i18n.t('providers.hideApiKey') : i18n.t('providers.showApiKey')}
            onclick={() => (apiKeyVisible = !apiKeyVisible)}
          >
            {#if apiKeyVisible}<EyeOff size={14} />{:else}<Eye size={14} />{/if}
          </Button>
        </div>
        <small class="muted">{i18n.t('providers.keepKeyHint')}</small>
      </div>
      <div class="field full">
        <label for="provider-headers">{i18n.t('providers.headersLabel')}</label>
        <textarea id="provider-headers" bind:value={headers} placeholder={`{ "Authorization": "..." }`}></textarea>
        <small class="muted">{i18n.t('providers.headersHint')}</small>
      </div>
    </div>
    <FormActions>
      <Button href="/providers" variant="outline">{i18n.t('common.cancel')}</Button>
      <Button type="submit" disabled={config.saving}>{i18n.t('providers.saveChanges')}</Button>
    </FormActions>
  </form>
{:else if !config.loading}
  <div class="state-banner error" role="alert">{i18n.t('providers.notFound')}</div>
{/if}
