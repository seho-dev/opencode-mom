<script lang="ts">
import { RefreshCw } from '@lucide/svelte';
import { onDestroy, untrack } from 'svelte';
import { Button } from '$src/components/button/index.js';
import DataTable from '$src/components/DataTable.svelte';
import * as Dialog from '$src/components/dialog/index.js';
import EmptyTableRow from '$src/components/EmptyTableRow.svelte';
import PageHead from '$src/components/PageHead.svelte';
import { createPagination } from '$src/components/pagination.svelte.js';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import type { PluginRemovalPlan, PluginRuntime, PluginRuntimeItem } from '$src/types/plugins.js';

const config = getConfig();
const i18n = getI18n();
let runtime = $state<PluginRuntime | null>(null);
let loadedIdentity = $state<number | null>(null);
let pending = $state<'' | 'read' | 'update' | 'prepare' | 'remove'>('');
let error = $state<'read' | 'update' | 'prepare' | 'remove' | ''>('');
let updateDiagnostic = $state<'timeout' | 'connection' | 'verification' | number | null>(null);
let success = $state<'' | 'update' | 'remove'>('');
let uncertain = $state(false);
let updateTarget = $state<string | null>(null);
let removalPlan = $state.raw<PluginRemovalPlan | null>(null);
let query = $state('');
let request = 0;
let mounted = true;
const pageSize = 5;
const pagination = createPagination(config, {
  key: () => JSON.stringify(['plugins', identity]),
  query: () => query,
  total: () => filtered.length,
  ready: () => loadedIdentity === identity && runtime !== null && !pending && !error && !uncertain,
  pageSize,
});
const sourceLabels = {
  builtin: 'plugins.runtimeBuiltin',
  package: 'plugins.runtimePackage',
  local: 'plugins.runtimeLocal',
  sdk: 'plugins.runtimeSdk',
} as const;
const identity = $derived(config.serviceTargetVersion);
const filtered = $derived(
  (runtime?.data ?? []).filter((item) =>
    `${sourceName(item)} ${item.source.type} ${i18n.t(sourceLabels[item.source.type])} ${item.source.type === 'package' ? (item.source.version ?? '') : ''}`
      .toLowerCase()
      .includes(query.trim().toLowerCase()),
  ),
);
const currentPage = $derived(Math.max(1, Math.min(pagination.page, Math.ceil(filtered.length / pageSize))));
const visible = $derived(filtered.slice((currentPage - 1) * pageSize, currentPage * pageSize));

$effect(() => {
  void identity;
  runtime = null;
  error = '';
  updateDiagnostic = null;
  success = '';
  pending = '';
  uncertain = false;
  updateTarget = null;
  removalPlan = null;
  query = '';
  untrack(() => {
    void refresh();
  });

  return () => {
    request += 1;
  };
});
onDestroy(() => {
  mounted = false;
  request += 1;
  removalPlan = null;
});

function sourceName(item: PluginRuntimeItem) {
  if (item.source.type === 'package') return item.source.target;

  if (item.source.type === 'local') return item.source.path;

  return item.id ?? i18n.t(sourceLabels[item.source.type]);
}

function setUpdateDiagnostic(cause: unknown) {
  if (
    !cause ||
    typeof cause !== 'object' ||
    Array.isArray(cause) ||
    !('code' in cause) ||
    cause.code !== 'configuration_failed' ||
    !('detail' in cause) ||
    typeof cause.detail !== 'string'
  )
    return;

  if (cause.detail === 'plugin_update_timeout') updateDiagnostic = 'timeout';
  else if (cause.detail === 'plugin_update_connection') updateDiagnostic = 'connection';
  else if (cause.detail === 'plugin_update_verification') updateDiagnostic = 'verification';
  else {
    const status = /^plugin_update_runtime_http:([1-5]\d{2})$/.exec(cause.detail);

    if (status && status[0] === cause.detail) updateDiagnostic = Number(status[1]);
  }
}

