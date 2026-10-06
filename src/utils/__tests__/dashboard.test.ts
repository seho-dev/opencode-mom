import { expect, test } from 'vitest';
import type { Group } from '$src/types/groups.js';
import type { TokenUsageRecord } from '$src/types/stats.js';
import {
  activityLevel,
  activityYears,
  calendarDate,
  dailyTokenUsage,
  formatTokensM,
  groupBindings,
  isCalendarDate,
  periodActivity,
} from '../dashboard.js';

test('tokens use fixed millions and preserve nonzero values below the display precision', () => {
  for (const [value, formatted] of [
    [0, '0.00M'],
    [1, '<0.01M'],
    [9_999, '<0.01M'],
    [10_000, '0.01M'],
    [1_000_000, '1.00M'],
    [1_250_000, '1.25M'],
    [12_340_000, '12.34M'],
    [1_000_000_000, '1000.00M'],
  ] as const) {
    expect(formatTokensM(value)).toBe(formatted);
  }
});

test('daily token usage groups call-start timestamps across timezones and DST and preserves zero days', () => {
  const midnight = [
    { time: Date.parse('2024-01-02T00:00:00Z'), input: 3, output: 2 },
    { time: Date.parse('2024-01-01T15:59:59.999Z'), input: 10, output: 1 },
    { time: Date.parse('2024-01-01T16:00:00Z'), input: 20, output: 2 },
    { time: Date.parse('2024-01-01T16:00:00Z'), input: 4, output: 5 },
    { time: Date.parse('2024-01-03T00:00:00Z'), input: 0, output: 0 },
  ];
  for (const [timezone, records, expected] of [
    [
      'UTC',
      midnight,
      [
        { date: '2024-01-01', input: 34, output: 8 },
        { date: '2024-01-02', input: 3, output: 2 },
        { date: '2024-01-03', input: 0, output: 0 },
      ],
    ],
    [
      'Asia/Shanghai',
      midnight,
      [
        { date: '2024-01-01', input: 10, output: 1 },
        { date: '2024-01-02', input: 27, output: 9 },
        { date: '2024-01-03', input: 0, output: 0 },
      ],
    ],
    [
      'America/Los_Angeles',
      [
        { time: Date.parse('2024-03-10T07:59:59.999Z'), input: 4, output: 1 },
        { time: Date.parse('2024-03-10T08:00:00Z'), input: 10, output: 2 },
        { time: Date.parse('2024-03-10T09:59:59.999Z'), input: 20, output: 3 },
        { time: Date.parse('2024-03-10T10:00:00Z'), input: 30, output: 4 },
        { time: Date.parse('2024-03-11T06:59:59.999Z'), input: 40, output: 5 },
        { time: Date.parse('2024-03-11T07:00:00Z'), input: 50, output: 6 },
        { time: Date.parse('2024-11-03T08:30:00Z'), input: 2, output: 3 },
        { time: Date.parse('2024-11-03T09:30:00Z'), input: 5, output: 7 },
      ],
      [
        { date: '2024-03-09', input: 4, output: 1 },
        { date: '2024-03-10', input: 100, output: 14 },
        { date: '2024-03-11', input: 50, output: 6 },
        { date: '2024-11-03', input: 7, output: 10 },
      ],
    ],
    [
      'UTC',
      [
        { time: 8_640_000_000_000_000, input: 1, output: 0 },
        { time: 0, input: 0, output: 1 },
      ],
      [
        { date: '1970-01-01', input: 0, output: 1 },
        { date: '275760-09-13', input: 1, output: 0 },
      ],
    ],
    ['UTC', [], []],
  ] satisfies [string, TokenUsageRecord[], { date: string; input: number; output: number }[]][]) {
    const original = structuredClone(records);
    expect(dailyTokenUsage(records, timezone)).toEqual(expected);
    expect(records).toEqual(original);
  }
});

test('daily token usage rejects invalid timestamps, counts, and timezones', () => {
  const valid = { time: Date.parse('2024-01-01T00:00:00Z'), input: 1, output: 1 };
  for (const field of ['time', 'input', 'output']) {
    for (const invalid of [-1, 0.5, Number.NaN, Number.POSITIVE_INFINITY, Number.MAX_SAFE_INTEGER + 1]) {
      expect(() => dailyTokenUsage([{ ...valid, [field]: invalid }], 'UTC')).toThrow(RangeError);
    }
  }
  expect(() => dailyTokenUsage([{ ...valid, time: 8_640_000_000_000_001 }], 'UTC')).toThrow(RangeError);
  expect(() => dailyTokenUsage([], 'Invalid/Timezone')).toThrow(RangeError);
});

