import { parseDate } from '@internationalized/date';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/svelte';
import { flushSync } from 'svelte';
import { afterAll, afterEach, beforeAll, beforeEach, expect, test, vi } from 'vitest';
import type { ConfigStore } from '$src/config/store.svelte.js';
import { createConfigStore } from '$src/config/store.svelte.js';
import Harness from '$src/test-utils/harness.svelte';
import type { Group } from '$src/types/groups.js';
import type { TokenUsageRecord } from '$src/types/stats.js';
import { formatTokensM } from '$src/utils/dashboard.js';
import Dashboard from '../+page.svelte';

const scrollIntoView = Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'scrollIntoView');
beforeAll(() => {
  Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', { configurable: true, value: vi.fn() });
});
afterAll(() => {
  if (scrollIntoView) Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', scrollIntoView);
  else Reflect.deleteProperty(HTMLElement.prototype, 'scrollIntoView');
});
beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] });
  vi.setSystemTime(new Date(2024, 2, 1, 12));
});
afterEach(() => vi.useRealTimers());

const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC';
const dayBounds = (date: string) => {
  const day = parseDate(date);
  return {
    from: day.toDate(timezone).getTime(),
    to: day.add({ days: 1 }).toDate(timezone).getTime() - 1,
  };
};
const records: TokenUsageRecord[] = [
  { time: dayBounds('2023-12-31').from + 43_200_000, input: 1_234_567_890, output: 6_000_012 },
  { time: dayBounds('2024-01-01').from + 43_200_000, input: 1_000_000, output: 2_000_000 },
  { time: dayBounds('2024-02-26').from + 43_200_000, input: 20_017, output: 5_006 },
  { time: dayBounds('2024-02-29').to, input: 200_003, output: 40_002 },
  { time: dayBounds('2024-03-01').from, input: 100_000, output: 0 },
  { time: dayBounds('2024-03-01').to, input: 23_456, output: 7_890 },
  { time: dayBounds('2024-03-02').from, input: 9_000_000, output: 8_000_000 },
];
const makeGroup = (id: string, type: Group['type'], isEnabled = false): Group => ({
  id,
  name: id,
  description: '',
  type,
  isEnabled,
  updatedAt: '',
  openCodeAgentOverrides: [{ agentName: 'build', modelRef: 'example/core' }],
  slimAgentOverrides: type === 'slim' ? [{ agentName: 'oracle', modelRef: 'example/slim' }] : null,
  omoAgentOverrides: null,
  omoCategoryMappings: null,
});
function mount(overrides: Record<string, unknown> = {}) {
  const config = {
    preferences: { locale: 'en', theme: 'dark' },
    providers: [],
    groups: [],
    selectedGroupId: null,
    loading: false,
    saving: false,
    switching: false,
    reloading: false,
    error: null,
    switchGroup: vi.fn().mockResolvedValue(undefined),
    reloadOpencode: vi.fn().mockResolvedValue(undefined),
    loadTokenUsageRecords: vi.fn().mockResolvedValue(records),
    ...overrides,
  } as unknown as ConfigStore;
  const view = render(Dashboard, {}, { wrapper: Harness, wrapperProps: { config } });
  return { config, ...view };
}
function tokenCard() {
  return screen.getByRole('heading', { name: 'Token usage' }).closest('.metric') as HTMLElement;
}
function expectTokens(input: number | null, output: number | null) {
  const card = within(tokenCard());
  const inputValue = card.getByText('Input').nextElementSibling;
  const outputValue = card.getByText('Output').nextElementSibling;
  expect(inputValue?.textContent).toBe(input === null ? '—' : formatTokensM(input));
  expect(outputValue?.textContent).toBe(output === null ? '—' : formatTokensM(output));
  expect(inputValue?.getAttribute('title')).toBe(input === null ? null : input.toLocaleString('en-US'));
  expect(outputValue?.getAttribute('title')).toBe(output === null ? null : output.toLocaleString('en-US'));
}
function labeledButton(name: string) {
  return screen.getByLabelText(name, { selector: 'button' }) as HTMLButtonElement;
}
async function select(name: string, option: string) {
  await fireEvent.click(labeledButton(name));
  await fireEvent.click(within(screen.getByRole('listbox')).getByRole('option', { name: option }));
}
async function readyDialog(name: string) {
  const dialog = await screen.findByRole('dialog', { name });
  const layers = (
    globalThis as typeof globalThis & {
      bitsDismissableLayers: Map<{ opts: { ref: { current: HTMLElement | null } } }, unknown>;
    }
  ).bitsDismissableLayers;
  await waitFor(() => expect([...layers.keys()].some((layer) => layer.opts.ref.current === dialog)).toBe(true));
  return dialog;
}

