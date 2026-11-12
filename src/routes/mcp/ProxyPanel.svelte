<script lang="ts">
import { Pencil, Plus, Power, PowerOff, RefreshCw, ShieldAlert, Trash2, XCircle } from '@lucide/svelte';
import { onDestroy, onMount } from 'svelte';
import { Button } from '$src/components/button/index.js';
import { readError } from '$src/components/configuration/read-error.js';
import DataTable from '$src/components/DataTable.svelte';
import * as Dialog from '$src/components/dialog/index.js';
import EmptyTableRow from '$src/components/EmptyTableRow.svelte';
import { Input } from '$src/components/input/index.js';
import { Switch } from '$src/components/switch/index.js';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import { toast } from '$src/shell/toast.svelte.js';
import type { McpProxyServerDto, McpProxyServerSummaryDto, McpProxyStateDto } from '$src/types/mcp-proxy.js';
import ProxyServerForm from './ProxyServerForm.svelte';

const config = getConfig();
const i18n = getI18n();

let proxyState = $state<McpProxyStateDto | null>(null);
let loading = $state(true);
let querying = $state(false);
let refreshing = $state(false);
let error = $state('');

let page = $state(1);
const pageSize = 10;
let query = $state('');

// Form dialog state
let formOpen = $state(false);
let editingServer = $state<McpProxyServerDto | null>(null);
let editingRevision = $state(0);
let serverPending = false;

// Delete dialog state
let deletingServer = $state<McpProxyServerSummaryDto | null>(null);
let deletePending = $state(false);
let deleteError = $state('');

// Close unit dialog state
let closeUnitOpen = $state(false);
let processes = $state<{ id: string; serviceId: string; pid: number | null; uptime: number }[]>([]);
let processesLoading = $state(false);
let processGeneration = $state(0);
let processQuery = $state('');
const normalizedProcessQuery = $derived(processQuery.trim().toLowerCase());
const filteredProcesses = $derived(
  processes.filter(
    (process) =>
      process.serviceId.toLowerCase().includes(normalizedProcessQuery) ||
      process.id.toLowerCase().includes(normalizedProcessQuery) ||
      (process.pid !== null && String(process.pid).includes(normalizedProcessQuery)),
  ),
);

async function openProcesses() {
  if (processesLoading || closeUnitPending) return;
  closeUnitId = '';
  processQuery = '';
  processes = [];
  closeUnitOpen = true;
  processesLoading = true;
  closeUnitError = '';
  try {
    const result = await config.listMcpProxyProcesses();
    if (mounted && closeUnitOpen) {
      processGeneration = result.generation;
      processes = result.processes;
    }
  } catch (cause) {
    if (mounted && closeUnitOpen) closeUnitError = readError(cause);
  } finally {
    if (mounted) processesLoading = false;
  }
}
let closeUnitId = $state('');
let closeUnitPending = $state(false);
let closeUnitError = $state('');

let mounted = false;
let request = 0;
let refreshQueued = false;

const isRunning = $derived(!loading && !error && proxyState?.actualState === 'running');
const services = $derived(proxyState?.services ?? []);

const filtered = $derived(
  services.filter(
    (s) =>
      s.name.toLowerCase().includes(query.trim().toLowerCase()) ||
      s.id.toLowerCase().includes(query.trim().toLowerCase()) ||
      s.protocol.toLowerCase().includes(query.trim().toLowerCase()),
  ),
);

const currentPage = $derived(Math.min(page, Math.max(1, Math.ceil(filtered.length / pageSize))));
const rows = $derived(filtered.slice((currentPage - 1) * pageSize, currentPage * pageSize));

