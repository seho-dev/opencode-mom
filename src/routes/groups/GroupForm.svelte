<script lang="ts">
import { Trash2 } from '@lucide/svelte';
import { createForm } from '@tanstack/svelte-form';
import { z } from 'zod';
import { goto } from '$app/navigation';
import { Button } from '$src/components/button/index.js';
import { Combobox } from '$src/components/combobox/index.js';
import * as Dialog from '$src/components/dialog/index.js';
import FormActions from '$src/components/FormActions.svelte';
import Select from '$src/components/Select.svelte';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import { toast } from '$src/shell/toast.svelte.js';
import type { AgentModelBinding, CategoryMapping, Group } from '$src/types/groups.js';
import {
  BUILTIN_CATEGORIES,
  GROUP_TYPE_LABELS,
  GROUP_TYPE_NATIVE,
  GROUP_TYPE_OMO,
  GROUP_TYPE_OPTIONS,
  GROUP_TYPE_SLIM,
  type GroupType,
  MAPPING_KINDS,
  type MappingKind,
} from '$src/utils/constants.js';
import { BUILTIN_AGENTS, buildModelVariants } from '$src/utils/index.js';

const config = getConfig();
const i18n = getI18n();
let { group }: { group?: Group } = $props();
const defaultValues = $derived({
  name: group?.name ?? '',
  description: group?.description ?? '',
  type: group?.type ?? GROUP_TYPE_NATIVE,
  native: group?.openCodeAgentOverrides ?? ([] as AgentModelBinding[]),
  slim: group?.slimAgentOverrides ?? ([] as AgentModelBinding[]),
  omo: group?.omoAgentOverrides ?? ([] as AgentModelBinding[]),
  categories: group?.omoCategoryMappings ?? ([] as CategoryMapping[]),
  isEnabled: group?.isEnabled ?? false,
});
const form = createForm(() => ({
  defaultValues,
  validators: {
    onChange: z.object({ name: z.string().trim().min(1, i18n.t('groupForm.nameRequired')) }),
  },
  onSubmit: async ({ value }) => {
    let id = group?.id;
    const updatedAt = group?.updatedAt ?? new Date().toISOString();
    if (!id) {
      if (!globalThis.crypto?.randomUUID) {
        toast({ variant: 'error', description: i18n.t('groupForm.noRandomUuid') });
        return;
      }
      id = globalThis.crypto.randomUUID();
    }
    try {
      await config.saveGroup({
        id,
        name: value.name.trim(),
        description: value.description,
        type: value.type,
        openCodeAgentOverrides: value.native,
        slimAgentOverrides: value.slim.length ? value.slim : null,
        omoAgentOverrides: value.omo.length ? value.omo : null,
        omoCategoryMappings: value.categories.length ? value.categories : null,
        isEnabled: value.isEnabled,
        updatedAt,
      });
      await goto('/groups');
    } catch {
      /* global feedback */
    }
  },
}));
const values = form.useSelector((state) => state.values);
const type = $derived(values.current.type);
const native = $derived(values.current.native);
const slim = $derived(values.current.slim);
const omo = $derived(values.current.omo);
const categories = $derived(values.current.categories);
let initialized = $state('');
let seenReset = $state(-1);
let tab = $state<MappingKind>(MAPPING_KINDS[0]);
let pending = $state<GroupType | null>(null);
const builtinSystem = $derived(tab === GROUP_TYPE_SLIM || tab === GROUP_TYPE_OMO ? tab : null);
const mappingOptions = $derived(
  tab === GROUP_TYPE_NATIVE
    ? config.agents.map((agent) => ({ value: agent.id, hint: agent.description ?? agent.mode }))
    : builtinSystem
      ? BUILTIN_AGENTS.filter((agent) => agent.system === builtinSystem).map((agent) => ({
          value: agent.id,
          hint: agent.description,
        }))
      : BUILTIN_CATEGORIES.map((category) => ({ value: category })),
);
const mappingPlaceholder = $derived(
  tab === 'category' ? i18n.t('groupForm.selectCategory') : i18n.t('groupForm.selectAgent'),
);
// Model refs come from the CLI catalog plus already-bound refs missing from a stale catalog.
const modelOptions = $derived.by(() => {
  const options: { value: string; label?: string }[] = config.catalogModels().map((entry) => ({ value: entry.ref }));
  const known = new Set(options.map((option) => option.value));
  for (const entry of [...native, ...slim, ...omo, ...categories]) {
    if (entry.modelRef && !known.has(entry.modelRef)) {
      known.add(entry.modelRef);
      options.push({ value: entry.modelRef, label: i18n.t('agents.optionUnavailable', { name: entry.modelRef }) });
    }
  }
  return options;
});

