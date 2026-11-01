<script lang="ts">
import { Eye, Pencil, Plus, RefreshCw, Trash2 } from '@lucide/svelte';
import { untrack } from 'svelte';
import { goto } from '$app/navigation';
import { page as route } from '$app/state';
import { Button } from '$src/components/button/index.js';
import Diagnostics from '$src/components/configuration/Diagnostics.svelte';
import { readError } from '$src/components/configuration/read-error.js';
import DataTable from '$src/components/DataTable.svelte';
import EmptyTableRow from '$src/components/EmptyTableRow.svelte';
import PageHead from '$src/components/PageHead.svelte';
import ResourceScope from '$src/components/ResourceScope.svelte';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import type { McpServer } from '$src/types/mcp.js';
import { maskTarget } from './config-view.js';
import DeleteMcpDialog from './DeleteMcpDialog.svelte';
import ProxyPanel from './ProxyPanel.svelte';

type McpTab = 'config' | 'proxy';

const config = getConfig();
const i18n = getI18n();

const tabOrder: McpTab[] = ['config', 'proxy'];
let configTabEl = $state<HTMLButtonElement | null>(null);
let proxyTabEl = $state<HTMLButtonElement | null>(null);
let proxyPanelRef = $state<ReturnType<typeof ProxyPanel> | null>(null);

let activeTab = $state<McpTab>(route.url.searchParams.get('tab') === 'proxy' ? 'proxy' : 'config');

let servers = $state<McpServer[]>([]);
let globalServers = $state<McpServer[]>([]);
let globalDiagnostics = $state<string[]>([]);
let globalError = $state('');
let globalLoading = $state(false);
let showGlobal = $state(false);
let projectId = $derived(config.selectedProjectId ?? null);
let diagnostics = $state<string[]>([]);
let loading = $state(true);
let error = $state('');
let query = $state('');
let deleting = $state<McpServer | null>(null);
let removed = $state(false);
let page = $state(1);
const pageSize = 10;
let request = 0;
let globalRequest = 0;

const allServers = $derived([
  ...servers.map((entry) => ({ entry, projectId })),
  ...(projectId && showGlobal ? globalServers.map((entry) => ({ entry, projectId: null })) : []),
]);

const filtered = $derived(
  allServers.filter(({ entry: server }) =>
    `${server.name} ${server.type} ${maskTarget(server)}`.toLowerCase().includes(query.trim().toLowerCase()),
  ),
);

const currentPage = $derived(Math.min(page, Math.max(1, Math.ceil(filtered.length / pageSize))));
const rows = $derived(filtered.slice((currentPage - 1) * pageSize, currentPage * pageSize));

$effect(() => {
  query;
  servers.length;
  page = 1;
});

function selectTab(tab: McpTab) {
  if (activeTab === tab) return;
  activeTab = tab;
  const url = new URL(route.url);
  if (tab === 'config') {
    url.searchParams.delete('tab');
  } else {
    url.searchParams.set('tab', tab);
  }
  void goto(url.toString(), { replaceState: true, keepFocus: true, noScroll: true });
}

function onTabKeys(event: KeyboardEvent) {
  const current = tabOrder.indexOf(activeTab);
  let next: McpTab | undefined;
  if (event.key === 'ArrowRight') next = tabOrder[(current + 1) % tabOrder.length];
  else if (event.key === 'ArrowLeft') next = tabOrder[(current - 1 + tabOrder.length) % tabOrder.length];
  else if (event.key === 'Home') next = tabOrder[0];
  else if (event.key === 'End') next = tabOrder[tabOrder.length - 1];
  if (!next) return;
  event.preventDefault();
  selectTab(next);
  if (next === 'config') configTabEl?.focus();
  else if (next === 'proxy') proxyTabEl?.focus();
}

async function load() {
  const scope = projectId;
  const id = ++request;
  loading = true;
  error = '';
  try {
    const result = await (scope === null ? config.listMcps() : config.listMcps(scope));
    if (id !== request) return;
    servers = result.data;
    diagnostics = result.diagnostics;
  } catch (cause) {
    if (id !== request) return;
    error = readError(cause);
    servers = [];
    diagnostics = [];
  } finally {
    if (id === request) loading = false;
  }
}

$effect(() => {
  projectId;
  servers = [];
  diagnostics = [];
  query = '';
  page = 1;
  deleting = null;
  removed = false;
  untrack(() => {
    void load();
  });
  return () => {
    request += 1;
  };
});

$effect(() => {
  if (projectId === null || !showGlobal) {
    globalServers = [];
    globalDiagnostics = [];
    globalError = '';
    globalLoading = false;
    return;
  }
  untrack(() => {
    void loadGlobal();
  });
  return () => {
    globalRequest += 1;
  };
});

async function loadGlobal() {
  const id = ++globalRequest;
  globalLoading = true;
  globalError = '';
  globalServers = [];
  globalDiagnostics = [];
  try {
    const result = await config.listMcps();
    if (id !== globalRequest) return;
    globalServers = result.data;
    globalDiagnostics = result.diagnostics;
  } catch (cause) {
    if (id === globalRequest) globalError = readError(cause);
  } finally {
    if (id === globalRequest) globalLoading = false;
  }
}

function refresh() {
  void load();
  if (projectId && showGlobal) void loadGlobal();
}

function href(name: string, scope: string | null) {
  return `/mcp/${encodeURIComponent(name)}${scope !== null ? `?project=${encodeURIComponent(scope)}` : projectId ? '?readonly=1' : ''}`;
}
</script>