async function readState(manual = false, afterMutation = false) {
  if (!mounted) return;

  if (querying) {
    refreshQueued ||= afterMutation;
    return;
  }

  refreshQueued = false;

  const seq = ++request;

  querying = true;
  refreshing = manual;

  try {
    const res = await config.getMcpProxyState();

    if (!mounted || seq !== request) return;

    proxyState = res;
    error = '';
  } catch (cause) {
    if (mounted && seq === request) {
      error = readError(cause);
    }
  } finally {
    if (mounted) {
      querying = false;
      refreshing = false;
      loading = false;
      if (refreshQueued) await readState(true, true);
    }
  }
}

let togglingGateway = $state(false);

async function toggleGateway(next: boolean) {
  if (!mounted || togglingGateway) return;

  togglingGateway = true;

  try {
    const current = await config.getMcpProxySettings();

    if (!mounted) return;

    await config.saveMcpProxySettings(
      {
        enabled: next,
        idleTimeout: current.idleTimeout,
        maxUnits: current.maxUnits,
        revision: current.revision,
      },
      current.revision,
    );
    await readState(true, true);
  } catch (cause) {
    if (mounted) {
      toast({
        variant: 'destructive',
        description: readError(cause),
      });
    }
  } finally {
    if (mounted) togglingGateway = false;
  }
}

async function openEdit(summary: McpProxyServerSummaryDto) {
  if (!mounted || !proxyState || serverPending) return;

  const revision = proxyState.revision;

  serverPending = true;

  try {
    const full = await config.getMcpProxyServer(summary.id);

    if (!mounted) return;

    editingServer = full;
    editingRevision = revision;
    formOpen = true;
  } catch (cause) {
    if (mounted) toast({ variant: 'destructive', description: readError(cause) });
  } finally {
    if (mounted) serverPending = false;
  }
}

async function toggleEnabled(summary: McpProxyServerSummaryDto) {
  if (!mounted || !proxyState || serverPending) return;

  const revision = proxyState.revision;

  serverPending = true;

  try {
    const full = await config.getMcpProxyServer(summary.id);

    if (!mounted) return;

    const updated: McpProxyServerDto = {
      ...full,
      enabled: !full.enabled,
    };

    await config.updateMcpProxyServer(updated, revision);
    await readState(true, true);
  } catch (cause) {
    if (mounted) toast({ variant: 'destructive', description: readError(cause) });
  } finally {
    if (mounted) serverPending = false;
  }
}

async function confirmDelete() {
  if (!mounted || !deletingServer || !proxyState || deletePending) return;

  deletePending = true;
  deleteError = '';

  try {
    await config.deleteMcpProxyServer(deletingServer.id, proxyState.revision);
    if (!mounted) return;

    deletingServer = null;
    await readState(true, true);
  } catch (cause) {
    if (mounted) deleteError = readError(cause);
  } finally {
    if (mounted) deletePending = false;
  }
}

async function submitCloseUnit() {
  if (!mounted || !closeUnitId.trim() || !proxyState || closeUnitPending) return;

  closeUnitPending = true;
  closeUnitError = '';

  try {
    await config.closeMcpProxyUnit({
      unitId: closeUnitId.trim(),
      expectedGeneration: processGeneration,
    });
    if (!mounted) return;

    closeUnitOpen = false;
    closeUnitId = '';
    await readState(true, true);
  } catch (cause) {
    if (mounted) closeUnitError = readError(cause);
  } finally {
    if (mounted) closeUnitPending = false;
  }
}

onMount(() => {
  mounted = true;
  void readState();
});

onDestroy(() => {
  mounted = false;
  request += 1;
});

export function refresh() {
  return readState(true);
}
</script>

