import type { AppStateResponse, ModelGroup, Uuid } from './contracts';

export type SwitchTarget = Readonly<{
  id: Uuid;
  name: string;
  description: string | null;
  active: boolean;
}>;

export type QuickSwitchViewData = Readonly<{
  currentGroup: ModelGroup | null;
  switchTargets: readonly SwitchTarget[];
  hasEnabledGroups: boolean;
  warning: string | null;
  error: string | null;
  discoveryError: string | null;
  emptySwitchTargetText: string;
}>;

export const emptyQuickSwitchView: QuickSwitchViewData = {
  currentGroup: null,
  switchTargets: [],
  hasEnabledGroups: false,
  warning: null,
  error: null,
  discoveryError: null,
  emptySwitchTargetText: 'No enabled groups'
};

export const toQuickSwitchView = (response: AppStateResponse): QuickSwitchViewData => {
  const currentGroupId = response.appState.selectedGroupID;
  const currentGroup = currentGroupId
    ? response.groups.find((group) => group.id === currentGroupId) ?? null
    : null;
  const switchTargets = response.groups
    .filter((group) => group.isEnabled)
    .map((group) => ({
      id: group.id,
      name: group.name,
      description: group.description,
      active: group.id === currentGroupId
    }));

  return {
    currentGroup,
    switchTargets,
    hasEnabledGroups: switchTargets.length > 0,
    warning: response.appState.lastWarningSummary?.message ?? null,
    error: response.appState.lastErrorSummary?.message ?? null,
    discoveryError: response.openCodeAgentDiscoveryError,
    emptySwitchTargetText: emptyQuickSwitchView.emptySwitchTargetText
  };
};