test('metric headers use shared icon states while cards retain native navigation semantics', () => {
  const { container } = mount();
  const cards = [...container.querySelectorAll<HTMLElement>('.metric')];
  expect(cards).toHaveLength(4);
  for (const card of cards) {
    const icon = card.querySelector('.metric-label svg');
    expect(icon?.getAttribute('aria-hidden')).toBe('true');
    expect(icon?.getAttribute('class')).not.toContain('text-[var(--accent-primary)]');
  }
  const links = cards.filter((card) => card.tagName === 'A');
  expect(links.map((link) => link.getAttribute('href'))).toEqual(['/providers', '/models', '/groups']);
  for (const link of links) {
    expect(link.tabIndex).toBe(0);
    link.focus();
    expect(document.activeElement).toBe(link);
  }
  const usage = tokenCard();
  expect(usage.tagName).toBe('DIV');
  expect(usage.hasAttribute('role')).toBe(false);
  expect(usage.hasAttribute('tabindex')).toBe(false);
});

test('current selection is global; multiple enabled groups remain selectable and switching does not change eligibility', async () => {
  const groups = [
    makeGroup('Native current', 'native', true),
    makeGroup('Native other', 'native', true),
    makeGroup('Native disabled', 'native'),
    makeGroup('Slim available', 'slim', true),
    makeGroup('OMO group', 'omo', true),
  ];
  let selectedGroupId = 'Native current';
  const adapter = {
    loadAppState: vi.fn(async () => ({
      providers: [],
      agents: [],
      groups,
      selectedGroupId,
      preferences: { locale: 'en', theme: 'dark' },
    })),
    opencodeListModels: vi.fn().mockResolvedValue([]),
    opencodeTokenUsageRecords: vi.fn().mockResolvedValue(records),
    switchGroup: vi.fn(async (id: string) => {
      selectedGroupId = id;
    }),
  };
  const config = createConfigStore(adapter as unknown as Parameters<typeof createConfigStore>[0]);
  await waitFor(() => expect(config.loading).toBe(false));
  const switchGroup = vi.spyOn(config, 'switchGroup');
  render(Dashboard, {}, { wrapper: Harness, wrapperProps: { config } });
  expect(screen.getAllByText('No group selected')).toHaveLength(2);
  expect(screen.getAllByText('Current selection')).toHaveLength(1);
  expect(screen.queryByRole('link', { name: 'Edit Slim available' })).toBeNull();
  expect(screen.getByRole('heading', { name: 'Enabled groups' }).parentElement?.nextElementSibling?.textContent).toBe(
    '4',
  );
  const trigger = labeledButton('Switch Native group');
  await fireEvent.click(trigger);
  const dialog = await readyDialog('Switch group');
  expect(within(dialog).getAllByRole('radio')).toHaveLength(3);
  expect(within(dialog).queryByRole('radio', { name: 'Slim available' })).toBeNull();
  expect((within(dialog).getByRole('radio', { name: 'Native current' }) as HTMLInputElement).checked).toBe(true);
  expect((within(dialog).getByRole('radio', { name: 'Native disabled' }) as HTMLInputElement).disabled).toBe(true);
  expect((within(dialog).getByRole('radio', { name: 'Native other' }) as HTMLInputElement).disabled).toBe(false);
  expect(within(dialog).getAllByText('Current selection')).toHaveLength(1);
  expect((within(dialog).getByRole('button', { name: 'Confirm switch' }) as HTMLButtonElement).disabled).toBe(true);
  await fireEvent.input(within(dialog).getByRole('textbox'), { target: { value: 'other' } });
  expect(within(dialog).getAllByRole('radio')).toHaveLength(1);
  await fireEvent.click(within(dialog).getByRole('radio', { name: 'Native other' }));
  await fireEvent.click(within(dialog).getByRole('button', { name: 'Confirm switch' }));
  await waitFor(() => expect(switchGroup).toHaveBeenCalledExactlyOnceWith('Native other'));
  expect(adapter.switchGroup).toHaveBeenCalledExactlyOnceWith('Native other');
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect(screen.getAllByRole('link', { name: 'Edit Native other' })[0].textContent).toContain('Native other');
  expect(config.selectedGroupId).toBe('Native other');
  expect(config.groups.map((group) => group.isEnabled)).toEqual([true, true, false, true, true]);
  await waitFor(() => expect(document.activeElement).toBe(trigger));

  const slimTrigger = labeledButton('Switch Slim group');
  await fireEvent.click(slimTrigger);
  const slimDialog = await readyDialog('Switch group');
  const slimRadio = within(slimDialog).getByRole('radio', { name: 'Slim available' }) as HTMLInputElement;
  expect(slimRadio.checked).toBe(false);
  expect(within(slimDialog).queryByText('Current selection')).toBeNull();
  expect(within(slimDialog).queryByText('example/core')).toBeNull();
  expect(within(slimDialog).getByText('example/slim')).toBeTruthy();
  await fireEvent.click(slimRadio);
  await fireEvent.click(within(slimDialog).getByRole('button', { name: 'Confirm switch' }));
  const reloadDialog = await readyDialog('Reload opencode now?');
  expect(config.selectedGroupId).toBe('Slim available');
  expect(screen.queryByRole('link', { name: 'Edit Native other' })).toBeNull();
  expect(screen.getAllByText('No group selected')).toHaveLength(2);
  await fireEvent.click(within(reloadDialog).getByRole('button', { name: 'Later' }));
  await waitFor(() => expect(document.activeElement).toBe(slimTrigger));
});

