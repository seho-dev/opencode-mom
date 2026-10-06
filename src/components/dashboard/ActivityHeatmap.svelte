<script lang="ts">
import { CalendarDays, RefreshCw } from '@lucide/svelte';
import { Button } from '$src/components/button/index.js';
import Select from '$src/components/Select.svelte';
import { getI18n } from '$src/i18n/context.js';
import { type ActivityPeriod, formatTokensM, type periodActivity } from '$src/utils/dashboard.js';

let {
  available,
  loading,
  error,
  years,
  calendar,
  period,
  periodLabel,
  year,
  onperiod,
  onyear,
  onrefresh,
}: {
  available: boolean;
  loading: boolean;
  error: string | null;
  years: number[];
  calendar: ReturnType<typeof periodActivity>;
  period: ActivityPeriod;
  periodLabel: string;
  year: number;
  onperiod: (value: string) => void;
  onyear: (value: string) => void;
  onrefresh: () => Promise<void>;
} = $props();
const i18n = getI18n();
let inspected = $state<number | null>(null);
let focused = $state<number | null>(null);
let grid = $state<HTMLFieldSetElement | null>(null);
const shortRange = $derived(period === 'day' || period === 'week');
const firstDay = $derived(calendar.cells.findIndex((cell) => cell.count !== null));
const lastDay = $derived(calendar.cells.findLastIndex((cell) => cell.count !== null));
const inspectedCell = $derived(inspected === null ? null : calendar.cells[inspected]);
const locale = $derived(i18n.locale === 'zh' ? 'zh-CN' : 'en-US');
const number = (value: number) => value.toLocaleString(locale);
const dateLabel = (
  date: string,
  options: Intl.DateTimeFormatOptions = { month: 'short', day: 'numeric', year: 'numeric' },
) =>
  new Intl.DateTimeFormat(locale, {
    timeZone: 'UTC',
    ...options,
  }).format(new Date(`${date}T00:00:00Z`));
const rangeLabel = $derived(
  period === 'year'
    ? String(year)
    : calendar.from === calendar.to
      ? dateLabel(calendar.from)
      : `${dateLabel(calendar.from)} – ${dateLabel(calendar.to)}`,
);
$effect(() => {
  focused = calendar.cells.findIndex((cell) => cell.count !== null);
  inspected = null;
});
function navigate(event: KeyboardEvent, index: number) {
  let next = index;
  if (event.key === 'ArrowRight') next += period === 'year' ? 7 : 1;
  else if (event.key === 'ArrowLeft') next -= period === 'year' ? 7 : 1;
  else if (event.key === 'ArrowDown') next += period === 'month' ? 7 : 1;
  else if (event.key === 'ArrowUp') next -= period === 'month' ? 7 : 1;
  else if (event.key === 'Home') next = firstDay;
  else if (event.key === 'End') next = lastDay;
  else return;
  event.preventDefault();
  next = Math.max(firstDay, Math.min(lastDay, next));
  focused = next;
  grid?.querySelector<HTMLButtonElement>(`[data-index="${next}"]`)?.focus();
}
</script>

<section
  class="heatmap rounded-[3px] border border-[var(--border-default)] bg-[var(--surface-panel)] p-5"
  aria-labelledby="activity-title"
  aria-busy={loading}