test('daily and period aggregation reject individual and combined count overflow without losing precision', () => {
  const max = Number.MAX_SAFE_INTEGER;
  for (const counts of [
    [{ input: max, output: 1 }],
    [
      { input: max, output: 0 },
      { input: 1, output: 0 },
    ],
    [
      { input: 0, output: max },
      { input: 0, output: 1 },
    ],
    [
      { input: max, output: 0 },
      { input: 0, output: 1 },
    ],
  ]) {
    expect(() =>
      dailyTokenUsage(
        counts.map((usage) => ({ time: Date.parse('2024-01-01T00:00:00Z'), ...usage })),
        'UTC',
      ),
    ).toThrow(RangeError);
    for (const duplicate of [true, false]) {
      expect(() =>
        periodActivity(
          counts.map((usage, index) => ({ date: duplicate || index === 0 ? '2024-01-01' : '2024-01-02', ...usage })),
          'year',
          '2024-01-02',
        ),
      ).toThrow(RangeError);
    }
  }
  const limit = [{ time: 0, input: max - 1, output: 1 }];
  expect(dailyTokenUsage(limit, 'UTC')).toEqual([{ date: '1970-01-01', input: max - 1, output: 1 }]);
  expect(periodActivity(dailyTokenUsage(limit, 'UTC'), 'day', '1970-01-01').total).toBe(max);
});

test('calendar validates real ISO dates, includes leap days, and pads Sunday-aligned weeks', () => {
  expect(isCalendarDate('2024-02-29')).toBe(true);
  for (const invalid of ['2023-02-29', '2024-02-30', '2024-2-01', 'not-a-date'])
    expect(isCalendarDate(invalid)).toBe(false);
  const leap = periodActivity([], 'year', '2025-01-01', 2024);
  expect(leap.cells.filter((cell) => cell.date)).toHaveLength(366);
  expect(leap.cells[0].date).toBeNull();
  expect(leap.cells[1].date).toBe('2024-01-01');
  expect(leap.cells.find((cell) => cell.date === '2024-02-29')?.count).toBe(0);
  expect(leap.cells.length % 7).toBe(0);
  const sunday = periodActivity([], 'year', '2024-01-01', 2023);
  expect(sunday.cells[0].date).toBe('2023-01-01');
  expect(sunday.columns).toBe(53);
  expect(periodActivity([], 'year', '2029-01-01', 2028).columns).toBe(54);
  expect(leap.months[1]).toEqual({ date: '2024-02-01', column: 4 });
  expect(leap.months).toHaveLength(12);
  expect([leap.from, leap.to]).toEqual(['2024-01-01', '2024-12-31']);
  for (const today of ['invalid', '2024-02-30', '0999-12-31'])
    expect(() => periodActivity([], 'year', today, 2024)).toThrow(RangeError);
  for (const year of [999, 10000, 2024.5, Number.NaN, Number.POSITIVE_INFINITY])
    expect(() => periodActivity([], 'year', '2024-01-01', year)).toThrow(RangeError);
});

test('counts sum duplicate dates, exclude other years and invalid counts, and never turn future dates into zero', () => {
  const activity = [
    { date: '2023-12-31', input: 80, output: 0 },
    { date: '2024-01-01', input: 4, output: 0 },
    { date: '2024-01-01', input: 7, output: 0 },
    { date: '2024-01-02', input: 11, output: 0 },
    { date: '2024-01-03', input: 999, output: 0 },
    { date: '2024-02-30', input: 4, output: 0 },
    ...[-1, Number.NaN, 1.5, Number.MAX_SAFE_INTEGER + 1, Number.POSITIVE_INFINITY].flatMap((invalid) => [
      { date: '2024-01-01', input: invalid, output: 0 },
      { date: '2024-01-01', input: 0, output: invalid },
    ]),
  ];
  const calendar = periodActivity(activity, 'year', '2024-01-02', 2024);
  expect(calendar.total).toBe(22);
  expect(calendar.activeDays).toBe(2);
  expect([calendar.input, calendar.output]).toEqual([22, 0]);
  expect(calendar.peak).toEqual({ date: '2024-01-01', count: 11, input: 11, output: 0 });
  expect(calendar.cells.find((cell) => cell.date === '2024-01-03')?.count).toBeNull();
  expect(calendar.cells.filter((cell) => cell.count !== null)).toHaveLength(2);
  expect(activityYears(activity, '2024-01-02')).toEqual([2024, 2023]);
  expect([0, 1, 100_000, 100_001, 1_000_000, 1_000_001, 10_000_000, 10_000_001].map(activityLevel)).toEqual([
    0, 1, 1, 2, 2, 3, 3, 4,
  ]);
  expect(calendarDate(new Date('2025-01-01T00:30:00Z'), 'America/Los_Angeles')).toBe('2024-12-31');
});

