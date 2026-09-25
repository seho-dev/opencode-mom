<script lang="ts">
import { Plus, Trash2 } from '@lucide/svelte';
import { Button } from '$lib/components/ui/button/index.js';
import { Checkbox } from '$lib/components/ui/checkbox/index.js';
import { Switch } from '$lib/components/ui/switch/index.js';
import type { ModelDef, ModelModality, ModelVariant, ProviderDef } from '$lib/features/config/types.js';
import { getI18n } from '$lib/features/i18n/context.js';
import FormActions from './FormActions.svelte';
import Select from './Select.svelte';
import { toast } from './toast.svelte.js';

type VariantRow = { key: string; json: string };

const MODALITIES: ModelModality[] = ['text', 'audio', 'image', 'video', 'pdf']; // fixed schema enum
const emptyVariant = (): VariantRow => ({ key: '', json: '{}' });
const pretty = (value: unknown) => JSON.stringify(value ?? {}, null, 2);
const numStr = (value: number | undefined) => (value === undefined ? '' : String(value));
const initMods = (values?: ModelModality[]) => {
  const selected: Record<ModelModality, boolean> = {
    text: false,
    audio: false,
    image: false,
    video: false,
    pdf: false,
  };
  for (const value of values ?? []) selected[value] = true;
  return selected;
};

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

let providerId = $state('');
let id = $state('');
let family = $state('');
let disabled = $state(false);
let tools = $state(false);
// Whether the stored model already carries a capabilities block (edit mode only).
let capabilitiesPresent = $state(false);
let costInput = $state('');
let costOutput = $state('');
let costCacheRead = $state('');
let costCacheWrite = $state('');
let limitContext = $state('');
let limitOutput = $state('');
let limitInput = $state('');
let modalityInput = $state<Record<ModelModality, boolean>>(initMods());
let modalityOutput = $state<Record<ModelModality, boolean>>(initMods());
let settings = $state('{}');
let headers = $state('{}');
let variants = $state<VariantRow[]>([emptyVariant()]);
let errors = $state<Record<string, string>>({});
let initialized = $state('');

$effect(() => {
  const key = initial ? `${defaultProviderId}/${initial.id}` : '';
  if (mode !== 'edit' || !initial || initialized === key) return;
  initialized = key;
  providerId = defaultProviderId;
  id = initial.id;
  family = initial.family ?? '';
  disabled = initial.disabled === true;
  capabilitiesPresent = initial.capabilities !== undefined;
  tools = initial.capabilities?.tools ?? false;
  costInput = numStr(initial.cost?.input);
  costOutput = numStr(initial.cost?.output);
  costCacheRead = numStr(initial.cost?.cache?.read);
  costCacheWrite = numStr(initial.cost?.cache?.write);
  limitContext = numStr(initial.limit?.context);
  limitOutput = numStr(initial.limit?.output);
  limitInput = numStr(initial.limit?.input);
  modalityInput = initMods(initial.capabilities?.input);
  modalityOutput = initMods(initial.capabilities?.output);
  settings = pretty(initial.settings);
  headers = pretty(initial.headers);
  const rows = initial.variants ?? [];
  variants = rows.length
    ? rows.map((entry) => {
        const { id: variantId, ...rest } = entry ?? {};
        return { key: variantId ?? '', json: pretty(rest) };
      })
    : [emptyVariant()];
});

function addVariant() {
  variants = [...variants, emptyVariant()];
}
function removeVariant(index: number) {
  variants = variants.filter((_, i) => i !== index);
}

