<script lang="ts">
  import { Plus, Trash2 } from '@lucide/svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import { Checkbox } from '$lib/components/ui/checkbox/index.js';
  import { Switch } from '$lib/components/ui/switch/index.js';
  import FormActions from './FormActions.svelte';
  import { toast } from './toast.svelte.js';
  import type { ModelDef, ModelModality, ProviderDef } from '$lib/features/config/types.js';

  type InterleavedChoice = 'unset' | 'true' | 'false' | 'reasoning' | 'reasoning_content' | 'reasoning_text' | 'custom';
  type VariantRow = { key: string; json: string; disabled: boolean };

  const MODALITIES: ModelModality[] = ['text', 'audio', 'image', 'video', 'pdf']; // fixed schema enum
  const emptyVariant = (): VariantRow => ({ key: '', json: '{}', disabled: false });
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

  let {
    mode,
    providers = [],
    defaultProviderId = '',
    initial,
    saving = false,
    submitLabel = 'Save model',
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
  let releaseDate = $state('');
  let status = $state<ModelDef['status']>('active');
  let temperature = $state(''); // '' | 'true' | 'false'
  let reasoning = $state(true);
  let toolCall = $state(true);
  let attachment = $state(false);
  let experimental = $state(false);
  let interleaved = $state<InterleavedChoice>('unset');
  let interleavedCustom = $state('{}');
  let costInput = $state('');
  let costOutput = $state('');
  let costCacheRead = $state('');
  let costCacheWrite = $state('');
  let costCtxInput = $state('');
  let costCtxOutput = $state('');
  let costCtxCacheRead = $state('');
  let costCtxCacheWrite = $state('');
  let limitContext = $state('');
  let limitOutput = $state('');
  let limitInput = $state('');
  let modalityInput = $state<Record<ModelModality, boolean>>(initMods(['text']));
  let modalityOutput = $state<Record<ModelModality, boolean>>(initMods(['text']));
  let options = $state('{}');
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
    releaseDate = initial.release_date ?? '';
    status = initial.status ?? 'active';
    temperature = initial.temperature === undefined ? '' : String(initial.temperature);
    reasoning = initial.reasoning ?? false;
    toolCall = initial.tool_call ?? false;
    attachment = initial.attachment ?? false;
    experimental = initial.experimental ?? false;
    const value = initial.interleaved;
    if (value === undefined) interleaved = 'unset';
    else if (typeof value === 'boolean') interleaved = String(value) as InterleavedChoice;
    else if (typeof value === 'string') interleaved = value as InterleavedChoice;
    else {
      interleaved = 'custom';
      interleavedCustom = pretty(value);
    }
    costInput = numStr(initial.cost?.input);
    costOutput = numStr(initial.cost?.output);
    costCacheRead = numStr(initial.cost?.cache_read);
    costCacheWrite = numStr(initial.cost?.cache_write);
    costCtxInput = numStr(initial.cost?.context_over_200k?.input);
    costCtxOutput = numStr(initial.cost?.context_over_200k?.output);
    costCtxCacheRead = numStr(initial.cost?.context_over_200k?.cache_read);
    costCtxCacheWrite = numStr(initial.cost?.context_over_200k?.cache_write);
    limitContext = numStr(initial.limit?.context);
    limitOutput = numStr(initial.limit?.output);
    limitInput = numStr(initial.limit?.input);
    modalityInput = initMods(initial.modalities?.input);
    modalityOutput = initMods(initial.modalities?.output);
    options = pretty(initial.options);
    headers = pretty(initial.headers);
    const rows = Object.entries(initial.variants ?? {});
    variants = rows.length
      ? rows.map(([variantKey, entry]) => {
          const { disabled, ...rest } = (entry ?? {}) as { disabled?: boolean } & Record<string, unknown>;
          return { key: variantKey, json: pretty(rest), disabled: disabled === true };
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
        errors[field] = 'Must be a valid JSON object, not an array or scalar.';
        return undefined;
      }
    };
    const num = (value: string) => (value.trim() === '' ? undefined : Number(value));
    const anyFilled = (...values: string[]) => values.some((value) => value.trim() !== '');

    const optionsValue = parseObject(options, 'options');
    const headersValue = parseObject(headers, 'headers');
    if (headersValue && Object.entries(headersValue).some(([, header]) => typeof header !== 'string'))
      errors['headers'] = 'Header values must be strings.';
    const interleavedValue = interleaved === 'custom' ? parseObject(interleavedCustom, 'interleaved') : undefined;
    if (
      interleavedValue &&
      (typeof interleavedValue['field'] !== 'string' || !String(interleavedValue['field']).trim())
    )
      errors['interleaved'] = 'Must be an object with a non-empty string "field", e.g. { "field": "reasoning" }.';

    const variantEntries: { key: string; entry: Record<string, unknown> }[] = [];
    for (const row of variants) {
      if (!row.key.trim()) continue;
      const parsed = parseObject(row.json, 'variants');
      if (parsed === undefined) continue;
      variantEntries.push({
        key: row.key.trim(),
        entry: { ...parsed, ...(row.disabled ? { disabled: true } : {}) },
      });
    }

    const costUsed = anyFilled(costInput, costOutput, costCacheRead, costCacheWrite);
    const ctxUsed = anyFilled(costCtxInput, costCtxOutput, costCtxCacheRead, costCtxCacheWrite);
    const limitUsed = anyFilled(limitContext, limitOutput, limitInput);
    // context_over_200k sits inside cost, so base input/output are required whenever either block is used.
    if ((costUsed || ctxUsed) && (!costInput.trim() || !costOutput.trim()))
      errors['cost'] = 'Input and output are required.';
    if (ctxUsed && (!costCtxInput.trim() || !costCtxOutput.trim()))
      errors['costCtx'] = 'context_over_200k needs input and output.';
    if (limitUsed && (!limitContext.trim() || !limitOutput.trim()))
      errors['limit'] = 'Context and output are required.';

    if (Object.keys(errors).length) {
      toast({ variant: 'error', description: 'Fix the highlighted fields before saving.' });
      return;
    }

    const inputModalities = MODALITIES.filter((modality) => modalityInput[modality]);
    const outputModalities = MODALITIES.filter((modality) => modalityOutput[modality]);
    const value: ModelDef = {
      id: id.trim(),
      ...(family.trim() && { family: family.trim() }),
      ...(releaseDate && { release_date: releaseDate }),
      status,
      ...(temperature !== '' && { temperature: temperature === 'true' }),
      ...(reasoning && { reasoning: true }),
      ...(toolCall && { tool_call: true }),
      ...(attachment && { attachment: true }),
      ...(experimental && { experimental: true }),
      ...(interleaved !== 'unset' && {
        interleaved:
          interleaved === 'custom'
            ? { field: String(interleavedValue?.['field']) }
            : interleaved === 'true'
              ? true
              : interleaved === 'false'
                ? false
                : interleaved,
      }),
      ...((costUsed || ctxUsed) && {
        cost: {
          input: num(costInput) as number,
          output: num(costOutput) as number,
          ...(costCacheRead.trim() && { cache_read: num(costCacheRead) as number }),
          ...(costCacheWrite.trim() && { cache_write: num(costCacheWrite) as number }),
          ...(ctxUsed && {
            context_over_200k: {
              input: num(costCtxInput) as number,
              output: num(costCtxOutput) as number,
              ...(costCtxCacheRead.trim() && { cache_read: num(costCtxCacheRead) as number }),
              ...(costCtxCacheWrite.trim() && { cache_write: num(costCtxCacheWrite) as number }),
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
      ...(inputModalities.length || outputModalities.length
        ? {
            modalities: {
              ...(inputModalities.length && { input: inputModalities }),
              ...(outputModalities.length && { output: outputModalities }),
            },
          }
        : {}),
      ...(optionsValue && Object.keys(optionsValue).length && { options: optionsValue }),
      ...(headersValue && Object.keys(headersValue).length && { headers: headersValue as Record<string, string> }),
      ...(variantEntries.length && {
        variants: Object.fromEntries(variantEntries.map(({ key, entry }) => [key, entry])),
      }),
    };

    try {
      // The owning provider is fixed when editing; only a new model chooses one.
      await onSave(mode === 'edit' ? defaultProviderId : providerId, value);
    } catch (error) {
      toast({ variant: 'error', description: error instanceof Error ? error.message : 'Save failed.' });
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
      <legend>Identity</legend>
      <div class="form-grid">
        {#if mode === 'new'}
          <div class="field">
            <label for="model-provider">Provider</label>
            <select id="model-provider" bind:value={providerId} required disabled={saving}>
              <option value="" disabled>Select a provider</option>
              {#each providers as provider}<option value={provider.name}>{provider.name}</option>{/each}
            </select>
          </div>
          <div class="field">
            <label for="model-id">Model ID</label>
            <input id="model-id" bind:value={id} required disabled={saving} placeholder="claude-sonnet-4" />
          </div>
        {:else}
          <div class="field">
            <label for="model-provider">Provider</label>
            <input id="model-provider" value={defaultProviderId} disabled />
          </div>
          <div class="field">
            <label for="model-id">Model ID</label>
            <input id="model-id" value={id} disabled />
          </div>
        {/if}
        <div class="field">
          <label for="model-family">Family</label>
          <input id="model-family" bind:value={family} disabled={saving} placeholder="claude" />
        </div>
        <div class="field">
          <label for="model-release-date">Release date</label>
          <input id="model-release-date" type="date" bind:value={releaseDate} disabled={saving} />
        </div>
        <div class="field">
          <label for="model-status">Status</label>
          <select id="model-status" bind:value={status} disabled={saving}>
            <option value="active">active</option>
            <option value="alpha">alpha</option>
            <option value="beta">beta</option>
            <option value="deprecated">deprecated</option>
          </select>
        </div>
      </div>
    </fieldset>

    <fieldset class="group-fieldset">
      <legend>Capabilities</legend>
      <div class="form-grid">
        <div class="field full">
          <div class="grid grid-cols-2 gap-x-6 gap-y-3">
            <div class="check-row">
              <Switch id="model-reasoning" bind:checked={reasoning} disabled={saving} /><label for="model-reasoning"
                >Reasoning</label
              >
            </div>
            <div class="check-row">
              <Switch id="model-tool-call" bind:checked={toolCall} disabled={saving} /><label for="model-tool-call"
                >Tool call</label
              >
            </div>
            <div class="check-row">
              <Switch id="model-attachment" bind:checked={attachment} disabled={saving} /><label for="model-attachment"
                >Attachment</label
              >
            </div>
            <div class="check-row">
              <Switch id="model-experimental" bind:checked={experimental} disabled={saving} /><label
                for="model-experimental">Experimental</label
              >
            </div>
          </div>
          <small class="muted">Unticked capabilities stay unset and inherit the provider or opencode defaults.</small>
        </div>
        <div class="field">
          <label for="model-temperature">Temperature</label>
          <select id="model-temperature" bind:value={temperature} disabled={saving}>
            <option value="">Unset</option>
            <option value="true">true</option>
            <option value="false">false</option>
          </select>
          <small class="muted">Whether the model accepts a temperature setting.</small>
        </div>
        <div class="field">
          <label for="model-interleaved">Interleaved</label>
          <select id="model-interleaved" bind:value={interleaved} disabled={saving}>
            <option value="unset">Unset</option>
            <option value="true">true</option>
            <option value="false">false</option>
            <option value="reasoning">reasoning</option>
            <option value="reasoning_content">reasoning_content</option>
            <option value="reasoning_text">reasoning_text</option>
            <option value="custom">custom (JSON)</option>
          </select>
          <small class="muted">Where reasoning content is placed in the response when present.</small>
        </div>
        {#if interleaved === 'custom'}<div class="field full">
            <label for="model-interleaved-custom">Interleaved object</label>
            <textarea id="model-interleaved-custom" class="small" bind:value={interleavedCustom} disabled={saving}
            ></textarea>
            <small class="muted">JSON object of the form {'{ "field": "reasoning" }'}.</small>
            {#if errors['interleaved']}<p class="field-error">{errors['interleaved']}</p>{/if}
          </div>{/if}
        <div class="field full">
          <div class="grid grid-cols-2 gap-x-8">
            <div>
              <p class="group-title">Input modalities</p>
              <div class="grid gap-y-2">
                {#each MODALITIES as modality}<div class="check-row">
                    <Checkbox id="model-input-{modality}" bind:checked={modalityInput[modality]} disabled={saving} />
                    <label for="model-input-{modality}">{modality}</label>
                  </div>{/each}
              </div>
            </div>
            <div>
              <p class="group-title">Output modalities</p>
              <div class="grid gap-y-2">
                {#each MODALITIES as modality}<div class="check-row">
                    <Checkbox id="model-output-{modality}" bind:checked={modalityOutput[modality]} disabled={saving} />
                    <label for="model-output-{modality}">{modality}</label>
                  </div>{/each}
              </div>
            </div>
          </div>
        </div>
      </div>
    </fieldset>

    <fieldset class="group-fieldset">
      <legend>Limits &amp; cost</legend>
      <div class="form-grid">
        <div class="field full">
          <p class="group-title">Cost</p>
          <div class="num-grid">
            <div class="sub-field">
              <label for="cost-input">Input *</label>
              <input id="cost-input" type="number" step="any" bind:value={costInput} disabled={saving} />
            </div>
            <div class="sub-field">
              <label for="cost-output">Output *</label>
              <input id="cost-output" type="number" step="any" bind:value={costOutput} disabled={saving} />
            </div>
            <div class="sub-field">
              <label for="cost-cache-read">Cache read</label>
              <input id="cost-cache-read" type="number" step="any" bind:value={costCacheRead} disabled={saving} />
            </div>
            <div class="sub-field">
              <label for="cost-cache-write">Cache write</label>
              <input id="cost-cache-write" type="number" step="any" bind:value={costCacheWrite} disabled={saving} />
            </div>
          </div>
          <p class="num-section-label">context_over_200k</p>
          <div class="num-grid">
            <div class="sub-field">
              <label for="cost-ctx-input">Input</label>
              <input id="cost-ctx-input" type="number" step="any" bind:value={costCtxInput} disabled={saving} />
            </div>
            <div class="sub-field">
              <label for="cost-ctx-output">Output</label>
              <input id="cost-ctx-output" type="number" step="any" bind:value={costCtxOutput} disabled={saving} />
            </div>
            <div class="sub-field">
              <label for="cost-ctx-cache-read">Cache read</label>
              <input
                id="cost-ctx-cache-read"
                type="number"
                step="any"
                bind:value={costCtxCacheRead}
                disabled={saving}
              />
            </div>
            <div class="sub-field">
              <label for="cost-ctx-cache-write">Cache write</label>
              <input
                id="cost-ctx-cache-write"
                type="number"
                step="any"
                bind:value={costCtxCacheWrite}
                disabled={saving}
              />
            </div>
          </div>
          <small class="muted">Price in USD per 1M tokens; cache rates sit on top of the base input price.</small>
          {#if errors['cost']}<p class="field-error">{errors['cost']}</p>{/if}
          {#if errors['costCtx']}<p class="field-error">{errors['costCtx']}</p>{/if}
        </div>
        <div class="field full">
          <p class="group-title">Limits</p>
          <div class="num-grid">
            <div class="sub-field">
              <label for="limit-context">Context *</label>
              <input id="limit-context" type="number" min="0" step="1" bind:value={limitContext} disabled={saving} />
            </div>
            <div class="sub-field">
              <label for="limit-output">Output *</label>
              <input id="limit-output" type="number" min="0" step="1" bind:value={limitOutput} disabled={saving} />
            </div>
            <div class="sub-field">
              <label for="limit-input">Input</label>
              <input id="limit-input" type="number" min="0" step="1" bind:value={limitInput} disabled={saving} />
            </div>
          </div>
          <small class="muted"
            >Token counts: context is the full window, output caps generation, input optionally caps the prompt.</small
          >
          {#if errors['limit']}<p class="field-error">{errors['limit']}</p>{/if}
        </div>
      </div>
    </fieldset>

    <fieldset class="group-fieldset">
      <legend>Runtime</legend>
      <div class="form-grid">
        <div class="field">
          <p class="group-title">Options</p>
          <textarea id="model-options" bind:value={options} disabled={saving}></textarea>
          <small class="muted">Arbitrary provider options passed to the model client.</small>
          {#if errors['options']}<p class="field-error">{errors['options']}</p>{/if}
        </div>
        <div class="field">
          <p class="group-title">Headers</p>
          <textarea id="model-headers" bind:value={headers} disabled={saving}></textarea>
          <small class="muted">Extra HTTP headers, e.g. {'{ "x-api-key": "..." }'}.</small>
          {#if errors['headers']}<p class="field-error">{errors['headers']}</p>{/if}
        </div>
        <div class="field full">
          <p class="group-title">Variants</p>
          {#each variants as variant, index}
            <div class="variant-row">
              <input aria-label="Variant name" placeholder="variant-name" bind:value={variant.key} disabled={saving} />
              <textarea aria-label="Variant options JSON" placeholder={'{}'} bind:value={variant.json} disabled={saving}
              ></textarea>
              <div class="variant-toggle">
                <Switch
                  id="variant-disabled-{index}"
                  aria-label="Disable variant {index + 1}"
                  bind:checked={variant.disabled}
                  disabled={saving}
                />
                <span>disabled</span>
              </div>
              <Button
                type="button"
                size="icon-sm"
                variant="ghost"
                aria-label="Remove variant"
                disabled={saving}
                onclick={() => removeVariant(index)}><Trash2 size={14} /></Button
              >
            </div>
          {/each}
          {#if errors['variants']}<p class="field-error">{errors['variants']}</p>{/if}
          <div class="variants-actions">
            <Button type="button" size="sm" variant="outline" disabled={saving} onclick={addVariant}
              ><Plus size={14} /> Add variant</Button
            >
          </div>
          <small class="muted"
            >Variant name → options expected by opencode, e.g. {'{ "reasoningEffort": "high" }'}. Use the toggle to
            disable a variant.</small
          >
        </div>
      </div>
    </fieldset>
  </div>
  <FormActions
    ><Button href="/models" variant="outline" disabled={saving}>Cancel</Button><Button
      type="submit"
      disabled={saving || (mode === 'new' && !providerId)}>{submitLabel}</Button
    ></FormActions
  >
</form>