test('token calendars keep input and output totals, zero and null cells, and the first chronological peak', () => {
  const activity = [
    { date: '2024-01-02', input: 0, output: 140_000 },
    { date: '2024-01-01', input: 80_000, output: 20_000 },
    { date: '2024-01-01', input: 10_000, output: 30_000 },
    { date: '2024-01-03', input: 0, output: 0 },
    { date: '2024-01-04', input: 999, output: 999 },
  ];
  for (const period of ['day', 'week', 'month', 'year'] as const) {
    const calendar = periodActivity(activity, period, '2024-01-03');
    expect([calendar.input, calendar.output, calendar.total, calendar.activeDays]).toEqual(
      period === 'day' ? [0, 0, 0, 0] : [90_000, 190_000, 280_000, 2],
    );
    expect(calendar.peak).toEqual(
      period === 'day' ? null : { date: '2024-01-01', count: 140_000, input: 90_000, output: 50_000 },
    );
    expect(calendar.cells.find((cell) => cell.date === '2024-01-03')).toEqual({
      date: '2024-01-03',
      count: 0,
      input: 0,
      output: 0,
      level: 0,
    });
    for (const cell of calendar.cells) {
      expect(cell.count).toBe(cell.input === null || cell.output === null ? null : cell.input + cell.output);
      expect(cell.level).toBe(cell.count === null ? 0 : activityLevel(cell.count));
    }
    expect(calendar.cells.reduce((sum, cell) => sum + (cell.input ?? 0), 0)).toBe(calendar.input);
    expect(calendar.cells.reduce((sum, cell) => sum + (cell.output ?? 0), 0)).toBe(calendar.output);
  }
  const month = periodActivity(activity, 'month', '2024-01-03');
  expect(month.cells.find((cell) => cell.date === '2024-01-04')).toEqual({
    date: '2024-01-04',
    count: null,
    input: null,
    output: null,
    level: 0,
  });
  expect(month.cells[0]).toEqual({ date: null, count: null, input: null, output: null, level: 0 });
  expect(activityYears([{ date: '2023-12-31' }, { date: '2025-01-01' }, { date: '2024-02-30' }], '2024-01-03')).toEqual(
    [2024, 2023],
  );
});

test('current periods share cell summaries, ignore unrelated years, and keep full future bounds', () => {
  const today = '2024-03-06';
  const activity = [
    { date: '2023-12-31', input: 512, output: 0 },
    { date: '2024-01-01', input: 8, output: 0 },
    { date: '2024-02-29', input: 4, output: 0 },
    { date: '2024-03-01', input: 7, output: 0 },
    { date: '2024-03-04', input: 2, output: 0 },
    { date: '2024-03-05', input: 1, output: 0 },
    { date: today, input: 2, output: 0 },
    { date: today, input: 3, output: 0 },
    { date: today, input: -1, output: 0 },
    { date: '2024-02-30', input: 300, output: 0 },
    { date: '2024-03-07', input: 999, output: 0 },
  ];
  for (const [period, from, to, total, activeDays, peak] of [
    ['day', today, today, 5, 1, { date: today, count: 5 }],
    ['week', '2024-03-04', '2024-03-10', 8, 3, { date: today, count: 5 }],
    ['month', '2024-03-01', '2024-03-31', 15, 4, { date: '2024-03-01', count: 7 }],
    ['year', '2024-01-01', '2024-12-31', 27, 6, { date: '2024-01-01', count: 8 }],
  ] as const) {
    const calendar = periodActivity(activity, period, today);
    expect([calendar.from, calendar.to, calendar.total, calendar.activeDays, calendar.peak]).toEqual([
      from,
      to,
      total,
      activeDays,
      { ...peak, input: peak.count, output: 0 },
    ]);
    expect(calendar.cells.reduce((sum, cell) => sum + (cell.count ?? 0), 0)).toBe(total);
    expect([calendar.input, calendar.output]).toEqual([total, 0]);
    expect(calendar.cells.filter((cell) => (cell.count ?? 0) > 0)).toHaveLength(activeDays);
    expect(calendar.cells.find((cell) => cell.date === today)?.count).toBe(5);
    expect(
      calendar.cells
        .filter((cell) => cell.date !== null && cell.date > today)
        .every((cell) => cell.count === null && cell.input === null && cell.output === null),
    ).toBe(true);
    expect(
      calendar.cells
        .filter((cell) => cell.date === null)
        .every((cell) => cell.count === null && cell.input === null && cell.output === null),
    ).toBe(true);
    if (period !== 'year') {
      expect(periodActivity(activity, period, today, 999)).toEqual(calendar);
      expect(calendar.months).toEqual([{ date: '2024-03-01', column: 0 }]);
    }
  }
  const history = periodActivity(activity, 'year', today, 2023);
  expect([history.from, history.to, history.total]).toEqual(['2023-01-01', '2023-12-31', 512]);
  expect(history.cells.every((cell) => cell.date === null || cell.count !== null)).toBe(true);
  expect(periodActivity(activity, 'month', today).cells.find((cell) => cell.date === '2024-03-02')?.count).toBe(0);
});