test('pending switch blocks duplicate submission and dismissal', async () => {
  let finish: () => void = () => {};
  const switchGroup = vi.fn(
    () =>
      new Promise<void>((resolve) => {
        finish = resolve;
      }),
  );
  mount({ groups: [makeGroup('Native next', 'native', true), makeGroup('Slim available', 'slim', true)], switchGroup });
  await fireEvent.click(labeledButton('Switch Native group'));
  const dialog = await readyDialog('Switch group');
  await fireEvent.click(within(dialog).getByRole('radio', { name: 'Native next' }));
  const confirm = within(dialog).getByRole('button', { name: 'Confirm switch' });
  confirm.dispatchEvent(new MouseEvent('click', { bubbles: true }));
  confirm.dispatchEvent(new MouseEvent('click', { bubbles: true }));
  await waitFor(() => expect(switchGroup).toHaveBeenCalledExactlyOnceWith('Native next'));
  expect((within(dialog).getByRole('button', { name: 'Switching…' }) as HTMLButtonElement).disabled).toBe(true);
  expect((within(dialog).getByRole('button', { name: 'Cancel' }) as HTMLButtonElement).disabled).toBe(true);
  await fireEvent.keyDown(within(dialog).getByRole('textbox'), { key: 'Escape' });
  expect(screen.getByRole('dialog', { name: 'Switch group' })).toBeTruthy();
  flushSync(() => finish());
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
});

test('failed switch keeps selected group and dialog for retry; plugin success asks before reload', async () => {
  const switchGroup = vi.fn().mockRejectedValueOnce(new Error('write failed')).mockResolvedValueOnce(undefined);
  const reloadOpencode = vi.fn().mockRejectedValueOnce(new Error('reload failed')).mockResolvedValueOnce(undefined);
  mount({
    groups: [
      makeGroup('Slim current', 'slim', true),
      makeGroup('Slim next', 'slim', true),
      makeGroup('Native hidden', 'native'),
    ],
    switchGroup,
    selectedGroupId: 'Slim current',
    reloadOpencode,
  });
  await fireEvent.click(labeledButton('Switch Slim group'));
  let dialog = await readyDialog('Switch group');
  await fireEvent.click(within(dialog).getByRole('radio', { name: 'Slim next' }));
  await fireEvent.click(within(dialog).getByRole('button', { name: 'Confirm switch' }));
  await waitFor(() => expect(within(dialog).getByRole('alert').textContent).toContain('Your selection is kept'));
  expect((within(dialog).getByRole('radio', { name: 'Slim next' }) as HTMLInputElement).checked).toBe(true);
  expect(reloadOpencode).not.toHaveBeenCalled();
  await fireEvent.click(within(dialog).getByRole('button', { name: 'Confirm switch' }));
  dialog = await readyDialog('Reload opencode now?');
  expect(switchGroup).toHaveBeenCalledTimes(2);
  expect(reloadOpencode).not.toHaveBeenCalled();
  await fireEvent.click(within(dialog).getByRole('button', { name: 'Reload now' }));
  await waitFor(() => expect(reloadOpencode).toHaveBeenCalledOnce());
  expect(screen.getByRole('dialog', { name: 'Reload opencode now?' })).toBeTruthy();
  await fireEvent.click(within(dialog).getByRole('button', { name: 'Reload now' }));
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect(reloadOpencode).toHaveBeenCalledTimes(2);
});

