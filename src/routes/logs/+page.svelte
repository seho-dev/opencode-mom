<script lang="ts">
import { AlertCircle, CheckCircle2, ChevronRight, FolderOpen, RefreshCw, Trash2 } from '@lucide/svelte';
import { onMount, tick } from 'svelte';
import { Button } from '$src/components/button/index.js';
import * as Dialog from '$src/components/dialog/index.js';
import EmptyTableRow from '$src/components/EmptyTableRow.svelte';
import PageHead from '$src/components/PageHead.svelte';
import Select from '$src/components/Select.svelte';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import {
  LOG_TYPES,
  type LoggingState,
  type LogPage,
  type LogQuery,
  type LogStatus,
  type LogType,
} from '$src/types/logging.js';
import LogDetails from './LogDetails.svelte';
import { logSummary } from './log-summary.js';

const config = getConfig();
const i18n = getI18n();
const PAGE_SIZE = 50;
let mounted = false;
let type = $state<LogType | ''>('');
let status = $state<LogStatus | ''>('');
let period = $state('24h');
const filterKey = $derived(`${type}/${status}/${period}`);
const typeOptions = $derived([
  { value: '', label: i18n.t('logs.allTypes') },
  ...LOG_TYPES.map((value) => ({ value, label: i18n.t(`logs.type.${value}`) })),
]);
const statusOptions = $derived([
  { value: '', label: i18n.t('logs.allStatuses') },
  { value: 'success', label: i18n.t('logs.success') },
  { value: 'error', label: i18n.t('logs.error') },
]);
const periodOptions = $derived([
  { value: '24h', label: i18n.t('logs.recent24h') },
  { value: '7d', label: i18n.t('logs.recent7d') },
  { value: 'all', label: i18n.t('logs.allTime') },
]);
const timeFormat = $derived(
  new Intl.DateTimeFormat(i18n.locale === 'zh' ? 'zh-CN' : 'en-GB', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  }),
);
const zone = Intl.DateTimeFormat().resolvedOptions().timeZone;

let logging = $state.raw<LoggingState | null>(null);
let stateLoading = $state(false);
let stateError = $state(false);
let stateRequest = 0;
let result = $state.raw<LogPage | null>(null);
let querying = $state(false);
let queryError = $state(false);
let queryRequest = 0;
let committedFilters = $state('');
let querySnapshot: LogQuery = {};
let historyEpoch = $state(0);
let opening = $state(false);
let openError = $state(false);
let confirmOpen = $state(false);
let clearing = $state(false);
let clearError = $state(false);
let cleared = $state(false);
let clearTrigger = $state<HTMLButtonElement | null>(null);
let cancelButton = $state<HTMLButtonElement | null>(null);
let confirmButton = $state<HTMLButtonElement | null>(null);
const needsAttention = $derived(!!logging && (!!logging.error || (logging.enabled && !logging.recording)));
const retained = $derived(!!result && (querying || queryError || committedFilters !== filterKey));
const pageReady = $derived(!!result && !retained && !clearing && !confirmOpen);

function firstQuery(): LogQuery {
  const now = Date.now();

  return {
    ...(type ? { type } : {}),
    ...(status ? { status } : {}),
    ...(period === 'all'
      ? {}
      : {
          from: new Date(now - (period === '7d' ? 7 : 1) * 86_400_000).toISOString(),
          to: new Date(now).toISOString(),
        }),
    limit: PAGE_SIZE,
  };
}

async function loadPage(query = firstQuery()) {
  if (!mounted || clearing) return;

  historyEpoch += 1;

  const request = ++queryRequest;
  const filters = filterKey;

  querying = true;
  queryError = false;
  cleared = false;

  try {
    const page = await config.queryLogs(query);

    if (!mounted || request !== queryRequest) return;

    result = page;
    querySnapshot = { ...query, cursor: undefined };
    committedFilters = filters;
  } catch {
    if (mounted && request === queryRequest) queryError = true;
  } finally {
    if (mounted && request === queryRequest) querying = false;
  }
}

async function loadState() {
  if (!mounted || clearing || stateLoading) return;

  const request = ++stateRequest;

  stateLoading = true;
  stateError = false;

  try {
    const state = await config.getLoggingState();

    if (mounted && request === stateRequest) logging = state;
  } catch {
    if (mounted && request === stateRequest) stateError = true;
  } finally {
    if (mounted && request === stateRequest) stateLoading = false;
  }
}