// Variant choices come from the selected model's catalog entry; keep an out-of-catalog value selectable-but-flagged.
function variantOptions(ref: string, current?: string): { value: string; label: string; disabled?: boolean }[] {
  const names = buildModelVariants(config.catalogModels(), config.models(), ref);
  const options: { value: string; label: string; disabled?: boolean }[] = [
    { value: '', label: i18n.t('agents.optionNone') },
    ...names.map((name) => ({ value: name, label: name })),
  ];
  if (current && !names.includes(current)) {
    options.push({ value: current, label: i18n.t('agents.optionUnavailable', { name: current }), disabled: true });
  }
  return options;
}

function defaultTab(value: GroupType): MappingKind {
  if (value === GROUP_TYPE_SLIM) return GROUP_TYPE_SLIM;
  if (value === GROUP_TYPE_OMO) return GROUP_TYPE_OMO;
  return GROUP_TYPE_NATIVE;
}
function loadGroup() {
  if (!group) return;
  initialized = group.id;
  form.reset(defaultValues);
  tab = defaultTab(group.type);
  pending = null;
}
$effect(() => {
  if (!group) return;
  if (initialized !== group.id) {
    seenReset = config.formResetVersion;
    loadGroup();
  } else if (seenReset !== config.formResetVersion) {
    seenReset = config.formResetVersion;
    loadGroup();
  }
});
function changeType(next: GroupType) {
  const has =
    (type === GROUP_TYPE_NATIVE && native.length > 0) ||
    (type === GROUP_TYPE_SLIM && slim.length > 0) ||
    (type === GROUP_TYPE_OMO && omo.length + categories.length > 0);
  if (next !== type && has) {
    pending = next;
    return;
  }
  form.setFieldValue('type', next);
  tab = defaultTab(next);
}
function resolve(choice: 'confirm' | 'cancel') {
  if (choice === 'confirm' && pending) {
    form.setFieldValue('type', pending);
    tab = defaultTab(pending);
  }
  pending = null;
}
function add(kind: MappingKind) {
  const model = config.catalogModels()[0]?.ref;
  if (!model) {
    toast({
      variant: 'error',
      description: i18n.t('groupForm.noModelCatalog'),
    });
    return;
  }
  if (kind === GROUP_TYPE_NATIVE) form.setFieldValue('native', [...native, { agentName: '', modelRef: model }]);
  if (kind === GROUP_TYPE_SLIM) form.setFieldValue('slim', [...slim, { agentName: '', modelRef: model }]);
  if (kind === GROUP_TYPE_OMO) form.setFieldValue('omo', [...omo, { agentName: '', modelRef: model }]);
  if (kind === 'category') form.setFieldValue('categories', [...categories, { categoryName: '', modelRef: model }]);
}
function update(kind: MappingKind, index: number, field: string, value: string) {
  const list =
    kind === GROUP_TYPE_NATIVE ? native : kind === GROUP_TYPE_SLIM ? slim : kind === GROUP_TYPE_OMO ? omo : categories;
  const next = list.map((entry, i) => {
    if (i !== index) return entry;
    if (
      field === 'modelRef' &&
      entry.variant &&
      !buildModelVariants(config.catalogModels(), config.models(), value).includes(entry.variant)
    ) {
      return { ...entry, modelRef: value, variant: '' };
    }
    return { ...entry, [field]: value };
  });
  if (kind === GROUP_TYPE_NATIVE) form.setFieldValue('native', next as AgentModelBinding[]);
  if (kind === GROUP_TYPE_SLIM) form.setFieldValue('slim', next as AgentModelBinding[]);
  if (kind === GROUP_TYPE_OMO) form.setFieldValue('omo', next as AgentModelBinding[]);
  if (kind === 'category') form.setFieldValue('categories', next as CategoryMapping[]);
}
function remove(kind: MappingKind, index: number) {
  if (kind === GROUP_TYPE_NATIVE)
    form.setFieldValue(
      'native',
      native.filter((_, i) => i !== index),
    );
  if (kind === GROUP_TYPE_SLIM)
    form.setFieldValue(
      'slim',
      slim.filter((_, i) => i !== index),
    );
  if (kind === GROUP_TYPE_OMO)
    form.setFieldValue(
      'omo',
      omo.filter((_, i) => i !== index),
    );
  if (kind === 'category')
    form.setFieldValue(
      'categories',
      categories.filter((_, i) => i !== index),
    );
}
</script>