async function refresh(keepSuccess = false) {
  if (pending) return;

  const seq = ++request;
  const key = identity;

  pending = 'read';
  error = '';
  updateDiagnostic = null;
  runtime = null;
  removalPlan = null;
  if (!keepSuccess) success = '';

  try {
    const result = await config.listPluginRuntime();

    if (!mounted || seq !== request || key !== identity) return;

    runtime = result;
    loadedIdentity = key;
    uncertain = false;
  } catch {
    if (mounted && seq === request && key === identity) {
      error = 'read';
      uncertain = true;
    }
  } finally {
    if (mounted && seq === request && key === identity) pending = '';
  }
}

function canUpdatePackage(item: PluginRuntimeItem) {
  return item.source.type === 'package' && item.source.outdated === true && item.canUpdate && !item.source.updating;
}

async function update() {
  if (
    pending ||
    uncertain ||
    !updateTarget ||
    !runtime?.data.some(
      (item) => item.source.type === 'package' && item.source.target === updateTarget && canUpdatePackage(item),
    )
  )
    return;

  const target = updateTarget;
  const key = identity;
  const seq = ++request;

  pending = 'update';
  error = '';
  updateDiagnostic = null;
  success = '';

  try {
    await config.updatePluginPackage(target);
    if (!mounted || seq !== request || key !== identity) return;

    success = 'update';
    uncertain = true;
    runtime = null;
    updateTarget = null;
    pending = '';
    await refresh(true);
  } catch (cause) {
    if (mounted && seq === request && key === identity) {
      error = 'update';
      setUpdateDiagnostic(cause);
      uncertain = true;
      updateTarget = null;
    }
  } finally {
    if (mounted && seq === request && key === identity) pending = '';
  }
}

async function prepareRemoval(item: PluginRuntimeItem) {
  if (pending || uncertain || !item.canRemove || (item.source.type !== 'package' && item.source.type !== 'local'))
    return;

  const target =
    item.source.type === 'package'
      ? { type: 'package' as const, target: item.source.target }
      : { type: 'local' as const, path: item.source.path };
  const seq = ++request;
  const key = identity;

  pending = 'prepare';
  error = '';
  updateDiagnostic = null;
  success = '';
  removalPlan = null;

  try {
    const plan = await config.preparePluginRemoval(target);

    if (!mounted || seq !== request || key !== identity) return;

    removalPlan = plan;
  } catch {
    if (mounted && seq === request && key === identity) {
      error = 'prepare';
      removalPlan = null;
    }
  } finally {
    if (mounted && seq === request && key === identity) pending = '';
  }
}

async function removeDeclaration() {
  if (pending || uncertain || !removalPlan) return;

  const plan = removalPlan;
  const seq = ++request;
  const key = identity;

  pending = 'remove';
  error = '';
  updateDiagnostic = null;
  success = '';

  try {
    await config.removePluginDeclaration(plan);
    if (!mounted || seq !== request || key !== identity) return;

    removalPlan = null;
    success = 'remove';
    pending = '';
    await refresh(true);
  } catch {
    if (mounted && seq === request && key === identity) {
      error = 'remove';
      removalPlan = null;
    }
  } finally {
    if (mounted && seq === request && key === identity) pending = '';
  }
}
</script>

<PageHead eyebrow={i18n.t('plugins.eyebrow')} title={i18n.t('plugins.title')}>
  <div class="flex flex-wrap items-center justify-end gap-2">
    <span class="mr-1 text-xs text-[var(--text-muted)]">{i18n.t('plugins.globalDeclarations')}</span>
    <Button variant="outline" size="sm" disabled={!!pending} onclick={() => refresh()}
      ><RefreshCw size={14} class={pending === 'read' ? 'animate-spin motion-reduce:animate-none' : ''} />
      {i18n.t('plugins.runtimeRefresh')}</Button
    >
  </div>
