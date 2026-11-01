<script lang="ts">
import { Activity, Bot, Boxes, Cloud, Layers3, RefreshCw, Server } from '@lucide/svelte';
import { onMount } from 'svelte';
import { Button } from '$src/components/button/index.js';
import ActivityHeatmap from '$src/components/dashboard/ActivityHeatmap.svelte';
import RuntimeCard from '$src/components/dashboard/RuntimeCard.svelte';
import SwitchGroupDialog from '$src/components/dashboard/SwitchGroupDialog.svelte';
import TokenUsageChart from '$src/components/dashboard/TokenUsageChart.svelte';
import Select from '$src/components/Select.svelte';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import type { McpProxyStateDto } from '$src/types/mcp-proxy.js';
import type { DailyTokenUsage } from '$src/types/stats.js';
import { GROUP_TYPE_NATIVE, GROUP_TYPES, type GroupType } from '$src/utils/constants.js';
import {
  type ActivityPeriod,
  activityYears,
  calendarDate,
  dailyTokenUsage,
  formatTokensM,
  periodActivity,
} from '$src/utils/dashboard.js';

const config = getConfig();
const i18n = getI18n();
let runtime = $state<GroupType | null>(null);
let switchTrigger = $state<HTMLButtonElement | null>(null);
let dailyUsage = $state<DailyTokenUsage[] | null>(null);
let loading = $state(true);
let error = $state<string | null>(null);
let proxyState = $state<McpProxyStateDto | null>(null);
let proxyLoading = $state(true);
let request = false;
let mounted = false;
let timezone = $state('UTC');
let today = $state(calendarDate(new Date(), 'UTC'));
let usagePeriod = $state<ActivityPeriod>('day');
let usageSelectedYear = $state<number | null>(null);
let heatmapPeriod = $state<ActivityPeriod>('year');
let heatmapSelectedYear = $state<number | null>(null);
const usageYear = $derived(usageSelectedYear ?? Number(today.slice(0, 4)));
const heatmapYear = $derived(heatmapSelectedYear ?? Number(today.slice(0, 4)));
const years = $derived(
  [...new Set([...activityYears(dailyUsage ?? [], today), usageYear, heatmapYear])].sort((a, b) => b - a),
);
const usageCalendar = $derived(periodActivity(dailyUsage ?? [], usagePeriod, today, usageYear));
const heatmapCalendar = $derived(periodActivity(dailyUsage ?? [], heatmapPeriod, today, heatmapYear));
const usageCaption = $derived(
  usagePeriod === 'year'
    ? String(usageYear)
    : i18n.t(
        usagePeriod === 'day'
          ? 'dashboard.today'
          : usagePeriod === 'week'
            ? 'dashboard.thisWeek'
            : 'dashboard.thisMonth',
      ),
);
const heatmapPeriodLabel = $derived(
  heatmapPeriod === 'year'
    ? String(heatmapYear)
    : i18n.t(
        heatmapPeriod === 'day'
          ? 'dashboard.today'
          : heatmapPeriod === 'week'
            ? 'dashboard.thisWeek'
            : 'dashboard.thisMonth',
      ),
);
const busy = $derived(config.loading || config.saving || config.switching || config.reloading);
const configUnavailable = $derived(
  !config.loading && !config.groups.length && !config.providers.length && !!config.error,
);
const defaultPlugin = $derived<GroupType>(config.preferences.defaultPlugin ?? GROUP_TYPE_NATIVE);
const featuredType = $derived<GroupType>(
  config.groups.find((group) => group.id === config.selectedGroupId)?.type ?? defaultPlugin,
);
const runtimeTiles = $derived<GroupType[]>([defaultPlugin, ...GROUP_TYPES.filter((type) => type !== defaultPlugin)]);
const modelCount = $derived(
  config.providers.reduce((count, provider) => count + Object.keys(provider.models).length, 0),
);
const enabledGroups = $derived(config.groups.filter((group) => group.isEnabled).length);
const number = (count: number) => count.toLocaleString(i18n.locale === 'zh' ? 'zh-CN' : 'en-US');

const quickActions = [
  { href: '/groups/new', labelKey: 'dashboard.quickActionGroup' as const, Icon: Layers3 },
  { href: '/providers/new', labelKey: 'dashboard.quickActionProvider' as const, Icon: Cloud },
  { href: '/models/new', labelKey: 'dashboard.quickActionModel' as const, Icon: Boxes },
  { href: '/agents/new', labelKey: 'dashboard.quickActionAgent' as const, Icon: Bot },
];

