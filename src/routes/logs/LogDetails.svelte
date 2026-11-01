<script lang="ts">
import { onMount, untrack } from 'svelte';
import { Button } from '$src/components/button/index.js';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import type { LogPage, LogRecord } from '$src/types/logging.js';
import LogDetails from './LogDetails.svelte';
import { logSummary } from './log-summary.js';

let {
  record,
  trace = false,
  epoch = 0,
  disabled = false,
}: { record: LogRecord; trace?: boolean; epoch?: number; disabled?: boolean } = $props();
const config = getConfig();
const i18n = getI18n();
const TRACE_LIMIT = 100;
let details = $state<HTMLDetailsElement | null>(null);
let expanded = $state(false);
let result = $state.raw<LogPage | null>(null);
let loading = $state(false);
let error = $state(false);
let mounted = false;
let request = 0;
const records = $derived(result ? [...result.records].reverse() : []);
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

async function loadTrace() {
  if (!mounted || !trace || !expanded || disabled || loading || !record.operationId) return;
  const seq = ++request;
  const version = epoch;
  const operationId = record.operationId;
  loading = true;
  error = false;
  try {
    const page = await config.queryLogs({ operationId, limit: TRACE_LIMIT });
    if (mounted && expanded && !disabled && seq === request && version === epoch && operationId === record.operationId)
      result = page;
  } catch {
    if (mounted && expanded && !disabled && seq === request && version === epoch && operationId === record.operationId)
      error = true;
  } finally {
    if (mounted && seq === request) loading = false;
  }
}

function onToggle(event: Event) {
  expanded = (event.currentTarget as HTMLDetailsElement).open;
  if (!expanded) {
    request += 1;
    loading = false;
  } else if (!result && !error) void loadTrace();
}

$effect(() => {
  epoch;
  disabled;
  record.operationId;
  untrack(() => {
    request += 1;
    result = null;
    error = false;
    loading = false;
    expanded = false;
    if (details) details.open = false;
  });
});

onMount(() => {
  mounted = true;
  return () => {
    mounted = false;
    request += 1;
  };
});

function displayTime(timestamp: string) {
  const date = new Date(timestamp);
  return Number.isNaN(date.getTime()) ? timestamp : timeFormat.format(date);
}
</script>

<details bind:this={details} ontoggle={onToggle} class="mt-2 min-w-0 max-w-full text-[11px] leading-[18px]">
  <summary class="w-fit cursor-pointer text-[var(--text-secondary)] hover:text-[var(--accent-primary)]">
    {i18n.t('logs.details')}
  </summary>
  {#if expanded}
    <dl
      class="mt-2 mb-0 grid min-w-0 max-w-full max-h-64 grid-cols-1 gap-x-4 gap-y-1 overflow-auto rounded-[var(--radius)] bg-[var(--surface-input)] p-3 sm:grid-cols-[max-content_minmax(0,1fr)] [&_dt]:text-[var(--text-secondary)] [&_dd]:m-0 [&_dd]:min-w-0 [&_dd]:whitespace-pre-wrap [&_dd]:break-words [&_dd]:[overflow-wrap:anywhere]"
    >
      <dt>{i18n.t('logs.timestamp')}</dt>
      <dd>{record.timestamp}</dd>
      <dt>{i18n.t('logs.event')}</dt>
      <dd>{record.event}</dd>
      <dt>{i18n.t('logs.message')}</dt>
      <dd>{logSummary(record.message, i18n) || '—'}</dd>
      <dt>{i18n.t('logs.operationId')}</dt>
      <dd>{record.operationId || '—'}</dd>
      <dt>{i18n.t('logs.sessionId')}</dt>
      <dd>{record.sessionId || '—'}</dd>
      <dt>{i18n.t('logs.sequence')}</dt>
      <dd>{record.sequence}</dd>
      <dt>{i18n.t('logs.duration')}</dt>
      <dd>{record.durationMs == null ? '—' : i18n.t('logs.durationMs', { count: record.durationMs })}</dd>
      <dt>{i18n.t('logs.errorCode')}</dt>
      <dd>{record.errorCode ?? '—'}</dd>
      {#if record.context?.step != null}
        <dt>{i18n.t('logs.step')}</dt>
        <dd>{record.context.step}</dd>
      {/if}
      {#if record.context?.count != null}
        <dt>{i18n.t('logs.count')}</dt>
        <dd>{record.context.count}</dd>
      {/if}
      {#if record.context?.targetWritten != null}
        <dt>{i18n.t('logs.targetWritten')}</dt>
        <dd>{i18n.t(record.context.targetWritten ? 'logs.yes' : 'logs.no')}</dd>
      {/if}
    </dl>
    {#if trace && record.operationId}
      <section
        class="mt-3 min-w-0 max-w-full max-h-80 overflow-auto border-t border-[var(--border-default)] pt-3"
        aria-label={i18n.t('logs.traceTitle')}
        aria-busy={loading}
      >
        <h3 class="mt-0 mb-1 text-xs font-semibold">{i18n.t('logs.traceTitle')}</h3>
        <p class="field-hint">{i18n.t('logs.traceHint', { count: TRACE_LIMIT })}</p>
        <p class="mt-1 mb-2 text-[10px] text-[var(--text-muted)]">{i18n.t('logs.localTime', { zone })}</p>
        {#if loading}
          <p role="status">{i18n.t('logs.loading')}</p>
        {:else if error}
          <p class="mb-2 break-words text-xs leading-5 text-[var(--status-error)]" role="alert">
            {i18n.t('logs.traceFailed')}
          </p>
          <Button variant="outline" size="sm" {disabled} onclick={loadTrace}>{i18n.t('configuration.retry')}</Button>
        {:else if result}
          {#if result.invalidLines > 0}
            <p class="mb-2 text-xs leading-5 text-[var(--status-warning)]" role="status">
              {i18n.t('logs.invalidLines', { count: result.invalidLines })}
            </p>
          {/if}
          {#if !records.length}
            <p class="field-hint">{i18n.t('logs.traceEmpty')}</p>
          {:else}
            <ol class="m-0 list-none space-y-3 p-0" aria-label={i18n.t('logs.traceTitle')}>
              {#each records as item (`${item.sessionId}/${item.sequence}`)}
                <li class="min-w-0 max-w-full rounded-[var(--radius)] border border-[var(--border-default)] p-3">
                  <div
                    class="flex flex-wrap items-center justify-between gap-2 text-[10px] text-[var(--text-secondary)]"
                  >
                    <time datetime={item.timestamp} title={item.timestamp}>{displayTime(item.timestamp)}</time>
                    <span
                      class:text-[var(--status-error)]={item.status === 'error'}
                      class:text-[var(--status-success)]={item.status === 'success'}
                    >
                      {i18n.t(`logs.type.${item.type}`)}
                      · {i18n.t(item.status === 'success' ? 'logs.success' : 'logs.error')}
                    </span>
                  </div>
                  <p class="mt-2 mb-0 break-words font-semibold [overflow-wrap:anywhere]">{item.event}</p>
                  <p
                    class="mt-1 mb-0 max-h-28 overflow-auto whitespace-pre-wrap break-words text-[var(--text-secondary)] [overflow-wrap:anywhere]"
                  >
                    {logSummary(item.message, i18n)}
                  </p>
                  <LogDetails record={item} />
                </li>
              {/each}
            </ol>
          {/if}
          {#if result.nextCursor}
            <p class="field-hint mt-3">{i18n.t('logs.traceMore', { count: TRACE_LIMIT })}</p>
          {/if}
        {/if}
      </section>
    {/if}
  {/if}
</details>
