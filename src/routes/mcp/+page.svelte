<script lang="ts">
import { Eye, Pencil, Plus, RefreshCw, Trash2 } from '@lucide/svelte';
import { onDestroy, untrack } from 'svelte';
import { goto } from '$app/navigation';
import { page as route } from '$app/state';
import { Button } from '$src/components/button/index.js';
import Diagnostics from '$src/components/configuration/Diagnostics.svelte';
import { readError } from '$src/components/configuration/read-error.js';
import DataTable from '$src/components/DataTable.svelte';
import EmptyTableRow from '$src/components/EmptyTableRow.svelte';
import PageHead from '$src/components/PageHead.svelte';
import { createPagination } from '$src/components/pagination.svelte.js';
import ResourceScope from '$src/components/ResourceScope.svelte';
import { Switch } from '$src/components/switch/index.js';
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
let loadedProjectId = $state<string | null | undefined>(undefined);
let globalServers = $state<McpServer[]>([]);
let globalReady = $state(false);
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
let toggling = $state<McpServer | null>(null);
let toggleError = $state('');
let toggled = $state(false);
const pageSize = 5;
const pagination = createPagination(config, {
  key: () => JSON.stringify(['mcp', projectId, projectId !== null && showGlobal]),
  query: () => query,
  total: () => filtered.length,
  ready: () =>
    loadedProjectId === projectId &&
    !loading &&
    !error &&
    (projectId === null || !showGlobal || (globalReady && !globalLoading && !globalError)),
  pageSize,
});
let request = 0;
let globalRequest = 0;
let mutation = 0;
const busy = $derived(config.saving || config.projectSaving || toggling !== null || deleting !== null);

const allServers = $derived([
  ...(loadedProjectId === projectId ? servers.map((entry) => ({ entry, projectId })) : []),
  ...(projectId && showGlobal ? globalServers.map((entry) => ({ entry, projectId: null })) : []),
]);

const filtered = $derived(
  allServers.filter(({ entry: server }) =>
    `${server.name} ${server.type} ${maskTarget(server)}`.toLowerCase().includes(query.trim().toLowerCase()),
  ),
);

const currentPage = $derived(Math.min(pagination.page, Math.max(1, Math.ceil(filtered.length / pageSize))));
const rows = $derived(filtered.slice((currentPage - 1) * pageSize, currentPage * pageSize));

