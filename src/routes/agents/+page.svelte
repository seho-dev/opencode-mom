<script lang="ts">
  import { Pencil, Plus, Trash2 } from '@lucide/svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import DataTable from '$lib/components/app/DataTable.svelte';
  import EmptyTableRow from '$lib/components/app/EmptyTableRow.svelte';
  import PageHead from '$lib/components/app/PageHead.svelte';
  import { getConfig } from '$lib/features/config/context.js';
  import type { AgentStorage } from '$lib/features/config/types.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  const config = getConfig();
  let query = $state('');
  let deleting = $state<string | null>(null);
  let storage = $state<AgentStorage>('inline');
  const pageSize = 5;
  let page = $state(1);
  const agents = $derived(
    config.agents.filter((agent) =>
      `${agent.id} ${agent.description ?? ''}`.toLowerCase().includes(query.toLowerCase()),
    ),
  );
  const maxPage = $derived(Math.max(1, Math.ceil(agents.length / pageSize)));
  const currentPage = $derived(Math.min(page, maxPage));
  const pagedAgents = $derived(agents.slice((currentPage - 1) * pageSize, currentPage * pageSize));
  $effect(() => {
    // Reset to the first page whenever the filtered row count changes (search or delete).
    void agents.length;
    page = 1;
  });
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
  >{#snippet children()}<Button href="/agents/new"><Plus size={14} /> New agent</Button>{/snippet}</PageHead
>
<div class="search-toolbar"><input aria-label="Search agents" bind:value={query} placeholder="Search agent ID" /></div>
<DataTable label="Agent directory" total={agents.length} bind:page {pageSize}
  ><thead><tr><th>ID</th><th>Mode</th><th>Source</th><th>Model</th><th class="th-actions">Actions</th></tr></thead
  ><tbody
    >{#if config.loading}<tr><td colspan="5" class="empty-table-row">Loading configuration...</td></tr
      >{:else if !agents.length}<EmptyTableRow colspan={5} message="No agents." />{:else}{#each pagedAgents as agent}<tr
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
        <label for="source-to-delete">Source to delete</label><select
          id="source-to-delete"
          class="dialog-select"
          bind:value={storage}
          ><option value="inline">Inline</option><option value="global_markdown">Global Markdown</option><option
            value="project_markdown">Project Markdown</option
          ></select
        >
        <p class="text-xs text-muted-foreground">Only the selected source is deleted; the other one is kept.</p>
      </div>{/if}
    <Dialog.Footer>
      <Button variant="outline" onclick={() => (deleting = null)}>Cancel</Button>
      <Button variant="destructive" onclick={remove}>Delete</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