test('record loading and failure hide both token views without blocking groups; either retry restores the shared data', async () => {
  let reject: (reason: Error) => void = () => {};
  const loadTokenUsageRecords = vi
    .fn()
    .mockImplementationOnce(() => new Promise<TokenUsageRecord[]>((_, fail) => (reject = fail)))
    .mockRejectedValueOnce(new Error('Still unavailable'))
    .mockResolvedValueOnce(records);
  mount({
    groups: [makeGroup('Native current', 'native', true)],
    selectedGroupId: 'Native current',
    loadTokenUsageRecords,
  });
  await screen.findByText('Loading OpenCode token usage…');
  expectTokens(null, null);
  expect(tokenCard().getAttribute('aria-busy')).toBe('true');
  for (const name of ['Usage period', 'Heatmap period', 'Year', 'Refresh activity']) {
    expect(labeledButton(name).disabled).toBe(true);
  }
  const switchTrigger = labeledButton('Switch Native group');
  expect(switchTrigger.disabled).toBe(false);
  expect(screen.queryByLabelText(/ tokens$/, { selector: 'button' })).toBeNull();
  flushSync(() => reject(new Error('CLI unavailable')));
  await screen.findByText('CLI unavailable');
  expect(screen.getAllByRole('alert')).toHaveLength(2);
  expectTokens(null, null);
  expect(screen.queryByLabelText(/ tokens$/, { selector: 'button' })).toBeNull();
  expect(switchTrigger.disabled).toBe(false);

  await fireEvent.click(labeledButton('Retry token usage'));
  await screen.findByText('Still unavailable');
  expect(loadTokenUsageRecords).toHaveBeenCalledTimes(2);
  expectTokens(null, null);
  await fireEvent.click(screen.getByRole('button', { name: 'Retry' }));
  await waitFor(() => expect(screen.queryByRole('alert')).toBeNull());
  expect(loadTokenUsageRecords.mock.calls).toEqual([[], [], []]);
  expect(labeledButton('Heatmap period').disabled).toBe(false);
  expect(labeledButton('Feb 29, 2024: 0.24M tokens')).toBeTruthy();
});

test('usage periods and historical usage year leave the heatmap and inspector unchanged', async () => {
  const { config } = mount();
  await screen.findByText('2024 total');
  expectTokens(123_456, 7_890);
  expect(within(tokenCard()).getByText('Today')).toBeTruthy();
  expect(labeledButton('Usage period').textContent).toContain('Day');
  expect(labeledButton('Heatmap period').textContent).toContain('Year');
  expect(screen.queryByLabelText('Usage year', { selector: 'button' })).toBeNull();
  expect(screen.getByText('2024 total').textContent).toContain('3.40M');
  expect(screen.getByText('Active days').textContent).toContain('4');
  expect(screen.getByText('Peak day').textContent).toContain('Jan 1, 2024 · 3.00M');
  expect(screen.getByTitle('3,396,374')).toBeTruthy();
  expect(screen.queryByText(/agent.steps/i)).toBeNull();
  for (const title of ['0.00M', '≤0.10M', '≤1.00M', '≤10.00M', '>10.00M']) {
    expect(screen.getByTitle(title)).toBeTruthy();
  }
  const leapDay = labeledButton('Feb 29, 2024: 0.24M tokens');
  expect(leapDay.getAttribute('title')).toBe('Feb 29, 2024: 240,005 tokens');
  expect(leapDay.getAttribute('data-level')).toBe('2');
  await fireEvent.pointerEnter(leapDay);
  const inspector = screen.getByText('Cell inspector:').parentElement as HTMLElement;
  expect(inspector.textContent).toContain('Input 0.20M');
  expect(inspector.textContent).toContain('Output 0.04M');
  for (const [period, caption, input, output] of [
    ['Week', 'This week', 343_476, 52_898],
    ['Month', 'This month', 123_456, 7_890],
    ['Year', '2024', 1_343_476, 2_052_898],
  ] as const) {
    await select('Usage period', period);
    expectTokens(input, output);
    expect(within(tokenCard()).getByText(caption)).toBeTruthy();
    expect(screen.getByRole('heading', { name: 'Token usage heatmap · 2024' })).toBeTruthy();
    expect(labeledButton('Year').textContent).toContain('2024');
    expect(inspector.textContent).toContain('Feb 29, 2024: 0.24M tokens');
    expect(labeledButton('Jan 1, 2024: 3.00M tokens')).toBeTruthy();
  }
  await select('Usage year', '2023');
  expectTokens(1_234_567_890, 6_000_012);
  expect(within(tokenCard()).getByText('1234.57M')).toBeTruthy();
  expect(labeledButton('Year').textContent).toContain('2024');
  expect(inspector.textContent).toContain('Feb 29, 2024: 0.24M tokens');
  await select('Usage period', 'Day');
  expectTokens(123_456, 7_890);
  expect(config.loadTokenUsageRecords).toHaveBeenCalledExactlyOnceWith();
});