onDestroy(() => {
  mutation += 1;
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
  toggleError = '';
  toggled = false;

  try {
    const result = await (scope === null ? config.listMcps() : config.listMcps(scope));

    if (id !== request) return;

    servers = result.data;
    loadedProjectId = scope;
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
  query = '';
  servers = [];
  loadedProjectId = undefined;
  diagnostics = [];
  deleting = null;
  removed = false;
  toggling = null;
  toggleError = '';
  toggled = false;
  mutation += 1;
  untrack(() => {
    void load();
  });

  return () => {
    request += 1;
  };
});

$effect(() => {
  globalReady = false;

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
    globalReady = true;
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

async function toggle(server: McpServer, scope: string | null) {
  if (scope !== projectId || loadedProjectId !== scope || !servers.includes(server) || busy || loading) return;

  const id = ++mutation;
  const read = request;
  const { name, disabled, sourcePath, sourceSnapshot } = server;
  const draft = {
    name,
    disabled: !disabled,
    expectedSourcePath: sourcePath,
    expectedDisabled: disabled,
    expectedSourceSnapshot: sourceSnapshot,
  };

  toggling = server;
  toggleError = '';
  toggled = false;

  try {
    const result = await (scope === null ? config.setMcpDisabled(draft) : config.setMcpDisabled(draft, scope));

    if (id !== mutation || scope !== projectId || read !== request) return;

    servers = servers.map((entry) => (entry === server ? result : entry));
    await load();
    if (id === mutation && scope === projectId && activeTab === 'config') toggled = true;
  } catch {
    if (id === mutation && scope === projectId && read === request && activeTab === 'config')
      toggleError = i18n.t('mcp.toggleFailed');
  } finally {
    if (id === mutation) toggling = null;
  }
}
</script>

<svelte:head><title>{i18n.t('mcp.metaTitle')}</title></svelte:head>
<PageHead eyebrow={i18n.t('mcp.eyebrow')} title={i18n.t('mcp.title')}>
  <div class="flex flex-wrap justify-end gap-2">
    <Button
      variant="outline"
      size="sm"
      disabled={activeTab === 'config' ? (loading || globalLoading || busy) : false}
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
      <Button size="sm" disabled={busy} href="/mcp/new"><Plus size={14} />{i18n.t('mcp.newTitle')}</Button>
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
    <ResourceScope disabled={busy} writable bind:showGlobal />
    {#if projectId !== null}
      <p class="form-hint">{i18n.t('mcp.projectEditOnly')}</p>
    {/if}
    {#if removed || (route.url.searchParams.get('removed') === '1' && route.url.searchParams.get('project') === projectId)}
      <div class="state-banner success" role="status">
        {i18n.t(projectId === null ? 'mcp.removed' : 'mcp.projectRemoved')}
      </div>
    {/if}
    {#if toggled || (route.url.searchParams.get('saved') === '1' && route.url.searchParams.get('project') === projectId)}
      <div class="state-banner success" role="status">
        {i18n.t(projectId === null ? 'configuration.saved' : 'mcp.projectSaved')}
      </div>
    {/if}
    {#if toggleError}
      <div class="state-banner error" role="alert">{toggleError}</div>
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
    <DataTable
      label={i18n.t('mcp.title')}
      total={loading || error ? 0 : filtered.length}
      bind:page={pagination.page}
      {pageSize}
    >
      <thead>
        <tr>
          <th>{i18n.t('common.name')}</th>
          <th>{i18n.t('common.type')}</th>
          <th>{i18n.t('mcp.target')}</th>
          <th>{i18n.t('common.status')}</th>
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
                <div class="flex items-center gap-2 whitespace-nowrap">
                  <Switch
                    checked={!server.disabled}
                    disabled={rowProjectId !== projectId || busy || loading}
                    aria-label={i18n.t(server.disabled ? 'mcp.enableAria' : 'mcp.disableAria', { name: server.name, scope: i18n.t(rowProjectId === null ? 'scope.global' : 'scope.project') })}
                    aria-busy={toggling === server}
                    title={rowProjectId !== projectId ? i18n.t('scope.globalReadOnly') : undefined}
                    onclick={(event) => { event.preventDefault(); void toggle(server, rowProjectId); }}
                  />
                </div>
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
                {#if rowProjectId === projectId}
                  <Button
                    href={`${href(server.name, rowProjectId)}${rowProjectId === null ? '?' : '&'}edit=1`}
                    variant="ghost"
                    size="icon-sm"
                    disabled={busy}
                    aria-label={i18n.t('configuration.editAria', { name: server.name })}
                    ><Pencil size={14} /></Button
                  >
                {/if}
                {#if rowProjectId === projectId}
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    disabled={busy}
                    aria-label={i18n.t('mcp.deleteAria', { name: server.name })}
                    onclick={() => {
                      if (rowProjectId === projectId && loadedProjectId === projectId && !busy && !loading)
                        deleting = server;
                    }}
                    ><Trash2 size={14} /></Button
                  >
                {/if}
              </td>
            </tr>
          {/each}
        {/if}
      </tbody>
    </DataTable>
    {#key projectId}
      <DeleteMcpDialog bind:server={deleting} {projectId} onDeleted={async () => { removed = true; await load(); }} />
    {/key}
  </div>
{:else if activeTab === 'proxy'}
  <div id="mcp-tabpanel-proxy" role="tabpanel" aria-labelledby="mcp-tab-proxy">
    <ProxyPanel bind:this={proxyPanelRef} />
  </div>
{/if}
