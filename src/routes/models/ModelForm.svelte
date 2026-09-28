<script lang="ts">
import { Plus, Trash2 } from '@lucide/svelte';
import { createForm } from '@tanstack/svelte-form';
import { z } from 'zod';
import { Button } from '$src/components/button/index.js';
import { Checkbox } from '$src/components/checkbox/index.js';
import { Combobox } from '$src/components/combobox/index.js';
import FormActions from '$src/components/FormActions.svelte';
import Select from '$src/components/Select.svelte';
import { Switch } from '$src/components/switch/index.js';
import { getI18n } from '$src/i18n/context.js';
import { toast } from '$src/shell/toast.svelte.js';
import type { ModelDef } from '$src/types/models.js';
import type { ProviderDef } from '$src/types/providers.js';
import { isValidTokenCount } from '$src/utils/index.js';
import {
  anyFilled,
  emptyVariant,
  formValuesToModel,
  initMods,
  MODALITIES,
  modelToFormValues,
  parseObject,
} from './modelForm.js';

const LIMIT_PRESETS = [128_000, 256_000, 400_000, 500_000, 1_000_000].map((count) => ({
  value: String(count),
  label: count === 1_000_000 ? '1M' : `${count / 1_000}K`,
}));

const i18n = getI18n();

let {
  mode,
  providers = [],
  defaultProviderId = '',
  initial,
  saving = false,
  submitLabel = i18n.t('modelForm.save'),
  onSave,
}: {
  mode: 'new' | 'edit';
  providers?: ProviderDef[];
  defaultProviderId?: string;
  initial?: ModelDef;
  saving?: boolean;
  submitLabel?: string;
  onSave: (providerId: string, value: ModelDef) => Promise<void>;
} = $props();

const defaults = () => ({
  providerId: '',
  id: '',
  family: '',
  disabled: false,
  tools: mode === 'new',
  costInput: '',
  costOutput: '',
  costCacheRead: '',
  costCacheWrite: '',
  limitContext: '',
  limitOutput: '',
  limitInput: '',
  modalityInput: initMods(mode === 'new' ? ['text', 'image', 'pdf'] : undefined),
  modalityOutput: initMods(mode === 'new' ? ['text', 'image', 'pdf'] : undefined),
  settings: '{}',
  headers: '{}',
  variants: [emptyVariant()],
});
const errorText = (errors: readonly unknown[]) =>
  errors
    .map((error) => (typeof error === 'string' ? error : String((error as { message: string }).message)))
    .join(', ');
const schema = z
  .object({
    providerId: z.string(),
    id: z.string(),
    family: z.string(),
    disabled: z.boolean(),
    tools: z.boolean(),
    costInput: z.string(),
    costOutput: z.string(),
    costCacheRead: z.string(),
    costCacheWrite: z.string(),
    limitContext: z.string(),
    limitOutput: z.string(),
    limitInput: z.string(),
    modalityInput: z.record(z.enum(MODALITIES), z.boolean()),
    modalityOutput: z.record(z.enum(MODALITIES), z.boolean()),
    settings: z.string(),
    headers: z.string(),
    variants: z.array(z.object({ key: z.string(), json: z.string() })),
  })
  .superRefine((value, ctx) => {
    const issue = (path: (string | number)[], message: string) => ctx.addIssue({ code: 'custom', path, message });
    if (!value.id.trim()) issue(['id'], i18n.t('validation.modelIdRequired'));
    if (anyFilled(value.costInput, value.costOutput, value.costCacheRead, value.costCacheWrite)) {
      if (!value.costInput.trim()) issue(['costInput'], i18n.t('validation.costRequired'));
      if (!value.costOutput.trim()) issue(['costOutput'], i18n.t('validation.costRequired'));
    }
    if (anyFilled(value.limitContext, value.limitOutput, value.limitInput)) {
      if (!value.limitContext.trim()) issue(['limitContext'], i18n.t('validation.limitRequired'));
      if (!value.limitOutput.trim()) issue(['limitOutput'], i18n.t('validation.limitRequired'));
    }
    for (const key of ['limitContext', 'limitOutput', 'limitInput'] as const) {
      if (value[key].trim() && !isValidTokenCount(value[key])) issue([key], i18n.t('validation.limitInvalid'));
    }
    if (parseObject(value.settings) === undefined) issue(['settings'], i18n.t('validation.jsonObject'));
    const headers = parseObject(value.headers);
    if (headers === undefined) issue(['headers'], i18n.t('validation.jsonObject'));
    else if (Object.values(headers).some((header) => typeof header !== 'string'))
      issue(['headers'], i18n.t('validation.headersStrings'));
    value.variants.forEach((row, index) => {
      if (row.key.trim() && parseObject(row.json) === undefined)
        issue(['variants', index, 'json'], i18n.t('validation.jsonObject'));
    });
  });