async function submit() {
  errors = {};
  const parseObject = (text: string, field: string): Record<string, unknown> | undefined => {
    const trimmed = text.trim();
    if (!trimmed) return {};
    try {
      const parsed: unknown = JSON.parse(trimmed);
      if (!parsed || Array.isArray(parsed) || typeof parsed !== 'object') throw new Error('not an object');
      return parsed as Record<string, unknown>;
    } catch {
      errors[field] = i18n.t('validation.jsonObject');
      return undefined;
    }
  };
  const num = (value: string) => (value.trim() === '' ? undefined : Number(value));
  const anyFilled = (...values: string[]) => values.some((value) => value.trim() !== '');

  const settingsValue = parseObject(settings, 'settings');
  const headersValue = parseObject(headers, 'headers');
  if (headersValue && Object.entries(headersValue).some(([, header]) => typeof header !== 'string'))
    errors['headers'] = i18n.t('validation.headersStrings');

  const variantEntries: { key: string; entry: Record<string, unknown> }[] = [];
  for (const row of variants) {
    if (!row.key.trim()) continue;
    const parsed = parseObject(row.json, 'variants');
    if (parsed === undefined) continue;
    variantEntries.push({ key: row.key.trim(), entry: parsed });
  }

  const costUsed = anyFilled(costInput, costOutput, costCacheRead, costCacheWrite);
  const limitUsed = anyFilled(limitContext, limitOutput, limitInput);
  if (costUsed && (!costInput.trim() || !costOutput.trim())) errors['cost'] = i18n.t('validation.costRequired');
  if (limitUsed && (!limitContext.trim() || !limitOutput.trim())) errors['limit'] = i18n.t('validation.limitRequired');

  if (Object.keys(errors).length) {
    toast({ variant: 'error', description: i18n.t('toast.fixFields') });
    return;
  }

  const inputModalities = MODALITIES.filter((modality) => modalityInput[modality]);
  const outputModalities = MODALITIES.filter((modality) => modalityOutput[modality]);
  const cacheUsed = anyFilled(costCacheRead, costCacheWrite);
  // Only touch capabilities when the stored model has them or the user set something.
  const capabilitiesNeeded = capabilitiesPresent || tools || inputModalities.length > 0 || outputModalities.length > 0;
  const value: ModelDef = {
    id: id.trim(),
    ...(family.trim() && { family: family.trim() }),
    // An explicit disable is written; an existing disable can only be undone explicitly.
    ...((disabled || (mode === 'edit' && initial?.disabled === true)) && { disabled }),
    ...(capabilitiesNeeded && {
      capabilities: {
        tools,
        ...(inputModalities.length && { input: inputModalities }),
        ...(outputModalities.length && { output: outputModalities }),
      },
    }),
    ...(costUsed && {
      cost: {
        input: num(costInput) as number,
        output: num(costOutput) as number,
        ...(cacheUsed && {
          cache: {
            ...(costCacheRead.trim() && { read: num(costCacheRead) as number }),
            ...(costCacheWrite.trim() && { write: num(costCacheWrite) as number }),
          },
        }),
      },
    }),
    ...(limitUsed && {
      limit: {
        context: num(limitContext) as number,
        output: num(limitOutput) as number,
        ...(limitInput.trim() && { input: num(limitInput) as number }),
      },
    }),
    ...(settingsValue && Object.keys(settingsValue).length && { settings: settingsValue }),
    ...(headersValue && Object.keys(headersValue).length && { headers: headersValue as Record<string, string> }),
    ...(variantEntries.length && {
      variants: variantEntries.map(({ key, entry }) => ({ id: key, ...entry }) as ModelVariant),
    }),
  };

  try {
    // The owning provider is fixed when editing; only a new model chooses one.
    await onSave(mode === 'edit' ? defaultProviderId : providerId, value);
  } catch (error) {
    toast({ variant: 'error', description: error instanceof Error ? error.message : i18n.t('toast.saveFailed') });
  }
}
</script>

<form
  class="panel form-panel"
  onsubmit={(event) => {
    event.preventDefault();
    submit();
  }}
