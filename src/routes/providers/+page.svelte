<script lang="ts">
  import { Pencil, Plus, Trash2 } from '@lucide/svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import PageHead from '$lib/components/app/PageHead.svelte';
  import DataTable from '$lib/components/app/DataTable.svelte';
  import EmptyTableRow from '$lib/components/app/EmptyTableRow.svelte';
  import StatusBadge from '$lib/components/app/StatusBadge.svelte';
  import { getConfig } from '$lib/features/config/context.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';

  const config = getConfig();
  let query = $state('');
  let deleting = $state<string | null>(null);
  const pageSize = 5;
  let page = $state(1);
  const filtered = $derived(
    config.providers.filter((provider) => provider.name.toLowerCase().includes(query.toLowerCase())),
  );
  const maxPage = $derived(Math.max(1, Math.ceil(filtered.length / pageSize)));
  const currentPage = $derived(Math.min(page, maxPage));
  const pagedProviders = $derived(filtered.slice((currentPage - 1) * pageSize, currentPage * pageSize));
  $effect(() => {
    // Reset to the first page whenever the filtered row count changes (search or delete).
    void filtered.length;
    page = 1;
  });
  async function remove() {
    if (!deleting) return;
    try {
      await config.deleteProvider(deleting);
      deleting = null;
    } catch {
      /* ConfigFeedback renders the command error. */
    }
  }
</script>

<svelte:head><title>Providers · opencode-mom</title></svelte:head>
<PageHead eyebrow="CONFIG / PROVIDERS" title="Provider Registry"
  >{#snippet children()}<Button href="/providers/new"><Plus size={14} /> New provider</Button>{/snippet}</PageHead
>
<div class="search-toolbar">
  <input aria-label="Search providers" bind:value={query} placeholder="Search provider name" />
</div>
<DataTable label="Provider Registry" total={filtered.length} bind:page {pageSize}
  ><thead
    ><tr><th>Name</th><th>NPM</th><th>Base URL</th><th>Models</th><th>Status</th><th class="th-actions">Actions</th></tr
    ></thead
  ><tbody>
    {#if config.loading}<tr><td colspan="6" class="empty-table-row">Loading configuration...</td></tr>
    {:else if !filtered.length}<EmptyTableRow
        colspan={6}
        message="No providers. Models must be attached to a provider first."
      />
    {:else}{#each pagedProviders as provider}<tr
          ><td class="model-name">{provider.name}</td><td>{provider.npm ?? '—'}</td><td
            >{provider.options?.baseURL || '—'}</td
          ><td>{Object.keys(provider.models).length}</td><td><StatusBadge variant="success">Loaded</StatusBadge></td><td
            class="row-actions"
            ><Button
              href={`/providers/${provider.name}/edit`}
              variant="ghost"
              size="icon-sm"
              aria-label={`Edit ${provider.name}`}><Pencil size={14} /></Button
            ><Button
              variant="ghost"
              size="icon-sm"
              aria-label={`Delete ${provider.name}`}
              onclick={() => (deleting = provider.name)}><Trash2 size={14} /></Button
            ></td
          ></tr
        >{/each}{/if}
  </tbody></DataTable
>
<Dialog.Root
  open={deleting !== null}
  onOpenChange={(open) => {
    if (!open) deleting = null;
  }}
>
  <Dialog.Content>
    <Dialog.Header>
      <Dialog.Title>Delete provider</Dialog.Title>
      <Dialog.Description>Delete provider <strong>{deleting}</strong>? This cannot be undone.</Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer>
      <Button variant="outline" onclick={() => (deleting = null)}>Cancel</Button>
      <Button variant="destructive" onclick={remove}>Delete</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