<form
  class="panel form-panel"
  novalidate
  onsubmit={(event) => {
    event.preventDefault();
    form.handleSubmit();
  }}
>
  <div class="form-grid">
    <div class="field">
      <form.Field name="name">
        {#snippet children(field)}
          <label for="group-name"
            >{i18n.t('groupForm.groupName')}<span class="field-required" aria-hidden="true">*</span></label
          ><input
            id="group-name"
            name={field.name}
            value={field.state.value}
            required
            aria-invalid={field.state.meta.errors.length ? 'true' : undefined}
            aria-describedby={field.state.meta.errors.length ? 'group-name-error' : undefined}
            oninput={(event) => field.handleChange(event.currentTarget.value)}
            onblur={field.handleBlur}
          >
          {#if field.state.meta.errors.length}
            <p id="group-name-error" class="field-error" role="alert">{field.state.meta.errors[0]?.message}</p>
          {/if}
        {/snippet}
      </form.Field>
    </div>
    <div class="field">
      <form.Field name="description">
        {#snippet children(field)}
          <label for="group-description">{i18n.t('groupForm.description')}</label>
          <input
            id="group-description"
            name={field.name}
            value={field.state.value}
            oninput={(event) => field.handleChange(event.currentTarget.value)}
            onblur={field.handleBlur}
          >
        {/snippet}
      </form.Field>
    </div>
    <form.Field name="isEnabled">
      {#snippet children(field)}
        <label class="check-row full"
          ><input
            type="checkbox"
            name={field.name}
            checked={field.state.value}
            onchange={(event) => field.handleChange(event.currentTarget.checked)}
            onblur={field.handleBlur}
          > {i18n.t('groupForm.enable')}</label
        >
      {/snippet}
    </form.Field>
    <fieldset class="group-fieldset full">
      <legend>{i18n.t('groupForm.groupType')}</legend>
      <div class="architecture-grid" role="radiogroup" aria-label={i18n.t('groupForm.groupType')}>
        <form.Field name="type">
          {#snippet children(field)}
            {#each GROUP_TYPE_OPTIONS as option}
              <label class:active={field.state.value === option.value} class="architecture-card"
                ><input
                  type="radio"
                  name="group-type"
                  value={option.value}
                  checked={field.state.value === option.value}
                  onchange={(event) => {
                    changeType(option.value);
                    // Native radios toggle before the type switch is confirmed; re-assert the
                    // controlled state so the checked input always matches `type`.
                    for (const input of event.currentTarget.closest('.architecture-grid')?.querySelectorAll('input') ??
                      []) {
                      input.checked = input.value === form.state.values.type;
                    }
                  }}
                ><span>{i18n.t(option.label)}</span></label
              >
            {/each}
          {/snippet}
        </form.Field>
      </div>
    </fieldset>
    {#if config.providers.length === 0}
      <div class="state-banner error full" role="alert">
        {i18n.t('groupForm.noModels')}
      </div>
    {:else}
      <div class="field full">
        <div class="tabs" role="tablist" aria-label={i18n.t('groupForm.mappings')}>
          {#if type === GROUP_TYPE_NATIVE}
            <button
              type="button"
              role="tab"
              aria-selected={tab === GROUP_TYPE_NATIVE}
              onclick={() => (tab = GROUP_TYPE_NATIVE)}
            >
              {i18n.t('groupForm.tabNative')}
            </button>
          {/if}
          {#if type === GROUP_TYPE_SLIM}
            <button
              type="button"
              role="tab"
              aria-selected={tab === GROUP_TYPE_SLIM}
              onclick={() => (tab = GROUP_TYPE_SLIM)}
            >
              {i18n.t('groupForm.tabSlim')}
            </button>
          {/if}
          {#if type === GROUP_TYPE_OMO}
            <button
              type="button"
              role="tab"
              aria-selected={tab === GROUP_TYPE_OMO}
              onclick={() => (tab = GROUP_TYPE_OMO)}
            >
              {i18n.t('groupForm.tabOmo')}
            </button><button
              type="button"
              role="tab"
              aria-selected={tab === 'category'}
              onclick={() => (tab = 'category')}
            >
              {i18n.t('groupForm.tabCategories')}
            </button>
          {/if}
        </div>
        {#each tab === GROUP_TYPE_NATIVE ? native : tab === GROUP_TYPE_SLIM ? slim : tab === GROUP_TYPE_OMO ? omo : categories as entry, index (`${tab}:${index}`)}
          <div class="mapping-row">
            <Combobox
              aria-label={i18n.t('groupForm.mappingName')}
              value={'agentName' in entry ? entry.agentName : entry.categoryName}
              options={mappingOptions}
              placeholder={mappingPlaceholder}
              onchange={(value) => update(tab, index, 'agentName' in entry ? 'agentName' : 'categoryName', value)}
            /><Select
              ariaLabel={i18n.t('groupForm.modelRef')}
              value={entry.modelRef}
              onchange={(value) => update(tab, index, 'modelRef', value)}
              options={modelOptions}
            /><Select
              ariaLabel={i18n.t('groupForm.mappingVariant')}
              value={entry.variant ?? ''}
              disabled={!entry.modelRef || config.catalogLoading}
              options={variantOptions(entry.modelRef, entry.variant)}
              onchange={(value) => update(tab, index, 'variant', value)}
            /><Button
              type="button"
              size="icon-sm"
              variant="ghost"
              aria-label={i18n.t('groupForm.removeMapping')}
              onclick={() => remove(tab, index)}
              ><Trash2 size={14} /></Button
            >
          </div>
        {/each}
        <Button type="button" size="sm" variant="outline" onclick={() => add(tab)}
          >{i18n.t('groupForm.addMapping')}</Button
        >
      </div>
    {/if}
  </div>
  <FormActions
    ><Button href="/groups" variant="outline">{i18n.t('common.cancel')}</Button
    ><Button type="submit" disabled={config.saving}>{i18n.t('groupForm.save')}</Button>
  </FormActions>
</form>
<Dialog.Root
  open={pending !== null}
  onOpenChange={(open) => {
    if (!open) resolve('cancel');
  }}
>
  <Dialog.Content class="max-h-[calc(100vh-64px)] max-w-[560px] overflow-y-auto">
    <Dialog.Header>
      <Dialog.Title>{i18n.t('groupForm.switchTitle')}</Dialog.Title>
      <Dialog.Description
        >{i18n.t('groupForm.switchDescription', {
          type: pending ? i18n.t(GROUP_TYPE_LABELS[pending]) : '',
        })}</Dialog.Description
      >
    </Dialog.Header>
    <Dialog.Footer>
      <Button variant="outline" onclick={() => resolve('cancel')}>{i18n.t('common.cancel')}</Button
      ><Button onclick={() => resolve('confirm')}>{i18n.t('common.confirm')}</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
