<script lang="ts">
import { getI18n } from '$src/i18n/context.js';
import type { ActivityPeriod, periodActivity } from '$src/utils/dashboard.js';

type CalendarData = ReturnType<typeof periodActivity>;

let {
  calendar,
  period,
}: {
  calendar: CalendarData | null;
  period: ActivityPeriod;
} = $props();

const i18n = getI18n();

interface Bucket {
  key: string;
  label: string;
  input: number;
  output: number;
  total: number;
}

const buckets = $derived.by<Bucket[]>(() => {
  if (!calendar) return [];
  const validCells = calendar.cells.filter(
    (cell): cell is { date: string; count: number; input: number; output: number } =>
      cell.date !== null && cell.count !== null && cell.input !== null && cell.output !== null,
  );
  if (!validCells.length) return [];

  if (period === 'year') {
    const byMonth = new Map<string, Bucket>();
    for (let m = 1; m <= 12; m++) {
      const mm = String(m).padStart(2, '0');
      byMonth.set(mm, { key: mm, label: mm, input: 0, output: 0, total: 0 });
    }
    for (const cell of validCells) {
      const mm = cell.date.slice(5, 7);
      const bucket = byMonth.get(mm);
      if (bucket) {
        bucket.input += cell.input;
        bucket.output += cell.output;
        bucket.total += cell.count;
      }
    }
    return [...byMonth.values()];
  }

  if (period === 'day') {
    const day = validCells[validCells.length - 1];
    if (!day) return [];
    return [
      {
        key: `${day.date}-in`,
        label: i18n.t('dashboard.inputShort'),
        input: day.input,
        output: 0,
        total: day.input,
      },
      {
        key: `${day.date}-out`,
        label: i18n.t('dashboard.outputShort'),
        input: 0,
        output: day.output,
        total: day.output,
      },
    ];
  }

  return validCells.map((cell) => ({
    key: cell.date,
    label: cell.date.slice(8, 10),
    input: cell.input,
    output: cell.output,
    total: cell.count,
  }));
});

const maxTotal = $derived(buckets.reduce((max, b) => Math.max(max, b.total), 0));
const inputShare = $derived(calendar && calendar.total > 0 ? Math.round((calendar.input / calendar.total) * 100) : 0);
const outputShare = $derived(calendar && calendar.total > 0 ? Math.max(0, 100 - inputShare) : 0);
</script>

<div
  class="mt-3 flex flex-1 flex-col justify-between rounded-[2px] border border-[var(--border-default)]/55 bg-[var(--surface-input)]/75 p-3"
>
  {#if calendar && buckets.length}
    <div class="relative flex flex-1 flex-col justify-end">
      <svg viewBox="0 0 240 88" preserveAspectRatio="none" class="h-24 w-full overflow-visible" aria-hidden="true">
        <line
          x1="0"
          y1="16"
          x2="240"
          y2="16"
          stroke="var(--border-default)"
          stroke-dasharray="2 2"
          stroke-width="0.5"
          opacity="0.5"
        />
        <line
          x1="0"
          y1="48"
          x2="240"
          y2="48"
          stroke="var(--border-default)"
          stroke-dasharray="2 2"
          stroke-width="0.5"
          opacity="0.5"
        />
        <line x1="0" y1="80" x2="240" y2="80" stroke="var(--border-default)" stroke-width="0.75" opacity="0.8" />
        {#each buckets as bucket, i (bucket.key)}
          {@const count = buckets.length}
          {@const slotWidth = 240 / Math.max(count, 1)}
          {@const barWidth = Math.max(3, Math.min(28, slotWidth * 0.66))}
          {@const x = i * slotWidth + (slotWidth - barWidth) / 2}
          {@const totalH = maxTotal > 0 ? Math.max(bucket.total > 0 ? 3 : 1, (bucket.total / maxTotal) * 68) : 1}
          {@const outH = bucket.total > 0 ? (bucket.output / bucket.total) * totalH : 0}
          {@const inH = Math.max(0, totalH - outH)}
          <rect
            {x}
            y={80 - totalH}
            width={barWidth}
            height={inH}
            rx="1"
            fill="var(--accent-primary)"
            opacity={bucket.total > 0 ? 0.88 : 0.2}
          />
          {#if outH > 0}
            <rect {x} y={80 - outH} width={barWidth} height={outH} rx="1" fill="var(--status-success)" opacity="0.95" />
          {/if}
        {/each}
      </svg>
      <div class="mt-1 flex items-center justify-between text-[9px] font-mono text-[var(--text-muted)]">
        <span>{buckets[0]?.label ?? ''}</span>
        {#if buckets.length > 2}
          <span>{buckets[Math.floor(buckets.length / 2)]?.label ?? ''}</span>
        {/if}
        <span>{buckets[buckets.length - 1]?.label ?? ''}</span>
      </div>
    </div>

    <div class="mt-2.5 space-y-1.5 border-t border-[var(--border-default)]/40 pt-2">
      <div class="flex h-1.5 w-full overflow-hidden rounded-[1px] bg-[var(--surface-panel)]">
        <div class="h-full bg-[var(--accent-primary)] transition-all" style={`width: ${inputShare}%`}></div>
        <div class="h-full bg-[var(--status-success)] transition-all" style={`width: ${outputShare}%`}></div>
      </div>
      <div class="flex items-center justify-between text-[10px] font-mono tabular-nums text-[var(--text-secondary)]">
        <span class="inline-flex items-center gap-1.5">
          <span class="size-1.5 rounded-[1px] bg-[var(--accent-primary)]" aria-hidden="true"></span>
          {i18n.t('dashboard.inputShort')} {inputShare}%
        </span>
        <span class="inline-flex items-center gap-1.5">
          <span class="size-1.5 rounded-[1px] bg-[var(--status-success)]" aria-hidden="true"></span>
          {i18n.t('dashboard.outputShort')} {outputShare}%
        </span>
      </div>
    </div>
  {:else}
    <div class="flex h-28 flex-1 items-center justify-center text-[11px] text-[var(--text-muted)]">—</div>
  {/if}
</div>
