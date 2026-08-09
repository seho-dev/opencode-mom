import type { ModelGroup, SettingsCommandClient, Uuid } from './contracts';
import {
  cloneGroup,
  createDraftGroup,
  discoverPresentation,
  duplicateDraftGroup,
  emptyMatchCounts,
  emptyOpenCodePresentation,
  equalEditableGroup,
  validateGroup
} from './settingsGroupDraft';
import { matchCountsForDraft, type SettingsMatchActions } from './settingsMatchActions';
import {
  errorMessage,
  info,
  success,
  warning,
  type SettingsFacts,
  type SettingsState
} from './settingsState';

type GroupActionContext = Readonly<{
  client: SettingsCommandClient;
  readFacts: () => SettingsFacts;
  readState: () => SettingsState;
  updateFacts: (updater: (facts: SettingsFacts) => SettingsFacts) => void;
  advanceContext: () => void;
  currentContext: () => number;
  matchActions: SettingsMatchActions;
}>;

export type SettingsGroupActions = Readonly<{
  selectGroup: (id: Uuid) => Promise<void>;
  createGroup: () => Promise<void>;
  copySelectedGroup: () => Promise<void>;
  deleteSelectedGroup: () => Promise<void>;
  saveDraftGroup: () => Promise<void>;
  cancelDraftGroup: () => Promise<void>;
  switchToSelectedGroup: () => Promise<void>;
  updateDraftGroup: (patch: Partial<ModelGroup>) => Promise<void>;
}>;

const withDraftMatchCounts = (facts: SettingsFacts, draftGroup: ModelGroup | null) => ({
  draftGroup,
  matchCounts: matchCountsForDraft(facts.matchSearch, draftGroup)
});