// Whether the stored model already carries a capabilities block (edit mode only).
let capabilitiesPresent = $state(false);
let initialized = $state('');
const form = createForm(() => ({
  defaultValues: defaults(),
  validators: { onSubmit: schema },
  onSubmitInvalid: () => toast({ variant: 'error', description: i18n.t('toast.fixFields') }),
  onSubmit: async ({ value }) => {
    try {
      await onSave(
        mode === 'edit' ? defaultProviderId : value.providerId,
        formValuesToModel(value, mode, initial, capabilitiesPresent),
      );
    } catch (error) {
      toast({ variant: 'error', description: error instanceof Error ? error.message : i18n.t('toast.saveFailed') });
    }
  },
}));
const variants = form.useSelector((state) => state.values.variants);
const selectedProvider = form.useSelector((state) => state.values.providerId);

$effect(() => {
  if (mode === 'new') {
    if (initialized === 'new') return;
    initialized = 'new';
    capabilitiesPresent = false;
    form.reset(defaults());
    return;
  }
  const key = initial ? `${defaultProviderId}/${initial.id}` : '';
  if (!initial || initialized === key) return;
  initialized = key;
  capabilitiesPresent = initial.capabilities !== undefined;
  form.reset(modelToFormValues(initial, defaultProviderId, defaults()));
});

function addVariant() {
  form.pushFieldValue('variants', emptyVariant());
}
function removeVariant(index: number) {
  form.removeFieldValue('variants', index);
}
</script>

<form
  class="panel form-panel"
  onsubmit={(event) => {
    event.preventDefault();
    form.handleSubmit();
  }}