test('Monday weeks, leap months, and empty ranges retain calendar boundaries and padding', () => {
  for (const [today, from, to] of [
    ['2024-02-26', '2024-02-26', '2024-03-03'],
    ['2024-03-03', '2024-02-26', '2024-03-03'],
    ['2025-01-01', '2024-12-30', '2025-01-05'],
    ['2024-03-31', '2024-03-25', '2024-03-31'],
  ]) {
    const calendar = periodActivity([], 'week', today);
    expect([calendar.from, calendar.to]).toEqual([from, to]);
    expect(calendar.cells.filter((cell) => cell.date)).toHaveLength(7);
    expect(calendar.columns).toBe(2);
    expect(calendar.cells[0].date).toBeNull();
    expect(calendar.cells[1].date).toBe(from);
    expect(calendar.cells[7].date).toBe(to);
    expect(calendar.cells.slice(8).every((cell) => cell.date === null)).toBe(true);
  }
  expect(periodActivity([], 'week', '2025-01-01').months).toEqual([
    { date: '2024-12-01', column: 0 },
    { date: '2025-01-01', column: 0 },
  ]);
  expect(periodActivity([], 'week', '2024-03-31').months).toEqual([{ date: '2024-03-01', column: 0 }]);
  const sundayMonth = periodActivity([], 'week', '2024-09-01');
  expect(sundayMonth.months).toEqual([
    { date: '2024-08-01', column: 0 },
    { date: '2024-09-01', column: 1 },
  ]);
  for (const today of ['2024-02-01', '2024-02-29']) {
    const calendar = periodActivity([], 'month', today);
    expect([calendar.from, calendar.to]).toEqual(['2024-02-01', '2024-02-29']);
    expect(calendar.cells.filter((cell) => cell.date)).toHaveLength(29);
    expect(calendar.months).toEqual([{ date: '2024-02-01', column: 0 }]);
    expect(calendar.cells.find((cell) => cell.date === '2024-02-29')?.count).toBe(today === '2024-02-29' ? 0 : null);
  }
  for (const period of ['day', 'week', 'month', 'year'] as const) {
    const calendar = periodActivity([], period, '2024-03-06');
    expect([calendar.total, calendar.activeDays, calendar.peak]).toEqual([0, 0, null]);
    expect(calendar.cells.find((cell) => cell.date === calendar.from)?.count).toBe(0);
    expect(calendar.cells.length).toBe(calendar.columns * 7);
  }
});

test('bindings only include mappings projected for the group runtime', () => {
  const group: Group = {
    id: 'test',
    name: 'Test',
    description: '',
    type: 'omo',
    isEnabled: true,
    updatedAt: '',
    openCodeAgentOverrides: [{ agentName: 'build', modelRef: 'example/core' }],
    slimAgentOverrides: [{ agentName: 'oracle', modelRef: 'example/slim' }],
    omoAgentOverrides: [{ agentName: 'oracle', modelRef: 'example/omo', variant: 'fast' }],
    omoCategoryMappings: [{ categoryName: 'quick', modelRef: 'example/quick' }],
  };
  expect(groupBindings(group).map((binding) => [binding.kind, binding.name])).toEqual([
    ['omo', 'oracle'],
    ['category', 'quick'],
  ]);
  expect(groupBindings({ ...group, type: 'slim' }).map((binding) => binding.modelRef)).toEqual(['example/slim']);
  expect(groupBindings({ ...group, type: 'native' }).map((binding) => binding.modelRef)).toEqual(['example/core']);
  expect(
    groupBindings({
      ...group,
      type: 'native',
      openCodeAgentOverrides: [
        { agentName: ' ', modelRef: 'example/core' },
        { agentName: 'build', modelRef: ' ' },
      ],
    }),
  ).toEqual([]);
});
