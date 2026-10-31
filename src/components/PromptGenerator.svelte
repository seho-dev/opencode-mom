<script lang="ts">
import { Sparkles } from '@lucide/svelte';
import { onDestroy } from 'svelte';
import { page } from '$app/state';
import { Button } from '$src/components/button/index.js';
import * as Dialog from '$src/components/dialog/index.js';
import Select from '$src/components/Select.svelte';
import { Spinner } from '$src/components/spinner/index.js';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import type { ModelRef } from '$src/types/models.js';
import {
  PROMPT_GENERATION_PRESETS,
  type PromptGenerationKind,
  type PromptGenerationPreset,
} from '$src/types/prompt-generation.js';
import { buildModelOptions, buildModelVariants } from '$src/utils/index.js';

let {
  kind,
  value,
  disabled = false,
  onapply,
}: {
  kind: PromptGenerationKind;
  value: string;
  disabled?: boolean;
  onapply: (text: string) => void;
} = $props();

const config = getConfig();
const i18n = getI18n();
const uid = $props.id();
let open = $state(false);
let preset = $state<PromptGenerationPreset>('generate');
let model = $state('');
let variant = $state('');
let instructions = $state('');
let originalPrompt = $state('');
let originalKind = $state<PromptGenerationKind>('agent');
let originalRoute = $state('');
let originalTarget = $state(0);
let invalidated = $state(false);
let pending = $state(false);
let result = $state<string | null>(null);
let error = $state('');
let instructionsElement = $state<HTMLTextAreaElement | null>(null);
let requestToken = 0;
let active = true;

const modelOptions = $derived(
  open ? buildModelOptions(config.catalogModels(), config.models(), model, i18n.t('promptGenerator.defaultModel')) : [],
);
const variants = $derived(open ? buildModelVariants(config.catalogModels(), config.models(), model) : []);
const contextChanged = $derived(
  value !== originalPrompt ||
    kind !== originalKind ||
    page.url.href !== originalRoute ||
    config.serviceTargetVersion !== originalTarget,
);
const blocked = $derived(disabled || invalidated || contextChanged);
const canGenerate = $derived(open && !pending && !blocked && !!(instructions.trim() || originalPrompt.trim()));
const canApply = $derived(open && !pending && !blocked && result !== null && !!result.trim());

onDestroy(() => {
  active = false;
  requestToken += 1;
});

$effect(() => {
  if (open && !invalidated && (contextChanged || disabled)) {
    invalidated = true;
    requestToken += 1;
    pending = false;
  }
});

function setOpen(next: boolean) {
  if (next && disabled) return;
  requestToken += 1;
  pending = false;
  invalidated = false;
  error = '';
  result = null;
  instructions = '';
  model = '';
  variant = '';
  originalPrompt = next ? value : '';
  originalKind = kind;
  originalRoute = page.url.href;
  originalTarget = config.serviceTargetVersion;
  preset = originalPrompt.trim() ? 'improve' : 'generate';
  open = next;
}

async function generate() {
  if (!canGenerate) return;
  const token = ++requestToken;
  const current = () => active && open && token === requestToken && !blocked;
  pending = true;
  error = '';
  result = null;
  try {
    const text = await config.generatePrompt({
      kind: originalKind,
      preset,
      instructions: instructions.trim(),
      currentPrompt: originalPrompt,
      model: model ? (model as ModelRef) : undefined,
      variant: model && variants.includes(variant) ? variant : undefined,
    });
    if (!current()) return;
    if (text.trim()) result = text;
    else error = i18n.t('promptGenerator.emptyResult');
  } catch {
    if (current()) error = i18n.t('promptGenerator.failed');
  } finally {
    if (current()) pending = false;
  }
}

function apply() {
  if (!canApply || result === null) return;
  const text = result;
  setOpen(false);
  onapply(text);
}
</script>

