<script lang="ts">
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
      throw new Error(i18n.t('validation.mustBeJsonObject', { label: i18n.t('providers.headersLabel') }));
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
        description: error instanceof Error ? error.message : i18n.t('toast.saveFailedProvider'),
      });
    }
  }
</script>

<svelte:head><title>{i18n.t('providers.newMetaTitle')}</title></svelte:head>
<PageHead eyebrow={i18n.t('providers.eyebrow')} title={i18n.t('providers.newTitle')} />

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
      <input id="provider-name" bind:value={name} required placeholder={i18n.t('providers.namePlaceholder')} />
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
    </div>
    <div class="field full">
      <label for="provider-headers">{i18n.t('providers.headersLabel')}</label>
      <textarea id="provider-headers" bind:value={headers} placeholder={`{ "Authorization": "..." }`}></textarea>
      <small class="muted">{i18n.t('providers.headersHint')}</small>
    </div>
  </div>
  <FormActions>
    <Button href="/providers" variant="outline">{i18n.t('common.cancel')}</Button>
    <Button type="submit" disabled={config.saving}>{i18n.t('providers.saveProvider')}</Button>
  </FormActions>
</form>
