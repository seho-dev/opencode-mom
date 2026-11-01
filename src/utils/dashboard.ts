import type { Group } from '$src/types/groups.js';
import type { DailyTokenUsage, TokenUsageRecord } from '$src/types/stats.js';

export function formatTokensM(value: number): string {
  return value > 0 && value < 10_000 ? '<0.01M' : `${(value / 1_000_000).toFixed(2)}M`;
}

export function groupBindings(group: Group) {
  return [
    ...(group.type === 'native' ? group.openCodeAgentOverrides : []).map((binding) => ({
      ...binding,
      kind: 'native',
      name: binding.agentName,
    })),
    ...(group.type === 'slim' ? (group.slimAgentOverrides ?? []) : []).map((binding) => ({
      ...binding,
      kind: 'slim',
      name: binding.agentName,
    })),
    ...(group.type === 'omo' ? (group.omoAgentOverrides ?? []) : []).map((binding) => ({
      ...binding,
      kind: 'omo',
      name: binding.agentName,
    })),
    ...(group.type === 'omo' ? (group.omoCategoryMappings ?? []) : []).map((binding) => ({
      ...binding,
      kind: 'category',
      name: binding.categoryName,
    })),
  ].filter((binding) => binding.name.trim() && binding.modelRef.trim());
}

export function isCalendarDate(value: string): boolean {
  if (!/^[1-9]\d{3}-\d{2}-\d{2}$/.test(value)) return false;
  const date = new Date(`${value}T00:00:00Z`);
  return Number.isFinite(date.getTime()) && date.toISOString().slice(0, 10) === value;
}

export function calendarDate(now: Date, timezone: string): string {
  const parts = new Intl.DateTimeFormat('en-US', {
    timeZone: timezone,
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
  }).formatToParts(now);
  return ['year', 'month', 'day'].map((type) => parts.find((part) => part.type === type)?.value).join('-');
}

function tokenCount(input: number, output: number): number {
  const count = input + output;
  if (
    !Number.isSafeInteger(input) ||
    input < 0 ||
    !Number.isSafeInteger(output) ||
    output < 0 ||
    !Number.isSafeInteger(count)
  ) {
    throw new RangeError('Token counts and their sums must be nonnegative safe integers.');
  }
  return count;
}

export function dailyTokenUsage(records: TokenUsageRecord[], timezone: string): DailyTokenUsage[] {
  const formatter = new Intl.DateTimeFormat('en-US', {
    timeZone: timezone,
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
  });
  const days = new Map<string, DailyTokenUsage>();
  for (const record of records) {
    if (!Number.isSafeInteger(record.time) || record.time < 0 || record.time > 8_640_000_000_000_000) {
      throw new RangeError('Token usage timestamps must be valid nonnegative integer milliseconds.');
    }
    tokenCount(record.input, record.output);
    const parts = formatter.formatToParts(new Date(record.time));
    const date = ['year', 'month', 'day'].map((type) => parts.find((part) => part.type === type)?.value).join('-');
    const previous = days.get(date);
    const input = (previous?.input ?? 0) + record.input;
    const output = (previous?.output ?? 0) + record.output;
    tokenCount(input, output);
    days.set(date, { date, input, output });
  }
  return [...days.values()].sort((a, b) => a.date.length - b.date.length || a.date.localeCompare(b.date));
}

export function activityLevel(count: number): number {
  return count === 0 ? 0 : count <= 100_000 ? 1 : count <= 1_000_000 ? 2 : count <= 10_000_000 ? 3 : 4;
}

export function activityYears(activity: Pick<DailyTokenUsage, 'date'>[], today: string): number[] {
  return [
    ...new Set([
      Number(today.slice(0, 4)),
      ...activity
        .filter((day) => isCalendarDate(day.date) && day.date <= today)
        .map((day) => Number(day.date.slice(0, 4))),
    ]),
  ].sort((a, b) => b - a);
}

export type ActivityPeriod = 'day' | 'week' | 'month' | 'year';

export function periodActivity(activity: DailyTokenUsage[], period: ActivityPeriod, today: string, year?: number) {
  year = period === 'year' ? (year ?? Number(today.slice(0, 4))) : Number(today.slice(0, 4));
  if (!Number.isInteger(year) || year < 1000 || year > 9999 || !isCalendarDate(today)) {
    throw new RangeError('A valid calendar year and current date are required.');
  }
  const current = new Date(`${today}T00:00:00Z`);
  let first = current.getTime();
  let last = first;
  if (period === 'week') {
    first -= ((current.getUTCDay() + 6) % 7) * 86_400_000;
    last = first + 6 * 86_400_000;
  } else if (period === 'month') {
    first = Date.UTC(year, current.getUTCMonth(), 1);
    last = Date.UTC(year, current.getUTCMonth() + 1, 1) - 86_400_000;
  } else if (period === 'year') {
    first = Date.UTC(year, 0, 1);
    last = Date.UTC(year + 1, 0, 1) - 86_400_000;
  }
  const from = new Date(first).toISOString().slice(0, 10);
  const to = new Date(last).toISOString().slice(0, 10);
  const counts = new Map<string, { input: number; output: number }>();
  for (const day of activity) {
    if (
      !isCalendarDate(day.date) ||
      day.date < from ||
      day.date > to ||
      day.date > today ||
      !Number.isSafeInteger(day.input) ||
      day.input < 0 ||
      !Number.isSafeInteger(day.output) ||
      day.output < 0
    )
      continue;
    const previous = counts.get(day.date);
    const input = (previous?.input ?? 0) + day.input;
    const output = (previous?.output ?? 0) + day.output;
    tokenCount(input, output);
    counts.set(day.date, { input, output });
  }
  const offset = new Date(first).getUTCDay();
  const days = (last - first) / 86_400_000 + 1;
  const columns = Math.ceil((offset + days) / 7);
  const cells: {
    date: string | null;
    count: number | null;
    input: number | null;
    output: number | null;
    level: number;
  }[] = [];
  let total = 0;
  let input = 0;
  let output = 0;
  let activeDays = 0;
  let peak: { date: string; count: number; input: number; output: number } | null = null;
  for (let index = 0; index < columns * 7; index++) {
    const day = index - offset;
    const date = day < 0 || day >= days ? null : new Date(first + day * 86_400_000).toISOString().slice(0, 10);
    const usage = date === null || date > today ? null : (counts.get(date) ?? { input: 0, output: 0 });
    const count = usage === null ? null : tokenCount(usage.input, usage.output);
    if (date !== null && usage !== null && count !== null) {
      input += usage.input;
      output += usage.output;
      total = tokenCount(input, output);
      if (count > 0) activeDays++;
      if (count > 0 && (!peak || count > peak.count)) peak = { date, count, ...usage };
    }
    cells.push({
      date,
      count,
      input: usage?.input ?? null,
      output: usage?.output ?? null,
      level: count === null ? 0 : activityLevel(count),
    });
  }
  const months: { date: string; column: number }[] = [];
  const month = new Date(first);
  month.setUTCDate(1);
  while (month.getTime() <= last) {
    months.push({
      date: month.toISOString().slice(0, 10),
      column: Math.max(0, Math.floor((offset + (month.getTime() - first) / 86_400_000) / 7)),
    });
    month.setUTCMonth(month.getUTCMonth() + 1);
  }
  return { cells, columns, months, total, input, output, activeDays, peak, from, to };
}