>
  <header
    class="mb-4 flex flex-wrap items-center justify-between gap-4 border-b border-[var(--border-default)]/40 pb-4"
  >
    <div>
      <div class="flex items-center gap-2">
        <CalendarDays size={16} class="shrink-0 text-[var(--accent-primary)]" aria-hidden="true" />
        <h2 id="activity-title" class="font-[var(--font-heading)] text-xs font-bold uppercase tracking-wider">
          {i18n.t('dashboard.activityTitle', { period: periodLabel })}
        </h2>
      </div>
      <p class="mt-0.5 text-xs text-[var(--text-secondary)]">{i18n.t('dashboard.activityDescription')}· {rangeLabel}</p>
    </div>
    <div class="heatmap-controls flex flex-wrap items-center gap-2">
      <span
        class="rounded-[2px] border border-[var(--accent-solid)] bg-[var(--accent-solid)] px-2.5 py-1 font-[var(--font-heading)] text-[10px] font-bold uppercase tracking-wide text-[var(--text-on-accent)]"
        >{i18n.t('dashboard.allRuntimes')}</span
      >
      <label for="heatmap-period" class="text-[10px] text-[var(--text-secondary)]"
        >{i18n.t('dashboard.heatmapPeriod')}</label
      >
      <Select
        id="heatmap-period"
        class="w-[100px]"
        value={period}
        options={[
          { value: 'day', label: i18n.t('dashboard.day') },
          { value: 'week', label: i18n.t('dashboard.week') },
          { value: 'month', label: i18n.t('dashboard.month') },
          { value: 'year', label: i18n.t('dashboard.year') },
        ]}
        onchange={onperiod}
        disabled={loading || !available || !!error}
        ariaLabel={i18n.t('dashboard.heatmapPeriod')}
      />
      {#if period === 'year'}
        <div
          class="year-picker flex h-7 items-center gap-2 rounded-[2px] border border-[var(--border-default)] bg-[var(--surface-input)] pl-2.5 text-[10px] text-[var(--text-secondary)]"
        >
          <label for="activity-year">{i18n.t('dashboard.year')}</label>
          <Select
            id="activity-year"
            class="w-[68px]"
            value={String(year)}
            options={years.map((value) => ({ value: String(value) }))}
            onchange={onyear}
            disabled={loading || !available || !!error}
            ariaLabel={i18n.t('dashboard.year')}
          />
        </div>
      {/if}
      <Button
        variant="secondary"
        size="icon-sm"
        disabled={loading}
        onclick={onrefresh}
        aria-label={i18n.t('dashboard.refreshActivity')}
        title={i18n.t('dashboard.refreshActivity')}
        ><RefreshCw size={13} class={loading ? 'animate-spin motion-reduce:animate-none' : ''} /></Button
      >
    </div>
  </header>
  {#if loading}
    <div class="flex min-h-52 items-center justify-center gap-2 text-xs text-[var(--text-secondary)]" role="status">
      <RefreshCw size={14} class="animate-spin motion-reduce:animate-none" />{i18n.t('dashboard.loadingActivity')}
    </div>
  {:else if error}
    <div class="flex min-h-52 flex-col items-center justify-center gap-3 text-center" role="alert">
      <p class="text-xs text-[var(--status-error)]">{i18n.t('dashboard.activityFailed')}</p>
      <p class="max-w-xl break-words text-[11px] text-[var(--text-secondary)]">{error}</p>
      <Button variant="outline" size="sm" onclick={onrefresh}
        ><RefreshCw size={12} />{i18n.t('dashboard.retry')}</Button
      >
    </div>
  {:else if available}
    <div
      class="mb-4 flex flex-wrap items-center justify-between gap-x-6 gap-y-2 rounded-[2px] border border-[var(--border-default)]/40 bg-[var(--surface-input)] px-3 py-2 text-[10px]"
    >
      <p aria-live="polite" class="flex flex-wrap items-center gap-2 text-[var(--text-secondary)]">
        <span class="font-[var(--font-heading)] font-bold uppercase tracking-wider"
          >{i18n.t('dashboard.cellInspector')}:</span
        >
        <span class="text-[var(--accent-primary)]"
          >{inspectedCell?.date && inspectedCell.count !== null ? i18n.t('dashboard.dayCount', { date: dateLabel(inspectedCell.date), count: formatTokensM(inspectedCell.count) }) : i18n.t(calendar.total ? 'dashboard.inspectDay' : 'dashboard.noActivity', { range: rangeLabel })}</span
        >
        {#if inspectedCell?.date && inspectedCell.input !== null && inspectedCell.output !== null}
          <span title={number(inspectedCell.input)}
            >{i18n.t('dashboard.input')} {formatTokensM(inspectedCell.input)}</span
          >
          <span title={number(inspectedCell.output)}
            >{i18n.t('dashboard.output')} {formatTokensM(inspectedCell.output)}</span
          >
        {/if}
      </p>
      <div class="flex flex-wrap items-center gap-x-4 gap-y-2">
        <span class="text-[var(--text-secondary)]"
          >{period === 'year' ? i18n.t('dashboard.yearTotal', { year }) : i18n.t('dashboard.periodTotal', { period: periodLabel })}
          <strong class="ml-2 text-[var(--accent-primary)]" title={number(calendar.total)}
            >{formatTokensM(calendar.total)}</strong
          ></span
        >
        <span class="text-[var(--text-secondary)]"
          >{i18n.t('dashboard.activeDays')}
          <strong class="ml-2 text-[var(--text-primary)]">{number(calendar.activeDays)}</strong></span
        >
        <span class="text-[var(--text-secondary)]"
          >{i18n.t('dashboard.peakDay')}
          <strong
            class="ml-2 text-[var(--text-primary)]"
            title={calendar.peak ? number(calendar.peak.count) : undefined}
            >{calendar.peak ? `${dateLabel(calendar.peak.date)} · ${formatTokensM(calendar.peak.count)}` : '—'}</strong
          ></span
        >
      </div>
    </div>
    <section class="overflow-x-auto pb-2" aria-label={i18n.t('dashboard.heatmapLabel', { range: rangeLabel })}>
      <div
        class="calendar-canvas"
        class:annual={period === 'year'}
        class:monthly={period === 'month'}
        class:short-range={shortRange}
        style={`--columns: ${calendar.columns}`}
      >
        {#if period === 'year'}
          <div
            class="month-labels mb-2 grid gap-[3px] font-[var(--font-heading)] text-[10px] uppercase tracking-wide text-[var(--text-secondary)]"
            aria-hidden="true"
          >
            {#each calendar.months as month}
              <span style={`grid-column: ${month.column + 1} / span 3`}
                >{dateLabel(month.date, { month: 'short' })}</span
              >
            {/each}
          </div>
        {:else if period === 'month'}
          <div
            class="weekday-labels mb-1 grid gap-1 text-center text-[9px] text-[var(--text-secondary)]"
            aria-hidden="true"
          >
            {#each [7, 8, 9, 10, 11, 12, 13] as day}
              <span>{dateLabel(`2024-01-${String(day).padStart(2, '0')}`, { weekday: 'short' })}</span>
            {/each}
          </div>
        {/if}
        <fieldset
          bind:this={grid}
          class="day-grid m-0 grid gap-[3px] border-0"
          aria-label={i18n.t('dashboard.heatmapLabel', { range: rangeLabel })}
        >
          {#each calendar.cells as cell, index (`${period}-${calendar.from}-${index}`)}
            {#if !shortRange || cell.date}
              {#if cell.date && cell.count !== null}
                {@const label = i18n.t('dashboard.dayCount', { date: dateLabel(cell.date), count: formatTokensM(cell.count) })}
                <button
                  type="button"
                  class="day-cell"
                  data-level={shortRange ? undefined : cell.level}
                  data-index={index}
                  tabindex={(focused ?? firstDay) === index ? 0 : -1}
                  aria-label={label}
                  title={i18n.t('dashboard.dayCount', { date: dateLabel(cell.date), count: number(cell.count) })}
                  onpointerenter={() => (inspected = index)}
                  onfocus={() => { inspected = index; focused = index; }}
                  onkeydown={(event) => navigate(event, index)}
                >
                  {#if shortRange}
                    <span class="text-[9px] text-[var(--text-secondary)]"
                      >{dateLabel(cell.date, { weekday: 'short' })}</span
                    >
                    <span class="text-[10px]">{dateLabel(cell.date, { month: 'short', day: 'numeric' })}</span>
                    <span class="flex items-center justify-center gap-1.5 text-[11px] font-semibold">
                      <span class="legend-cell shrink-0" data-level={cell.level} aria-hidden="true"></span
                      ><span class="min-w-0 break-all">{formatTokensM(cell.count)}</span>
                    </span>
                  {:else if period === 'month'}
                    {Number(cell.date.slice(-2))}
                  {/if}
                </button>
              {:else}
                <span
                  class="day-cell"
                  class:future={!!cell.date}
                  aria-hidden="true"
                  title={cell.date ? i18n.t('dashboard.futureDate', { date: dateLabel(cell.date) }) : undefined}
                >
                  {#if cell.date && shortRange}
                    <span class="text-[9px] text-[var(--text-secondary)]"
                      >{dateLabel(cell.date, { weekday: 'short' })}</span
                    >
                    <span class="text-[10px]">{dateLabel(cell.date, { month: 'short', day: 'numeric' })}</span>
                    <span class="text-[11px]">—</span>
                  {:else if cell.date && period === 'month'}
                    {Number(cell.date.slice(-2))}
                  {/if}
                </span>
              {/if}
            {/if}
          {/each}
        </fieldset>
      </div>
    </section>
    <div class="mt-2 flex min-h-7 items-center justify-end text-[10px]">
      <div class="flex items-center gap-2 text-[var(--text-secondary)]">
        <span>{i18n.t('dashboard.less')}</span>
        <div class="flex gap-1" aria-hidden="true">
          {#each [0, 1, 2, 3, 4] as level}
            <span
              class="legend-cell"
              data-level={level}
              title={['0.00M', '≤0.10M', '≤1.00M', '≤10.00M', '>10.00M'][level]}
            ></span>
          {/each}
        </div>
        <span>{i18n.t('dashboard.more')}</span>
      </div>
    </div>
  {/if}
</section>

<style>
.heatmap {
  --heat-0: var(--surface-input);
  --heat-1: color-mix(in srgb, var(--accent-primary) 28%, var(--surface-input));
  --heat-2: color-mix(in srgb, var(--accent-primary) 54%, var(--surface-input));
  --heat-3: color-mix(in srgb, var(--accent-primary) 77%, var(--surface-input));
  --heat-4: var(--accent-primary);
}
.heatmap-controls :global(.select-trigger) {
  min-height: 28px;
  padding: 4px 8px;
  font-size: 11px;
}
.heatmap-controls .year-picker :global(.select-trigger) {
  min-height: 26px;
  height: 26px;
  border: 0;
  background: transparent;
  padding: 0 6px;
  font-size: 11px;
}
.calendar-canvas.annual {
  min-width: 760px;
}
.calendar-canvas.monthly {
  width: min(100%, 308px);
}
.month-labels,
.day-grid {
  grid-template-columns: repeat(var(--columns), minmax(0, 1fr));
}
.day-grid {
  grid-auto-flow: column;
  grid-template-rows: repeat(7, 11px);
  padding: 3px 0;
}
.weekday-labels,
.monthly .day-grid {
  grid-template-columns: repeat(7, minmax(0, 1fr));
}
.monthly .day-grid {
  grid-auto-flow: row;
  grid-template-rows: none;
  grid-auto-rows: 32px;
  gap: 4px;
  font-size: 10px;
}
.short-range .day-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  padding: 3px;
}
.day-cell,
.legend-cell {
  border-radius: 1px;
}
.day-cell {
  min-width: 0;
  padding: 0;
  border: 0;
  background: transparent;
}
button.day-cell {
  transition:
    transform 100ms,
    box-shadow 100ms;
}
button.day-cell:hover,
button.day-cell:focus-visible {
  transform: scale(1.25);
  outline: 1px solid var(--text-primary);
  outline-offset: 1px;
  position: relative;
  z-index: 1;
}
.day-cell.future {
  border: 1px dashed var(--border-default);
  opacity: 0.28;
}
.legend-cell {
  display: block;
  width: 10px;
  height: 10px;
}
[data-level="0"] {
  background: var(--heat-0);
  border: 1px solid var(--border-default);
}
[data-level="1"] {
  background: var(--heat-1);
}
[data-level="2"] {
  background: var(--heat-2);
}
[data-level="3"] {
  background: var(--heat-3);
}
[data-level="4"] {
  background: var(--heat-4);
  box-shadow: 0 0 6px color-mix(in srgb, var(--accent-primary) 60%, transparent);
}
.monthly .day-cell {
  display: grid;
  place-items: center;
}
.monthly [data-level="4"] {
  color: var(--text-on-accent);
}
.short-range .day-cell {
  display: flex;
  width: 68px;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 8px 6px;
  border: 1px solid var(--border-default);
  background: var(--surface-input);
}
.short-range .day-cell.future {
  border-style: dashed;
  opacity: 0.45;
}
.short-range button.day-cell:hover,
.short-range button.day-cell:focus-visible,
.monthly button.day-cell:hover,
.monthly button.day-cell:focus-visible {
  transform: scale(1.08);
}
@media (prefers-reduced-motion: reduce) {
  button.day-cell {
    transition: none;
  }
}
</style>
