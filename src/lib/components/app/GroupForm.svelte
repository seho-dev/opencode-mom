<script lang="ts">
  import { goto } from '$app/navigation';
  import { Trash2 } from '@lucide/svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import FormActions from './FormActions.svelte';
  import { getConfig } from '$lib/features/config/context.js';
  import type { AgentModelBinding, CategoryMapping, Group, GroupType } from '$lib/features/config/types.js';
  type MappingKind = 'native' | 'slim' | 'omo' | 'category';
  const config = getConfig();
  let { group }: { group?: Group } = $props();
  let name = $state('');
  let description = $state('');
  let type = $state<GroupType>('opencode');
  let native = $state<AgentModelBinding[]>([]);
  let slim = $state<AgentModelBinding[]>([]);
  let omo = $state<AgentModelBinding[]>([]);
  let categories = $state<CategoryMapping[]>([]);
  let isEnabled = $state(false);
  let initialized = $state('');
  let seenReset = $state(-1);
  let tab = $state<MappingKind>('native');
  let pending = $state<GroupType | null>(null);
  let message = $state('');
  function loadGroup() {
    if (!group) return;
    initialized = group.id;
    name = group.name;
    description = group.description;
    type = group.type;
    native = group.openCodeAgentOverrides;
    slim = group.slimAgentOverrides ?? [];
    omo = group.omoAgentOverrides ?? [];
    categories = group.omoCategoryMappings ?? [];
    isEnabled = group.isEnabled;
    pending = null;
    message = '';
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
      (type === 'slim' && slim.length > 0) || (type === 'oh-my-openagent' && omo.length + categories.length > 0);
    if (next !== type && has) {
      pending = next;
      return;
    }
    type = next;
    tab = 'native';
  }
  function resolve(choice: 'keep' | 'clear' | 'cancel') {
    if (!pending || choice === 'cancel') {
      pending = null;
      return;
    }
    if (choice === 'clear') {
      if (type === 'slim') slim = [];
      if (type === 'oh-my-openagent') {
        omo = [];
        categories = [];
      }
    }
    type = pending;
    pending = null;
    tab = 'native';
  }
  function add(kind: 'native' | 'slim' | 'omo' | 'category') {
    const model = config.models()[0]?.ref;
    if (!model) return;
    if (kind === 'native') native = [...native, { agentName: 'build', modelRef: model }];
    if (kind === 'slim') slim = [...slim, { agentName: 'orchestrator', modelRef: model }];
    if (kind === 'omo') omo = [...omo, { agentName: 'sisyphus', modelRef: model }];
    if (kind === 'category') categories = [...categories, { categoryName: 'quick', modelRef: model }];
  }
  function update(kind: MappingKind, index: number, field: string, value: string) {
    const list = kind === 'native' ? native : kind === 'slim' ? slim : kind === 'omo' ? omo : categories;
    const next = list.map((entry, i) => (i === index ? { ...entry, [field]: value } : entry));
    if (kind === 'native') native = next as AgentModelBinding[];
    if (kind === 'slim') slim = next as AgentModelBinding[];
    if (kind === 'omo') omo = next as AgentModelBinding[];
    if (kind === 'category') categories = next as CategoryMapping[];
  }
  function remove(kind: MappingKind, index: number) {
    if (kind === 'native') native = native.filter((_, i) => i !== index);
    if (kind === 'slim') slim = slim.filter((_, i) => i !== index);
    if (kind === 'omo') omo = omo.filter((_, i) => i !== index);
    if (kind === 'category') categories = categories.filter((_, i) => i !== index);
  }
  async function submit() {
    let id = group?.id;
    const updatedAt = group?.updatedAt ?? new Date().toISOString();
    if (!id) {
      if (!globalThis.crypto?.randomUUID) {
        message = 'Cannot generate a group ID in this environment. Refresh and try again.';
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
        {#each [['opencode', 'OpenCode'], ['slim', 'Slim'], ['oh-my-openagent', 'OhMyOpenAgent']] as option}<label
            class:active={type === option[0]}
            class="architecture-card"
            ><input
              type="radio"
              name="group-type"
              value={option[0]}
              checked={type === option[0]}
              onchange={() => changeType(option[0] as GroupType)}
            /><span>{option[1]}</span></label
          >{/each}
      </div>
    </fieldset>
    {#if config.providers.length === 0}<div class="state-banner error full" role="alert">
        No real models available; mappings cannot be added.
      </div>{:else}<div class="field full">
        <div class="tabs" role="tablist" aria-label="Group mappings">
          <button type="button" role="tab" aria-selected={tab === 'native'} onclick={() => (tab = 'native')}
            >OpenCode Agents</button
          >{#if type === 'slim'}<button
              type="button"
              role="tab"
              aria-selected={tab === 'slim'}
              onclick={() => (tab = 'slim')}>Slim preset</button
            >{/if}{#if type === 'oh-my-openagent'}<button
              type="button"
              role="tab"
              aria-selected={tab === 'omo'}
              onclick={() => (tab = 'omo')}>OMO Agents</button
            ><button type="button" role="tab" aria-selected={tab === 'category'} onclick={() => (tab = 'category')}
              >OMO Categories</button
            >{/if}
        </div>
        {#each tab === 'native' ? native : tab === 'slim' ? slim : tab === 'omo' ? omo : categories as entry, index}<div
            class="mapping-row"
          >
            <input
              aria-label="Mapping name"
              value={'agentName' in entry ? entry.agentName : entry.categoryName}
              oninput={(event) =>
                update(tab, index, 'agentName' in entry ? 'agentName' : 'categoryName', event.currentTarget.value)}
            /><select
              aria-label="Model reference"
              value={entry.modelRef}
              onchange={(event) => update(tab, index, 'modelRef', event.currentTarget.value)}
              >{#each config.models() as model}<option value={model.ref}>{model.ref}</option>{/each}</select
            ><input
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
    ></FormActions
  >
</form>
{#if pending}<div class="confirm-strip" role="alert">
    Switching the type affects dedicated mappings.<Button size="sm" variant="outline" onclick={() => resolve('keep')}
      >Keep draft</Button
    ><Button size="sm" variant="destructive" onclick={() => resolve('clear')}>Clear incompatible mappings</Button
    ><Button size="sm" variant="ghost" onclick={() => resolve('cancel')}>Cancel switch</Button>
  </div>{/if}{#if message}<div class="state-banner error" role="alert">{message}</div>{/if}