</PageHead>
<section class="min-w-0" aria-label={i18n.t('plugins.runtimeTitle')} aria-busy={!!pending}>
  {#if error === 'read' && success === 'update'}
    <div class="state-banner !border-l-[var(--status-warning)] text-[var(--status-warning)]" role="status">
      {i18n.t('plugins.packageUpdatedRuntimeUnavailable')}
    </div>
  {:else if error}
    <div class="state-banner error" role="alert">
      {i18n.t(error === 'read' ? 'plugins.runtimeUnavailable' : error === 'update' ? 'plugins.updatePackageFailed' : error === 'prepare' ? 'plugins.removalPrepareFailed' : 'plugins.removalFailed')}
      {#if error === 'update' && updateDiagnostic !== null}
        <p class="m-0 w-full break-words text-xs leading-5 [overflow-wrap:anywhere]">
          {i18n.t(typeof updateDiagnostic === 'number' ? 'plugins.updateDiagnosticHttp' : updateDiagnostic === 'timeout' ? 'plugins.updateDiagnosticTimeout' : updateDiagnostic === 'connection' ? 'plugins.updateDiagnosticConnection' : 'plugins.updateDiagnosticVerification', { status: typeof updateDiagnostic === 'number' ? updateDiagnostic : '' })}
        </p>
      {/if}
    </div>
  {/if}
  {#if success && !(error === 'read' && success === 'update')}
    <div class="state-banner success" role="status">
      {i18n.t(success === 'remove' ? 'plugins.declarationRemoved' : 'plugins.packageUpdated')}
    </div>
  {/if}
  <div class="search-toolbar">
    <input
      type="search"
      name="plugins-search"
      aria-label={i18n.t('plugins.search')}
      placeholder={i18n.t('plugins.searchPlaceholder')}
      bind:value={query}
    >
  </div>
  {#if runtime || pending === 'read'}
    <DataTable label={i18n.t('plugins.runtimeTitle')} total={filtered.length} bind:page={pagination.page} {pageSize}>
      <thead>
        <tr>
          <th>{i18n.t('plugins.runtimeSource')}</th>
          <th>{i18n.t('common.status')}</th>
          <th>{i18n.t('plugins.runtimeVersion')}</th>
          <th class="th-actions">{i18n.t('common.actions')}</th>
        </tr>
      </thead>
      <tbody>
        {#if pending === 'read'}
          <EmptyTableRow colspan={4} message={i18n.t('plugins.runtimeLoading')} />
        {:else}
          {#each visible as item, index (JSON.stringify([identity, item.id, sourceName(item), index]))}
            <tr>
              <td>
                <span class="inline-block border border-[var(--border-default)] px-2 py-0.5 text-[10px]"
                  >{i18n.t(sourceLabels[item.source.type])}</span
                >
                {#if item.source.type === 'package' || item.source.type === 'local' || item.id}
                  <span class="mt-1 block max-w-80 break-words [overflow-wrap:anywhere]">{sourceName(item)}</span>
                {/if}
                <small class="mt-1 block text-[var(--text-muted)]"
                  >{[item.features.server && i18n.t('plugins.runtimeServer'), item.features.tui && i18n.t('plugins.runtimeTui'), item.features.rpc && i18n.t('plugins.runtimeRpc')].filter(Boolean).join(' · ')}</small
                >
              </td>
              <td>
                <span
                  class={item.state.status === 'failed' ? 'text-[var(--status-error)]' : 'text-[var(--text-secondary)]'}
                  >{i18n.t(item.state.status === 'active' ? 'plugins.runtimeActive' : 'plugins.runtimeFailed')}</span
                >
                {#if item.state.status === 'failed'}
                  <span class="mt-1 block max-w-72 whitespace-pre-wrap break-words text-xs [overflow-wrap:anywhere]"
                    >{item.state.error}</span
                  >
                {/if}
              </td>
              <td>
                {#if item.source.type === 'package'}
                  {item.source.version || '—'}
                  {#if item.source.updating}
                    <small class="mt-1 block">{i18n.t('plugins.updating')}</small>
                  {:else if item.source.outdated}
                    <small class="mt-1 block text-[var(--status-warning)]">{i18n.t('plugins.outdated')}</small>
                  {/if}
                {:else}
                  —
                {/if}
              </td>
              <td class="row-actions">
                <Button
                  variant="outline"
                  size="sm"
                  disabled={!canUpdatePackage(item) || !!pending || uncertain}
                  title={!canUpdatePackage(item) ? i18n.t('plugins.updateUnavailable') : undefined}
                  onclick={() => { if (canUpdatePackage(item) && item.source.type === 'package' && !pending && !uncertain) updateTarget = item.source.target; }}
                  >{i18n.t('plugins.updatePackage')}</Button
                >
                <Button
                  variant="outline"
                  size="sm"
                  disabled={!item.canRemove || !!pending || uncertain}
                  onclick={() => prepareRemoval(item)}
                  >{i18n.t('common.delete')}</Button
                >
              </td>
            </tr>
          {:else}
            <EmptyTableRow colspan={4} message={i18n.t(query.trim() ? 'common.noMatches' : 'plugins.runtimeEmpty')} />
          {/each}
        {/if}
      </tbody>
    </DataTable>
  {/if}
</section>
<Dialog.Root open={updateTarget !== null} onOpenChange={(open) => { if (!open && !pending) updateTarget = null; }}>
  <Dialog.Content
    aria-busy={pending === 'update'}
    onEscapeKeydown={(event) => { if (pending) event.preventDefault(); }}
    onInteractOutside={(event) => { if (pending) event.preventDefault(); }}
  >
    <Dialog.Header
      ><Dialog.Title>{i18n.t('plugins.updateConfirmTitle')}</Dialog.Title
      ><Dialog.Description class="break-words [overflow-wrap:anywhere]"
        >⚠️ {i18n.t('plugins.updateConfirm', { name: updateTarget ?? '' })}</Dialog.Description
      ></Dialog.Header
    >
    <p class="m-0 break-words text-xs text-[var(--text-secondary)] [overflow-wrap:anywhere]">
      {i18n.t('plugins.runtimeDirectory')}: {runtime?.directory}
    </p>
    <Dialog.Footer
      ><Button variant="outline" disabled={!!pending} onclick={() => { updateTarget = null; }}
        >{i18n.t('common.cancel')}</Button
      ><Button disabled={!!pending || uncertain} onclick={update}
        >{i18n.t(pending === 'update' ? 'plugins.updatingPackage' : 'plugins.updatePackage')}</Button
      ></Dialog.Footer
    >
  </Dialog.Content>
</Dialog.Root>
<Dialog.Root open={removalPlan !== null} onOpenChange={(open) => { if (!open && !pending) removalPlan = null; }}>
  <Dialog.Content
    aria-busy={pending === 'remove'}
    onEscapeKeydown={(event) => { if (pending) event.preventDefault(); }}
    onInteractOutside={(event) => { if (pending) event.preventDefault(); }}
  >
    <Dialog.Header>
      <Dialog.Title>{i18n.t('plugins.removalConfirmTitle')}</Dialog.Title>
      <Dialog.Description>{i18n.t('plugins.removalConfirm')}</Dialog.Description>
    </Dialog.Header>
    {#if removalPlan}
      <p class="m-0 break-words text-sm [overflow-wrap:anywhere]">
        {removalPlan.target.type === 'package' ? removalPlan.target.target : removalPlan.target.path}
      </p>
      <ul class="m-0 max-h-48 space-y-2 overflow-y-auto pl-4 text-xs text-[var(--text-secondary)]">
        {#each removalPlan.sources as source, index (index)}
          <li class="break-words [overflow-wrap:anywhere]">{source.path}</li>
        {/each}
      </ul>
    {/if}
    <Dialog.Footer>
      <Button variant="outline" disabled={!!pending} onclick={() => { removalPlan = null; }}
        >{i18n.t('common.cancel')}</Button
      >
      <Button variant="destructive" disabled={!!pending || uncertain} onclick={removeDeclaration}>
        {i18n.t('common.delete')}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