test('heatmap day, week, and month leave usage selections unchanged and reset the inspector', async () => {
  const { config } = mount();
  await screen.findByText('2024 total');
  await fireEvent.pointerEnter(labeledButton('Feb 29, 2024: 0.24M tokens'));
  const inspector = screen.getByText('Cell inspector:').parentElement as HTMLElement;
  for (const [period, caption, total, cells] of [
    ['Week', 'This week', '0.40M', 5],
    ['Month', 'This month', '0.13M', 1],
    ['Day', 'Today', '0.13M', 1],
  ] as const) {
    await select('Heatmap period', period);
    expectTokens(123_456, 7_890);
    expect(within(tokenCard()).getByText('Today')).toBeTruthy();
    expect(screen.getByText(`Total · ${caption}`).textContent).toContain(total);
    expect(screen.getAllByLabelText(/ tokens$/, { selector: 'button' })).toHaveLength(cells);
    expect(screen.queryByLabelText(/Jan 1, 2024:/, { selector: 'button' })).toBeNull();
    expect(inspector.textContent).toContain('Focus or hover a day to inspect');
  }
  expect(config.loadTokenUsageRecords).toHaveBeenCalledExactlyOnceWith();
});

test('historical heatmap and usage years persist independently through period changes', async () => {
  const { config } = mount();
  await screen.findByText('2024 total');
  await select('Usage period', 'Year');
  await select('Usage year', '2023');
  await select('Usage period', 'Day');
  const inspector = screen.getByText('Cell inspector:').parentElement as HTMLElement;
  await select('Year', '2023');
  expect(screen.getByText('2023 total').textContent).toContain('1240.57M');
  expectTokens(123_456, 7_890);
  expect(within(tokenCard()).getByText('Today')).toBeTruthy();
  for (const period of ['Day', 'Week', 'Month', 'Year']) await select('Heatmap period', period);
  expect(labeledButton('Year').textContent).toContain('2023');
  const historicDay = labeledButton('Dec 31, 2023: 1240.57M tokens');
  await fireEvent.focus(historicDay);
  await select('Usage period', 'Year');
  expect(labeledButton('Usage year').textContent).toContain('2023');
  expectTokens(1_234_567_890, 6_000_012);
  await select('Usage year', '2024');
  expectTokens(1_343_476, 2_052_898);
  expect(screen.getByRole('heading', { name: 'Token usage heatmap · 2023' })).toBeTruthy();
  expect(inspector.textContent).toContain('Dec 31, 2023: 1240.57M tokens');
  expect(config.loadTokenUsageRecords).toHaveBeenCalledExactlyOnceWith();
});

