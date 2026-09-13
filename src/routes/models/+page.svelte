<script lang="ts">
  import { Pencil, Plus, Trash2, RefreshCw } from '@lucide/svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import DataTable from '$lib/components/app/DataTable.svelte';
  import EmptyTableRow from '$lib/components/app/EmptyTableRow.svelte';
  import PageHead from '$lib/components/app/PageHead.svelte';
  import { getConfig } from '$lib/features/config/context.js';
  import { getI18n } from '$lib/features/i18n/context.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import { toast } from '$lib/components/app/toast.svelte.js';
  const config = getConfig();
  const i18n = getI18n();
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
      description: i18n.t('models.catalogFailed', { message: catalogError.message }),
      action: { label: i18n.t('models.retry'), onclick: () => retryCatalog() },
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

<svelte:head><title>{i18n.t('models.metaTitle')}</title></svelte:head><PageHead
  eyebrow={i18n.t('models.eyebrow')}
  title={i18n.t('models.title')}
  >{#snippet children()}<div class="flex items-center gap-2">
      <Button variant="outline" size="sm" onclick={refreshCatalog} disabled={config.catalogLoading}
        ><RefreshCw size={14} /> {i18n.t('models.refresh')}</Button
      ><Button href="/models/new"><Plus size={14} /> {i18n.t('models.new')}</Button>
    </div>{/snippet}</PageHead
>
<div class="search-toolbar">
  <input
    aria-label={i18n.t('models.searchLabel')}
    bind:value={query}
    placeholder={i18n.t('models.searchPlaceholder')}
  />
</div>
<div class="flex gap-2 mb-3">
  <Button variant={activeTab === 'custom' ? 'default' : 'outline'} size="sm" onclick={() => (activeTab = 'custom')}
    >{i18n.t('models.tabCustom', { count: customModels.length })}</Button
  >
  <Button variant={activeTab === 'builtin' ? 'default' : 'outline'} size="sm" onclick={() => (activeTab = 'builtin')}
    >{i18n.t('models.tabBuiltin', { count: builtinModels.length })}</Button
  >
</div>
<DataTable label={i18n.t('models.tableLabel')} total={visibleModels.length} bind:page {pageSize}
  ><thead
    ><tr
      ><th>{i18n.t('models.colProvider')}</th><th>{i18n.t('models.colName')}</th><th>{i18n.t('models.colContext')}</th
      ><th class="th-actions">{i18n.t('models.colActions')}</th></tr
    ></thead
  ><tbody
    >{#if config.catalogLoading}<tr><td colspan="4" class="empty-table-row">{i18n.t('models.loading')}</td></tr
      >{:else if !visibleModels.length}<EmptyTableRow
        colspan={4}
        message={activeTab === 'custom' ? i18n.t('empty.modelsCustom') : i18n.t('empty.modelsBuiltin')}
      />{:else}{#each pagedModels as model}<tr
          ><td>{model.providerId}</td><td>{model.name ?? i18n.t('common.unnamed')}</td><td
            >{(model.limit as { context?: number })?.context ?? i18n.t('common.unset')}</td
          ><td class="row-actions"
            >{#if model.isCustom}<Button
                href={`/models/${encodeURIComponent(model.ref)}/edit`}
                variant="ghost"
                size="icon-sm"
                title={i18n.t('models.editTitle')}
                aria-label={i18n.t('models.editAria', { ref: model.ref })}><Pencil size={14} /></Button
              ><Button
                variant="ghost"
                size="icon-sm"
                title={i18n.t('models.deleteTitle')}
                aria-label={i18n.t('models.deleteAria', { ref: model.ref })}
                onclick={() => {
                  deleting = model.ref;
                }}><Trash2 size={14} /></Button
              >{:else}<span class="text-xs text-muted-foreground">{i18n.t('common.readOnly')}</span>{/if}</td
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
      <Dialog.Title>{i18n.t('models.deleteTitle')}</Dialog.Title>
      <Dialog.Description>{i18n.t('models.deleteConfirm', { name: deleting ?? '' })}</Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer>
      <Button variant="outline" onclick={() => (deleting = null)}>{i18n.t('common.cancel')}</Button>
      <Button variant="destructive" onclick={remove}>{i18n.t('common.delete')}</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