export const createSettingsGroupActions = ({
  client,
  readFacts,
  readState,
  updateFacts,
  advanceContext,
  currentContext
}: GroupActionContext): SettingsGroupActions => {
  let presentationInvocation = 0;
  let groupMutationInvocation = 0;

  const refreshPresentation = async (group: ModelGroup): Promise<void> => {
    const invocation = ++presentationInvocation;
    const presentation = await discoverPresentation(client, group);
    const current = readFacts().draftGroup;
    if (invocation === presentationInvocation && current?.id === group.id) {
      updateFacts((facts) => ({ ...facts, openCodePresentation: presentation }));
    }
  };

  const selectedDraft = (): ModelGroup | null => {
    const draft = readFacts().draftGroup;
    if (!draft) updateFacts((facts) => ({ ...facts, message: warning('Select a group first.') }));
    return draft;
  };

  const selectGroup = async (id: Uuid): Promise<void> => {
    const group = readState().groups.find((candidate) => candidate.id === id) ?? null;
    const draftGroup = group ? cloneGroup(group) : null;
    advanceContext();
    updateFacts((facts) => ({
      ...facts,
      draftGroup,
      matchSearch: '',
      matchReplace: '',
      matchCounts: emptyMatchCounts,
      message: null
    }));
    if (group) await refreshPresentation(group);
  };

  const createGroup = async (): Promise<void> => {
    const draftGroup = createDraftGroup(readFacts().persistedGroups);
    advanceContext();
    updateFacts((facts) => ({
      ...facts,
      draftGroup,
      matchSearch: '',
      matchReplace: '',
      matchCounts: emptyMatchCounts,
      openCodePresentation: emptyOpenCodePresentation,
      message: info('New group draft created.')
    }));
    await refreshPresentation(draftGroup);
  };

  const copySelectedGroup = async (): Promise<void> => {
    const selected = selectedDraft();
    if (!selected) return;
    const facts = readFacts();
    if (!facts.persistedGroups.some((group) => group.id === selected.id)) {
      const draftGroup = duplicateDraftGroup(selected, facts.persistedGroups);
      advanceContext();
      updateFacts((current) => ({
        ...current,
        ...withDraftMatchCounts(current, draftGroup),
        message: success(`Copied ${selected.name}.`)
      }));
      return;
    }
    const invocation = ++groupMutationInvocation;
    const context = currentContext();
    try {
      const response = await client.copyGroup(selected.id);
      if (invocation !== groupMutationInvocation) return;
      const contextIsCurrent = context === currentContext();
      updateFacts((current) => ({
        ...current,
        persistedGroups: response.groups,
        appState: response.appState,
        ...withDraftMatchCounts(current, contextIsCurrent ? cloneGroup(response.group) : current.draftGroup),
        message: contextIsCurrent ? success(`Copied ${selected.name}.`) : current.message
      }));
      if (contextIsCurrent) {
        advanceContext();
        await refreshPresentation(response.group);
      }
    } catch (error) {
      if (context === currentContext() && invocation === groupMutationInvocation) updateFacts((current) => ({ ...current, message: errorMessage(error) }));
    }
  };

  const deleteSelectedGroup = async (): Promise<void> => {
    const selected = selectedDraft();
    if (!selected) return;
    const facts = readFacts();
    if (!facts.persistedGroups.some((group) => group.id === selected.id)) {
      const fallback = facts.persistedGroups[0] ?? null;
      const draftGroup = fallback ? cloneGroup(fallback) : null;
      advanceContext();
      updateFacts((current) => ({
        ...current,
        ...withDraftMatchCounts(current, draftGroup),
        message: warning('Discarded unsaved group draft.')
      }));
      if (fallback) await refreshPresentation(fallback);
      return;
    }
    const invocation = ++groupMutationInvocation;
    const context = currentContext();
    try {
      const response = await client.deleteGroup(selected.id);
      if (invocation !== groupMutationInvocation) return;
      const fallback = response.groups[0] ?? null;
      const contextIsCurrent = context === currentContext();
      updateFacts((current) => ({
        ...current,
        persistedGroups: response.groups,
        appState: response.appState,
        ...withDraftMatchCounts(
          current,
          contextIsCurrent ? fallback ? cloneGroup(fallback) : null : current.draftGroup
        ),
        message: contextIsCurrent ? success(`Deleted ${selected.name}.`) : current.message
      }));
      if (contextIsCurrent) {
        advanceContext();
        if (fallback) await refreshPresentation(fallback);
      }
    } catch (error) {
      if (context === currentContext() && invocation === groupMutationInvocation) updateFacts((current) => ({ ...current, message: errorMessage(error) }));
    }
  };

  const saveDraft = async (submitted: ModelGroup, publishMessage: boolean): Promise<ModelGroup | null> => {
    const validation = validateGroup(submitted, readFacts().persistedGroups);
    if (validation.kind === 'invalid') {
      updateFacts((facts) => ({ ...facts, message: { tone: 'error', text: validation.message } }));
      return null;
    }
    const invocation = ++groupMutationInvocation;
    const context = currentContext();
    try {
      const response = await client.saveGroup(validation.value);
      if (invocation !== groupMutationInvocation) return null;
      const current = readFacts().draftGroup;
      const draftIsCurrent = context === currentContext() && current?.id === submitted.id && equalEditableGroup(current, submitted);
      updateFacts((facts) => ({
        ...facts,
        persistedGroups: response.groups,
        appState: response.appState,
        ...withDraftMatchCounts(facts, draftIsCurrent ? cloneGroup(response.group) : facts.draftGroup),
        message: draftIsCurrent && publishMessage ? success('Group saved.') : facts.message
      }));
      if (draftIsCurrent) {
        advanceContext();
        await refreshPresentation(response.group);
      }
      return response.group;
    } catch (error) {
      if (invocation === groupMutationInvocation && context === currentContext()) updateFacts((facts) => ({ ...facts, message: errorMessage(error) }));
      return null;
    }
  };

  const saveDraftGroup = async (): Promise<void> => {
    const selected = selectedDraft();
    if (selected) await saveDraft(selected, true);
  };

  const cancelDraftGroup = async (): Promise<void> => {
    const facts = readFacts();
    const persisted = facts.draftGroup ? facts.persistedGroups.find((group) => group.id === facts.draftGroup?.id) ?? null : null;
    const fallback = persisted ?? facts.persistedGroups[0] ?? null;
    const draftGroup = fallback ? cloneGroup(fallback) : null;
    advanceContext();
    updateFacts((current) => ({
      ...current,
      ...withDraftMatchCounts(current, draftGroup),
      message: info('Changes discarded.')
    }));
    if (fallback) await refreshPresentation(fallback);
  };

  const switchToSelectedGroup = async (): Promise<void> => {
    const selected = selectedDraft();
    if (!selected) return;
    if (!readState().canSwitchDraft) {
      updateFacts((facts) => ({ ...facts, message: warning('Save and enable this group before switching.') }));
      return;
    }
    const saved = await saveDraft(selected, false);
    const current = readFacts().draftGroup;
    if (!saved || current?.id !== saved.id || !equalEditableGroup(current, saved)) return;
    const context = currentContext();
    try {
      const response = await client.switchGroup(saved.id);
      if (context === currentContext()) updateFacts((facts) => ({ ...facts, appState: response.appState, message: response.warnings.length > 0 ? warning(response.warnings.join('; ')) : success(`Switched to ${saved.name}.`) }));
    } catch (error) {
      if (context === currentContext()) updateFacts((facts) => ({ ...facts, message: errorMessage(error) }));
    }
  };

  const updateDraftGroup = async (patch: Partial<ModelGroup>): Promise<void> => {
    const draft = readFacts().draftGroup;
    if (!draft) return;
    const draftGroup = { ...draft, ...patch };
    advanceContext();
    updateFacts((facts) => ({
      ...facts,
      ...withDraftMatchCounts(facts, draftGroup),
      message: null
    }));
  };

  return {
    selectGroup, createGroup, copySelectedGroup, deleteSelectedGroup, saveDraftGroup, cancelDraftGroup,
    switchToSelectedGroup, updateDraftGroup
  };
};
