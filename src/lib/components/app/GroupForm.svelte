<script lang="ts">
  import { goto } from '$app/navigation';
  import { Trash2 } from '@lucide/svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import FormActions from './FormActions.svelte';
  import Select from './Select.svelte';
  import Combobox from './Combobox.svelte';
  import { getConfig } from '$lib/features/config/context.js';
  import { BUILTIN_AGENTS } from '$lib/features/config/builtinAgents.js';
  import { BUILTIN_CATEGORIES } from '$lib/features/config/builtinCategories.js';
  import {
    BUILTIN_SYSTEMS,
    GROUP_TYPE_OMO,
    GROUP_TYPE_OPENCODE,
    GROUP_TYPE_OPTIONS,
    GROUP_TYPE_SLIM,
    MAPPING_KINDS,
    type MappingKind,
  } from '$lib/features/config/constants.js';
  import type { AgentModelBinding, CategoryMapping, Group, GroupType } from '$lib/features/config/types.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import { toast } from './toast.svelte.js';
  const config = getConfig();
  let { group }: { group?: Group } = $props();
  let name = $state('');
  let description = $state('');
  let type = $state<GroupType>(GROUP_TYPE_OPENCODE);
  let native = $state<AgentModelBinding[]>([]);
  let slim = $state<AgentModelBinding[]>([]);
  let omo = $state<AgentModelBinding[]>([]);
  let categories = $state<CategoryMapping[]>([]);
  let isEnabled = $state(false);
  let initialized = $state('');
  let seenReset = $state(-1);
  let tab = $state<MappingKind>(MAPPING_KINDS[0]);
  let pending = $state<GroupType | null>(null);
  const builtinSystem = $derived(tab === GROUP_TYPE_SLIM ? GROUP_TYPE_SLIM : tab === 'omo' ? GROUP_TYPE_OMO : null);
  const mappingOptions = $derived(
    tab === 'native'
      ? config.agents.map((agent) => ({ value: agent.id, hint: agent.description ?? agent.mode }))
      : builtinSystem && BUILTIN_SYSTEMS.includes(builtinSystem)
        ? BUILTIN_AGENTS.filter((agent) => agent.system === builtinSystem).map((agent) => ({
            value: agent.id,
            hint: agent.description,
          }))
        : BUILTIN_CATEGORIES.map((category) => ({ value: category })),
  );
  const mappingPlaceholder = $derived(tab === 'category' ? 'Select category' : 'Select agent');
  // Model refs come from the CLI catalog plus already-bound refs missing from a stale catalog.
  const modelOptions = $derived.by(() => {
    const options: { value: string; label?: string }[] = config.catalogModels().map((entry) => ({ value: entry.ref }));
    const known = new Set(options.map((option) => option.value));
    for (const entry of [...native, ...slim, ...omo, ...categories]) {
      if (entry.modelRef && !known.has(entry.modelRef)) {
        known.add(entry.modelRef);
        options.push({ value: entry.modelRef, label: `${entry.modelRef} (unavailable)` });
      }
    }
    return options;
  });
  function defaultTab(value: GroupType): MappingKind {
    if (value === GROUP_TYPE_SLIM) return GROUP_TYPE_SLIM;
    if (value === GROUP_TYPE_OMO) return 'omo';
    return 'native';
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
      (type === GROUP_TYPE_SLIM && slim.length > 0) || (type === GROUP_TYPE_OMO && omo.length + categories.length > 0);
    if (next !== type && has) {
      pending = next;
      return;
    }
    type = next;
    tab = defaultTab(next);
  }
  function resolve(choice: 'keep' | 'clear' | 'cancel') {
    if (!pending || choice === 'cancel') {
      pending = null;
      return;
    }
    if (choice === 'clear') {
      if (type === GROUP_TYPE_SLIM) slim = [];
      if (type === GROUP_TYPE_OMO) {
        omo = [];
        categories = [];
      }
    }
    const next = pending;
    type = next;
    pending = null;
    tab = defaultTab(next);
  }
  function add(kind: MappingKind) {
    const model = config.catalogModels()[0]?.ref;
    if (!model) {
      toast({
        variant: 'error',
        description: 'No model is available in the catalog. Add a provider model before mapping.',
      });
      return;
    }
    if (kind === 'native') native = [...native, { agentName: '', modelRef: model }];
    if (kind === GROUP_TYPE_SLIM) slim = [...slim, { agentName: '', modelRef: model }];
    if (kind === 'omo') omo = [...omo, { agentName: '', modelRef: model }];
    if (kind === 'category') categories = [...categories, { categoryName: '', modelRef: model }];
  }
  function update(kind: MappingKind, index: number, field: string, value: string) {
    const list = kind === 'native' ? native : kind === GROUP_TYPE_SLIM ? slim : kind === 'omo' ? omo : categories;
    const next = list.map((entry, i) => (i === index ? { ...entry, [field]: value } : entry));
    if (kind === 'native') native = next as AgentModelBinding[];
    if (kind === GROUP_TYPE_SLIM) slim = next as AgentModelBinding[];
    if (kind === 'omo') omo = next as AgentModelBinding[];
    if (kind === 'category') categories = next as CategoryMapping[];
  }
  function remove(kind: MappingKind, index: number) {
    if (kind === 'native') native = native.filter((_, i) => i !== index);
    if (kind === GROUP_TYPE_SLIM) slim = slim.filter((_, i) => i !== index);
    if (kind === 'omo') omo = omo.filter((_, i) => i !== index);
    if (kind === 'category') categories = categories.filter((_, i) => i !== index);
  }
  async function submit() {
    let id = group?.id;
    const updatedAt = group?.updatedAt ?? new Date().toISOString();
    if (!id) {
      if (!globalThis.crypto?.randomUUID) {
        toast({
          variant: 'error',
          description: 'Cannot generate a group ID in this environment. Refresh and try again.',
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
      <label for="group-name">Group name</label><input id="group-name" bind:value={name} required />
    </div>
    <div class="field">
      <label for="group-description">Description</label><input id="group-description" bind:value={description} />
    </div>
    <label class="check-row full"><input type="checkbox" bind:checked={isEnabled} /> Enable this group</label>
    <fieldset class="group-fieldset full">
      <legend>Group type</legend>
      <div class="architecture-grid" role="radiogroup" aria-label="Group type">
        {#each GROUP_TYPE_OPTIONS as option}<label class:active={type === option.value} class="architecture-card"
            ><input
              type="radio"
              name="group-type"
              value={option.value}
              checked={type === option.value}
              onchange={() => changeType(option.value)}
            /><span>{option.label}</span></label
          >{/each}
      </div>
    </fieldset>
    {#if config.providers.length === 0}<div class="state-banner error full" role="alert">
        No real models available; mappings cannot be added.
      </div>{:else}<div class="field full">
        <div class="tabs" role="tablist" aria-label="Group mappings">
          <button type="button" role="tab" aria-selected={tab === 'native'} onclick={() => (tab = 'native')}
            >OpenCode Agents</button
          >{#if type === GROUP_TYPE_SLIM}<button
              type="button"
              role="tab"
              aria-selected={tab === GROUP_TYPE_SLIM}
              onclick={() => (tab = GROUP_TYPE_SLIM)}>Slim preset</button
            >{/if}{#if type === GROUP_TYPE_OMO}<button
              type="button"
              role="tab"
              aria-selected={tab === 'omo'}
              onclick={() => (tab = 'omo')}>OMO Agents</button
            ><button type="button" role="tab" aria-selected={tab === 'category'} onclick={() => (tab = 'category')}
              >OMO Categories</button
            >{/if}
        </div>
        {#each tab === 'native' ? native : tab === GROUP_TYPE_SLIM ? slim : tab === 'omo' ? omo : categories as entry, index (tab + ':' + index)}<div
            class="mapping-row"
          >
            <Combobox
              aria-label="Mapping name"
              value={'agentName' in entry ? entry.agentName : entry.categoryName}
              options={mappingOptions}
              placeholder={mappingPlaceholder}
              onchange={(value) => update(tab, index, 'agentName' in entry ? 'agentName' : 'categoryName', value)}
            /><Select
              ariaLabel="Model reference"
              value={entry.modelRef}
              onchange={(value) => update(tab, index, 'modelRef', value)}
              options={modelOptions}
            /><input
              aria-label="Mapping variant"
              value={entry.variant ?? ''}
              placeholder="Variant"
              oninput={(event) => update(tab, index, 'variant', event.currentTarget.value)}
            /><Button
              type="button"
              size="icon-sm"
              variant="ghost"
              aria-label="Remove mapping"
              onclick={() => remove(tab, index)}><Trash2 size={14} /></Button
            >
          </div>{/each}<Button type="button" size="sm" variant="outline" onclick={() => add(tab)}>Add mapping</Button>
      </div>{/if}
  </div>
  <FormActions
    ><Button href="/groups" variant="outline">Cancel</Button><Button type="submit" disabled={config.saving}
      >Save group</Button
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
      <Dialog.Title>Switch group type</Dialog.Title>
      <Dialog.Description>Switching to <strong>{pending}</strong> affects dedicated mappings.</Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer>
      <Button variant="outline" onclick={() => resolve('keep')}>Keep draft</Button><Button
        variant="destructive"
        onclick={() => resolve('clear')}>Clear incompatible mappings</Button
      ><Button variant="ghost" onclick={() => resolve('cancel')}>Cancel switch</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
