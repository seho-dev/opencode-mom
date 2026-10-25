import { render, screen } from '@testing-library/svelte';
import { expect, test } from 'vitest';
import { en } from '$src/i18n/dictionaries/en.js';
import Harness from '$src/test-utils/harness.svelte';
import { periodActivity } from '$src/utils/dashboard.js';
import TokenUsageChart from '../TokenUsageChart.svelte';

const today = '2024-06-15';

function makeConfig() {
  return { preferences: { locale: 'en', theme: 'light' as const } };
}

test('monthly exact aggregated ratios and SVG heights', () => {
  const calendar = periodActivity([{ date: '2024-06-10', input: 600, output: 400 }], 'month', today);
  expect(calendar.total).toBe(1000);
  expect(calendar.input).toBe(600);
  expect(calendar.output).toBe(400);
  render(TokenUsageChart, { calendar, period: 'month' }, { wrapper: Harness, wrapperProps: { config: makeConfig() } });
  expect(screen.getByText(`${en['dashboard.inputShort']} 60%`)).toBeTruthy();
  expect(screen.getByText(`${en['dashboard.outputShort']} 40%`)).toBeTruthy();
  const rects = Array.from(document.querySelectorAll('svg rect')) as SVGRectElement[];
  const bars = rects.filter((r) => Number(r.getAttribute('height')) > 1);
  expect(bars).toHaveLength(2);
  expect(Number(bars[0]?.getAttribute('height'))).toBeCloseTo(40.8, 2);
  expect(Number(bars[1]?.getAttribute('height'))).toBeCloseTo(27.2, 2);
});

test('annual two january and one march buckets', () => {
  const calendar = periodActivity(
    [
      { date: '2024-01-05', input: 100, output: 50 },
      { date: '2024-01-20', input: 100, output: 50 },
      { date: '2024-03-10', input: 80, output: 20 },
    ],
    'year',
    today,
    2024,
  );
  expect(calendar.cells.some((c) => c.date === '2024-01-05')).toBe(true);
  expect(calendar.cells.some((c) => c.date === '2024-01-20')).toBe(true);
  expect(calendar.cells.some((c) => c.date === '2024-03-10')).toBe(true);
  const janCell = calendar.cells.find((c) => c.date === '2024-01-05');
  expect(janCell?.input).toBeGreaterThanOrEqual(100);
  const marCell = calendar.cells.find((c) => c.date === '2024-03-10');
  expect(marCell?.count ?? 0).toBeGreaterThan(0);
  render(TokenUsageChart, { calendar, period: 'year' }, { wrapper: Harness, wrapperProps: { config: makeConfig() } });
  const heights = Array.from(document.querySelectorAll('svg rect'))
    .map((rect) => Number(rect.getAttribute('height')))
    .filter((height) => height > 1);
  expect(heights).toHaveLength(4);
  for (const [index, expected] of [68 * (2 / 3), 68 / 3, (68 / 3) * 0.8, (68 / 3) * 0.2].entries()) {
    expect(heights[index]).toBeCloseTo(expected, 4);
  }
});

test('null unavailable and zero total finite', () => {
  render(
    TokenUsageChart,
    { calendar: null, period: 'month' },
    { wrapper: Harness, wrapperProps: { config: makeConfig() } },
  );
  expect(screen.getByText('—')).toBeTruthy();
  const zeroCalendar = periodActivity([{ date: '2024-01-01', input: 0, output: 0 }], 'month', today);
  expect(zeroCalendar.total).toBe(0);
  expect(zeroCalendar.cells[0]?.count).toBeNull();
  render(
    TokenUsageChart,
    { calendar: zeroCalendar, period: 'month' },
    { wrapper: Harness, wrapperProps: { config: makeConfig() } },
  );
  const shareEls = screen.getAllByText(/%/);
  shareEls.forEach((el) => {
    expect(el.textContent).not.toContain('NaN');
    expect(el.textContent).not.toContain('Infinity');
  });
});

test('day period splits input and output into two bars', () => {
  const calendar = periodActivity([{ date: '2024-06-15', input: 600, output: 400 }], 'day', today);
  expect(calendar.total).toBe(1000);
  expect(calendar.input).toBe(600);
  expect(calendar.output).toBe(400);
  render(TokenUsageChart, { calendar, period: 'day' }, { wrapper: Harness, wrapperProps: { config: makeConfig() } });
  expect(screen.getByText(`${en['dashboard.inputShort']} 60%`)).toBeTruthy();
  expect(screen.getByText(`${en['dashboard.outputShort']} 40%`)).toBeTruthy();
  const rects = Array.from(document.querySelectorAll('svg rect')) as SVGRectElement[];
  const bars = rects.filter((r) => Number(r.getAttribute('height')) > 1);
  expect(bars).toHaveLength(2);
});

test('week period keeps daily buckets and shares finite', () => {
  const calendar = periodActivity(
    [
      { date: '2024-06-10', input: 100, output: 50 },
      { date: '2024-06-11', input: 200, output: 100 },
    ],
    'week',
    today,
  );
  expect(calendar.input + calendar.output).toBe(450);
  render(TokenUsageChart, { calendar, period: 'week' }, { wrapper: Harness, wrapperProps: { config: makeConfig() } });
  const shareEls = screen.getAllByText(/%/);
  shareEls.forEach((el) => {
    expect(el.textContent).not.toContain('NaN');
    expect(el.textContent).not.toContain('Infinity');
  });
  const rects = Array.from(document.querySelectorAll('svg rect')) as SVGRectElement[];
  const bars = rects.filter((r) => Number(r.getAttribute('height')) > 1);
  expect(bars).toHaveLength(4);
  expect(screen.getByText(`${en['dashboard.inputShort']} 67%`)).toBeTruthy();
  expect(screen.getByText(`${en['dashboard.outputShort']} 33%`)).toBeTruthy();
});