>
  <div class="form-grid">
    <fieldset class="group-fieldset">
      <legend>{i18n.t('modelForm.identity')}</legend>
      <div class="form-grid">
        {#if mode === 'new'}
          <div class="field">
            <label for="model-provider">{i18n.t('modelForm.provider')}</label>
            <Select
              id="model-provider"
              bind:value={providerId}
              disabled={saving}
              placeholder={i18n.t('modelForm.selectProvider')}
              options={providers.map((provider) => ({ value: provider.name }))}
            />
          </div>
          <div class="field">
            <label for="model-id">{i18n.t('modelForm.modelId')}</label>
            <input
              id="model-id"
              bind:value={id}
              required
              disabled={saving}
              placeholder={i18n.t('modelForm.modelIdPlaceholder')}
            >
          </div>
        {:else}
          <div class="field">
            <label for="model-provider">{i18n.t('modelForm.provider')}</label>
            <input id="model-provider" value={defaultProviderId} disabled>
          </div>
          <div class="field">
            <label for="model-id">{i18n.t('modelForm.modelId')}</label>
            <input id="model-id" value={id} disabled>
          </div>
        {/if}
        <div class="field">
          <label for="model-family">{i18n.t('modelForm.family')}</label>
          <input
            id="model-family"
            bind:value={family}
            disabled={saving}
            placeholder={i18n.t('modelForm.familyPlaceholder')}
          >
        </div>
        <div class="field">
          <div class="check-row">
            <Switch id="model-disabled" bind:checked={disabled} disabled={saving} />
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
            <Switch id="model-tools" bind:checked={tools} disabled={saving} />
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
                    <Checkbox id="model-input-{modality}" bind:checked={modalityInput[modality]} disabled={saving} />
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
                    <Checkbox id="model-output-{modality}" bind:checked={modalityOutput[modality]} disabled={saving} />
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
              <label for="cost-input">{i18n.t('modelForm.inputRequired')}</label>
              <input id="cost-input" type="number" step="any" bind:value={costInput} disabled={saving}>
            </div>
            <div class="sub-field">
              <label for="cost-output">{i18n.t('modelForm.outputRequired')}</label>
              <input id="cost-output" type="number" step="any" bind:value={costOutput} disabled={saving}>
            </div>
            <div class="sub-field">
              <label for="cost-cache-read">{i18n.t('modelForm.cacheRead')}</label>
              <input id="cost-cache-read" type="number" step="any" bind:value={costCacheRead} disabled={saving}>
            </div>
            <div class="sub-field">
              <label for="cost-cache-write">{i18n.t('modelForm.cacheWrite')}</label>
              <input id="cost-cache-write" type="number" step="any" bind:value={costCacheWrite} disabled={saving}>
            </div>
          </div>
          <small class="muted">{i18n.t('modelForm.costHint')}</small>
          {#if errors['cost']}
            <p class="field-error">{errors['cost']}</p>
          {/if}
        </div>
        <div class="field full">
          <p class="group-title">{i18n.t('modelForm.limits')}</p>
          <div class="num-grid">
            <div class="sub-field">
              <label for="limit-context">{i18n.t('modelForm.contextRequired')}</label>
              <input id="limit-context" type="number" min="0" step="1" bind:value={limitContext} disabled={saving}>
            </div>
            <div class="sub-field">
              <label for="limit-output">{i18n.t('modelForm.outputRequired')}</label>
              <input id="limit-output" type="number" min="0" step="1" bind:value={limitOutput} disabled={saving}>
            </div>
            <div class="sub-field">
              <label for="limit-input">{i18n.t('modelForm.input')}</label>
              <input id="limit-input" type="number" min="0" step="1" bind:value={limitInput} disabled={saving}>
            </div>
          </div>
          <small class="muted">{i18n.t('modelForm.limitsHint')}</small>
          {#if errors['limit']}
            <p class="field-error">{errors['limit']}</p>
          {/if}
        </div>
      </div>
    </fieldset>

    <fieldset class="group-fieldset">
      <legend>{i18n.t('modelForm.runtime')}</legend>
      <div class="form-grid">
        <div class="field">
          <p class="group-title">{i18n.t('modelForm.options')}</p>
          <textarea id="model-settings" bind:value={settings} disabled={saving}></textarea>
          <small class="muted">{i18n.t('modelForm.optionsHint')}</small>
          {#if errors['settings']}
            <p class="field-error">{errors['settings']}</p>
          {/if}
        </div>
        <div class="field">
          <p class="group-title">{i18n.t('modelForm.headers')}</p>
          <textarea id="model-headers" bind:value={headers} disabled={saving}></textarea>
          <small class="muted">{i18n.t('modelForm.headersHint')}</small>
          {#if errors['headers']}
            <p class="field-error">{errors['headers']}</p>
          {/if}
        </div>
        <div class="field full">
          <p class="group-title">{i18n.t('modelForm.variants')}</p>
          {#each variants as variant, index}
            <div class="variant-row">
              <input
                aria-label={i18n.t('modelForm.variantName')}
                placeholder={i18n.t('modelForm.variantNamePlaceholder')}
                bind:value={variant.key}
                disabled={saving}
              >
              <textarea
                aria-label={i18n.t('modelForm.variantOptionsJson')}
                placeholder={'{}'}
                bind:value={variant.json}
                disabled={saving}
              ></textarea>
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
          {#if errors['variants']}
            <p class="field-error">{errors['variants']}</p>
          {/if}
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
    ><Button type="submit" disabled={saving || (mode === 'new' && !providerId)}>{submitLabel}</Button></FormActions
  >
</form>