test('heatmap keyboard navigation and selector Escape preserve selection and roving focus', async () => {
  const { config } = mount();
  await screen.findByText('2024 total');
  const first = labeledButton('Jan 1, 2024: 3.00M tokens');
  expect(first.tabIndex).toBe(0);
  await fireEvent.focus(first);
  await fireEvent.keyDown(first, { key: 'ArrowRight' });
  expect(document.activeElement?.getAttribute('aria-label')).toBe('Jan 8, 2024: 0.00M tokens');
  await fireEvent.keyDown(document.activeElement as HTMLElement, { key: 'ArrowDown' });
  expect(document.activeElement?.getAttribute('aria-label')).toBe('Jan 9, 2024: 0.00M tokens');
  await fireEvent.keyDown(document.activeElement as HTMLElement, { key: 'End' });
  expect(document.activeElement?.getAttribute('aria-label')).toBe('Mar 1, 2024: 0.13M tokens');
  const inspector = screen.getByText('Cell inspector:').parentElement as HTMLElement;
  expect(inspector.textContent).toContain('Input 0.12M');
  expect(inspector.textContent).toContain('Output <0.01M');
  const year = labeledButton('Year');
  await fireEvent.keyDown(year, { key: 'ArrowDown' });
  await fireEvent.keyDown(year, { key: 'ArrowDown' });
  await fireEvent.keyDown(year, { key: 'Escape' });
  expect(year.textContent).toContain('2024');
  expect(inspector.textContent).toContain('Mar 1, 2024: 0.13M tokens');
  expect(document.activeElement).toBe(year);
  await fireEvent.keyDown(year, { key: 'ArrowDown' });
  await fireEvent.keyDown(year, { key: 'ArrowDown' });
  await fireEvent.keyDown(year, { key: 'Enter' });
  expect(year.textContent).toContain('2023');
  expect(inspector.textContent).toContain('Focus or hover a day to inspect');
  expect(labeledButton('Jan 1, 2023: 0.00M tokens').tabIndex).toBe(0);
  expect(document.activeElement).toBe(year);
  await select('Year', '2024');
  const usagePeriod = labeledButton('Usage period');
  await fireEvent.keyDown(usagePeriod, { key: 'ArrowDown' });
  await fireEvent.keyDown(usagePeriod, { key: 'ArrowDown' });
  await fireEvent.keyDown(usagePeriod, { key: 'Escape' });
  expect(usagePeriod.textContent).toContain('Day');
  expectTokens(123_456, 7_890);
  expect(config.loadTokenUsageRecords).toHaveBeenCalledExactlyOnceWith();
});

test.each([
  ['Day', 'Today', 123_456, 7_890],
  ['Week', 'This week', 343_476, 52_898],
  ['Month', 'This month', 123_456, 7_890],
  ['Year', '2024', 1_343_476, 2_052_898],
] as const)(
  '%s usage and heatmap agree on exact Input and Output and keep one keyboard focus target',
  async (period, caption, input, output) => {
    const { config } = mount();
    await screen.findByText('2024 total');
    const inspector = screen.getByText('Cell inspector:').parentElement as HTMLElement;
    await select('Usage period', period);
    await select('Heatmap period', period);
    expectTokens(input, output);
    const total = screen.getByText(period === 'Year' ? '2024 total' : `Total · ${caption}`);
    expect(total.querySelector('strong')?.getAttribute('title')).toBe((input + output).toLocaleString('en-US'));
    const dates = screen.getAllByLabelText(/ tokens$/, { selector: 'button' }) as HTMLButtonElement[];
    expect(dates.filter((cell) => cell.tabIndex === 0)).toHaveLength(1);
    let inspectedInput = 0;
    let inspectedOutput = 0;
    for (const cell of dates.filter((cell) => !cell.getAttribute('aria-label')?.endsWith('0.00M tokens'))) {
      await fireEvent.focus(cell);
      const inputTitle = within(inspector)
        .getByText(/^Input /)
        .getAttribute('title') as string;
      const outputTitle = within(inspector)
        .getByText(/^Output /)
        .getAttribute('title') as string;
      inspectedInput += Number(inputTitle.replaceAll(',', ''));
      inspectedOutput += Number(outputTitle.replaceAll(',', ''));
    }
    expect([inspectedInput, inspectedOutput]).toEqual([input, output]);
    if (period !== 'Day') {
      expect(screen.getByTitle('Mar 2, 2024: future date').tagName).toBe('SPAN');
      expect(screen.queryByLabelText(/Mar 2, 2024:/, { selector: 'button' })).toBeNull();
    }
    if (period === 'Week') {
      const monday = labeledButton('Feb 26, 2024: 0.03M tokens');
      await fireEvent.focus(monday);
      await fireEvent.keyDown(monday, { key: 'ArrowRight' });
      expect(document.activeElement?.getAttribute('aria-label')).toBe('Feb 27, 2024: 0.00M tokens');
      await fireEvent.keyDown(document.activeElement as HTMLElement, { key: 'End' });
      expect(document.activeElement?.getAttribute('aria-label')).toBe('Mar 1, 2024: 0.13M tokens');
    }
    if (period === 'Month') expect(screen.getByTitle('Mar 31, 2024: future date')).toBeTruthy();
    expect(config.loadTokenUsageRecords).toHaveBeenCalledExactlyOnceWith();
  },
);

