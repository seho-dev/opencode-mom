import { cleanup, render, screen, waitFor, within } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import SwitchTargets from './SwitchTargets.svelte';
import type { SwitchTarget } from '../../quickSwitchView';

const activeId = 'default';
const researchId = 'research';

const targets: readonly SwitchTarget[] = [
  { id: activeId, name: 'Default', description: 'Default group', active: true },
  { id: researchId, name: 'Research', description: null, active: false }
];

const renderTargets = (overrides: Partial<{
  targets: readonly SwitchTarget[];
  hasEnabledGroups: boolean;
  switchingGroupId: typeof activeId | typeof researchId | null;
  onSwitch: (id: typeof activeId | typeof researchId) => Promise<void>;
}> = {}) => render(SwitchTargets, {
  targets,
  hasEnabledGroups: true,
  emptyText: 'No enabled groups',
  switchingGroupId: null,
  onSwitch: vi.fn().mockResolvedValue(undefined),
  ...overrides
});

describe('SwitchTargets', () => {
  afterEach(() => cleanup());

  it('preserves the no-enabled-groups empty state without actions', () => {
    renderTargets({ hasEnabledGroups: false });

    expect(screen.getByText('No enabled groups')).toBeInstanceOf(HTMLElement);
    expect(screen.queryByRole('button', { name: /Switch to/ })).toBeNull();
  });

  it('keeps active rows actionless and exposes the semantic Active marker', () => {
    renderTargets();

    expect(screen.getByLabelText('Default, active group')).toBeInstanceOf(HTMLElement);
    expect(screen.getByText('Active')).toBeInstanceOf(HTMLElement);
    expect(screen.queryByRole('button', { name: 'Switch to Default' })).toBeNull();
  });

  it('keeps target actions after their copy in visual and DOM order', () => {
    const { container } = renderTargets();
    const row = container.querySelector('[data-group-id="research"]');

    expect(row).toBeInstanceOf(HTMLElement);
    expect(row && Array.from(row.children).map((child) => child.tagName)).toEqual(['DIV', 'BUTTON']);
    expect(within(row as HTMLElement).getByRole('button', { name: 'Switch to Research' })).toBeInstanceOf(HTMLElement);
  });

  it('keeps long target names complete to assistive technology while the row can ellipsize visually', () => {
    const longName = 'A target name that is intentionally long enough to exercise compact row overflow handling';
    renderTargets({
      targets: [{ id: researchId, name: longName, description: null, active: false }]
    });

    expect(screen.getByTitle(longName).textContent).toBe(longName);
    expect(screen.getByRole('button', { name: `Switch to ${longName}` })).toBeInstanceOf(HTMLElement);
  });

  it('disables every target while one switch is pending and marks the pending action busy', () => {
    renderTargets({ switchingGroupId: researchId });

    const action = screen.getByRole('button', { name: 'Switch to Research' });
    expect((action as HTMLButtonElement).disabled).toBe(true);
    expect(action.getAttribute('aria-busy')).toBe('true');
  });

  it('uses the exact accessible switch label and invokes the requested target', async () => {
    const onSwitch = vi.fn().mockResolvedValue(undefined);
    renderTargets({ onSwitch });

    await screen.getByRole('button', { name: 'Switch to Research' }).click();

    expect(onSwitch).toHaveBeenCalledWith(researchId);
  });

  it('recovers after the parent handles a rejected switch and clears pending state', async () => {
    const onSwitch = vi.fn(async () => {
      await Promise.reject(new Error('switch failed')).catch(() => undefined);
    });
    const view = renderTargets({ onSwitch });
    const action = screen.getByRole('button', { name: 'Switch to Research' });

    action.click();
    await waitFor(() => expect(onSwitch).toHaveBeenCalledTimes(1));
    await view.rerender({
      targets,
      hasEnabledGroups: true,
      emptyText: 'No enabled groups',
      switchingGroupId: null,
      onSwitch
    });

    expect((screen.getByRole('button', { name: 'Switch to Research' }) as HTMLButtonElement).disabled).toBe(false);
    expect(screen.queryByText('Research, active group')).toBeNull();
  });
});
