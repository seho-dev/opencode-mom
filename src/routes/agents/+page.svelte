<script lang="ts">
  import { Pencil, Plus, Trash2 } from '@lucide/svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import DataTable from '$lib/components/app/DataTable.svelte';
  import EmptyTableRow from '$lib/components/app/EmptyTableRow.svelte';
  import PageHead from '$lib/components/app/PageHead.svelte';
  import { getConfig } from '$lib/features/config/context.js';
  import type { AgentStorage } from '$lib/features/config/types.js';
  const config = getConfig();
  let query = $state('');
  let deleting = $state<string | null>(null);
  let storage = $state<AgentStorage>('inline');
  const agents = $derived(
    config.agents.filter((agent) =>
      `${agent.id} ${agent.description ?? ''}`.toLowerCase().includes(query.toLowerCase()),
    ),
  );
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
<DataTable label="Agent directory"
  ><thead><tr><th>ID</th><th>Mode</th><th>Source</th><th>Model</th><th>Actions</th></tr></thead><tbody
    >{#if config.loading}<tr><td colspan="5" class="empty-table-row">Loading configuration...</td></tr
      >{:else if !agents.length}<EmptyTableRow colspan={5} message="No agents." />{:else}{#each agents as agent}<tr
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
>{#if deleting}<div class="confirm-strip" role="alert">
    Delete <strong>{deleting}</strong>?{#if canSelect}<select aria-label="Source to delete" bind:value={storage}
        ><option value="inline">Inline</option><option value="global_markdown">Global Markdown</option><option
          value="project_markdown">Project Markdown</option
        ></select
      ><span class="muted">Only the selected source is deleted; the other one is kept.</span>{/if}<Button
      size="sm"
      variant="outline"
      onclick={() => (deleting = null)}>Cancel</Button
    ><Button size="sm" variant="destructive" onclick={remove}>Delete</Button>
  </div>{/if}