<div class="space-y-4">
  <!-- Status Overview Banner -->
  <div class="panel p-4 bg-[var(--surface-input)] flex flex-wrap items-center justify-between gap-4">
    <div class="flex items-center gap-3">
      <span
        class={`h-2.5 w-2.5 shrink-0 rounded-full ${isRunning ? 'bg-[var(--status-success)]' : 'bg-[var(--text-muted)]'}`}
        aria-hidden="true"
      ></span>
      <div>
        <div class="flex items-center gap-2">
          <span class="font-semibold text-xs text-[var(--text-primary)]">
            {i18n.t(loading ? 'common.loadingConfiguration' : error || !proxyState ? 'common.unknownStatus' : isRunning ? 'mcp.proxy.running' : 'mcp.proxy.stopped')}
          </span>
          {#if proxyState?.boundAddr}
            <span class="text-[var(--text-muted)] text-xs">·</span>
            <span class="font-mono text-xs text-[var(--text-secondary)]">{proxyState.boundAddr}</span>
          {/if}
        </div>
        {#if proxyState}
          <div class="text-[11px] text-[var(--text-muted)] mt-0.5 flex flex-wrap gap-x-3">
            <span>{i18n.t('mcp.proxy.activeUnits')}: {proxyState.activeUnits} / {proxyState.maxUnits}</span>
            <span>{i18n.t('mcp.proxy.activeSessions')}: {proxyState.activeSessions}</span>
            <span>{i18n.t('mcp.proxy.revision')}: {proxyState.revision}</span>
          </div>
        {/if}
      </div>
    </div>

    <div class="flex items-center gap-2">
      <div class="flex items-center gap-2 mr-2">
        <label for="proxy-gateway-master-switch" class="font-medium text-xs text-[var(--text-primary)] cursor-pointer">
          {i18n.t('settings.proxy.enableGateway')}
        </label>
        <Switch
          id="proxy-gateway-master-switch"
          name="proxy-gateway-master-switch"
          checked={proxyState?.desiredState ?? false}
          disabled={loading || togglingGateway}
          onclick={(e) => {
            e.preventDefault();
            void toggleGateway(!(proxyState?.desiredState ?? false));
          }}
        />
      </div>
      <Button variant="outline" size="sm" disabled={loading || querying} onclick={openProcesses}>
        <XCircle size={14} />
        {i18n.t('mcp.proxy.closeUnit')}
      </Button>
      <Button
        variant="outline"
        size="icon-sm"
        disabled={loading || querying}
        onclick={() => readState(true)}
        aria-label={i18n.t('mcp.proxy.refresh')}
        title={i18n.t('mcp.proxy.refresh')}
      >
        <RefreshCw size={14} class={refreshing ? 'animate-spin motion-reduce:animate-none' : ''} />
      </Button>
      <Button variant="default" size="sm" href="/mcp/proxy/new">
        <Plus size={14} />
        {i18n.t('mcp.proxy.addServer')}
      </Button>
    </div>
  </div>

  {#if proxyState?.securityErrors && proxyState.securityErrors.length > 0}
    <div class="state-banner warning space-y-1" role="alert">
      <div class="flex items-center gap-2 font-medium text-xs">
        <ShieldAlert size={15} />
        {i18n.t('mcp.proxy.securityErrors')}
      </div>
      <ul class="list-disc pl-5 text-xs m-0 space-y-0.5">
        {#each proxyState.securityErrors as secErr}
          <li>{secErr}</li>
        {/each}
      </ul>
    </div>
  {/if}

  {#if error}
    <div class="state-banner error" role="alert">
      <span class="min-w-0 break-words [overflow-wrap:anywhere]">{error}</span>
      <Button variant="outline" size="sm" onclick={() => readState(true)}>
        {i18n.t('configuration.retry')}
      </Button>
    </div>
  {/if}

  <!-- Search Toolbar -->
  <div class="search-toolbar">
    <input aria-label={i18n.t('mcp.search')} placeholder={i18n.t('mcp.searchPlaceholder')} bind:value={query}>
  </div>

  <!-- Server Table -->
  <DataTable
    label={i18n.t('mcp.proxy.serversTitle')}
    total={loading || error ? 0 : filtered.length}
    bind:page
    {pageSize}
  >
    <thead>
      <tr>
        <th>{i18n.t('mcp.proxy.serverName')}</th>
        <th>{i18n.t('mcp.proxy.protocol')}</th>
        <th>{i18n.t('mcp.proxy.status')}</th>
        <th>{i18n.t('mcp.proxy.sessions')}</th>
        <th class="th-actions">{i18n.t('mcp.proxy.actions')}</th>
      </tr>
    </thead>
    <tbody>
      {#if loading}
        <EmptyTableRow colspan={5} message={i18n.t('common.loadingConfiguration')} />
      {:else if error}
        <EmptyTableRow colspan={5} message={i18n.t('mcp.proxy.loadFailed')} />
      {:else if !filtered.length}
        <EmptyTableRow colspan={5} message={i18n.t(query.trim() ? 'common.noMatches' : 'mcp.proxy.noServers')} />
      {:else}
        {#each rows as srv (srv.id)}
          <tr>
            <td class="model-name">
              <span class="font-medium text-[var(--text-primary)] block">{srv.name}</span>
              <span class="text-[10px] text-[var(--text-muted)] font-mono">{srv.id}</span>
            </td>
            <td>
              <span class="border border-[var(--border-default)] px-1.5 py-0.5 text-[10px] uppercase">
                {srv.protocol}
              </span>
            </td>
            <td>
              <span class="inline-flex items-center gap-1.5 text-xs">
                <span
                  class={`size-1.5 rounded-full ${srv.enabled ? 'bg-[var(--accent-primary)]' : 'bg-[var(--text-muted)]'}`}
                  aria-hidden="true"
                ></span>
                {i18n.t(srv.enabled ? 'common.enabled' : 'common.disabled')}
              </span>
            </td>
            <td>
              <span class="text-xs">{srv.activeSessions}</span>
            </td>
            <td class="row-actions">
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label={srv.enabled ? i18n.t('common.disabled') : i18n.t('common.enabled')}
                title={srv.enabled ? i18n.t('common.disabled') : i18n.t('common.enabled')}
                onclick={() => toggleEnabled(srv)}
              >
                {#if srv.enabled}
                  <Power size={14} class="text-[var(--status-success)]" />
                {:else}
                  <PowerOff size={14} class="text-[var(--text-muted)]" />
                {/if}
              </Button>
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label={i18n.t('mcp.proxy.editServer')}
                title={i18n.t('mcp.proxy.editServer')}
                onclick={() => openEdit(srv)}
              >
                <Pencil size={14} />
              </Button>
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label={i18n.t('mcp.proxy.deleteServer')}
                title={i18n.t('mcp.proxy.deleteServer')}
                onclick={() => (deletingServer = srv)}
              >
                <Trash2 size={14} />
              </Button>
            </td>
          </tr>
        {/each}
      {/if}
    </tbody>
  </DataTable>
</div>

<!-- Add / Edit Server Form Dialog -->
<ProxyServerForm
  bind:open={formOpen}
  server={editingServer}
  expectedRevision={editingRevision}
  onSaved={() => readState(true, true)}
/>

<!-- Delete Confirmation Dialog -->
<Dialog.Root
  open={deletingServer !== null}
  onOpenChange={(open) => { if (!open) { deletingServer = null; deleteError = ''; } }}
>
  <Dialog.Content variant="destructive" aria-busy={deletePending}>
    <Dialog.Header>
      <Dialog.Title>{i18n.t('mcp.proxy.deleteServer')}</Dialog.Title>
      <Dialog.Description class="break-words [overflow-wrap:anywhere]">
        {i18n.t('mcp.proxy.deleteConfirm', { name: deletingServer?.name ?? '' })}
      </Dialog.Description>
    </Dialog.Header>
    {#if deleteError}
      <p class="m-0 text-xs text-[var(--status-error)]" role="alert">{deleteError}</p>
    {/if}
    <Dialog.Footer>
      <Button variant="outline" disabled={deletePending} onclick={() => { deletingServer = null; deleteError = ''; }}>
        {i18n.t('common.cancel')}
      </Button>
      <Button variant="destructive" disabled={deletePending} onclick={confirmDelete}>
        {deletePending ? i18n.t('settings.saving') : i18n.t('mcp.proxy.deleteServer')}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>

<!-- Close Unit Dialog -->
<Dialog.Root
  open={closeUnitOpen}
  onOpenChange={(open) => { if (!open && !closeUnitPending) { closeUnitOpen = false; closeUnitError = ''; } }}
>
  <Dialog.Content
    class="flex h-[640px] max-h-[calc(100dvh-32px)] max-w-[720px] flex-col overflow-y-auto"
    aria-busy={closeUnitPending}
  >
    <Dialog.Header class="shrink-0">
      <Dialog.Title>{i18n.t('mcp.proxy.closeUnit')}</Dialog.Title>
      <Dialog.Description>
        {i18n.t('mcp.proxy.closeUnitPrompt')}
      </Dialog.Description>
    </Dialog.Header>
    <div class="flex min-h-0 flex-1 flex-col gap-3">
      <div class="shrink-0 space-y-2">
        <Input
          type="search"
          aria-label={i18n.t('mcp.proxy.searchProcesses')}
          aria-controls="proxy-process-results"
          placeholder={i18n.t('mcp.proxy.searchProcessesPlaceholder')}
          bind:value={processQuery}
          oninput={() => (closeUnitId = '')}
          disabled={closeUnitPending}
          autocomplete="off"
          spellcheck={false}
        />
        <p class="m-0 text-xs text-[var(--text-secondary)]" role="status" aria-atomic="true">
          {processesLoading
            ? i18n.t('common.loadingConfiguration')
            : i18n.t('mcp.proxy.processesCount', { count: filteredProcesses.length, total: processes.length })}
        </p>
      </div>
      <div id="proxy-process-results" class="min-h-0 flex-1 overflow-y-auto overscroll-contain">
        {#if !processesLoading}
          {#if filteredProcesses.length === 0}
            <p class="m-0 py-6 text-center text-[var(--text-secondary)]">
              {i18n.t(processes.length ? 'common.noMatches' : 'mcp.proxy.noProcesses')}
            </p>
          {:else}
            <div class="space-y-2" role="radiogroup" aria-label={i18n.t('mcp.proxy.closeUnit')}>
              {#each filteredProcesses as process (process.id)}
                <label
                  class="flex items-start gap-3 rounded-[2px] border border-[var(--border-default)] p-3 hover:bg-[var(--surface-hover)] has-checked:border-[var(--accent-primary)]"
                >
                  <input
                    class="mt-1 shrink-0 accent-[var(--accent-primary)]"
                    type="radio"
                    name="proxy-process"
                    value={process.id}
                    bind:group={closeUnitId}
                    disabled={closeUnitPending}
                  >
                  <span class="min-w-0 break-words [overflow-wrap:anywhere]">
                    <span class="block font-medium">{process.serviceId}</span>
                    <span class="block text-xs text-[var(--text-secondary)]"
                      >PID {process.pid ?? '—'} · {process.uptime}s</span
                    >
                    <small class="block font-mono text-[var(--text-muted)]">{process.id}</small>
                  </span>
                </label>
              {/each}
            </div>
          {/if}
        {/if}
      </div>
    </div>
    {#if closeUnitError}
      <p
        class="m-0 max-h-20 shrink-0 overflow-y-auto break-words text-xs text-[var(--status-error)] [overflow-wrap:anywhere]"
        role="alert"
      >
        {closeUnitError}
      </p>
    {/if}
    <Dialog.Footer class="shrink-0">
      <Button
        variant="outline"
        disabled={closeUnitPending}
        onclick={() => { closeUnitOpen = false; closeUnitError = ''; }}
      >
        {i18n.t('common.cancel')}
      </Button>
      <Button variant="destructive" disabled={!closeUnitId.trim() || closeUnitPending} onclick={submitCloseUnit}>
        {closeUnitPending ? i18n.t('settings.saving') : i18n.t('mcp.proxy.terminate')}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