async function refresh() {
  if (!mounted || querying || stateLoading || clearing || confirmOpen) return;

  await Promise.all([loadPage(), loadState()]);
}

function changeFilter(field: 'type' | 'status' | 'period', value: string) {
  if (!mounted || clearing || confirmOpen) return;

  if (field === 'type') type = value as LogType | '';
  else if (field === 'status') status = value as LogStatus | '';
  else period = value;

  void loadPage();
}

function loadMore() {
  if (!pageReady || !result?.nextCursor) return;

  void loadPage({ ...querySnapshot, cursor: result.nextCursor });
}

async function openDirectory() {
  if (!mounted || opening || clearing || confirmOpen) return;

  opening = true;
  openError = false;

  try {
    await config.openLogsDirectory();
  } catch {
    if (mounted) openError = true;
  } finally {
    if (mounted) opening = false;
  }
}

async function clearHistory() {
  if (!mounted || !confirmOpen || clearing || opening) return;

  clearing = true;
  clearError = false;
  committedFilters = '';
  historyEpoch += 1;
  queryRequest += 1;
  stateRequest += 1;
  querying = false;
  stateLoading = false;
  stateError = true;

  try {
    const state = await config.clearLogs();

    if (!mounted) return;

    logging = state;
    stateError = false;
    queryError = false;
    result = { records: [], nextCursor: null, invalidLines: 0 };
    committedFilters = filterKey;
    cleared = true;
    confirmOpen = false;
  } catch {
    if (mounted) clearError = true;
  } finally {
    if (mounted) {
      clearing = false;
      await tick();
      if (mounted && confirmOpen && document.activeElement === document.body) confirmButton?.focus();
    }
  }
}

function displayTime(timestamp: string) {
  const date = new Date(timestamp);

  return Number.isNaN(date.getTime()) ? timestamp : timeFormat.format(date);
}

onMount(() => {
  mounted = true;
  void refresh();

  return () => {
    mounted = false;
    queryRequest += 1;
    stateRequest += 1;
  };
});
</script>

