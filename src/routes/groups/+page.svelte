<script lang="ts">
  import { Pencil, Play, Plus, Trash2 } from '@lucide/svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import DataTable from '$lib/components/app/DataTable.svelte';
  import EmptyTableRow from '$lib/components/app/EmptyTableRow.svelte';
  import PageHead from '$lib/components/app/PageHead.svelte';
  import { getConfig } from '$lib/features/config/context.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  const config = getConfig();
  let query = $state('');
  let deleting = $state<string | null>(null);
  const pageSize = 5;
  let page = $state(1);
  const groups = $derived(config.groups.filter((group) => group.name.toLowerCase().includes(query.toLowerCase())));
  const maxPage = $derived(Math.max(1, Math.ceil(groups.length / pageSize)));
  const currentPage = $derived(Math.min(page, maxPage));
  const pagedGroups = $derived(groups.slice((currentPage - 1) * pageSize, currentPage * pageSize));
  $effect(() => {
    // Reset to the first page whenever the filtered row count changes (search or delete).
    void groups.length;
    page = 1;
  });
  async function remove() {
    if (!deleting) return;
    try {
      await config.deleteGroup(deleting);
      deleting = null;
    } catch {
      /* global feedback */
    }
  }
  async function activate(id: string) {
    try {
      await config.switchGroup(id);
    } catch {
      /* global feedback */
    }
  }
</script>

<svelte:head><title>Groups · opencode-mom</title></svelte:head>
<PageHead eyebrow="CONTROL PLANE / GROUPS" title="Groups"
  >{#snippet children()}<Button href="/groups/new"><Plus size={14} /> New group</Button>{/snippet}</PageHead
>
<div class="search-toolbar"><input aria-label="Search groups" bind:value={query} placeholder="Search groups" /></div>
<DataTable label="Group Directory" total={groups.length} bind:page {pageSize}
  ><thead><tr><th>Name</th><th>Type</th><th>Mappings</th><th>Status</th><th class="th-actions">Actions</th></tr></thead
  ><tbody
    >{#if config.loading}<tr><td colspan="5" class="empty-table-row">Loading configuration...</td></tr
      >{:else if !groups.length}<EmptyTableRow colspan={5} message="No groups." />{:else}{#each pagedGroups as group}<tr
          ><td class="model-name"
            >{group.name}
            <div class="muted">{group.description}</div></td
          ><td>{group.type}</td><td
            >{group.openCodeAgentOverrides.length +
              (group.slimAgentOverrides?.length ?? 0) +
              (group.omoAgentOverrides?.length ?? 0) +
              (group.omoCategoryMappings?.length ?? 0)}</td
          ><td>{group.isEnabled ? 'Enabled' : 'Disabled'}</td><td class="row-actions"
            ><Button
              variant="ghost"
              size="icon-sm"
              aria-label={`Switch to ${group.name}`}
              title="Switch group"
              onclick={() => activate(group.id)}
              disabled={config.switching}><Play size={14} /></Button
            ><Button href={`/groups/${group.id}/edit`} variant="ghost" size="icon-sm" aria-label={`Edit ${group.name}`}
              ><Pencil size={14} /></Button
            ><Button
              variant="ghost"
              size="icon-sm"
              aria-label={`Delete ${group.name}`}
              onclick={() => (deleting = group.id)}><Trash2 size={14} /></Button
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
      <Dialog.Title>Delete group</Dialog.Title>
      <Dialog.Description>Delete group <strong>{deleting}</strong>? This cannot be undone.</Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer>
      <Button variant="outline" onclick={() => (deleting = null)}>Cancel</Button>
      <Button variant="destructive" onclick={remove}>Delete</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