function changeUsagePeriod(value: string) {
  if (usagePeriod !== value) {
    usagePeriod = value as ActivityPeriod;
  }
}
function changeUsageYear(value: string) {
  const next = Number(value);
  if (usageYear !== next) {
    usageSelectedYear = next;
  }
}
function changeHeatmapPeriod(value: string) {
  if (heatmapPeriod !== value) {
    heatmapPeriod = value as ActivityPeriod;
  }
}
function changeHeatmapYear(value: string) {
  const next = Number(value);
  if (heatmapYear !== next) {
    heatmapSelectedYear = next;
  }
}
async function refreshActivity() {
  if (request || !mounted) return;
  request = true;
  loading = true;
  error = null;
  today = calendarDate(new Date(), timezone);
  try {
    const records = await config.loadTokenUsageRecords();
    if (!mounted) return;
    dailyUsage = dailyTokenUsage(records, timezone);
  } catch (cause) {
    if (!mounted) return;
    error =
      cause && typeof cause === 'object' && 'message' in cause
        ? String(cause.message)
        : typeof cause === 'string'
          ? cause
          : i18n.t('dashboard.activityFailed');
  } finally {
    if (mounted) loading = false;
    request = false;
  }
}

async function loadProxyStatus() {
  if (typeof config.getMcpProxyState !== 'function') {
    if (mounted) proxyLoading = false;
    return;
  }
  proxyLoading = true;
  try {
    const state = await config.getMcpProxyState();
    if (!mounted) return;
    proxyState = state;
  } catch {
    if (!mounted) return;
    proxyState = null;
  } finally {
    if (mounted) proxyLoading = false;
  }
}

onMount(() => {
  mounted = true;
  timezone = Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC';
  void refreshActivity();
  void loadProxyStatus();
  return () => {
    mounted = false;
  };
});
function openSwitch(type: GroupType, trigger: HTMLButtonElement) {
  switchTrigger = trigger;
  runtime = type;
}
</script>