<svelte:head><title>{i18n.t('logs.metaTitle')}</title></svelte:head>
<div class="min-w-0 w-full max-w-6xl">
  <PageHead eyebrow={i18n.t('logs.eyebrow')} title={i18n.t('logs.title')}>
    <Button
      variant="outline"
      size="sm"
      disabled={querying || stateLoading || clearing || confirmOpen}
      aria-busy={querying || stateLoading}
      onclick={refresh}
    >
      <RefreshCw class={querying || stateLoading ? 'animate-spin motion-reduce:animate-none' : ''} aria-hidden="true" />
      {i18n.t('configuration.refresh')}
    </Button>
  </PageHead>
  <p class="page-description">{i18n.t('logs.description')}</p>

  <section class="panel mb-4 min-w-0 max-w-full rounded-[var(--radius)] p-4" aria-label={i18n.t('logs.settings')}>
    <div class="flex flex-wrap items-center justify-between gap-3">
      <div class="flex min-w-0 flex-wrap items-center gap-x-4 gap-y-2">
        <span
          class="flex items-center gap-2 text-xs"
          class:text-[var(--status-warning)]={needsAttention}
          class:text-[var(--text-secondary)]={!needsAttention}
          role="status"
        >
          <span
            class="size-1.5 rounded-full bg-current"
            class:text-[var(--status-success)]={!!logging?.recording && !needsAttention && !stateLoading && !stateError && !clearing}
            aria-hidden="true"
          ></span>
          {i18n.t(stateLoading ? 'settings.loading' : clearing ? 'logs.clearing' : stateError || !logging ? 'common.unknownStatus' : logging.recording ? 'logs.recording' : 'logs.recordingOff')}
          {#if needsAttention && !stateLoading && !stateError && !clearing}
            · {i18n.t('logs.recordingFailed')}
          {/if}
        </span>
        <a class="text-xs" href="/settings?tab=general#settings-logging">{i18n.t('logs.settings')}</a>
      </div>
      <div class="flex flex-wrap gap-2">
        <Button
          variant="outline"
          size="sm"
          disabled={opening || clearing || confirmOpen}
          aria-busy={opening}
          onclick={openDirectory}
        >
          <FolderOpen aria-hidden="true" /> {i18n.t('logs.openDirectory')}
        </Button>
        <Button
          bind:ref={clearTrigger}
          variant="destructive"
          size="sm"
          disabled={opening || clearing || confirmOpen}
          onclick={() => { clearError = false; confirmOpen = true; }}
        >
          <Trash2 aria-hidden="true" /> {i18n.t('logs.clear')}
        </Button>
      </div>
    </div>
    {#if logging?.directory}
      <p class="mt-3 mb-0 break-words text-[11px] leading-[18px] text-[var(--text-secondary)] [overflow-wrap:anywhere]">
        {i18n.t('logs.directory')}: <span>{logging.directory}</span>
      </p>
    {/if}
    {#if logging && !logging.enabled && !logging.recording && !stateLoading && !stateError}
      <p class="field-hint mt-2">{i18n.t('logs.disabledHistory')}</p>
    {/if}
    {#if stateError || needsAttention}
      <p class="mt-2 mb-0 text-xs leading-5 text-[var(--status-error)]" role="alert">
        {i18n.t(stateError ? 'logs.stateFailed' : 'logs.recordingFailedHint')}
      </p>
    {/if}
    {#if logging && logging.droppedRecords > 0}
      <p class="mt-2 mb-0 text-xs leading-5 text-[var(--status-warning)]">
        {i18n.t('logs.droppedRecords', { count: logging.droppedRecords })}
      </p>
    {/if}
    {#if openError}
      <p class="mt-2 mb-0 text-xs leading-5 text-[var(--status-error)]" role="alert">{i18n.t('logs.openFailed')}</p>
    {/if}
  </section>

  <div class="mb-3 grid min-w-0 grid-cols-1 gap-3 sm:grid-cols-3">
    <div class="field min-w-0 gap-1.5">
      <label for="log-type">{i18n.t('logs.type')}</label>
      <Select
        id="log-type"
        value={type}
        options={typeOptions}
        disabled={clearing || confirmOpen}
        onchange={(value) => changeFilter('type', value)}
      />
    </div>
    <div class="field min-w-0 gap-1.5">
      <label for="log-status">{i18n.t('logs.status')}</label>
      <Select
        id="log-status"
        value={status}
        options={statusOptions}
        disabled={clearing || confirmOpen}
        onchange={(value) => changeFilter('status', value)}
      />
    </div>
    <div class="field min-w-0 gap-1.5">
      <label for="log-period">{i18n.t('logs.period')}</label>
      <Select
        id="log-period"
        value={period}
        options={periodOptions}
        disabled={clearing || confirmOpen}
        onchange={(value) => changeFilter('period', value)}
      />
    </div>
  </div>
  <p class="mb-2 text-[10px] leading-4 text-[var(--text-muted)]">{i18n.t('logs.localTime', { zone })}</p>
  {#if queryError}
    <p class="state-banner error" role="alert">{i18n.t('logs.loadFailed')}</p>
  {/if}
  {#if result && result.invalidLines > 0}
    <p class="mb-3 text-xs leading-5 text-[var(--status-warning)]" role="status">
      {i18n.t('logs.invalidLines', { count: result.invalidLines })}
    </p>
  {/if}
  {#if retained}
    <p class="field-hint mb-3" role="status">{i18n.t('logs.retained')}</p>
  {/if}
  {#if cleared}
    <p class="state-banner success" role="status">{i18n.t('logs.cleared')}</p>
  {/if}
  <section
    class="panel table-wrap min-w-0 max-w-full max-h-[36rem] overflow-auto rounded-[var(--radius)]"
    aria-label={i18n.t('logs.tableLabel')}
    aria-busy={querying}
  >
    <table class="data-table w-full min-w-0 max-w-full table-fixed">
      <thead>
        <tr>
          <th scope="col" class="w-[25%] !px-3 sm:w-40">{i18n.t('logs.time')}</th>
          <th scope="col" class="hidden w-28 !px-2 sm:table-cell">{i18n.t('logs.type')}</th>
          <th scope="col" class="w-20 !px-2 sm:w-24">{i18n.t('logs.status')}</th>
          <th scope="col" class="!px-3">{i18n.t('logs.event')}</th>
        </tr>
      </thead>
      <tbody>
        {#if result?.records.length}
          {#each result.records as record (`${record.sessionId}/${record.sequence}`)}
            <tr>
              <td class="!px-3 !align-top text-[10px] leading-[18px] text-[var(--text-secondary)] sm:text-[11px]">
                <time datetime={record.timestamp} title={record.timestamp}>{displayTime(record.timestamp)}</time>
              </td>
              <td class="hidden !px-2 !align-top text-[11px] text-[var(--text-secondary)] sm:table-cell">
                {i18n.t(`logs.type.${record.type}`)}
              </td>
              <td class="!px-2 !align-top">
                <span
                  class="inline-flex items-center gap-1 text-[10px]"
                  class:text-[var(--status-success)]={record.status === 'success'}
                  class:text-[var(--status-error)]={record.status === 'error'}
                >
                  {#if record.status === 'success'}
                    <CheckCircle2 size={12} aria-hidden="true" />
                  {:else}
                    <AlertCircle size={12} aria-hidden="true" />
                  {/if}
                  {i18n.t(record.status === 'success' ? 'logs.success' : 'logs.error')}
                </span>
              </td>
              <td class="min-w-0 !px-3 !align-top [overflow-wrap:anywhere]">
                <span class="text-[10px] text-[var(--text-muted)] sm:hidden">{i18n.t(`logs.type.${record.type}`)}</span>
                <p class="mt-1 mb-0 break-words font-semibold [overflow-wrap:anywhere]">{record.event}</p>
                <p
                  class="mt-1 mb-0 line-clamp-2 break-words text-[11px] leading-[18px] text-[var(--text-secondary)] [overflow-wrap:anywhere]"
                >
                  {logSummary(record.message, i18n)}
                </p>
                <LogDetails {record} trace epoch={historyEpoch} disabled={!pageReady} />
              </td>
            </tr>
          {/each}
        {:else}
          <EmptyTableRow
            colspan={4}
            message={i18n.t(querying ? 'logs.loading' : queryError ? 'logs.unavailable' : 'logs.empty')}
          />
        {/if}
      </tbody>
    </table>
  </section>
  <nav class="pagination flex-wrap" aria-label={i18n.t('common.pagination')}>
    <span class="pagination-info">{i18n.t('logs.recordCount', { count: result?.records.length ?? 0 })}</span>
    <div class="pagination-controls">
      <Button
        variant="outline"
        size="sm"
        disabled={!pageReady || !result?.nextCursor}
        aria-busy={querying}
        onclick={loadMore}
      >
        {i18n.t('logs.loadMore')} <ChevronRight aria-hidden="true" />
      </Button>
    </div>
  </nav>
</div>

<Dialog.Root open={confirmOpen} onOpenChange={(open) => { if (!clearing) confirmOpen = open; }}>
  <Dialog.Content
    aria-busy={clearing}
    onOpenAutoFocus={(event) => { event.preventDefault(); cancelButton?.focus(); }}
    onCloseAutoFocus={(event) => { event.preventDefault(); if (mounted) clearTrigger?.focus(); }}
    onEscapeKeydown={(event) => { if (clearing) event.preventDefault(); }}
    onInteractOutside={(event) => { if (clearing) event.preventDefault(); }}
  >
    <Dialog.Header>
      <Dialog.Title>{i18n.t('logs.clearTitle')}</Dialog.Title>
      <Dialog.Description>{i18n.t('logs.clearDescription')}</Dialog.Description>
    </Dialog.Header>
    {#if clearError}
      <p class="m-0 text-xs leading-5 text-[var(--status-error)]" role="alert">{i18n.t('logs.clearFailed')}</p>
    {/if}
    <Dialog.Footer>
      <Button bind:ref={cancelButton} variant="outline" disabled={clearing} onclick={() => (confirmOpen = false)}
        >{i18n.t('common.cancel')}</Button
      >
      <Button
        bind:ref={confirmButton}
        variant="destructive"
        disabled={clearing}
        aria-busy={clearing}
        onclick={clearHistory}
      >
        {i18n.t(clearing ? 'logs.clearing' : 'logs.clearConfirm')}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