<svelte:head><title>{i18n.t('mcp.metaTitle')}</title></svelte:head>
<PageHead eyebrow={i18n.t('mcp.eyebrow')} title={i18n.t('mcp.title')}>
  <div class="flex flex-wrap justify-end gap-2">
    <Button
      variant="outline"
      size="sm"
      disabled={activeTab === 'config' ? (loading || globalLoading || config.saving) : false}
      aria-busy={activeTab === 'config' ? (loading || globalLoading) : false}
      onclick={() => {
        if (activeTab === 'config') {
          refresh();
        } else {
          proxyPanelRef?.refresh();
        }
      }}
    >
      <RefreshCw
        size={14}
        class={activeTab === 'config' && (loading || globalLoading) ? 'animate-spin motion-reduce:animate-none' : ''}
        aria-hidden="true"
      />
      {i18n.t('configuration.refresh')}
    </Button>
    {#if activeTab === 'config' && projectId === null}
      <Button size="sm" href="/mcp/new"><Plus size={14} />{i18n.t('mcp.newTitle')}</Button>
    {/if}
  </div>
</PageHead>

<!-- Tab navigation -->
<div class="flex gap-2 mb-4" role="tablist" aria-label={i18n.t('mcp.title')}>
  <Button
    bind:ref={configTabEl}
    id="mcp-tab-config"
    role="tab"
    variant={activeTab === 'config' ? 'default' : 'outline'}
    size="sm"
    aria-selected={activeTab === 'config'}
    aria-controls="mcp-tabpanel-config"
    tabindex={activeTab === 'config' ? 0 : -1}
    onclick={() => selectTab('config')}
    onkeydown={onTabKeys}
  >
    {i18n.t('mcp.tabConfig')}
  </Button>
  <Button
    bind:ref={proxyTabEl}
    id="mcp-tab-proxy"
    role="tab"
    variant={activeTab === 'proxy' ? 'default' : 'outline'}
    size="sm"
    aria-selected={activeTab === 'proxy'}
    aria-controls="mcp-tabpanel-proxy"
    tabindex={activeTab === 'proxy' ? 0 : -1}
    onclick={() => selectTab('proxy')}
    onkeydown={onTabKeys}
  >
    {i18n.t('mcp.tabProxy')}
  </Button>
</div>

{#if activeTab === 'config'}
  <div id="mcp-tabpanel-config" role="tabpanel" aria-labelledby="mcp-tab-config">
    <ResourceScope disabled={!!deleting || config.saving} bind:showGlobal />
    {#if projectId === null && (removed || route.url.searchParams.get('removed') === '1')}
      <div class="state-banner success" role="status">{i18n.t('mcp.removed')}</div>
    {/if}
    <Diagnostics messages={diagnostics} />
    {#if projectId && showGlobal}
      <Diagnostics messages={globalDiagnostics} />
      {#if globalLoading}
        <div class="state-banner" role="status">{i18n.t('scope.global')} · {i18n.t('common.loadingConfiguration')}</div>
      {/if}
      {#if globalError}
        <div class="state-banner error" role="alert">
          <span class="min-w-0 break-words [overflow-wrap:anywhere]"
            >{i18n.t('scope.globalComparisonFailed')} {globalError}</span
          ><Button variant="outline" size="sm" onclick={loadGlobal}>{i18n.t('configuration.retry')}</Button>
        </div>
      {/if}
    {/if}
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
          <EmptyTableRow
            colspan={6}
            message={i18n.t(query.trim() ? 'common.noMatches' : projectId ? 'scope.empty' : 'mcp.empty')}
          />
        {:else}
          {#each rows as { entry: server, projectId: rowProjectId } (`${rowProjectId ?? 'global'}:${server.name}`)}
            <tr>
              <td class="model-name">
                <a
                  class="inline-block max-w-56 break-words [overflow-wrap:anywhere] hover:text-[var(--accent-primary)] hover:underline"
                  href={href(server.name, rowProjectId)}
                  >{server.name}</a
                >
              </td>
              <td>{i18n.t(server.type === 'local' ? 'mcp.local' : 'mcp.remote')}</td>
              <td>
                <span class="block max-w-80 break-words [overflow-wrap:anywhere]">{maskTarget(server) || '—'}</span>
              </td>
              <td>
                <span class="inline-flex items-center gap-2 whitespace-nowrap"
                  ><span
                    aria-hidden="true"
                    class={`size-1.5 rounded-full ${server.disabled ? 'bg-[var(--text-muted)]' : 'bg-[var(--accent-primary)]'}`}
                  ></span>{i18n.t(server.disabled ? 'common.disabled' : 'common.enabled')}</span
                >
              </td>
              <td>
                {#if projectId}
                  <span
                    class="mb-1 inline-block border border-[var(--border-default)] px-1.5 py-0.5 text-[10px] text-[var(--text-muted)]"
                    >{i18n.t(rowProjectId ? 'scope.project' : 'scope.global')}</span
                  >
                {/if}
                <span class="block max-w-64 break-words text-[var(--text-secondary)] [overflow-wrap:anywhere]"
                  >{server.sourcePath}</span
                >
              </td>
              <td class="row-actions">
                <Button
                  href={href(server.name, rowProjectId)}
                  variant="ghost"
                  size="icon-sm"
                  aria-label={i18n.t('configuration.viewAria', { name: server.name })}
                  ><Eye size={14} /></Button
                >
                {#if projectId === null}
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
                {/if}
              </td>
            </tr>
          {/each}
        {/if}
      </tbody>
    </DataTable>
    <DeleteMcpDialog bind:server={deleting} onDeleted={async () => { removed = true; await load(); }} />
  </div>
{:else if activeTab === 'proxy'}
  <div id="mcp-tabpanel-proxy" role="tabpanel" aria-labelledby="mcp-tab-proxy">
    <ProxyPanel bind:this={proxyPanelRef} />
  </div>
{/if}
