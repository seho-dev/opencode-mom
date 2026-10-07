<script lang="ts">
import { Eye, Pencil, Plus, RefreshCw, Trash2 } from '@lucide/svelte';
import { onDestroy, onMount } from 'svelte';
import { page as route } from '$app/state';
import { Button } from '$src/components/button/index.js';
import Diagnostics from '$src/components/configuration/Diagnostics.svelte';
import { readError } from '$src/components/configuration/read-error.js';
import DataTable from '$src/components/DataTable.svelte';
import EmptyTableRow from '$src/components/EmptyTableRow.svelte';
import PageHead from '$src/components/PageHead.svelte';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import type { McpServer } from '$src/types/mcp.js';
import { maskTarget } from './config-view.js';
import DeleteMcpDialog from './DeleteMcpDialog.svelte';

const config = getConfig();
const i18n = getI18n();
let servers = $state<McpServer[]>([]);
let diagnostics = $state<string[]>([]);
let loading = $state(true);
let error = $state('');
let query = $state('');
let deleting = $state<McpServer | null>(null);
let removed = $state(false);
let page = $state(1);
const pageSize = 10;
let request = 0;
const filtered = $derived(
  servers.filter((server) =>
    `${server.name} ${server.type} ${maskTarget(server)}`.toLowerCase().includes(query.trim().toLowerCase()),
  ),
);
const currentPage = $derived(Math.min(page, Math.max(1, Math.ceil(filtered.length / pageSize))));
const rows = $derived(filtered.slice((currentPage - 1) * pageSize, currentPage * pageSize));
$effect(() => {
  void query;
  void servers.length;
  page = 1;
});

async function load() {
  const id = ++request;
  loading = true;
  error = '';
  diagnostics = [];
  try {
    const result = await config.listMcps();
    if (id !== request) return;
    servers = result.data;
    diagnostics = result.diagnostics;
  } catch (cause) {
    if (id === request) {
      error = readError(cause);
      servers = [];
    }
  } finally {
    if (id === request) loading = false;
  }
}
onMount(() => {
  void load();
});
onDestroy(() => {
  request += 1;
});
</script>

<svelte:head><title>{i18n.t('mcp.metaTitle')}</title></svelte:head>
<PageHead eyebrow={i18n.t('mcp.eyebrow')} title={i18n.t('mcp.title')}>
  <div class="flex flex-wrap justify-end gap-2">
    <Button variant="outline" disabled={loading} onclick={load}>
      <RefreshCw size={14} />{i18n.t('configuration.refresh')}
    </Button>
    <Button href="/mcp/new"><Plus size={14} />{i18n.t('mcp.newTitle')}</Button>
  </div>
</PageHead>
{#if removed || route.url.searchParams.get('removed') === '1'}
  <div class="state-banner success" role="status">{i18n.t('mcp.removed')}</div>
{/if}
<Diagnostics messages={diagnostics} />
{#if error}
  <div class="state-banner error" role="alert">
    <span class="min-w-0 break-words [overflow-wrap:anywhere]">{i18n.t('mcp.loadFailed')} {error}</span>
    <Button variant="outline" size="sm" onclick={load}>{i18n.t('configuration.retry')}</Button>
  </div>
{/if}
<div class="search-toolbar">
  <input aria-label={i18n.t('mcp.search')} placeholder={i18n.t('mcp.searchPlaceholder')} bind:value={query}>
</div>
<DataTable label={i18n.t('mcp.title')} total={loading || error ? 0 : filtered.length} bind:page {pageSize}>
  <thead>
    <tr>
      <th>{i18n.t('common.name')}</th>
      <th>{i18n.t('common.type')}</th>
      <th>{i18n.t('mcp.target')}</th>
      <th>{i18n.t('mcp.configuration')}</th>
      <th>{i18n.t('configuration.source')}</th>
      <th class="th-actions">{i18n.t('common.actions')}</th>
    </tr>
  </thead>
  <tbody>
    {#if loading}
      <EmptyTableRow colspan={6} message={i18n.t('common.loadingConfiguration')} />
    {:else if error}
      <EmptyTableRow colspan={6} message={i18n.t('mcp.loadFailed')} />
    {:else if !filtered.length}
      <EmptyTableRow colspan={6} message={i18n.t(query.trim() ? 'common.noMatches' : 'mcp.empty')} />
    {:else}
      {#each rows as server (server.name)}
        <tr>
          <td class="model-name">
            <a
              class="inline-block max-w-56 break-words [overflow-wrap:anywhere] hover:text-[var(--accent-primary)] hover:underline"
              href={`/mcp/${encodeURIComponent(server.name)}`}
              >{server.name}</a
            >
          </td>
          <td>{i18n.t(server.type === 'local' ? 'mcp.local' : 'mcp.remote')}</td>
          <td><span class="block max-w-80 break-words [overflow-wrap:anywhere]">{maskTarget(server) || '—'}</span></td>
          <td>
            <span class="inline-flex items-center gap-2 whitespace-nowrap"
              ><span
                aria-hidden="true"
                class={`size-1.5 rounded-full ${server.disabled ? 'bg-[var(--text-muted)]' : 'bg-[var(--accent-primary)]'}`}
              ></span>{i18n.t(server.disabled ? 'common.disabled' : 'common.enabled')}</span
            >
          </td>
          <td>
            <span class="block max-w-64 break-words text-[var(--text-secondary)] [overflow-wrap:anywhere]"
              >{server.sourcePath}</span
            >
          </td>
          <td class="row-actions">
            <Button
              href={`/mcp/${encodeURIComponent(server.name)}`}
              variant="ghost"
              size="icon-sm"
              aria-label={i18n.t('configuration.viewAria', { name: server.name })}
              ><Eye size={14} /></Button
            >
            <Button
              href={`/mcp/${encodeURIComponent(server.name)}?edit=1`}
              variant="ghost"
              size="icon-sm"
              aria-label={i18n.t('configuration.editAria', { name: server.name })}
              ><Pencil size={14} /></Button
            >
            <Button
              variant="ghost"
              size="icon-sm"
              aria-label={i18n.t('mcp.deleteAria', { name: server.name })}
              onclick={() => (deleting = server)}
              ><Trash2 size={14} /></Button
            >
          </td>
        </tr>
      {/each}
    {/if}
  </tbody>
</DataTable>
<DeleteMcpDialog bind:server={deleting} onDeleted={async () => { removed = true; await load(); }} />