test('mount and manual refresh use one in-flight shared read and preserve both selections through refresh, failure, and retry', async () => {
  const pending: { resolve: (records: TokenUsageRecord[]) => void; reject: (error: Error) => void }[] = [];
  const loadTokenUsageRecords = vi.fn(
    () => new Promise<TokenUsageRecord[]>((resolve, reject) => pending.push({ resolve, reject })),
  );
  mount({
    groups: [makeGroup('Native current', 'native', true)],
    selectedGroupId: 'Native current',
    loadTokenUsageRecords,
  });
  await screen.findByText('Loading OpenCode token usage…');
  const refresh = labeledButton('Refresh activity');
  refresh.dispatchEvent(new MouseEvent('click', { bubbles: true }));
  refresh.dispatchEvent(new MouseEvent('click', { bubbles: true }));
  expect(loadTokenUsageRecords).toHaveBeenCalledExactlyOnceWith();
  expectTokens(null, null);
  expect(labeledButton('Switch Native group').disabled).toBe(false);
  pending[0].resolve(records);
  await screen.findByText('2024 total');
  await select('Usage period', 'Year');
  await select('Usage year', '2023');
  await select('Heatmap period', 'Week');
  expect(loadTokenUsageRecords).toHaveBeenCalledOnce();

  refresh.dispatchEvent(new MouseEvent('click', { bubbles: true }));
  refresh.dispatchEvent(new MouseEvent('click', { bubbles: true }));
  await screen.findByText('Loading OpenCode token usage…');
  expect(loadTokenUsageRecords.mock.calls).toEqual([[], []]);
  expectTokens(null, null);
  expect(labeledButton('Usage period').textContent).toContain('Year');
  expect(labeledButton('Usage year').textContent).toContain('2023');
  expect(labeledButton('Heatmap period').textContent).toContain('Week');
  pending[1].resolve([
    ...records,
    { time: dayBounds('2022-12-31').from, input: 3, output: 4 },
    { time: dayBounds('2023-12-31').from, input: 1, output: 2 },
    { time: dayBounds('2024-03-01').from, input: 1_000_000, output: 2_000_000 },
  ]);
  await screen.findByText('Total · This week');
  expectTokens(1_234_567_891, 6_000_014);
  expect(screen.getByText('Total · This week').textContent).toContain('3.40M');
  expect(labeledButton('Usage year').textContent).toContain('2023');
  expect(labeledButton('Heatmap period').textContent).toContain('Week');
  await select('Heatmap period', 'Year');
  expect(labeledButton('Year').textContent).toContain('2024');
  expect(screen.getByText('2024 total').textContent).toContain('6.40M');
  expect(loadTokenUsageRecords.mock.calls).toEqual([[], []]);
  expect(screen.queryByRole('alert')).toBeNull();

  await select('Year', '2022');
  await fireEvent.focus(labeledButton('Dec 31, 2022: <0.01M tokens'));
  await fireEvent.click(refresh);
  await screen.findByText('Loading OpenCode token usage…');
  const switchTrigger = labeledButton('Switch Native group');
  for (const [name, value] of [
    ['Usage period', 'Year'],
    ['Usage year', '2023'],
    ['Heatmap period', 'Year'],
    ['Year', '2022'],
  ]) {
    const trigger = labeledButton(name);
    expect(trigger.textContent).toContain(value);
    expect(trigger.disabled).toBe(true);
  }
  expectTokens(null, null);
  expect(screen.queryByLabelText(/ tokens$/, { selector: 'button' })).toBeNull();
  expect(screen.queryByText('Cell inspector:')).toBeNull();
  expect(screen.queryByText('2022 total')).toBeNull();
  expect(screen.queryByTitle(/future date$/)).toBeNull();
  expect(switchTrigger.disabled).toBe(false);
  pending[2].reject(new Error('Refresh unavailable'));
  await screen.findByText('Refresh unavailable');
  expect(screen.getAllByRole('alert')).toHaveLength(2);
  expectTokens(null, null);
  expect(screen.queryByLabelText(/ tokens$/, { selector: 'button' })).toBeNull();
  expect(screen.queryByText('Cell inspector:')).toBeNull();
  expect(screen.queryByText('2022 total')).toBeNull();
  expect(screen.queryByTitle(/future date$/)).toBeNull();
  expect(switchTrigger.disabled).toBe(false);
  for (const [name, value] of [
    ['Usage period', 'Year'],
    ['Usage year', '2023'],
    ['Heatmap period', 'Year'],
    ['Year', '2022'],
  ]) {
    const trigger = labeledButton(name);
    expect(trigger.textContent).toContain(value);
    expect(trigger.disabled).toBe(true);
  }
  await fireEvent.click(labeledButton('Retry token usage'));
  await screen.findByText('Loading OpenCode token usage…');
  expectTokens(null, null);
  expect(labeledButton('Usage year').textContent).toContain('2023');
  expect(labeledButton('Year').textContent).toContain('2022');
  pending[3].resolve(records.filter((record) => record.time >= dayBounds('2024-01-01').from));
  await screen.findByText('No tokens recorded for 2022.');
  expectTokens(0, 0);
  expect(screen.getByText('2022 total').textContent).toContain('0.00M');
  for (const [name, value] of [
    ['Usage period', 'Year'],
    ['Usage year', '2023'],
    ['Heatmap period', 'Year'],
    ['Year', '2022'],
  ]) {
    const trigger = labeledButton(name);
    expect(trigger.textContent).toContain(value);
    expect(trigger.disabled).toBe(false);
  }
  expect(loadTokenUsageRecords.mock.calls).toEqual([[], [], [], []]);
  expect(screen.queryByRole('alert')).toBeNull();
});

