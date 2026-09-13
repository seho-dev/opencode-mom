<script lang="ts">
  import { Pencil, Plus, Trash2 } from '@lucide/svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import DataTable from '$lib/components/app/DataTable.svelte';
  import EmptyTableRow from '$lib/components/app/EmptyTableRow.svelte';
  import PageHead from '$lib/components/app/PageHead.svelte';
  import Select from '$lib/components/app/Select.svelte';
  import { getConfig } from '$lib/features/config/context.js';
  import { BUILTIN_AGENTS } from '$lib/features/config/builtinAgents.js';
  import type { AgentStorage } from '$lib/features/config/types.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  type AgentTab = 'custom' | 'builtin';
  const tabOrder: AgentTab[] = ['custom', 'builtin'];
  const config = getConfig();
  let query = $state('');
  let deleting = $state<string | null>(null);
  let storage = $state<AgentStorage>('inline');
  let activeTab = $state<AgentTab>('custom');
  let customTabEl = $state<HTMLButtonElement | null>(null);
  let builtinTabEl = $state<HTMLButtonElement | null>(null);
  const pageSize = 5;
  let page = $state(1);
  const agents = $derived(
    config.agents.filter((agent) =>
      `${agent.id} ${agent.description ?? ''}`.toLowerCase().includes(query.toLowerCase()),
    ),
  );
  const slimAgents = $derived(BUILTIN_AGENTS.filter((agent) => agent.system === 'slim'));
  const omoAgents = $derived(BUILTIN_AGENTS.filter((agent) => agent.system === 'oh-my-openagent'));
  const maxPage = $derived(Math.max(1, Math.ceil(agents.length / pageSize)));
  const currentPage = $derived(Math.min(page, maxPage));
  const pagedAgents = $derived(agents.slice((currentPage - 1) * pageSize, currentPage * pageSize));
  $effect(() => {
    // Reset to the first page whenever the filtered row count changes (search or delete).
    void agents.length;
    page = 1;
  });
  function selectTab(tab: AgentTab) {
    activeTab = tab;
    query = '';
    page = 1;
  }
  function onTabKeys(event: KeyboardEvent) {
    const index = tabOrder.indexOf(activeTab);
    let next: AgentTab;
    if (event.key === 'ArrowRight') next = tabOrder[(index + 1) % tabOrder.length]!;
    else if (event.key === 'ArrowLeft') next = tabOrder[(index - 1 + tabOrder.length) % tabOrder.length]!;
    else if (event.key === 'Home') next = tabOrder[0]!;
    else if (event.key === 'End') next = tabOrder[tabOrder.length - 1]!;
    else return;
    event.preventDefault();
    selectTab(next);
    (next === 'custom' ? customTabEl : builtinTabEl)?.focus();
  }
  const canSelect = $derived(!!deleting && config.agents.find((agent) => agent.id === deleting)?.source === 'both');
  async function remove() {
    if (!deleting) return;
    try {
      await config.deleteAgent(deleting, canSelect ? storage : undefined);
      deleting = null;
    } catch {
      /* ConfigFeedback renders the command error. */
    }
  }
</script>

