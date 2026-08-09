import type {
  AppSelectionState,
  ModelGroup,
  OpenCodeAgentMappingPresentation,
  Uuid
} from './contracts';
import {
  emptyMatchCounts,
  emptyOpenCodePresentation,
  type ExactModelMatchCounts,
  validateGroup
} from './settingsGroupDraft';
import { commandErrorMessage } from './tauriClient';

export type AsyncStatus = 'idle' | 'loading' | 'ready' | 'error';
export type MessageTone = 'success' | 'warning' | 'error' | 'info';
export type StatusMessage = Readonly<{ tone: MessageTone; text: string }>;

export type SettingsFacts = Readonly<{
  status: AsyncStatus;
  persistedGroups: readonly ModelGroup[];
  appState: AppSelectionState | null;
  draftGroup: ModelGroup | null;
  openCodePresentation: OpenCodeAgentMappingPresentation;
  matchSearch: string;
  matchReplace: string;
  matchCounts: ExactModelMatchCounts;
  message: StatusMessage | null;
}>;

export type SettingsState = Readonly<{
  status: AsyncStatus;
  groups: readonly ModelGroup[];
  appState: AppSelectionState | null;
  selectedGroupId: Uuid | null;
  draftGroup: ModelGroup | null;
  draftIsPersisted: boolean;
  groupValidationMessage: string | null;
  canSaveDraft: boolean;
  canSwitchDraft: boolean;
  openCodePresentation: OpenCodeAgentMappingPresentation;
  matchSearch: string;
  matchReplace: string;
  matchCounts: ExactModelMatchCounts;
  message: StatusMessage | null;
}>;

export const initialSettingsFacts: SettingsFacts = {
  status: 'idle', persistedGroups: [], appState: null,
  draftGroup: null, openCodePresentation: emptyOpenCodePresentation, matchSearch: '', matchReplace: '',
  matchCounts: emptyMatchCounts, message: null
};

export const deriveSettingsState = (facts: SettingsFacts): SettingsState => {
  const draftIsPersisted = facts.draftGroup !== null && facts.persistedGroups.some((group) => group.id === facts.draftGroup?.id);
  const groups = facts.draftGroup && !draftIsPersisted ? [...facts.persistedGroups, facts.draftGroup] : facts.persistedGroups;
  const groupValidation = facts.draftGroup ? validateGroup(facts.draftGroup, facts.persistedGroups) : null;
  return {
    status: facts.status,
    groups,
    appState: facts.appState,
    selectedGroupId: facts.draftGroup?.id ?? null,
    draftGroup: facts.draftGroup,
    draftIsPersisted,
    groupValidationMessage: groupValidation?.kind === 'invalid' ? groupValidation.message : null,
    canSaveDraft: groupValidation?.kind === 'valid' && !facts.openCodePresentation.isReadOnly,
    canSwitchDraft: draftIsPersisted && facts.draftGroup?.isEnabled === true && facts.draftGroup.id !== facts.appState?.selectedGroupID,
    openCodePresentation: facts.openCodePresentation,
    matchSearch: facts.matchSearch,
    matchReplace: facts.matchReplace,
    matchCounts: facts.matchCounts,
    message: facts.message
  };
};

export const success = (text: string): StatusMessage => ({ tone: 'success', text });
export const warning = (text: string): StatusMessage => ({ tone: 'warning', text });
export const info = (text: string): StatusMessage => ({ tone: 'info', text });
export const errorMessage = (error: unknown): StatusMessage => ({ tone: 'error', text: commandErrorMessage(error) });
