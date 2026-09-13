<script lang="ts">
  import { Pencil, Plus, Trash2 } from '@lucide/svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import PageHead from '$lib/components/app/PageHead.svelte';
  import DataTable from '$lib/components/app/DataTable.svelte';
  import EmptyTableRow from '$lib/components/app/EmptyTableRow.svelte';
  import StatusBadge from '$lib/components/app/StatusBadge.svelte';
  import { getConfig } from '$lib/features/config/context.js';
  import { getI18n } from '$lib/features/i18n/context.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';

  const config = getConfig();
  const i18n = getI18n();
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

<svelte:head><title>{i18n.t('providers.metaTitle')}</title></svelte:head>
<PageHead eyebrow={i18n.t('providers.eyebrow')} title={i18n.t('providers.title')}
  >{#snippet children()}<Button href="/providers/new"><Plus size={14} /> {i18n.t('providers.new')}</Button
    >{/snippet}</PageHead
>
<div class="search-toolbar">
  <input
    aria-label={i18n.t('providers.searchLabel')}
    bind:value={query}
    placeholder={i18n.t('providers.searchPlaceholder')}
  />
</div>
<DataTable label={i18n.t('providers.tableLabel')} total={filtered.length} bind:page {pageSize}
  ><thead
    ><tr
      ><th>{i18n.t('providers.colName')}</th><th>{i18n.t('providers.colNpm')}</th><th
        >{i18n.t('providers.colBaseUrl')}</th
      ><th>{i18n.t('providers.colModels')}</th><th>{i18n.t('providers.colStatus')}</th><th class="th-actions"
        >{i18n.t('providers.colActions')}</th
      ></tr
    ></thead
  ><tbody>
    {#if config.loading}<tr><td colspan="6" class="empty-table-row">{i18n.t('providers.loading')}</td></tr>
    {:else if !filtered.length}<EmptyTableRow colspan={6} message={i18n.t('empty.providers')} />
    {:else}{#each pagedProviders as provider}<tr
          ><td class="model-name">{provider.name}</td><td>{provider.npm ?? '—'}</td><td
            >{provider.options?.baseURL || '—'}</td
          ><td>{Object.keys(provider.models).length}</td><td
            ><StatusBadge variant="success">{i18n.t('providers.loaded')}</StatusBadge></td
          ><td class="row-actions"
            ><Button
              href={`/providers/${provider.name}/edit`}
              variant="ghost"
              size="icon-sm"
              aria-label={i18n.t('providers.editAria', { name: provider.name })}><Pencil size={14} /></Button
            ><Button
              variant="ghost"
              size="icon-sm"
              aria-label={i18n.t('providers.deleteAria', { name: provider.name })}
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
      <Dialog.Title>{i18n.t('providers.deleteTitle')}</Dialog.Title>
      <Dialog.Description>{i18n.t('providers.deleteConfirm', { name: deleting ?? '' })}</Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer>
      <Button variant="outline" onclick={() => (deleting = null)}>{i18n.t('common.cancel')}</Button>
      <Button variant="destructive" onclick={remove}>{i18n.t('common.delete')}</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
