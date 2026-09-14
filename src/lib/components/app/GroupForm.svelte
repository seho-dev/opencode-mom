<script lang="ts">
  import { goto } from '$app/navigation';
  import { Trash2 } from '@lucide/svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import FormActions from './FormActions.svelte';
  import Select from './Select.svelte';
  import Combobox from './Combobox.svelte';
  import { getConfig } from '$lib/features/config/context.js';
  import { getI18n } from '$lib/features/i18n/context.js';
  import { BUILTIN_AGENTS } from '$lib/features/config/builtinAgents.js';
  import { BUILTIN_CATEGORIES } from '$lib/features/config/builtinCategories.js';
  import { buildModelVariants } from '$lib/features/config/agentForm.js';
  import {
    GROUP_TYPE_LABELS,
    GROUP_TYPE_NATIVE,
    GROUP_TYPE_OMO,
    GROUP_TYPE_OPTIONS,
    GROUP_TYPE_SLIM,
    MAPPING_KINDS,
    type MappingKind,
  } from '$lib/features/config/constants.js';
  import type { AgentModelBinding, CategoryMapping, Group, GroupType } from '$lib/features/config/types.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import { toast } from './toast.svelte.js';
  const config = getConfig();
  const i18n = getI18n();
  let { group }: { group?: Group } = $props();
  let name = $state('');
  let nameError = $state('');
  let description = $state('');
  let type = $state<GroupType>(GROUP_TYPE_NATIVE);
  let native = $state<AgentModelBinding[]>([]);
  let slim = $state<AgentModelBinding[]>([]);
  let omo = $state<AgentModelBinding[]>([]);
  let categories = $state<CategoryMapping[]>([]);
  let isEnabled = $state(false);
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

  function selectModel(index: number, value: string) {
    update(tab, index, 'modelRef', value);
    const current = listFor(tab)[index];
    if (
      current?.variant &&
      !buildModelVariants(config.catalogModels(), config.models(), value).includes(current.variant)
    ) {
      update(tab, index, 'variant', '');
    }
  }

  function listFor(kind: MappingKind): (AgentModelBinding | CategoryMapping)[] {
    return { [GROUP_TYPE_NATIVE]: native, [GROUP_TYPE_SLIM]: slim, [GROUP_TYPE_OMO]: omo, category: categories }[kind];
  }

  function defaultTab(value: GroupType): MappingKind {
    if (value === GROUP_TYPE_SLIM) return GROUP_TYPE_SLIM;
    if (value === GROUP_TYPE_OMO) return GROUP_TYPE_OMO;
    return GROUP_TYPE_NATIVE;
  }
  function loadGroup() {
    if (!group) return;
    initialized = group.id;
    name = group.name;
    description = group.description;
    type = group.type;
    tab = defaultTab(group.type);
    native = group.openCodeAgentOverrides;
    slim = group.slimAgentOverrides ?? [];
    omo = group.omoAgentOverrides ?? [];
    categories = group.omoCategoryMappings ?? [];
    isEnabled = group.isEnabled;
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
    type = next;
    tab = defaultTab(next);
  }
  function resolve(choice: 'confirm' | 'cancel') {
    if (choice === 'confirm' && pending) {
      type = pending;
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
    if (kind === GROUP_TYPE_NATIVE) native = [...native, { agentName: '', modelRef: model }];
    if (kind === GROUP_TYPE_SLIM) slim = [...slim, { agentName: '', modelRef: model }];
    if (kind === GROUP_TYPE_OMO) omo = [...omo, { agentName: '', modelRef: model }];
    if (kind === 'category') categories = [...categories, { categoryName: '', modelRef: model }];
  }
  function update(kind: MappingKind, index: number, field: string, value: string) {
    const list =
      kind === GROUP_TYPE_NATIVE
        ? native
        : kind === GROUP_TYPE_SLIM
          ? slim
          : kind === GROUP_TYPE_OMO
            ? omo
            : categories;
    const next = list.map((entry, i) => (i === index ? { ...entry, [field]: value } : entry));
    if (kind === GROUP_TYPE_NATIVE) native = next as AgentModelBinding[];
    if (kind === GROUP_TYPE_SLIM) slim = next as AgentModelBinding[];
    if (kind === GROUP_TYPE_OMO) omo = next as AgentModelBinding[];
    if (kind === 'category') categories = next as CategoryMapping[];
  }
  function remove(kind: MappingKind, index: number) {
    if (kind === GROUP_TYPE_NATIVE) native = native.filter((_, i) => i !== index);
    if (kind === GROUP_TYPE_SLIM) slim = slim.filter((_, i) => i !== index);
    if (kind === GROUP_TYPE_OMO) omo = omo.filter((_, i) => i !== index);
    if (kind === 'category') categories = categories.filter((_, i) => i !== index);
  }
  async function submit() {
    if (!name.trim()) {
      nameError = i18n.t('groupForm.nameRequired');
      return;
    }
    let id = group?.id;
    const updatedAt = group?.updatedAt ?? new Date().toISOString();
    if (!id) {
      if (!globalThis.crypto?.randomUUID) {
        toast({
          variant: 'error',
          description: i18n.t('groupForm.noRandomUuid'),
        });
        return;
      }
      id = globalThis.crypto.randomUUID();
    }
    try {
      const value = {
        id,
        name: name.trim(),
        description,
        type,
        openCodeAgentOverrides: native,
        slimAgentOverrides: slim.length ? slim : null,
        omoAgentOverrides: omo.length ? omo : null,
        omoCategoryMappings: categories.length ? categories : null,
        isEnabled,
        updatedAt,
      };
      await config.saveGroup(value as Group);
      await goto('/groups');
    } catch {
      /* global feedback */
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
    <div class="field">
      <label for="group-name"
        >{i18n.t('groupForm.groupName')}<span class="field-required" aria-hidden="true">*</span></label
      ><input
        id="group-name"
        bind:value={name}
        required
        aria-invalid={nameError ? 'true' : undefined}
        aria-describedby={nameError ? 'group-name-error' : undefined}
        oninput={() => {
          if (name.trim()) nameError = '';
        }}
      />
      {#if nameError}<p id="group-name-error" class="field-error" role="alert">{nameError}</p>{/if}
    </div>
    <div class="field">
      <label for="group-description">{i18n.t('groupForm.description')}</label><input
        id="group-description"
        bind:value={description}
      />
    </div>
    <label class="check-row full"><input type="checkbox" bind:checked={isEnabled} /> {i18n.t('groupForm.enable')}</label
    >
    <fieldset class="group-fieldset full">
      <legend>{i18n.t('groupForm.groupType')}</legend>
      <div class="architecture-grid" role="radiogroup" aria-label={i18n.t('groupForm.groupType')}>
        {#each GROUP_TYPE_OPTIONS as option}<label class:active={type === option.value} class="architecture-card"
            ><input
              type="radio"
              name="group-type"
              value={option.value}
              checked={type === option.value}
              onchange={(event) => {
                changeType(option.value);
                // Native radios toggle before the type switch is confirmed; re-assert the
                // controlled state so the checked input always matches `type`.
                for (const input of event.currentTarget.closest('.architecture-grid')?.querySelectorAll('input') ??
                  []) {
                  input.checked = input.value === type;
                }
              }}
            /><span>{i18n.t(option.label)}</span></label
          >{/each}
      </div>
    </fieldset>
    {#if config.providers.length === 0}<div class="state-banner error full" role="alert">
        {i18n.t('groupForm.noModels')}
      </div>{:else}<div class="field full">
        <div class="tabs" role="tablist" aria-label={i18n.t('groupForm.mappings')}>
          {#if type === GROUP_TYPE_NATIVE}<button
              type="button"
              role="tab"
              aria-selected={tab === GROUP_TYPE_NATIVE}
              onclick={() => (tab = GROUP_TYPE_NATIVE)}>{i18n.t('groupForm.tabNative')}</button
            >{/if}{#if type === GROUP_TYPE_SLIM}<button
              type="button"
              role="tab"
              aria-selected={tab === GROUP_TYPE_SLIM}
              onclick={() => (tab = GROUP_TYPE_SLIM)}>{i18n.t('groupForm.tabSlim')}</button
            >{/if}{#if type === GROUP_TYPE_OMO}<button
              type="button"
              role="tab"
              aria-selected={tab === GROUP_TYPE_OMO}
              onclick={() => (tab = GROUP_TYPE_OMO)}>{i18n.t('groupForm.tabOmo')}</button
            ><button type="button" role="tab" aria-selected={tab === 'category'} onclick={() => (tab = 'category')}
              >{i18n.t('groupForm.tabCategories')}</button
            >{/if}
        </div>
        {#each tab === GROUP_TYPE_NATIVE ? native : tab === GROUP_TYPE_SLIM ? slim : tab === GROUP_TYPE_OMO ? omo : categories as entry, index (tab + ':' + index)}<div
            class="mapping-row"
          >
            <Combobox
              aria-label={i18n.t('groupForm.mappingName')}
              value={'agentName' in entry ? entry.agentName : entry.categoryName}
              options={mappingOptions}
              placeholder={mappingPlaceholder}
              onchange={(value) => update(tab, index, 'agentName' in entry ? 'agentName' : 'categoryName', value)}
            /><Select
              ariaLabel={i18n.t('groupForm.modelRef')}
              value={entry.modelRef}
              onchange={(value) => selectModel(index, value)}
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
              onclick={() => remove(tab, index)}><Trash2 size={14} /></Button
            >
          </div>{/each}<Button type="button" size="sm" variant="outline" onclick={() => add(tab)}
          >{i18n.t('groupForm.addMapping')}</Button
        >
      </div>{/if}
  </div>
  <FormActions
    ><Button href="/groups" variant="outline">{i18n.t('common.cancel')}</Button><Button
      type="submit"
      disabled={config.saving}>{i18n.t('groupForm.save')}</Button
    >
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
      <Button variant="outline" onclick={() => resolve('cancel')}>{i18n.t('common.cancel')}</Button><Button
        onclick={() => resolve('confirm')}>{i18n.t('common.confirm')}</Button
      >
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
