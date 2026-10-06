<script lang="ts">
import { Activity, Boxes, Cloud, Layers3, RefreshCw } from '@lucide/svelte';
import { onMount } from 'svelte';
import { Button } from '$src/components/button/index.js';
import ActivityHeatmap from '$src/components/dashboard/ActivityHeatmap.svelte';
import RuntimeCard from '$src/components/dashboard/RuntimeCard.svelte';
import SwitchGroupDialog from '$src/components/dashboard/SwitchGroupDialog.svelte';
import Select from '$src/components/Select.svelte';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import type { DailyTokenUsage } from '$src/types/stats.js';
import { GROUP_TYPES, type GroupType } from '$src/utils/constants.js';
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
let request = false;
let active = true;
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
const usagePeriodLabel = $derived(
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
const modelCount = $derived(
  config.providers.reduce((count, provider) => count + Object.keys(provider.models).length, 0),
);
const enabledGroups = $derived(config.groups.filter((group) => group.isEnabled).length);
const number = (count: number) => count.toLocaleString(i18n.locale === 'zh' ? 'zh-CN' : 'en-US');
function changeUsagePeriod(value: string) {
  if (usagePeriod === value) return;
  usagePeriod = value as ActivityPeriod;
}
function changeUsageYear(value: string) {
  if (usageYear === Number(value)) return;
  usageSelectedYear = Number(value);
}
function changeHeatmapPeriod(value: string) {
  if (heatmapPeriod === value) return;
  heatmapPeriod = value as ActivityPeriod;
}
function changeHeatmapYear(value: string) {
  if (heatmapYear === Number(value)) return;
  heatmapSelectedYear = Number(value);
}
async function refreshActivity() {
  if (request || !active) return;
  request = true;
  loading = true;
  error = null;
  today = calendarDate(new Date(), timezone);
  try {
    const records = await config.loadTokenUsageRecords();
    if (active) dailyUsage = dailyTokenUsage(records, timezone);
  } catch (cause) {
    if (active)
      error =
        cause && typeof cause === 'object' && 'message' in cause
          ? String(cause.message)
          : typeof cause === 'string'
            ? cause
            : i18n.t('dashboard.activityFailed');
  } finally {
    if (active) loading = false;
    request = false;
  }
}
onMount(() => {
  timezone = Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC';
  void refreshActivity();
  return () => {
    active = false;
  };
});
function openSwitch(type: GroupType, trigger: HTMLButtonElement) {
  if (busy) return;
  switchTrigger = trigger;
  runtime = type;
}
</script>

<svelte:head><title>{i18n.t('dashboard.metaTitle')}</title></svelte:head>
<div class="dashboard @container relative isolate mx-auto w-full max-w-[1400px] space-y-6">
  <h1 class="sr-only">{i18n.t('dashboard.title')}</h1>
  <section aria-label={i18n.t('dashboard.runtimes')} class="grid grid-cols-1 gap-5 @min-[800px]:grid-cols-3">
    {#each GROUP_TYPES as type}
      <RuntimeCard {type} {busy} onswitch={openSwitch} />
    {/each}
  </section>
  <section class="space-y-3" aria-label={i18n.t('dashboard.eyebrow')}>
    <div class="usage-controls flex flex-wrap items-center justify-end gap-2 text-[11px] text-[var(--text-secondary)]">
      <label for="usage-period">{i18n.t('dashboard.usagePeriod')}</label>
      <Select
        id="usage-period"
        class="w-[100px]"
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
        <label for="usage-year">{i18n.t('dashboard.usageYear')}</label>
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
    <div class="grid grid-cols-2 gap-4 @min-[800px]:grid-cols-4">
      <a
        href="/providers"
        class="metric rounded-[3px] border border-[var(--border-default)] bg-[var(--surface-panel)] p-4 transition-colors"
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
          {config.loading ? i18n.t('common.loading') : configUnavailable ? i18n.t('dashboard.unavailable') : config.providers.map((provider) => provider.name).join(', ') || i18n.t('empty.providersShort')}
        </p>
      </a>
      <a
        href="/models"
        class="metric rounded-[3px] border border-[var(--border-default)] bg-[var(--surface-panel)] p-4 transition-colors"
      >
        <div class="metric-label">
          <h2>{i18n.t('dashboard.models')}</h2>
          <Boxes size={14} aria-hidden="true" />
        </div>
        <div class="metric-value">{config.loading || configUnavailable ? '—' : number(modelCount)}</div>
        <p>{i18n.t('dashboard.configured')}</p>
      </a>
      <div
        class="metric rounded-[3px] border border-[var(--border-default)] bg-[var(--surface-panel)] p-4 transition-colors"
        aria-busy={loading}
      >
        <div class="metric-label">
          <h2>{i18n.t('dashboard.tokenUsage')}</h2>
          <Activity size={14} aria-hidden="true" />
        </div>
        <dl class="space-y-1" aria-live="polite">
          {#each ['input', 'output'] as kind}
            <div class="flex flex-wrap items-baseline justify-between gap-x-3">
              <dt class="text-[10px] text-[var(--text-secondary)]">
                {i18n.t(kind === 'input' ? 'dashboard.input' : 'dashboard.output')}
              </dt>
              <dd
                class="m-0 break-all font-[var(--font-heading)] text-lg font-bold leading-tight tabular-nums text-[var(--accent-primary)]"
                title={!loading && !error && dailyUsage ? number(kind === 'input' ? usageCalendar.input : usageCalendar.output) : undefined}
              >
                {!loading && !error && dailyUsage ? formatTokensM(kind === 'input' ? usageCalendar.input : usageCalendar.output) : '—'}
              </dd>
            </div>
          {/each}
        </dl>
        <p>{usagePeriodLabel}{loading ? ` · ${i18n.t('common.loading')}` : ''}</p>
        {#if error}
          <div role="alert" class="mt-2 flex flex-wrap items-center gap-2">
            <span class="text-[10px] text-[var(--status-error)]">{i18n.t('dashboard.tokenUsageUnavailable')}</span>
            <Button
              variant="outline"
              size="icon-sm"
              onclick={refreshActivity}
              aria-label={i18n.t('dashboard.retryTokenUsage')}
              ><RefreshCw size={12} /></Button
            >
          </div>
        {/if}
      </div>
      <a
        href="/groups"
        class="metric rounded-[3px] border border-[var(--border-default)] bg-[var(--surface-panel)] p-4 transition-colors"
      >
        <div class="metric-label">
          <h2>{i18n.t('dashboard.enabledGroups')}</h2>
          <Layers3 size={14} aria-hidden="true" />
        </div>
        <div class="metric-value">{config.loading || configUnavailable ? '—' : number(enabledGroups)}</div>
        <p>{i18n.t('dashboard.acrossRuntimes')}</p>
      </a>
    </div>
  </section>
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
<SwitchGroupDialog bind:runtime trigger={switchTrigger} />

<style>
.dashboard::before {
  content: "";
  position: absolute;
  inset: -24px;
  z-index: -1;
  pointer-events: none;
  background-image:
    linear-gradient(to right, color-mix(in srgb, var(--border-default) 13%, transparent) 1px, transparent 1px),
    linear-gradient(to bottom, color-mix(in srgb, var(--border-default) 13%, transparent) 1px, transparent 1px);
  background-size: 32px 32px;
}
.metric {
  min-width: 0;
}
.metric:is(:hover, :focus-visible) {
  border-color: var(--accent-primary);
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
  margin-bottom: 8px;
  color: var(--text-secondary);
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
  letter-spacing: 0.06em;
}
.metric-value {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 8px;
  font-family: var(--font-heading);
  font-size: 24px;
  font-weight: 700;
  line-height: 1.25;
}
.metric-value span {
  font-family: var(--font-primary);
  font-size: 12px;
  font-weight: 400;
  color: var(--text-secondary);
}
.metric p {
  margin-top: 8px;
  color: var(--text-secondary);
  font-size: 11px;
  line-height: 1.5;
}
</style>
