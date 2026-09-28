<script lang="ts">
import { Eye, EyeOff } from '@lucide/svelte';
import { createForm } from '@tanstack/svelte-form';
import { goto } from '$app/navigation';
import { Button } from '$src/components/button/index.js';
import { Combobox } from '$src/components/combobox/index.js';
import FormActions from '$src/components/FormActions.svelte';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import { providerSchema } from '$src/routes/providers/schema.js';
import { toast } from '$src/shell/toast.svelte.js';
import type { ProviderDef } from '$src/types/providers.js';

const PACKAGE_OPTIONS = [
  { value: 'aisdk:@ai-sdk/openai-compatible', label: 'aisdk:@ai-sdk/openai-compatible' },
  { value: 'aisdk:@ai-sdk/openai', label: 'aisdk:@ai-sdk/openai' },
  { value: '@opencode/ai/providers/openai-compatible', label: '@opencode/ai/providers/openai-compatible' },
  { value: '@opencode/ai/providers/openai', label: '@opencode/ai/providers/openai' },
  { value: '@opencode/ai/providers/anthropic', label: '@opencode/ai/providers/anthropic' },
];

let { source }: { source?: ProviderDef } = $props();
const config = getConfig();
const i18n = getI18n();
const defaultValues = $derived({
  name: source?.name ?? '',
  package: source?.package ?? '',
  baseURL: source?.settings?.baseURL ?? '',
  apiKey: source?.settings?.apiKey ?? '',
  headers: source?.headers ? JSON.stringify(source.headers, null, 2) : '',
});
let loaded = $state('');
let apiKeyVisible = $state(false);

const schema = $derived(
  providerSchema(
    {
      required: i18n.t('toast.fixFields'),
      json: i18n.t('validation.jsonObject'),
      object: i18n.t('validation.mustBeJsonObject', { label: i18n.t('providers.headersLabel') }),
      strings: i18n.t('validation.headersStrings'),
    },
    !source,
  ),
);
const form = createForm(() => ({
  defaultValues,
  validators: { onSubmit: schema },
  onSubmit: async ({ value }) => {
    const parsed = schema.parse(value);
    try {
      if (source) {
        await config.updateProvider({
          ...source,
          package: parsed.package.trim() || undefined,
          settings: { apiKey: parsed.apiKey || undefined, baseURL: parsed.baseURL || undefined },
          headers: parsed.headers,
        } as ProviderDef);
      } else {
        const provider: ProviderDef = {
          name: parsed.name.trim(),
          package: parsed.package.trim(),
          settings: { apiKey: parsed.apiKey || undefined, baseURL: parsed.baseURL || undefined },
          headers: parsed.headers,
          models: {},
        };
        await config.createProvider(provider);
      }
      goto('/providers');
    } catch (error) {
      toast({
        variant: 'error',
        description: error instanceof Error ? error.message : i18n.t('toast.saveFailedProvider'),
      });
    }
  },
  onSubmitInvalid: () => toast({ variant: 'error', description: i18n.t('toast.fixFields') }),
}));

$effect(() => {
  if (source && loaded !== source.name) {
    loaded = source.name;
    form.reset(defaultValues);
  }
});
</script>

<form
  class="panel form-panel"
  onsubmit={(event) => {
    event.preventDefault();
    form.handleSubmit();
  }}