>
  <div class="form-grid">
    <fieldset class="group-fieldset">
      <legend>{i18n.t('modelForm.identity')}</legend>
      <div class="form-grid">
        {#if mode === 'new'}
          <div class="field">
            <label for="model-provider"
              >{i18n.t('modelForm.provider')}<span class="field-required" aria-hidden="true">*</span></label
            >
            <form.Field name="providerId">
              {#snippet children(field)}
                <Select
                  id="model-provider"
                  value={field.state.value}
                  onchange={(value) => field.handleChange(value)}
                  disabled={saving}
                  placeholder={i18n.t('modelForm.selectProvider')}
                  options={providers.map((provider) => ({ value: provider.name }))}
                />
              {/snippet}
            </form.Field>
          </div>
          <div class="field">
            <label for="model-id"
              >{i18n.t('modelForm.modelId')}<span class="field-required" aria-hidden="true">*</span></label
            >
            <form.Field name="id">
              {#snippet children(field)}
                <input
                  id="model-id"
                  value={field.state.value}
                  oninput={(event) => field.handleChange(event.currentTarget.value)}
                  onblur={field.handleBlur}
                  required
                  aria-invalid={field.state.meta.errors.length ? 'true' : undefined}
                  aria-describedby={field.state.meta.errors.length ? 'model-id-error' : undefined}
                  disabled={saving}
                  placeholder={i18n.t('modelForm.modelIdPlaceholder')}
                >
                {#if field.state.meta.errors.length}
                  <p id="model-id-error" class="field-error" role="alert">{errorText(field.state.meta.errors)}</p>
                {/if}
              {/snippet}
            </form.Field>
          </div>
        {:else}
          <div class="field">
            <label for="model-provider">{i18n.t('modelForm.provider')}</label>
            <input id="model-provider" value={defaultProviderId} disabled>
          </div>
          <div class="field">
            <label for="model-id">{i18n.t('modelForm.modelId')}</label>
            <form.Field name="id"
              >{#snippet children(field)}
                <input id="model-id" value={field.state.value} disabled>
              {/snippet}</form.Field
            >
          </div>
        {/if}
        <div class="field">
          <label for="model-family">{i18n.t('modelForm.family')}</label>
          <form.Field name="family">
            {#snippet children(field)}
              <input
                id="model-family"
                value={field.state.value}
                oninput={(event) => field.handleChange(event.currentTarget.value)}
                onblur={field.handleBlur}
                disabled={saving}
                placeholder={i18n.t('modelForm.familyPlaceholder')}
              >
            {/snippet}
          </form.Field>
        </div>
        <div class="field">
          <div class="check-row">
            <form.Field name="disabled"
              >{#snippet children(field)}
                <Switch
                  id="model-disabled"
                  checked={field.state.value}
                  onchange={(event) => field.handleChange(event.currentTarget.checked)}
                  disabled={saving}
                />
              {/snippet}</form.Field
            >
            <label for="model-disabled">{i18n.t('common.disabled')}</label>
          </div>
        </div>
      </div>
    </fieldset>

    <fieldset class="group-fieldset">
      <legend>{i18n.t('modelForm.capabilities')}</legend>
      <div class="form-grid">
        <div class="field full">
          <div class="check-row">
            <form.Field name="tools"
              >{#snippet children(field)}
                <Switch
                  id="model-tools"
                  checked={field.state.value}
                  onchange={(event) => field.handleChange(event.currentTarget.checked)}
                  disabled={saving}
                />
              {/snippet}</form.Field
            >
            <label for="model-tools">{i18n.t('modelForm.toolCall')}</label>
          </div>
          <small class="muted">{i18n.t('modelForm.capabilitiesHint')}</small>
        </div>
        <div class="field full">
          <div class="grid grid-cols-2 gap-x-8">
            <div>
              <p class="group-title">{i18n.t('modelForm.inputModalities')}</p>
              <div class="grid gap-y-2">
                {#each MODALITIES as modality}
                  <div class="check-row">
                    <form.Field name={`modalityInput.${modality}`}>
                      {#snippet children(field)}
                        <Checkbox
                          id="model-input-{modality}"
                          checked={field.state.value}
                          onCheckedChange={(checked) => field.handleChange(checked)}
                          disabled={saving}
                        />
                      {/snippet}
                    </form.Field>
                    <label for="model-input-{modality}">{modality}</label>
                  </div>
                {/each}
              </div>
            </div>
            <div>
              <p class="group-title">{i18n.t('modelForm.outputModalities')}</p>
              <div class="grid gap-y-2">
                {#each MODALITIES as modality}
                  <div class="check-row">
                    <form.Field name={`modalityOutput.${modality}`}>
                      {#snippet children(field)}
                        <Checkbox
                          id="model-output-{modality}"
                          checked={field.state.value}
                          onCheckedChange={(checked) => field.handleChange(checked)}
                          disabled={saving}
                        />
                      {/snippet}
                    </form.Field>
                    <label for="model-output-{modality}">{modality}</label>
                  </div>
                {/each}
              </div>
            </div>
          </div>
        </div>
      </div>
    </fieldset>

    <fieldset class="group-fieldset">
      <legend>{i18n.t('modelForm.limitsCost')}</legend>
      <div class="form-grid">
        <div class="field full">
          <p class="group-title">{i18n.t('modelForm.cost')}</p>
          <div class="num-grid">
            <div class="sub-field">
              <label for="cost-input">{i18n.t('modelForm.input')}</label>
              <form.Field name="costInput"
                >{#snippet children(field)}
                  <input
                    id="cost-input"
                    type="number"
                    step="any"
                    value={field.state.value}
                    oninput={(event) => field.handleChange(event.currentTarget.value)}
                    onblur={field.handleBlur}
                    aria-invalid={field.state.meta.errors.length ? 'true' : undefined}
                    aria-describedby={field.state.meta.errors.length ? 'cost-input-error' : undefined}
                    disabled={saving}
                  >
                  {#if field.state.meta.errors.length}
                    <p id="cost-input-error" class="field-error" role="alert">{errorText(field.state.meta.errors)}</p>
                  {/if}
                {/snippet}</form.Field
              >
            </div>
            <div class="sub-field">
              <label for="cost-output">{i18n.t('modelForm.output')}</label>
              <form.Field name="costOutput"
                >{#snippet children(field)}
                  <input
                    id="cost-output"
                    type="number"
                    step="any"
                    value={field.state.value}
                    oninput={(event) => field.handleChange(event.currentTarget.value)}
                    onblur={field.handleBlur}
                    aria-invalid={field.state.meta.errors.length ? 'true' : undefined}
                    aria-describedby={field.state.meta.errors.length ? 'cost-output-error' : undefined}
                    disabled={saving}
                  >
                  {#if field.state.meta.errors.length}
                    <p id="cost-output-error" class="field-error" role="alert">{errorText(field.state.meta.errors)}</p>
                  {/if}
                {/snippet}</form.Field
              >
            </div>
            <div class="sub-field">
              <label for="cost-cache-read">{i18n.t('modelForm.cacheRead')}</label>
              <form.Field name="costCacheRead"
                >{#snippet children(field)}
                  <input
                    id="cost-cache-read"
                    type="number"
                    step="any"
                    value={field.state.value}
                    oninput={(event) => field.handleChange(event.currentTarget.value)}
                    onblur={field.handleBlur}
                    disabled={saving}
                  >
                {/snippet}</form.Field
              >
            </div>
            <div class="sub-field">
              <label for="cost-cache-write">{i18n.t('modelForm.cacheWrite')}</label>
              <form.Field name="costCacheWrite"
                >{#snippet children(field)}
                  <input
                    id="cost-cache-write"
                    type="number"
                    step="any"
                    value={field.state.value}
                    oninput={(event) => field.handleChange(event.currentTarget.value)}
                    onblur={field.handleBlur}
                    disabled={saving}
                  >
                {/snippet}</form.Field
              >
            </div>
          </div>
          <small class="muted">{i18n.t('modelForm.costHint')}</small>
        </div>
        <div class="field full">
          <p class="group-title">{i18n.t('modelForm.limits')}</p>
          <div class="num-grid">
            <div class="sub-field">
              <label for="limit-context">{i18n.t('modelForm.context')}</label>
              <form.Field name="limitContext"
                >{#snippet children(field)}
                  <Combobox
                    id="limit-context"
                    value={field.state.value}
                    onchange={(value) => field.handleChange(value)}
                    onblur={field.handleBlur}
                    aria-invalid={field.state.meta.errors.length ? 'true' : undefined}
                    aria-describedby={field.state.meta.errors.length ? 'limit-context-error' : undefined}
                    options={LIMIT_PRESETS}
                    inputmode="numeric"
                    disabled={saving}
                  />
                  {#if field.state.meta.errors.length}
                    <p id="limit-context-error" class="field-error" role="alert">
                      {errorText(field.state.meta.errors)}
                    </p>
                  {/if}
                {/snippet}</form.Field
              >
            </div>
            <div class="sub-field">
              <label for="limit-output">{i18n.t('modelForm.output')}</label>
              <form.Field name="limitOutput"
                >{#snippet children(field)}
                  <Combobox
                    id="limit-output"
                    value={field.state.value}
                    onchange={(value) => field.handleChange(value)}
                    onblur={field.handleBlur}
                    aria-invalid={field.state.meta.errors.length ? 'true' : undefined}
                    aria-describedby={field.state.meta.errors.length ? 'limit-output-error' : undefined}
                    options={LIMIT_PRESETS}
                    inputmode="numeric"
                    disabled={saving}
                  />
                  {#if field.state.meta.errors.length}
                    <p id="limit-output-error" class="field-error" role="alert">{errorText(field.state.meta.errors)}</p>
                  {/if}
                {/snippet}</form.Field
              >
            </div>
            <div class="sub-field">
              <label for="limit-input">{i18n.t('modelForm.input')}</label>
              <form.Field name="limitInput"
                >{#snippet children(field)}
                  <Combobox
                    id="limit-input"
                    value={field.state.value}
                    onchange={(value) => field.handleChange(value)}
                    onblur={field.handleBlur}
                    aria-invalid={field.state.meta.errors.length ? 'true' : undefined}
                    aria-describedby={field.state.meta.errors.length ? 'limit-input-error' : undefined}
                    options={LIMIT_PRESETS}
                    inputmode="numeric"
                    disabled={saving}
                  />
                  {#if field.state.meta.errors.length}
                    <p id="limit-input-error" class="field-error" role="alert">{errorText(field.state.meta.errors)}</p>
                  {/if}
                {/snippet}</form.Field
              >
            </div>
          </div>
          <small class="muted">{i18n.t('modelForm.limitsHint')}</small>
        </div>
      </div>
    </fieldset>

    <fieldset class="group-fieldset">
      <legend>{i18n.t('modelForm.runtime')}</legend>
      <div class="form-grid">
        <div class="field">
          <p class="group-title">{i18n.t('modelForm.options')}</p>
          <form.Field name="settings"
            >{#snippet children(field)}
              <textarea
                id="model-settings"
                value={field.state.value}
                oninput={(event) => field.handleChange(event.currentTarget.value)}
                onblur={field.handleBlur}
                aria-invalid={field.state.meta.errors.length ? 'true' : undefined}
                aria-describedby={field.state.meta.errors.length ? 'model-settings-error' : undefined}
                disabled={saving}
              ></textarea>
              {#if field.state.meta.errors.length}
                <p id="model-settings-error" class="field-error" role="alert">{errorText(field.state.meta.errors)}</p>
              {/if}
            {/snippet}</form.Field
          >
          <small class="muted">{i18n.t('modelForm.optionsHint')}</small>
        </div>
        <div class="field">
          <p class="group-title">{i18n.t('modelForm.headers')}</p>
          <form.Field name="headers"
            >{#snippet children(field)}
              <textarea
                id="model-headers"
                value={field.state.value}
                oninput={(event) => field.handleChange(event.currentTarget.value)}
                onblur={field.handleBlur}
                aria-invalid={field.state.meta.errors.length ? 'true' : undefined}
                aria-describedby={field.state.meta.errors.length ? 'model-headers-error' : undefined}
                disabled={saving}
              ></textarea>
              {#if field.state.meta.errors.length}
                <p id="model-headers-error" class="field-error" role="alert">{errorText(field.state.meta.errors)}</p>
              {/if}
            {/snippet}</form.Field
          >
          <small class="muted">{i18n.t('modelForm.headersHint')}</small>
        </div>
        <div class="field full">
          <p class="group-title">{i18n.t('modelForm.variants')}</p>
          {#each variants.current as variant, index}
            <div class="variant-row">
              <form.Field name={`variants[${index}].key`}
                >{#snippet children(field)}
                  <input
                    aria-label={i18n.t('modelForm.variantName')}
                    placeholder={i18n.t('modelForm.variantNamePlaceholder')}
                    value={field.state.value}
                    oninput={(event) => field.handleChange(event.currentTarget.value)}
                    onblur={field.handleBlur}
                    disabled={saving}
                  >
                {/snippet}</form.Field
              >
              <form.Field name={`variants[${index}].json`}
                >{#snippet children(field)}
                  <textarea
                    aria-label={i18n.t('modelForm.variantOptionsJson')}
                    placeholder={'{}'}
                    value={field.state.value}
                    oninput={(event) => field.handleChange(event.currentTarget.value)}
                    onblur={field.handleBlur}
                    aria-invalid={field.state.meta.errors.length ? 'true' : undefined}
                    aria-describedby={field.state.meta.errors.length ? `variant-${index}-error` : undefined}
                    disabled={saving}
                  ></textarea>
                  {#if field.state.meta.errors.length}
                    <p id={`variant-${index}-error`} class="field-error" role="alert">
                      {errorText(field.state.meta.errors)}
                    </p>
                  {/if}
                {/snippet}</form.Field
              >
              <Button
                type="button"
                size="icon-sm"
                variant="ghost"
                aria-label={i18n.t('modelForm.removeVariant')}
                disabled={saving}
                onclick={() => removeVariant(index)}
                ><Trash2 size={14} /></Button
              >
            </div>
          {/each}
          <div class="variants-actions">
            <Button type="button" size="sm" variant="outline" disabled={saving} onclick={addVariant}
              ><Plus size={14} /> {i18n.t('modelForm.addVariant')}</Button
            >
          </div>
          <small class="muted">{i18n.t('modelForm.variantsHint')}</small>
        </div>
      </div>
    </fieldset>
  </div>
  <FormActions
    ><Button href="/models" variant="outline" disabled={saving}>{i18n.t('common.cancel')}</Button
    ><Button type="submit" disabled={saving || (mode === 'new' && !selectedProvider.current)}
      >{submitLabel}</Button
    ></FormActions
  >
</form>
