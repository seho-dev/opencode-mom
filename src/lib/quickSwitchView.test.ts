import { describe, expect, it } from 'vitest';

import { toQuickSwitchView } from './quickSwitchView';
import type { AppStateResponse, ModelGroup } from './contracts';

const currentGroupId = '11111111-1111-4111-8111-111111111111';
const disabledGroupId = '22222222-2222-4222-8222-222222222222';
const enabledGroupId = '33333333-3333-4333-8333-333333333333';

const group = (overrides: Partial<ModelGroup>): ModelGroup => ({
  id: currentGroupId,
  name: 'Default',
  description: 'Primary local model set',
  categoryMappings: [{ categoryName: 'build', modelRef: 'anthropic/claude-sonnet-4' }],
  agentOverrides: [{ agentName: 'planner', modelRef: 'openai/gpt-5.1' }],
  openCodeAgentOverrides: [{ agentName: 'reviewer', modelRef: 'anthropic/claude-opus-4' }],
  isEnabled: true,
  updatedAt: '2026-07-09T00:00:00Z',
  ...overrides
});

const response = (groups: readonly ModelGroup[]): AppStateResponse => ({
  groups,
  appState: {
    selectedGroupID: currentGroupId,
    selectedGroupName: 'Default',
    lastSuccessfulWrite: null,
    lastWarningSummary: { message: 'OpenCode config was skipped.', count: 1 },
    lastErrorSummary: null,
    migrationVersion: 2
  },
  discoveredOpenCodeAgentNames: ['reviewer'],
  openCodeAgentDiscoveryError: null,
});

describe('toQuickSwitchView', () => {
  it('Given groups When building the view Then only enabled switch targets remain and the current group is active', () => {
    const view = toQuickSwitchView(response([
      group({}),
      group({ id: disabledGroupId, name: 'Disabled', isEnabled: false }),
      group({ id: enabledGroupId, name: 'Research', isEnabled: true })
    ]));

    expect(view.currentGroup?.name).toBe('Default');
    expect(view.switchTargets.map((target) => target.name)).toEqual(['Default', 'Research']);
    expect(view.switchTargets.find((target) => target.id === currentGroupId)?.active).toBe(true);
    expect(view.hasEnabledGroups).toBe(true);
  });

  it('Given no enabled groups When building the view Then the empty state is exposed', () => {
    const view = toQuickSwitchView(response([group({ isEnabled: false })]));

    expect(view.switchTargets).toEqual([]);
    expect(view.hasEnabledGroups).toBe(false);
    expect(view.emptySwitchTargetText).toBe('No enabled groups');
  });

  it('Given persisted warnings, errors, and discovery errors When building the view Then status messages preserve every channel', () => {
    const appState = response([group({})]);
    const view = toQuickSwitchView({
      ...appState,
      appState: {
        ...appState.appState,
        lastErrorSummary: { message: 'The selected group could not be applied.', count: 1 }
      },
      openCodeAgentDiscoveryError: 'OpenCode config is malformed.'
    });

    expect(view.warning).toBe('OpenCode config was skipped.');
    expect(view.error).toBe('The selected group could not be applied.');
    expect(view.discoveryError).toBe('OpenCode config is malformed.');
  });
});