>
  <div class="form-grid">
    <div class="field">
      {#if source}
        <label for="provider-name">{i18n.t('providers.nameLabel')}</label>
        <input id="provider-name" value={source.name} disabled>
      {:else}
        <label for="provider-name"
          >{i18n.t('providers.nameLabel')}<span class="field-required" aria-hidden="true">*</span></label
        >
        <form.Field name="name">
          {#snippet children(field)}
            <input
              id="provider-name"
              name={field.name}
              value={field.state.value}
              oninput={(event) => field.handleChange(event.currentTarget.value)}
              onblur={field.handleBlur}
              placeholder={i18n.t('providers.namePlaceholder')}
              aria-invalid={field.state.meta.errors.length ? 'true' : undefined}
              aria-describedby={field.state.meta.errors.length ? 'provider-name-error' : undefined}
            >
            {#if field.state.meta.errors.length}
              <p id="provider-name-error" class="field-error" role="alert">
                {field.state.meta.errors.map((error) => error.message).join(', ')}
              </p>
            {/if}
          {/snippet}
        </form.Field>
      {/if}
    </div>
    <div class="field">
      <label for="provider-npm"
        >{i18n.t('providers.npmLabel')}
        {#if !source}
          <span class="field-required" aria-hidden="true">*</span>
        {/if}</label
      >
      <form.Field name="package">
        {#snippet children(field)}
          <div onfocusout={() => field.handleBlur()}>
            <Combobox
              id="provider-npm"
              bind:value={() => field.state.value, (value) => field.handleChange(value)}
              options={PACKAGE_OPTIONS}
              placeholder="aisdk:@ai-sdk/openai-compatible"
              aria-invalid={field.state.meta.errors.length ? 'true' : undefined}
              aria-describedby={field.state.meta.errors.length ? 'provider-npm-error' : undefined}
            />
          </div>
          {#if field.state.meta.errors.length}
            <p id="provider-npm-error" class="field-error" role="alert">
              {field.state.meta.errors.map((error) => error.message).join(', ')}
            </p>
          {/if}
        {/snippet}
      </form.Field>
    </div>
    <div class="field full">
      <label for="provider-base-url">{i18n.t('providers.baseUrlLabel')}</label>
      <form.Field name="baseURL">
        {#snippet children(field)}
          <input
            id="provider-base-url"
            name={field.name}
            value={field.state.value}
            oninput={(event) => field.handleChange(event.currentTarget.value)}
            onblur={field.handleBlur}
            placeholder={i18n.t('providers.baseUrlPlaceholder')}
            aria-invalid={field.state.meta.errors.length ? 'true' : undefined}
            aria-describedby={field.state.meta.errors.length ? 'provider-base-url-error' : undefined}
          >
          {#if field.state.meta.errors.length}
            <p id="provider-base-url-error" class="field-error" role="alert">
              {field.state.meta.errors.map((error) => error.message).join(', ')}
            </p>
          {/if}
        {/snippet}
      </form.Field>
    </div>
    <div class="field full">
      <label for="provider-api-key">{i18n.t('providers.apiKeyLabel')}</label>
      <div class="flex gap-2">
        <form.Field name="apiKey">
          {#snippet children(field)}
            <input
              id="provider-api-key"
              class="min-w-0 flex-1"
              type={apiKeyVisible ? 'text' : 'password'}
              name={field.name}
              value={field.state.value}
              oninput={(event) => field.handleChange(event.currentTarget.value)}
              onblur={field.handleBlur}
              autocomplete="off"
              aria-invalid={field.state.meta.errors.length ? 'true' : undefined}
              aria-describedby={field.state.meta.errors.length ? 'provider-api-key-error' : undefined}
            >
            {#if field.state.meta.errors.length}
              <p id="provider-api-key-error" class="field-error" role="alert">
                {field.state.meta.errors.map((error) => error.message).join(', ')}
              </p>
            {/if}
          {/snippet}
        </form.Field>
        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          aria-label={apiKeyVisible ? i18n.t('providers.hideApiKey') : i18n.t('providers.showApiKey')}
          onclick={() => (apiKeyVisible = !apiKeyVisible)}
        >
          {#if apiKeyVisible}
            <EyeOff size={14} />
          {:else}
            <Eye size={14} />
          {/if}
        </Button>
      </div>
      {#if source}
        <small class="muted">{i18n.t('providers.keepKeyHint')}</small>
      {/if}
    </div>
    <div class="field full">
      <label for="provider-headers">{i18n.t('providers.headersLabel')}</label>
      <form.Field name="headers">
        {#snippet children(field)}
          <textarea
            id="provider-headers"
            name={field.name}
            value={field.state.value}
            oninput={(event) => field.handleChange(event.currentTarget.value)}
            onblur={field.handleBlur}
            placeholder={`{ "Authorization": "..." }`}
            aria-invalid={field.state.meta.errors.length ? 'true' : undefined}
            aria-describedby={field.state.meta.errors.length ? 'provider-headers-error' : undefined}
          ></textarea>
          {#if field.state.meta.errors.length}
            <p id="provider-headers-error" class="field-error" role="alert">
              {field.state.meta.errors.map((error) => error.message).join(', ')}
            </p>
          {/if}
        {/snippet}
      </form.Field>
      <small class="muted">{i18n.t('providers.headersHint')}</small>
    </div>
  </div>
  <FormActions>
    <Button href="/providers" variant="outline">{i18n.t('common.cancel')}</Button>
    <Button type="submit" disabled={config.saving}
      >{i18n.t(source ? 'providers.saveChanges' : 'providers.saveProvider')}</Button
    >
  </FormActions>
</form>