<svelte:head><title>Agents · opencode-mom</title></svelte:head><PageHead eyebrow="CONFIG / AGENTS" title="Agents"
  >{#snippet children()}{#if activeTab === 'custom'}<Button href="/agents/new"><Plus size={14} /> New agent</Button
      >{/if}{/snippet}</PageHead
>
<div class="flex gap-2 mb-3" role="tablist" aria-label="Agent source">
  <Button
    bind:ref={customTabEl}
    id="agents-tab-custom"
    role="tab"
    variant={activeTab === 'custom' ? 'default' : 'outline'}
    size="sm"
    aria-selected={activeTab === 'custom'}
    aria-controls="agents-tabpanel-custom"
    tabindex={activeTab === 'custom' ? 0 : -1}
    onclick={() => selectTab('custom')}
    onkeydown={onTabKeys}>Custom ({config.agents.length})</Button
  >
  <Button
    bind:ref={builtinTabEl}
    id="agents-tab-builtin"
    role="tab"
    variant={activeTab === 'builtin' ? 'default' : 'outline'}
    size="sm"
    aria-selected={activeTab === 'builtin'}
    aria-controls="agents-tabpanel-builtin"
    tabindex={activeTab === 'builtin' ? 0 : -1}
    onclick={() => selectTab('builtin')}
    onkeydown={onTabKeys}>Builtin ({BUILTIN_AGENTS.length})</Button
  >
</div>
{#if activeTab === 'custom'}
  <div id="agents-tabpanel-custom" role="tabpanel" aria-labelledby="agents-tab-custom" tabindex="0">
    <div class="search-toolbar">
      <input aria-label="Search agents" bind:value={query} placeholder="Search agent ID" />
    </div>
    <DataTable label="Agent directory" total={agents.length} bind:page {pageSize}
      ><thead><tr><th>ID</th><th>Mode</th><th>Source</th><th>Model</th><th class="th-actions">Actions</th></tr></thead
      ><tbody
        >{#if config.loading}<tr><td colspan="5" class="empty-table-row">Loading configuration...</td></tr
          >{:else if !agents.length}<EmptyTableRow
            colspan={5}
            message="No agents."
          />{:else}{#each pagedAgents as agent}<tr
              ><td class="model-name">{agent.id}</td><td>{agent.mode ?? 'unset'}</td><td>{agent.source}</td><td
                >{agent.modelRef ?? 'inherited'}</td
              ><td class="row-actions"
                ><Button
                  href={`/agents/${encodeURIComponent(agent.id)}/edit`}
                  variant="ghost"
                  size="icon-sm"
                  aria-label={`Edit ${agent.id}`}><Pencil size={14} /></Button
                ><Button
                  variant="ghost"
                  size="icon-sm"
                  aria-label={`Delete ${agent.id}`}
                  onclick={() => (deleting = agent.id)}><Trash2 size={14} /></Button
                ></td
              ></tr
            >{/each}{/if}</tbody
      ></DataTable
    >
  </div>
{:else}
  <div
    id="agents-tabpanel-builtin"
    role="tabpanel"
    aria-labelledby="agents-tab-builtin"
    tabindex="0"
    class="flex flex-col gap-6"
  >
    <section aria-label="Slim builtin agents">
      <p class="group-title mb-2">Slim</p>
      <DataTable label="Slim builtin agents" total={slimAgents.length} pageSize={slimAgents.length}
        ><thead><tr><th>ID</th><th>Type</th><th>Description</th></tr></thead><tbody
          >{#each slimAgents as agent}<tr
              ><td class="model-name">{agent.id}</td><td>{agent.type}</td><td>{agent.description}</td></tr
            >{/each}</tbody
        ></DataTable
      >
    </section>
    <section aria-label="oh-my-openagent builtin agents">
      <p class="group-title mb-2">oh-my-openagent</p>
      <DataTable label="oh-my-openagent builtin agents" total={omoAgents.length} pageSize={omoAgents.length}
        ><thead><tr><th>ID</th><th>Type</th><th>Description</th></tr></thead><tbody
          >{#each omoAgents as agent}<tr
              ><td class="model-name">{agent.id}</td><td>{agent.type}</td><td>{agent.description}</td></tr
            >{/each}</tbody
        ></DataTable
      >
    </section>
  </div>
{/if}
<Dialog.Root
  open={deleting !== null}
  onOpenChange={(open) => {
    if (!open) deleting = null;
  }}
>
  <Dialog.Content>
    <Dialog.Header>
      <Dialog.Title>Delete agent</Dialog.Title>
      <Dialog.Description>Delete agent <strong>{deleting}</strong>?</Dialog.Description>
    </Dialog.Header>
    {#if canSelect}<div class="dialog-field">
        <label for="source-to-delete">Source to delete</label><Select
          id="source-to-delete"
          bind:value={storage}
          options={[
            { value: 'inline', label: 'Inline' },
            { value: 'global_markdown', label: 'Global Markdown' },
            { value: 'project_markdown', label: 'Project Markdown' },
          ]}
        />
        <p class="text-xs text-muted-foreground">Only the selected source is deleted; the other one is kept.</p>
      </div>{/if}
    <Dialog.Footer>
      <Button variant="outline" onclick={() => (deleting = null)}>Cancel</Button>
      <Button variant="destructive" onclick={remove}>Delete</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
