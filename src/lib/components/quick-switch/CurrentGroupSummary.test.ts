import { cleanup, render, screen, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it } from 'vitest';

import type { ModelGroup } from '../../contracts';
import CurrentGroupSummary from './CurrentGroupSummary.svelte';

const makeGroup = (overrides: Partial<ModelGroup> = {}): ModelGroup => ({
  id: 'group-1',
  name: 'Research',
  description: 'A group for long context work.',
  agentOverrides: [{ agentName: 'planner', modelRef: 'openai/gpt-5.1' }],
  openCodeAgentOverrides: [{ agentName: 'reviewer', modelRef: 'anthropic/claude-opus-4' }],
  categoryMappings: [{ categoryName: 'build', modelRef: 'anthropic/claude-sonnet-4' }],
  isEnabled: true,
  updatedAt: '2026-07-09T00:00:00Z',
  ...overrides
});

describe('CurrentGroupSummary', () => {
  afterEach(() => cleanup());

  it('preserves the focus target and renders a present group with an optional description', () => {
    const { container } = render(CurrentGroupSummary, { props: { group: makeGroup() } });

    const summary = screen.getByLabelText('Current group summary');
    expect(summary).toBeInstanceOf(HTMLElement);
    expect(summary.getAttribute('tabindex')).toBe('-1');
    expect(screen.getByRole('heading', { name: 'Research' }).id).toBe('quick-switch-title');
    expect(screen.getByText('A group for long context work.')).toBeInstanceOf(HTMLElement);
    expect(container.querySelector('.description')).toBeInstanceOf(HTMLElement);
  });

  it('renders the null-group state without value sections', () => {
    render(CurrentGroupSummary, { props: { group: null } });

    expect(screen.getByRole('heading', { name: 'None' }).className).toContain('muted-title');
    expect(screen.getByText('No group is selected.')).toBeInstanceOf(HTMLElement);
    expect(screen.queryByRole('region', { name: 'Agent Overrides' })).toBeNull();
    expect(screen.queryByRole('region', { name: 'OpenCode Overrides' })).toBeNull();
    expect(screen.queryByRole('region', { name: 'Category Mappings' })).toBeNull();
  });

  it('omits an empty description and empty arrays while preserving section order and titles', () => {
    render(CurrentGroupSummary, {
      props: {
        group: makeGroup({
          description: null,
          agentOverrides: [{ agentName: 'a very long planner name', modelRef: 'openai/a-very-long-model-ref' }],
          openCodeAgentOverrides: [],
          categoryMappings: [{ categoryName: 'a very long category name', modelRef: 'local/compact-model' }]
        })
      }
    });

    expect(screen.queryByText('A group for long context work.')).toBeNull();
    expect(screen.queryByRole('region', { name: 'OpenCode Overrides' })).toBeNull();

    const sections = screen.getAllByRole('region')
      .map((section) => section.getAttribute('aria-label'))
      .filter((label): label is string => label !== 'Current group summary');
    expect(sections).toEqual(['Agent Overrides', 'Category Mappings']);

    const agentSection = screen.getByRole('region', { name: 'Agent Overrides' });
    expect(within(agentSection).getByTitle('a very long planner name')).toBeInstanceOf(HTMLElement);
    expect(within(agentSection).getByTitle('openai/a-very-long-model-ref').tagName).toBe('CODE');

    const mappingSection = screen.getByRole('region', { name: 'Category Mappings' });
    expect(within(mappingSection).getByTitle('a very long category name')).toBeInstanceOf(HTMLElement);
    expect(within(mappingSection).getByTitle('local/compact-model').tagName).toBe('CODE');
  });

  it('keeps all three populated sections in the declared order', () => {
    render(CurrentGroupSummary, { props: { group: makeGroup() } });

    expect(screen.getAllByRole('region')
      .map((section) => section.getAttribute('aria-label'))
      .filter((label): label is string => label !== 'Current group summary')).toEqual([
        'Agent Overrides',
        'OpenCode Overrides',
        'Category Mappings'
      ]);
  });
});
