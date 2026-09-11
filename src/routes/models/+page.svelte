<script lang="ts">
  import { Pencil, Plus, Trash2, RefreshCw } from '@lucide/svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import DataTable from '$lib/components/app/DataTable.svelte';
  import EmptyTableRow from '$lib/components/app/EmptyTableRow.svelte';
  import PageHead from '$lib/components/app/PageHead.svelte';
  import { getConfig } from '$lib/features/config/context.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import { toast } from '$lib/components/app/toast.svelte.js';
  const config = getConfig();
  let query = $state('');
  let deleting = $state<string | null>(null);
  let activeTab = $state<'custom' | 'builtin'>('custom');
  const pageSize = 5;
  let page = $state(1);
  const errorMessage = (error: unknown) =>
    error && typeof error === 'object' && 'message' in error
      ? String((error as { message: unknown }).message)
      : String(error);

  // Surface catalog load failures as error toasts with a Retry action.
  $effect(() => {
    const catalogError = config.catalogError;
    if (!catalogError) return;
    toast({
      variant: 'error',
      description: `Failed to load models via opencode CLI: ${catalogError.message}`,
      action: { label: 'Retry', onclick: () => retryCatalog() },
    });
  });

  const filteredCatalog = $derived(
    config.catalog.filter((m) =>
      `${m.ref} ${m.name ?? ''} ${m.providerId}`.toLowerCase().includes(query.toLowerCase()),
    ),
  );
  const builtinModels = $derived(filteredCatalog.filter((m) => !m.isCustom));
  const customModels = $derived(filteredCatalog.filter((m) => m.isCustom));
  const visibleModels = $derived(activeTab === 'custom' ? customModels : builtinModels);
  const maxPage = $derived(Math.max(1, Math.ceil(visibleModels.length / pageSize)));
  const currentPage = $derived(Math.min(page, maxPage));
  const pagedModels = $derived(visibleModels.slice((currentPage - 1) * pageSize, currentPage * pageSize));
  $effect(() => {
    // Reset to the first page whenever the visible row count changes (search, tab switch, or delete).
    void visibleModels.length;
    page = 1;
  });

  async function refreshCatalog() {
    try {
      await config.loadCatalog();
    } catch {
      // Failures are surfaced as an error toast via the catalogError watcher above.
    }
  }
  async function retryCatalog() {
    try {
      await config.loadCatalog();
    } catch {
      // Failures are surfaced as an error toast via the catalogError watcher above.
    }
  }
  async function remove() {
    if (!deleting) return;
    try {
      await config.deleteModel(deleting as `${string}/${string}`);
      deleting = null;
      await config.loadCatalog();
    } catch (error) {
      toast({ variant: 'error', description: errorMessage(error) });
    }
  }
</script>

<svelte:head><title>Models · opencode-mom</title></svelte:head><PageHead eyebrow="CONFIG / MODELS" title="Models"
  >{#snippet children()}<div class="flex items-center gap-2">
      <Button variant="outline" size="sm" onclick={refreshCatalog} disabled={config.catalogLoading}
        ><RefreshCw size={14} /> Refresh</Button
      ><Button href="/models/new"><Plus size={14} /> New model</Button>
    </div>{/snippet}</PageHead
>
<div class="search-toolbar">
  <input aria-label="Search models" bind:value={query} placeholder="Search provider/model, e.g. anthropic/claude" />
</div>
<div class="flex gap-2 mb-3">
  <Button variant={activeTab === 'custom' ? 'default' : 'outline'} size="sm" onclick={() => (activeTab = 'custom')}
    >Custom ({customModels.length})</Button
  >
  <Button variant={activeTab === 'builtin' ? 'default' : 'outline'} size="sm" onclick={() => (activeTab = 'builtin')}
    >Builtin ({builtinModels.length})</Button
  >
</div>
<DataTable label="Model directory" total={visibleModels.length} bind:page {pageSize}
  ><thead><tr><th>Provider</th><th>Name</th><th>Context</th><th class="th-actions">Actions</th></tr></thead><tbody
    >{#if config.catalogLoading}<tr><td colspan="4" class="empty-table-row">Loading models from opencode CLI...</td></tr
      >{:else if !visibleModels.length}<EmptyTableRow
        colspan={4}
        message={activeTab === 'custom'
          ? 'No custom models. Create a provider and add models.'
          : 'No builtin models found.'}
      />{:else}{#each pagedModels as model}<tr
          ><td>{model.providerId}</td><td>{model.name ?? 'unnamed'}</td><td
            >{(model.limit as { context?: number })?.context ?? 'unset'}</td
          ><td class="row-actions"
            >{#if model.isCustom}<Button
                href={`/models/${encodeURIComponent(model.ref)}/edit`}
                variant="ghost"
                size="icon-sm"
                title="Edit model"
                aria-label={`Edit ${model.ref}`}><Pencil size={14} /></Button
              ><Button
                variant="ghost"
                size="icon-sm"
                title="Delete model"
                aria-label={`Delete ${model.ref}`}
                onclick={() => {
                  deleting = model.ref;
                }}><Trash2 size={14} /></Button
              >{:else}<span class="text-xs text-muted-foreground">read-only</span>{/if}</td
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
      <Dialog.Title>Delete model</Dialog.Title>
      <Dialog.Description
        >Delete <strong>{deleting}</strong>? Deletion is blocked while references exist.</Dialog.Description
      >
    </Dialog.Header>
    <Dialog.Footer>
      <Button variant="outline" onclick={() => (deleting = null)}>Cancel</Button>
      <Button variant="destructive" onclick={remove}>Delete</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