<svelte:head><title>{i18n.t('dashboard.metaTitle')}</title></svelte:head>
<div class="dashboard mx-auto w-full max-w-[1400px] space-y-5">
  <h1 class="sr-only">{i18n.t('dashboard.title')}</h1>

  <!-- Zone 1: Top 3 Runtime Selector Tiles -->
  <section aria-label={i18n.t('dashboard.runtimes')} class="grid grid-cols-1 gap-4 @min-[800px]:grid-cols-3">
    {#each runtimeTiles as type (type)}
      <RuntimeCard {type} {busy} variant="tile" onswitch={openSwitch} />
    {/each}
  </section>

  <!-- Zone 2: Active Runtime Workbench (left) + Quick Actions & System Status (right) -->
  <div class="grid grid-cols-1 items-stretch gap-5 @min-[960px]:grid-cols-12">
    <div class="flex flex-col @min-[960px]:col-span-7">
      <RuntimeCard type={featuredType} {busy} variant="featured" onswitch={openSwitch} />
    </div>

    <div class="flex flex-col gap-4 @min-[960px]:col-span-5">
      <!-- Quick Actions -->
      <section
        aria-label={i18n.t('dashboard.quickActions')}
        class="rounded-[3px] border border-[var(--border-default)] bg-[var(--surface-panel)] p-4"
      >
        <div class="mb-3 flex items-center gap-3">
          <h2
            class="font-[var(--font-heading)] text-[11px] font-bold uppercase tracking-[0.1em] text-[var(--text-secondary)]"
          >
            {i18n.t('dashboard.quickActions')}
          </h2>
          <span class="h-px flex-1 bg-[var(--border-default)]/40" aria-hidden="true"></span>
        </div>
        <div class="grid grid-cols-2 gap-2.5">
          {#each quickActions as action (action.href)}
            <a
              href={action.href}
              class="quick-action group flex min-w-0 items-center gap-2.5 rounded-[2px] border border-[var(--border-default)]/80 bg-[var(--surface-input)] px-3 py-2.5 font-[var(--font-heading)] text-xs font-semibold tracking-wide text-[var(--text-primary)] transition-colors hover:border-[var(--accent-primary)] hover:bg-[var(--surface-active)] hover:text-[var(--accent-primary)]"
            >
              <span
                class="flex size-7 shrink-0 items-center justify-center rounded-[2px] border border-[var(--border-default)]/70 bg-[var(--surface-panel)] text-[var(--accent-primary)] transition-colors group-hover:border-[var(--accent-primary)]/50"
              >
                <action.Icon size={14} aria-hidden="true" />
              </span>
              <span class="truncate">{i18n.t(action.labelKey)}</span>
            </a>
          {/each}
        </div>
      </section>

      <!-- System Status (2x2 compact grid) -->
      <section
        aria-label={i18n.t('dashboard.systemStatus')}
        class="flex flex-1 flex-col rounded-[3px] border border-[var(--border-default)] bg-[var(--surface-panel)] p-4"
      >
        <div class="mb-3 flex items-center gap-3">
          <h2
            class="font-[var(--font-heading)] text-[11px] font-bold uppercase tracking-[0.1em] text-[var(--text-secondary)]"
          >
            {i18n.t('dashboard.systemStatus')}
          </h2>
          <span class="h-px flex-1 bg-[var(--border-default)]/40" aria-hidden="true"></span>
        </div>
        <div class="grid flex-1 grid-cols-2 gap-2.5">
          <a
            href="/providers"
            class="metric flex flex-col justify-between rounded-[2px] border border-[var(--border-default)]/70 bg-[var(--surface-input)] p-3.5 transition-colors"
          >
            <div class="metric-label">
              <h2>{i18n.t('dashboard.providers')}</h2>
              <Cloud size={14} aria-hidden="true" />
            </div>
            <div class="metric-value">
              {config.loading || configUnavailable ? '—' : number(config.providers.length)}<span
                >{i18n.t('dashboard.configured')}</span
              >
            </div>
            <p class="truncate">
              {config.loading ? i18n.t('common.loading') : configUnavailable ? i18n.t('dashboard.unavailable') : config.providers.length ? config.providers.map((provider) => provider.name).join(', ') : i18n.t('empty.providersShort')}
            </p>
          </a>
          <a
            href="/models"
            class="metric flex flex-col justify-between rounded-[2px] border border-[var(--border-default)]/70 bg-[var(--surface-input)] p-3.5 transition-colors"
          >
            <div class="metric-label">
              <h2>{i18n.t('dashboard.models')}</h2>
              <Boxes size={14} aria-hidden="true" />
            </div>
            <div class="metric-value">{config.loading || configUnavailable ? '—' : number(modelCount)}</div>
            <p>{i18n.t('dashboard.configured')}</p>
          </a>
          <a
            href="/groups"
            class="metric flex flex-col justify-between rounded-[2px] border border-[var(--border-default)]/70 bg-[var(--surface-input)] p-3.5 transition-colors"
          >
            <div class="metric-label">
              <h2>{i18n.t('dashboard.enabledGroups')}</h2>
              <Layers3 size={14} aria-hidden="true" />
            </div>
            <div class="metric-value">{config.loading || configUnavailable ? '—' : number(enabledGroups)}</div>
            <p>{i18n.t('dashboard.acrossRuntimes')}</p>
          </a>
          <a
            href="/mcp?tab=proxy"
            class="status-card flex flex-col justify-between rounded-[2px] border border-[var(--border-default)]/70 bg-[var(--surface-input)] p-3.5 transition-colors"
          >
            <div class="metric-label">
              <h2>{i18n.t('dashboard.mcpProxy')}</h2>
              <Server size={14} aria-hidden="true" />
            </div>
            <div class="proxy-status-row my-1 flex min-w-0 items-center">
              <span
                class="proxy-badge"
                data-state={!proxyLoading && proxyState?.actualState === 'running'
                  ? 'running'
                  : !proxyLoading && proxyState
                    ? 'stopped'
                    : 'unknown'}
              >
                <span
                  class="proxy-dot size-2 shrink-0 rounded-full"
                  class:bg-[var(--status-success)]={!proxyLoading && proxyState?.actualState === 'running'}
                  class:bg-[var(--text-muted)]={proxyLoading || proxyState?.actualState !== 'running'}
                  aria-hidden="true"
                ></span><span class="min-w-0 break-words"
                  >{proxyLoading || !proxyState
                    ? '—'
                    : i18n.t(proxyState.actualState === 'running' ? 'mcp.proxy.running' : 'mcp.proxy.stopped')}</span
                >
              </span>
            </div>
            <p>
              {proxyLoading
                ? i18n.t('common.loading')
                : proxyState
                  ? i18n.t('dashboard.proxyServicesCount', { count: proxyState.enabledServices })
                  : i18n.t('dashboard.unavailable')}
            </p>
          </a>
        </div>
      </section>
    </div>
  </div>

  <!-- Zone 3: Activity Heatmap (left) + Token Usage & Inline Trend Chart (right) -->
  <section
    class="grid grid-cols-1 items-stretch gap-5 @min-[960px]:grid-cols-12"
    aria-label={i18n.t('dashboard.eyebrow')}
  >
    <div class="flex min-w-0 flex-col @min-[960px]:col-span-8">
      <ActivityHeatmap
        available={dailyUsage !== null}
        {loading}
        {error}
        {years}
        calendar={heatmapCalendar}
        period={heatmapPeriod}
        periodLabel={heatmapPeriodLabel}
        year={heatmapYear}
        onperiod={changeHeatmapPeriod}
        onyear={changeHeatmapYear}
        onrefresh={refreshActivity}
      />
    </div>

    <div class="flex min-w-0 flex-col @min-[960px]:col-span-4">
      <div
        class="usage-panel flex flex-1 flex-col rounded-[3px] border border-[var(--border-default)] bg-[var(--surface-panel)] p-5"
      >
        <div
          class="mb-4 flex flex-wrap items-center justify-between gap-3 border-b border-[var(--border-default)]/40 pb-4"
        >
          <div class="flex items-center gap-2">
            <Activity size={16} class="shrink-0 text-[var(--accent-primary)]" aria-hidden="true" />
            <span
              class="font-[var(--font-heading)] text-xs font-bold uppercase tracking-wider text-[var(--text-primary)]"
            >
              {i18n.t('dashboard.usageTrend')}
            </span>
          </div>
          <div class="usage-controls flex flex-wrap items-center gap-1.5 text-xs text-[var(--text-secondary)]">
            <label class="sr-only" for="usage-period">{i18n.t('dashboard.usagePeriod')}</label>
            <Select
              id="usage-period"
              class="w-[96px]"
              value={usagePeriod}
              options={[
                { value: 'day', label: i18n.t('dashboard.day') },
                { value: 'week', label: i18n.t('dashboard.week') },
                { value: 'month', label: i18n.t('dashboard.month') },
                { value: 'year', label: i18n.t('dashboard.year') },
              ]}
              onchange={changeUsagePeriod}
              disabled={loading || dailyUsage === null || !!error}
              ariaLabel={i18n.t('dashboard.usagePeriod')}
            />
            {#if usagePeriod === 'year'}
              <label class="sr-only" for="usage-year">{i18n.t('dashboard.usageYear')}</label>
              <Select
                id="usage-year"
                class="w-[82px]"
                value={String(usageYear)}
                options={years.map((value) => ({ value: String(value) }))}
                onchange={changeUsageYear}
                disabled={loading || dailyUsage === null || !!error}
                ariaLabel={i18n.t('dashboard.usageYear')}
              />
            {/if}
          </div>
        </div>

        <div class="metric flex flex-1 flex-col transition-colors" aria-busy={loading}>
          <div class="mb-2.5 flex flex-wrap items-center justify-between gap-2">
            <div class="metric-label m-0 shrink-0 whitespace-nowrap">
              <h2>{i18n.t('dashboard.tokenUsage')}</h2>
              <Activity size={14} aria-hidden="true" />
            </div>
            <p class="m-0 truncate font-mono text-[11px] text-[var(--text-secondary)]">
              {loading ? `${usageCaption} · ${i18n.t('common.loading')}` : error ? `${usageCaption} · ${i18n.t('dashboard.tokenUsageUnavailable')}` : usageCaption}
            </p>
          </div>

          <dl class="grid grid-cols-2 gap-2.5" aria-live="polite">
            {#each ['input', 'output'] as const as kind (kind)}
              <div
                class="rounded-[2px] border border-[var(--border-default)]/55 bg-[var(--surface-input)] px-3 py-2.5"
                class:border-t-2={true}
                class:border-t-[var(--accent-primary)]={kind === 'input'}
                class:border-t-[var(--status-success)]={kind === 'output'}
              >
                <dt
                  class="font-[var(--font-heading)] text-[10px] font-bold uppercase tracking-wider text-[var(--text-secondary)]"
                >
                  {i18n.t(kind === 'input' ? 'dashboard.input' : 'dashboard.output')}
                </dt>
                <dd
                  class="m-0 mt-1 break-all font-[var(--font-heading)] text-lg font-bold leading-tight tabular-nums"
                  class:text-[var(--accent-primary)]={kind === 'input'}
                  class:text-[var(--status-success)]={kind === 'output'}
                  title={!loading && !error && dailyUsage ? number(kind === 'input' ? usageCalendar.input : usageCalendar.output) : undefined}
                >
                  {loading || error || !dailyUsage ? '—' : formatTokensM(kind === 'input' ? usageCalendar.input : usageCalendar.output)}
                </dd>
              </div>
            {/each}
          </dl>

          {#if error && !loading}
            <div class="mt-2.5 flex flex-wrap items-center gap-2" role="alert">
              <span class="text-xs text-[var(--status-error)]">{i18n.t('dashboard.tokenUsageUnavailable')}</span>
              <Button
                variant="outline"
                size="icon-sm"
                onclick={refreshActivity}
                disabled={loading}
                aria-label={i18n.t('dashboard.retryTokenUsage')}
                title={i18n.t('dashboard.retryTokenUsage')}
                ><RefreshCw size={12} /></Button
              >
            </div>
          {/if}

          <TokenUsageChart calendar={!loading && !error && dailyUsage ? usageCalendar : null} period={usagePeriod} />
        </div>
      </div>
    </div>
  </section>
</div>
<SwitchGroupDialog bind:runtime trigger={switchTrigger} />

<style>
.dashboard {
  container-type: inline-size;
  position: relative;
  isolation: isolate;
}

.dashboard::before {
  content: "";
  position: fixed;
  inset: -24px;
  z-index: -1;
  pointer-events: none;
  background-image:
    linear-gradient(to right, color-mix(in srgb, var(--border-default) 7%, transparent) 1px, transparent 1px),
    linear-gradient(to bottom, color-mix(in srgb, var(--border-default) 7%, transparent) 1px, transparent 1px);
  background-size: 32px 32px;
}

.metric,
.status-card {
  min-width: 0;
}

a.metric:is(:hover, :focus-visible),
a.status-card:is(:hover, :focus-visible) {
  border-color: var(--accent-primary);
  background: var(--surface-hover);
  --metric-icon-color: var(--accent-primary);
}

.usage-controls :global(.select-trigger) {
  min-height: 28px;
  padding: 4px 8px;
  font-size: 11px;
}

.metric-label {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  margin-bottom: 6px;
  color: var(--text-secondary);
}

.usage-panel .metric-label {
  margin-bottom: 0;
  justify-content: flex-start;
}

.metric-label :global(svg) {
  color: var(--metric-icon-color, var(--text-secondary));
  transition: color var(--default-transition-duration) var(--default-transition-timing-function);
}

.metric-label h2 {
  font-family: var(--font-heading);
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.08em;
}

.usage-panel .metric-label h2 {
  font-size: 12px;
  color: var(--text-primary);
}

.metric-value {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 6px;
  font-family: var(--font-heading);
  font-size: 22px;
  font-weight: 700;
  line-height: 1.2;
  font-variant-numeric: tabular-nums;
}

.metric-value span {
  font-family: var(--font-primary);
  font-size: 11px;
  font-weight: 400;
  color: var(--text-secondary);
}

.proxy-badge {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  max-width: 100%;
  min-width: 0;
  padding: 3px 9px;
  border-radius: 2px;
  border: 1px solid var(--border-default);
  background: var(--surface-panel);
  font-family: var(--font-heading);
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.07em;
  line-height: 1.35;
  text-transform: uppercase;
  color: var(--text-secondary);
}

.proxy-badge[data-state="running"] {
  border-color: color-mix(in srgb, var(--status-success) 55%, var(--border-default));
  background: color-mix(in srgb, var(--status-success) 12%, var(--surface-panel));
  color: var(--status-success);
}

.proxy-badge[data-state="running"] .proxy-dot {
  box-shadow: 0 0 6px color-mix(in srgb, var(--status-success) 60%, transparent);
}

.proxy-badge[data-state="stopped"] {
  border-color: var(--border-default);
  background: var(--surface-panel);
  color: var(--text-primary);
}

.metric > p,
.status-card > p {
  margin-top: 6px;
  color: var(--text-secondary);
  font-size: 11px;
  line-height: 1.45;
}
</style>