test('empty records are valid zero tokens with future blanks and one keyboard focus target', async () => {
  mount({ loadTokenUsageRecords: vi.fn().mockResolvedValue([]) });
  await screen.findByText('No tokens recorded for 2024.');
  expectTokens(0, 0);
  expect(screen.queryByRole('alert')).toBeNull();
  expect(screen.getByText('2024 total').textContent).toContain('0.00M');
  expect(screen.getByText('Active days').textContent).toContain('0');
  expect(screen.getByText('Peak day').textContent).toContain('—');
  expect(screen.getByTitle('Mar 2, 2024: future date').tagName).toBe('SPAN');
  const dates = screen.getAllByLabelText(/: 0.00M tokens$/, { selector: 'button' }) as HTMLButtonElement[];
  expect(dates).toHaveLength(61);
  expect(dates.filter((cell) => cell.tabIndex === 0)).toHaveLength(1);
  const first = labeledButton('Jan 1, 2024: 0.00M tokens');
  await fireEvent.focus(first);
  await fireEvent.keyDown(first, { key: 'ArrowDown' });
  expect(document.activeElement?.getAttribute('aria-label')).toBe('Jan 2, 2024: 0.00M tokens');
  const inspector = screen.getByText('Cell inspector:').parentElement as HTMLElement;
  expect(inspector.textContent).toContain('Input 0.00M');
  expect(inspector.textContent).toContain('Output 0.00M');
  expect(dates.filter((cell) => cell.tabIndex === 0)).toHaveLength(1);
  await fireEvent.keyDown(document.activeElement as HTMLElement, { key: 'End' });
  expect(document.activeElement?.getAttribute('aria-label')).toBe('Mar 1, 2024: 0.00M tokens');
  expect(screen.getAllByRole('link', { name: 'Create group' })).toHaveLength(3);
});

test('unmounted dashboards ignore late shared-read success and failure', async () => {
  for (const fail of [false, true]) {
    let resolve: (records: TokenUsageRecord[]) => void = () => {};
    let reject: (error: Error) => void = () => {};
    const loadTokenUsageRecords = vi.fn(
      () =>
        new Promise<TokenUsageRecord[]>((done, error) => {
          resolve = done;
          reject = error;
        }),
    );
    const view = mount({ loadTokenUsageRecords });
    await screen.findByText('Loading OpenCode token usage…');
    view.unmount();
    if (fail) reject(new Error('Late read failed'));
    else resolve(records);
    await Promise.resolve();
    flushSync();
    expect(view.container.childElementCount).toBe(0);
    expect(screen.queryByRole('alert')).toBeNull();
    expect(loadTokenUsageRecords).toHaveBeenCalledExactlyOnceWith();
  }
});