<Dialog.Root bind:open={() => open, setOpen}>
  <div class="flex flex-wrap items-center gap-2">
    <Dialog.Trigger {disabled}>
      {#snippet child({ props })}
        <Button {...props} type="button" variant="outline" size="sm" {disabled}>
          <Sparkles size={14} aria-hidden="true" />
          {i18n.t('promptGenerator.open')}
        </Button>
      {/snippet}
    </Dialog.Trigger>
    <span class="field-hint">{i18n.t('promptGenerator.entryHint')}</span>
  </div>
  <Dialog.Content
    class="flex max-h-[calc(100dvh-32px)] max-w-[640px] flex-col gap-4 overflow-hidden"
    aria-busy={pending}
    onOpenAutoFocus={(event) => {
      event.preventDefault();
      instructionsElement?.focus();
    }}
  >
    <Dialog.Header>
      <Dialog.Title
        >{i18n.t(kind === 'agent' ? 'promptGenerator.agentTitle' : 'promptGenerator.commandTitle')}</Dialog.Title
      >
      <Dialog.Description>{i18n.t('promptGenerator.description')}</Dialog.Description>
    </Dialog.Header>

    <div class="form-body flex flex-col gap-4">
      <p class="m-0 text-[11px] leading-[18px] text-[var(--status-warning)]">
        {i18n.t('promptGenerator.experimental')}
      </p>
      {#if open && (invalidated || contextChanged)}
        <p class="field-error" role="alert">{i18n.t('promptGenerator.contextChanged')}</p>
      {/if}

      <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
        <div class="field min-w-0 content-start">
          <label for={`${uid}-model`}>{i18n.t('common.model')}</label>
          <Select
            id={`${uid}-model`}
            value={model}
            options={modelOptions}
            disabled={pending || blocked}
            onchange={(value) => {
              model = value;
              variant = '';
            }}
          />
        </div>
        <div class="field min-w-0 content-start">
          <label for={`${uid}-preset`}>{i18n.t('promptGenerator.preset')}</label>
          <Select
            id={`${uid}-preset`}
            value={preset}
            options={PROMPT_GENERATION_PRESETS.map((value) => ({
              value,
              label: i18n.t(`promptGenerator.preset.${value}`),
            }))}
            disabled={pending || blocked}
            onchange={(value) => (preset = value as PromptGenerationPreset)}
          />
          <small class="field-hint">{i18n.t(`promptGenerator.presetHint.${preset}`)}</small>
        </div>
        {#if model && variants.length > 0}
          <div class="field min-w-0 sm:col-span-2">
            <label for={`${uid}-variant`}>{i18n.t('agents.variant')}</label>
            <Select
              id={`${uid}-variant`}
              bind:value={variant}
              options={[
                { value: '', label: i18n.t('agents.optionNone') },
                ...variants.map((value) => ({ value })),
              ]}
              disabled={pending || blocked || config.catalogLoading}
            />
          </div>
        {/if}
      </div>

      <div class="field min-w-0">
        <label for={`${uid}-instructions`}>{i18n.t('promptGenerator.instructions')}</label>
        <textarea
          id={`${uid}-instructions`}
          class="!min-h-24 max-h-48 resize-y"
          rows="4"
          bind:this={instructionsElement}
          bind:value={instructions}
          disabled={pending || blocked}
          placeholder={i18n.t(kind === 'agent' ? 'promptGenerator.agentPlaceholder' : 'promptGenerator.commandPlaceholder')}
          aria-describedby={`${uid}-input-hint ${uid}-privacy`}
          spellcheck="false"
          autocomplete="off"
        ></textarea>
        <small id={`${uid}-input-hint`} class="field-hint">
          {i18n.t(originalPrompt.trim() ? 'promptGenerator.includesDraft' : 'promptGenerator.requirementsNeeded')}
        </small>
      </div>

      <p id={`${uid}-privacy`} class="field-hint">{i18n.t('promptGenerator.privacy')}</p>
      <div class="flex flex-wrap items-center gap-3">
        <Button type="button" variant="secondary" disabled={!canGenerate} aria-busy={pending} onclick={generate}>
          {#if pending}
            <Spinner size="sm" />
          {:else}
            <Sparkles size={14} aria-hidden="true" />
          {/if}
          {i18n.t(pending ? 'promptGenerator.generating' : 'promptGenerator.generate')}
        </Button>
        <span class="field-hint" role="status">
          {#if pending}
            {i18n.t('promptGenerator.waiting')}
          {:else if result !== null && !blocked}
            {i18n.t('promptGenerator.ready')}
          {/if}
        </span>
      </div>

      {#if error}
        <p class="field-error" role="alert">{error}</p>
      {/if}
      {#if result !== null}
        <div class="field min-w-0">
          <label for={`${uid}-preview`}>{i18n.t('promptGenerator.preview')}</label>
          <textarea
            id={`${uid}-preview`}
            class="!min-h-40 max-h-72 resize-y font-[var(--font-primary)] text-xs leading-6"
            rows="9"
            value={result}
            readonly
            spellcheck="false"
          ></textarea>
        </div>
      {/if}
    </div>

    <div class="flex shrink-0 flex-col gap-3 border-t border-[var(--border-subtle)] pt-4">
      <p id={`${uid}-close-hint`} class="field-hint">{i18n.t('promptGenerator.closeHint')}</p>
      <Dialog.Footer>
        <Button type="button" variant="outline" aria-describedby={`${uid}-close-hint`} onclick={() => setOpen(false)}>
          {i18n.t('common.close')}
        </Button>
        <Button type="button" disabled={!canApply} onclick={apply}>{i18n.t('promptGenerator.apply')}</Button>
      </Dialog.Footer>
    </div>
  </Dialog.Content>
</Dialog.Root>
