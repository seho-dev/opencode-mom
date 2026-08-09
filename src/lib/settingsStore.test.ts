import { get } from 'svelte/store';
import { describe, expect, it, vi } from 'vitest';

import type { DiscoverOpenCodeAgentsResponse, GroupMutationResponse, ModelGroup, SettingsCommandClient, SwitchGroupResponse, Uuid } from './contracts';
import { emptyOpenCodePresentation } from './settingsGroupDraft';
import { createSettingsStore } from './settingsStore';
import { baseAppState, baseGroups, defaultGroupId, fakeSettingsClient, researchGroupId } from './settingsTestSupport';

describe('createSettingsStore', () => {
  const researchGroup = baseGroups.find((group) => group.id === researchGroupId);
  if (!researchGroup) throw new Error('Expected the research group fixture.');

  it('Given settings data When loading Then group and OpenCode presentation are ready', async () => {
    const store = createSettingsStore(fakeSettingsClient());

    await store.load();

    const state = get(store);
    expect(state.status).toBe('ready');
    expect(state.draftGroup?.name).toBe('Default');
    expect(state.openCodePresentation.discoveredRows.map((row) => row.agentName)).toEqual(['reviewer', 'summarizer']);
  });

  it('Given OpenCode discovery throws synchronously When loading Then required app state remains ready and usable', async () => {
    const discoverOpenCodeAgents = vi.fn<SettingsCommandClient['discoverOpenCodeAgents']>().mockImplementation(() => {
      throw { code: 'malformedOpenCodeConfig', message: 'OpenCode discovery failed.', detail: 'OpenCode config is malformed.' };
    });
    const store = createSettingsStore({
      ...fakeSettingsClient(),
      discoverOpenCodeAgents
    });

    await store.load();

    const state = get(store);
    expect(state.status).toBe('ready');
    expect(state.appState).toEqual(baseAppState);
    expect(state.groups).toEqual(baseGroups);
    expect(state.draftGroup?.name).toBe('Default');
    expect(state.openCodePresentation).toEqual(emptyOpenCodePresentation);
    await store.updateDraftGroup({ description: 'Still usable' });
    expect(get(store).draftGroup?.description).toBe('Still usable');
  });

  it('Given canonical facts and a dirty draft When a later app-state reload fails Then usable content and local edits remain', async () => {
    const baselineClient = fakeSettingsClient();
    const loadAppState = vi.fn<SettingsCommandClient['loadAppState']>()
      .mockImplementationOnce(baselineClient.loadAppState)
      .mockRejectedValueOnce({ code: 'loadAppStateFailed', message: 'Settings reload failed.', detail: 'Settings data is unavailable.' });
    const store = createSettingsStore({ ...baselineClient, loadAppState });

    await store.load();
    await store.updateDraftGroup({ description: 'Keep local edit' });
    await store.load();

    const state = get(store);
    expect(state.status).toBe('error');
    expect(state.groups).toEqual(baseGroups);
    expect(state.appState).toEqual(baseAppState);
    expect(state.draftGroup?.description).toBe('Keep local edit');
    expect(state.message).toEqual({ tone: 'error', text: 'Settings data is unavailable.' });
  });

  it('Given a new group When saving Then create persists the draft and selects it', async () => {
    const saveGroup = vi.fn<(group: ModelGroup) => Promise<GroupMutationResponse>>().mockImplementation(async (group) => ({ group, groups: [...baseGroups, group], appState: baseAppState }));
    const client = fakeSettingsClient({ saveGroup });
    const store = createSettingsStore(client);

    await store.load();
    await store.createGroup();
    await store.updateDraftGroup({ name: 'Mobile' });
    await store.saveDraftGroup();

    expect(saveGroup).toHaveBeenCalledTimes(1);
    expect(get(store).message?.text).toBe('Group saved.');
    expect(get(store).draftGroup?.name).toBe('Mobile');
  });

  it('Given a new group When created Then the unsaved draft is visible and cannot switch before save', async () => {
    const switchGroup = vi.fn<(id: Uuid) => Promise<SwitchGroupResponse>>();
    const store = createSettingsStore(fakeSettingsClient({ switchGroup }));

    await store.load();
    await store.createGroup();

    const state = get(store);
    expect(state.groups.some((group) => group.id === state.draftGroup?.id)).toBe(true);
    expect(state.canSwitchDraft).toBe(false);
    await store.switchToSelectedGroup();
    expect(switchGroup).not.toHaveBeenCalled();
  });

  it('Given OpenCode agents When creating a group Then discovery refreshes presentation for the new draft', async () => {
    const baselineClient = fakeSettingsClient();
    const discoverOpenCodeAgents = vi.fn<SettingsCommandClient['discoverOpenCodeAgents']>()
      .mockImplementationOnce(baselineClient.discoverOpenCodeAgents)
      .mockResolvedValueOnce(discoveryResponse('new-draft', 'new/model'));
    const store = createSettingsStore(fakeSettingsClient({ discoverOpenCodeAgents }));

    await store.load();
    await store.createGroup();

    expect(discoverOpenCodeAgents).toHaveBeenCalledTimes(2);
    expect(get(store).openCodePresentation.discoveredRows).toEqual([
      { id: 'discovered:new-draft', agentName: 'new-draft', modelRef: 'new/model', isEditable: true }
    ]);
  });

  it('Given batch replace state from another group When creating a group Then the new draft starts with clean match state', async () => {
    const store = createSettingsStore(fakeSettingsClient());

    await store.load();
    await store.setMatchSearch('anthropic/claude-sonnet-4');
    store.setMatchReplace('openai/gpt-5.2');
    await store.createGroup();

    expect(get(store).matchSearch).toBe('');
    expect(get(store).matchReplace).toBe('');
    expect(get(store).matchCounts).toEqual({ categoryMappings: 0, agentOverrides: 0, openCodeAgentOverrides: 0 });
  });

  it('Given persisted groups When a draft is edited Then canonical rows remain unchanged', async () => {
    const store = createSettingsStore(fakeSettingsClient());

    await store.load();
    await store.updateDraftGroup({ name: 'Edited locally', description: 'Draft only' });

    const state = get(store);
    expect(state.groups.find((group) => group.id === baseGroups[0]?.id)?.name).toBe('Default');
    expect(state.draftGroup?.name).toBe('Edited locally');
    expect('persistedGroups' in state).toBe(false);
    expect('response' in state).toBe(false);
  });

  it('Given an unsaved draft When reselecting it Then it remains unsaved', async () => {
    const store = createSettingsStore(fakeSettingsClient());

    await store.load();
    await store.createGroup();
    const draftId = get(store).draftGroup?.id;
    if (!draftId) throw new Error('Expected an unsaved draft.');
    await store.selectGroup(draftId);

    expect(get(store).draftGroup?.id).toBe(draftId);
    expect(get(store).draftIsPersisted).toBe(false);
  });

  it('Given an unsaved draft When deleting it Then no backend delete occurs and the first persisted group is selected', async () => {
    const deleteGroup = vi.fn<SettingsCommandClient['deleteGroup']>();
    const store = createSettingsStore(fakeSettingsClient({ deleteGroup }));

    await store.load();
    await store.createGroup();
    await store.deleteSelectedGroup();

    const state = get(store);
    expect(deleteGroup).not.toHaveBeenCalled();
    expect(state.draftGroup?.id).toBe(baseGroups[0]?.id);
    expect(state.draftIsPersisted).toBe(true);
    expect(state.message).toEqual({ tone: 'warning', text: 'Discarded unsaved group draft.' });
  });

  it('Given matches in an unsaved draft When deleting it Then fallback counts describe the persisted group', async () => {
    const store = createSettingsStore(fakeSettingsClient());

    await store.load();
    await store.createGroup();
    await store.updateDraftGroup({ categoryMappings: [{ categoryName: 'build', modelRef: 'draft/model' }] });
    await store.setMatchSearch('draft/model');
    expect(get(store).matchCounts.categoryMappings).toBe(1);

    await store.deleteSelectedGroup();

    const state = get(store);
    expect(state.draftGroup?.id).toBe(defaultGroupId);
    expect(state.matchSearch).toBe('draft/model');
    expect(state.matchCounts).toEqual({ categoryMappings: 0, agentOverrides: 0, openCodeAgentOverrides: 0 });
  });

  it('Given an unsaved draft with different presentation When deleting it Then the persisted fallback presentation is refreshed', async () => {
    const discoverOpenCodeAgents = vi.fn<SettingsCommandClient['discoverOpenCodeAgents']>()
      .mockImplementation(async (savedOverrides) => savedOverrides[0]?.modelRef === 'anthropic/claude-opus-4'
        ? discoveryResponse('fallback-reviewer', 'fallback/model')
        : discoveryResponse('draft-reviewer', 'draft/model'));
    const store = createSettingsStore(fakeSettingsClient({ discoverOpenCodeAgents }));

    await store.load();
    await store.createGroup();
    await store.deleteSelectedGroup();

    expect(get(store).draftGroup?.id).toBe(baseGroups[0]?.id);
    expect(get(store).openCodePresentation.discoveredRows).toEqual([
      { id: 'discovered:fallback-reviewer', agentName: 'fallback-reviewer', modelRef: 'fallback/model', isEditable: true }
    ]);
    expect(discoverOpenCodeAgents).toHaveBeenLastCalledWith(baseGroups[0]?.openCodeAgentOverrides);
  });

  it('Given an unsaved draft When copying it Then no backend copy occurs and exactly one unsaved copy remains visible', async () => {
    const copyGroup = vi.fn<SettingsCommandClient['copyGroup']>();
    const store = createSettingsStore(fakeSettingsClient({ copyGroup }));

    await store.load();
    await store.createGroup();
    const originalDraftId = get(store).draftGroup?.id;
    if (!originalDraftId) throw new Error('Expected an unsaved draft.');
    await store.selectGroup(originalDraftId);
    await store.copySelectedGroup();

    const state = get(store);
    expect(copyGroup).not.toHaveBeenCalled();
    expect(state.draftIsPersisted).toBe(false);
    expect(state.draftGroup?.id).not.toBe(originalDraftId);
    expect(state.groups.filter((group) => !baseGroups.some((persisted) => persisted.id === group.id))).toEqual([state.draftGroup]);
  });

  it('Given an unsaved draft When selecting a persisted group Then the draft is discarded and persisted selection is restored', async () => {
    const store = createSettingsStore(fakeSettingsClient());

    await store.load();
    await store.createGroup();
    const draftId = get(store).draftGroup?.id;
    await store.selectGroup(researchGroupId);

    const state = get(store);
    expect(state.draftGroup?.id).toBe(researchGroupId);
    expect(state.draftIsPersisted).toBe(true);
    expect(state.groups.some((group) => group.id === draftId)).toBe(false);
  });

  it('Given an unsaved draft When cancelling Then the first persisted group becomes the selected fallback', async () => {
    const store = createSettingsStore(fakeSettingsClient());

    await store.load();
    await store.createGroup();
    await store.cancelDraftGroup();

    const state = get(store);
    expect(state.draftGroup?.id).toBe(baseGroups[0]?.id);
    expect(state.draftIsPersisted).toBe(true);
    expect(state.message).toEqual({ tone: 'info', text: 'Changes discarded.' });
  });

  it('Given matches in a dirty persisted draft When cancelling Then restored counts describe the persisted version', async () => {
    const store = createSettingsStore(fakeSettingsClient());

    await store.load();
    await store.updateDraftGroup({ categoryMappings: [{ categoryName: 'build', modelRef: 'draft/model' }] });
    await store.setMatchSearch('draft/model');
    expect(get(store).matchCounts.categoryMappings).toBe(1);

    await store.cancelDraftGroup();

    const state = get(store);
    expect(state.draftGroup?.id).toBe(defaultGroupId);
    expect(state.matchSearch).toBe('draft/model');
    expect(state.matchCounts).toEqual({ categoryMappings: 0, agentOverrides: 0, openCodeAgentOverrides: 0 });
  });

  it('Given match inputs and a presentation When selecting an unknown group Then the draft clears without refreshing presentation', async () => {
    const discoverOpenCodeAgents = vi.fn<SettingsCommandClient['discoverOpenCodeAgents']>()
      .mockImplementation(async (savedOverrides) => fakeSettingsClient().discoverOpenCodeAgents(savedOverrides));
    const store = createSettingsStore(fakeSettingsClient({ discoverOpenCodeAgents }));

    await store.load();
    await store.setMatchSearch('anthropic/claude-sonnet-4');
    store.setMatchReplace('replacement/model');
    const presentation = get(store).openCodePresentation;
    await store.selectGroup('99999999-9999-4999-8999-999999999999');

    const state = get(store);
    expect(state.draftGroup).toBeNull();
    expect(state.selectedGroupId).toBeNull();
    expect(state.matchSearch).toBe('');
    expect(state.matchReplace).toBe('');
    expect(state.matchCounts).toEqual({ categoryMappings: 0, agentOverrides: 0, openCodeAgentOverrides: 0 });
    expect(state.openCodePresentation).toBe(presentation);
    expect(state.message).toBeNull();
    expect(discoverOpenCodeAgents).toHaveBeenCalledTimes(1);
  });

  it('Given no selected draft When a selected-group action runs Then the existing warning is shown', async () => {
    const store = createSettingsStore(fakeSettingsClient());

    await store.load();
    await store.selectGroup('99999999-9999-4999-8999-999999999999');
    await store.copySelectedGroup();

    expect(get(store).message).toEqual({ tone: 'warning', text: 'Select a group first.' });
  });

  it('Given canonical groups When a persisted copy completes Then the group list is refreshed without a response snapshot', async () => {
    const store = createSettingsStore(fakeSettingsClient());

    await store.load();
    await store.copySelectedGroup();

    const state = get(store);
    expect(state.groups).toHaveLength(baseGroups.length + 1);
    expect(state.appState).toEqual(baseAppState);
    expect('response' in state).toBe(false);
  });

  it('Given overlapping group presentation requests When an older response resolves last Then the current selection keeps its presentation', async () => {
    let resolveResearch: ((value: DiscoverOpenCodeAgentsResponse) => void) | undefined;
    const baselineClient = fakeSettingsClient();
    const discoverOpenCodeAgents = vi.fn<SettingsCommandClient['discoverOpenCodeAgents']>()
      .mockImplementationOnce(baselineClient.discoverOpenCodeAgents)
      .mockImplementationOnce(() => new Promise((resolve) => { resolveResearch = resolve; }))
      .mockImplementationOnce(async () => ({
        agentNames: ['current'],
        error: null,
        presentation: {
          discoveredRows: [{ id: 'discovered:current', agentName: 'current', modelRef: 'current/model', isEditable: true }],
          staleOverrides: [],
          preservedOverrides: [],
          discoveryError: null,
          isReadOnly: false,
          allowsCustomAgentCreation: false
        }
      }));
    const store = createSettingsStore(fakeSettingsClient({ discoverOpenCodeAgents }));

    await store.load();
    const staleSelection = store.selectGroup(researchGroupId);
    await store.selectGroup(baseGroups[0]?.id ?? researchGroupId);
    resolveResearch?.({
      agentNames: ['stale'],
      error: null,
      presentation: {
        discoveredRows: [{ id: 'discovered:stale', agentName: 'stale', modelRef: 'stale/model', isEditable: true }],
        staleOverrides: [],
        preservedOverrides: [],
        discoveryError: null,
        isReadOnly: false,
        allowsCustomAgentCreation: false
      }
    });
    await staleSelection;

    expect(get(store).draftGroup?.id).toBe(baseGroups[0]?.id);
    expect(get(store).openCodePresentation.discoveredRows.map((row) => row.agentName)).toEqual(['current']);
  });

  it('Given overlapping presentation requests for one group When the older request resolves last Then only the newest presentation is published', async () => {
    let resolveOlder: ((value: DiscoverOpenCodeAgentsResponse) => void) | undefined;
    let resolveNewer: ((value: DiscoverOpenCodeAgentsResponse) => void) | undefined;
    const baselineClient = fakeSettingsClient();
    const discoverOpenCodeAgents = vi.fn<SettingsCommandClient['discoverOpenCodeAgents']>()
      .mockImplementationOnce(baselineClient.discoverOpenCodeAgents)
      .mockImplementationOnce(() => new Promise((resolve) => { resolveOlder = resolve; }))
      .mockImplementationOnce(() => new Promise((resolve) => { resolveNewer = resolve; }));
    const store = createSettingsStore(fakeSettingsClient({ discoverOpenCodeAgents }));

    await store.load();
    const olderSelection = store.selectGroup(researchGroupId);
    const newerSelection = store.selectGroup(researchGroupId);
    if (!resolveNewer || !resolveOlder) throw new Error('Expected both presentation requests.');
    resolveNewer(discoveryResponse('newest', 'newest/model'));
    await newerSelection;
    resolveOlder(discoveryResponse('stale', 'stale/model'));
    await olderSelection;

    expect(get(store).openCodePresentation.discoveredRows).toEqual([
      { id: 'discovered:newest', agentName: 'newest', modelRef: 'newest/model', isEditable: true }
    ]);
  });

  it('Given a duplicate group name When saving Then validation blocks persistence', async () => {
    const saveGroup = vi.fn<(group: ModelGroup) => Promise<GroupMutationResponse>>().mockImplementation(async (group) => ({ group, groups: [...baseGroups, group], appState: baseAppState }));
    const store = createSettingsStore(fakeSettingsClient({ saveGroup }));

    await store.load();
    await store.updateDraftGroup({ name: 'Research' });
    await store.saveDraftGroup();

    expect(saveGroup).not.toHaveBeenCalled();
    expect(get(store).groupValidationMessage).toBe('A group named "Research" already exists.');
    expect(get(store).canSaveDraft).toBe(false);
    expect(get(store).message).toEqual({ tone: 'error', text: 'A group named "Research" already exists.' });
  });

  it('Given draft edits When cancelling Then original metadata returns', async () => {
    const store = createSettingsStore(fakeSettingsClient());

    await store.load();
    await store.updateDraftGroup({ name: 'Changed' });
    await store.cancelDraftGroup();

    expect(get(store).draftGroup?.name).toBe('Default');
    expect(get(store).message?.text).toBe('Changes discarded.');
  });

  it('Given selected group When copy delete and switch run Then command-backed mutations are reflected', async () => {
    const client = fakeSettingsClient();
    const store = createSettingsStore(client);

    await store.load();
    await store.copySelectedGroup();
    expect(get(store).draftGroup?.name).toBe('Default Copy');
    expect(get(store).groups.some((group) => group.name === 'Default Copy')).toBe(true);

    await store.deleteSelectedGroup();
    expect(get(store).draftGroup?.name).toBe('Default');

    await store.selectGroup(researchGroupId);
    await store.switchToSelectedGroup();
    expect(get(store).appState?.selectedGroupID).toBe(researchGroupId);
    expect(get(store).message?.text).toBe('Switched to Research.');
  });

  it('Given a persistent draft match When copying or deleting Then replacement counts follow the returned draft', async () => {
    const store = createSettingsStore(fakeSettingsClient());

    await store.load();
    await store.updateDraftGroup({ categoryMappings: [{ categoryName: 'build', modelRef: 'draft/model' }] });
    await store.setMatchSearch('draft/model');
    expect(get(store).matchCounts.categoryMappings).toBe(1);

    await store.copySelectedGroup();
    expect(get(store).draftGroup?.name).toBe('Default Copy');
    expect(get(store).matchCounts).toEqual({ categoryMappings: 0, agentOverrides: 0, openCodeAgentOverrides: 0 });

    await store.selectGroup(defaultGroupId);
    await store.setMatchSearch('anthropic/claude-sonnet-4');
    expect(get(store).matchCounts.categoryMappings).toBe(1);
    await store.deleteSelectedGroup();

    const state = get(store);
    expect(state.draftGroup?.id).toBe(researchGroupId);
    expect(state.matchSearch).toBe('anthropic/claude-sonnet-4');
    expect(state.matchCounts).toEqual({ categoryMappings: 0, agentOverrides: 0, openCodeAgentOverrides: 0 });
  });

  it('Given persisted copy in flight When another group is selected Then the response reconciles groups without hijacking local context', async () => {
    let resolveCopy: ((value: GroupMutationResponse) => void) | undefined;
    const copiedGroup: ModelGroup = { ...researchGroup, id: '44444444-4444-4444-8444-444444444444', name: 'Default Copy' };
    const copyGroup = vi.fn<SettingsCommandClient['copyGroup']>()
      .mockImplementation(() => new Promise((resolve) => { resolveCopy = resolve; }));
    const store = createSettingsStore(fakeSettingsClient({ copyGroup }));

    await store.load();
    const copying = store.copySelectedGroup();
    await store.selectGroup(researchGroupId);
    await store.setMatchSearch('research/model');
    store.setMatchReplace('next/model');
    if (!resolveCopy) throw new Error('Expected a pending group copy.');
    resolveCopy({ group: copiedGroup, groups: [...baseGroups, copiedGroup], appState: baseAppState });
    await copying;

    const state = get(store);
    expect(state.groups.some((group) => group.id === copiedGroup.id)).toBe(true);
    expect(state.draftGroup?.id).toBe(researchGroupId);
    expect(state.openCodePresentation.discoveredRows.find((row) => row.agentName === 'reviewer')?.modelRef).toBe('');
    expect(state.matchSearch).toBe('research/model');
    expect(state.matchReplace).toBe('next/model');
    expect(state.message).toBeNull();
  });

  it('Given persisted delete in flight When another surviving group is selected Then the response reconciles groups without fallback hijack', async () => {
    let resolveDelete: ((value: GroupMutationResponse) => void) | undefined;
    const otherGroup: ModelGroup = { ...researchGroup, id: '55555555-5555-4555-8555-555555555555', name: 'Other' };
    const deleteGroup = vi.fn<SettingsCommandClient['deleteGroup']>()
      .mockImplementation(() => new Promise((resolve) => { resolveDelete = resolve; }));
    const store = createSettingsStore(fakeSettingsClient({ deleteGroup }));

    await store.load();
    const deleting = store.deleteSelectedGroup();
    await store.selectGroup(researchGroupId);
    await store.setMatchSearch('research/model');
    store.setMatchReplace('next/model');
    if (!resolveDelete) throw new Error('Expected a pending group delete.');
    resolveDelete({ group: baseGroups[0] ?? researchGroup, groups: [otherGroup, researchGroup], appState: baseAppState });
    await deleting;

    const state = get(store);
    expect(state.groups.map((group) => group.id)).toEqual([otherGroup.id, researchGroupId]);
    expect(state.draftGroup?.id).toBe(researchGroupId);
    expect(state.openCodePresentation.discoveredRows.find((row) => row.agentName === 'reviewer')?.modelRef).toBe('');
    expect(state.matchSearch).toBe('research/model');
    expect(state.matchReplace).toBe('next/model');
    expect(state.message).toBeNull();
  });

  it('Given a dirty persisted group When switching Then it saves before switching and keeps the saved group selected', async () => {
    const calls: string[] = [];
    const savedResearch: ModelGroup = { ...researchGroup, name: 'Research Saved', description: 'Dirty edit' };
    const saveGroup = vi.fn<SettingsCommandClient['saveGroup']>().mockImplementation(async () => {
      calls.push('save');
      return { group: savedResearch, groups: baseGroups.map((group) => group.id === savedResearch.id ? savedResearch : group), appState: baseAppState };
    });
    const switchGroup = vi.fn<SettingsCommandClient['switchGroup']>().mockImplementation(async (id) => {
      calls.push('switch');
      return { outcome: 'success', warnings: [], appState: { ...baseAppState, selectedGroupID: id, selectedGroupName: savedResearch.name } };
    });
    const store = createSettingsStore(fakeSettingsClient({ saveGroup, switchGroup }));

    await store.load();
    await store.selectGroup(researchGroupId);
    await store.updateDraftGroup({ description: 'Dirty edit' });
    await store.switchToSelectedGroup();

    expect(calls).toEqual(['save', 'switch']);
    expect(saveGroup).toHaveBeenCalledWith(expect.objectContaining({ id: researchGroupId, description: 'Dirty edit' }));
    expect(switchGroup).toHaveBeenCalledWith(savedResearch.id);
    expect(get(store).draftGroup?.name).toBe('Research Saved');
    expect(get(store).appState?.selectedGroupID).toBe(savedResearch.id);
  });

  it('Given a dirty persisted group When save before switch fails Then switching is aborted and the draft error remains', async () => {
    const saveGroup = vi.fn<SettingsCommandClient['saveGroup']>()
      .mockRejectedValue({ code: 'saveGroupsFailed', message: 'Save failed.', detail: 'groups.json is locked' });
    const switchGroup = vi.fn<SettingsCommandClient['switchGroup']>();
    const store = createSettingsStore(fakeSettingsClient({ saveGroup, switchGroup }));

    await store.load();
    await store.selectGroup(researchGroupId);
    await store.updateDraftGroup({ description: 'Keep dirty edit' });
    await store.switchToSelectedGroup();

    expect(saveGroup).toHaveBeenCalledTimes(1);
    expect(switchGroup).not.toHaveBeenCalled();
    expect(get(store).draftGroup?.description).toBe('Keep dirty edit');
    expect(get(store).message).toEqual({ tone: 'error', text: 'groups.json is locked' });
  });

  it('Given command failures When group save and delete run Then the draft and list remain recoverable', async () => {
    const saveGroup = vi.fn<SettingsCommandClient['saveGroup']>().mockRejectedValue({ code: 'saveGroupsFailed', message: 'Save failed.', detail: 'groups.json is locked' });
    const deleteGroup = vi.fn<SettingsCommandClient['deleteGroup']>().mockRejectedValue({ code: 'saveGroupsFailed', message: 'Delete failed.', detail: 'groups.json is locked' });
    const store = createSettingsStore(fakeSettingsClient({ saveGroup, deleteGroup }));

    await store.load();
    await store.updateDraftGroup({ description: 'Unsaved edit' });
    await store.saveDraftGroup();
    expect(get(store).draftGroup?.description).toBe('Unsaved edit');
    expect(get(store).message).toEqual({ tone: 'error', text: 'groups.json is locked' });

    await store.deleteSelectedGroup();
    expect(get(store).groups).toHaveLength(2);
    expect(get(store).message).toEqual({ tone: 'error', text: 'groups.json is locked' });
  });

  it('Given exact model references When replacing Then only exact matches are updated', async () => {
    const store = createSettingsStore(fakeSettingsClient());

    await store.load();
    await store.setMatchSearch('anthropic/claude-sonnet-4');
    await store.setMatchReplace('openai/gpt-5.2');
    await store.replaceExactMatches();

    expect(get(store).matchCounts.categoryMappings).toBe(0);
    expect(get(store).draftGroup?.categoryMappings[0]?.modelRef).toBe('openai/gpt-5.2');
  });

  it('Given a save in flight When the same draft is edited later Then the delayed save response does not overwrite the newer edit', async () => {
    let resolveSave: ((value: GroupMutationResponse) => void) | undefined;
    let submittedGroup: ModelGroup | undefined;
    const saveGroup = vi.fn<SettingsCommandClient['saveGroup']>().mockImplementation((group) => {
      submittedGroup = group;
      return new Promise((resolve) => { resolveSave = resolve; });
    });
    const store = createSettingsStore(fakeSettingsClient({ saveGroup }));

    await store.load();
    await store.updateDraftGroup({ description: 'Submitted edit' });
    const saving = store.saveDraftGroup();
    await store.updateDraftGroup({ description: 'Newer local edit' });
    if (!resolveSave || !submittedGroup) throw new Error('Expected a pending group save.');
    resolveSave({ group: submittedGroup, groups: [submittedGroup, researchGroup], appState: baseAppState });
    await saving;

    expect(get(store).draftGroup?.description).toBe('Newer local edit');
  });

  it('Given a save in flight When another group is selected Then the delayed save response does not hijack the newer selection', async () => {
    let resolveSave: ((value: GroupMutationResponse) => void) | undefined;
    let submittedGroup: ModelGroup | undefined;
    const saveGroup = vi.fn<SettingsCommandClient['saveGroup']>().mockImplementation((group) => {
      submittedGroup = group;
      return new Promise((resolve) => { resolveSave = resolve; });
    });
    const store = createSettingsStore(fakeSettingsClient({ saveGroup }));

    await store.load();
    await store.updateDraftGroup({ description: 'Submitted edit' });
    const saving = store.saveDraftGroup();
    await store.selectGroup(researchGroupId);
    if (!resolveSave || !submittedGroup) throw new Error('Expected a pending group save.');
    resolveSave({ group: submittedGroup, groups: [submittedGroup, researchGroup], appState: baseAppState });
    await saving;

    expect(get(store).draftGroup?.id).toBe(researchGroupId);
  });

  it('Given exact references across all sections When replacing twice Then each operation uses the refreshed draft', async () => {
    const store = createSettingsStore(fakeSettingsClient());

    await store.load();
    await store.updateDraftGroup({
      categoryMappings: [{ categoryName: 'build', modelRef: ' shared/model ' }],
      agentOverrides: [{ agentName: 'planner', modelRef: 'shared/model' }],
      openCodeAgentOverrides: [{ agentName: 'reviewer', modelRef: 'shared/model' }]
    });
    await store.setMatchSearch('shared/model');
    store.setMatchReplace('next/model');
    await store.replaceExactMatches();
    await store.setMatchSearch('next/model');
    store.setMatchReplace('final/model');
    await store.replaceExactMatches();

    const draft = get(store).draftGroup;
    expect(draft?.categoryMappings[0]?.modelRef).toBe('final/model');
    expect(draft?.agentOverrides[0]?.modelRef).toBe('final/model');
    expect(draft?.openCodeAgentOverrides[0]?.modelRef).toBe('final/model');
    expect(get(store).message?.text).toBe('Replaced 3 exact model references.');
  });

  it('Given OpenCode discovery failure When loading Then saved overrides are preserved read-only', async () => {
    const store = createSettingsStore(fakeSettingsClient({ discoveryError: 'OpenCode config is malformed.' }));

    await store.load();

    const presentation = get(store).openCodePresentation;
    expect(presentation.isReadOnly).toBe(true);
    expect(presentation.preservedOverrides[0]?.agentName).toBe('reviewer');
    expect(presentation.discoveryError).toBe('OpenCode config is malformed.');
  });

  it('Given valid group data with degraded discovery When loading Then preserved overrides remain read-only', async () => {
    const store = createSettingsStore(fakeSettingsClient({ discoveryError: 'OpenCode config is malformed.' }));

    await store.load();

    const state = get(store);
    expect(state.openCodePresentation.preservedOverrides).toEqual([
      expect.objectContaining({ agentName: 'reviewer', modelRef: 'anthropic/claude-opus-4' })
    ]);
    expect(state.canSaveDraft).toBe(false);
  });

  it('Given OpenCode discovery command rejection When loading Then saved overrides remain read-only and settings stay usable', async () => {
    const discoverOpenCodeAgents = vi.fn<SettingsCommandClient['discoverOpenCodeAgents']>()
      .mockRejectedValue({ code: 'malformedOpenCodeConfig', message: 'OpenCode discovery failed.', detail: 'Malformed OpenCode config.' });
    const store = createSettingsStore(fakeSettingsClient({ discoverOpenCodeAgents }));

    await store.load();

    const state = get(store);
    expect(state.status).toBe('ready');
    expect(state.openCodePresentation.isReadOnly).toBe(true);
    expect(state.openCodePresentation.preservedOverrides[0]?.modelRef).toBe('anthropic/claude-opus-4');
    expect(state.openCodePresentation.discoveryError).toBe('Malformed OpenCode config.');
  });

  it('Given unsaved non-OpenCode edits and degraded discovery When saving is denied Then the full draft survives', async () => {
    const saveGroup = vi.fn<SettingsCommandClient['saveGroup']>().mockRejectedValue({ code: 'saveGroupsFailed', message: 'Save failed.', detail: 'groups.json is read-only' });
    const store = createSettingsStore(fakeSettingsClient({ discoveryError: 'OpenCode config is malformed.', saveGroup }));

    await store.load();
    await store.updateDraftGroup({
      description: 'Keep this draft',
      categoryMappings: [{ categoryName: 'build', modelRef: 'draft/model' }]
    });
    await store.saveDraftGroup();

    const state = get(store);
    expect(saveGroup).toHaveBeenCalledTimes(1);
    expect(state.draftGroup?.description).toBe('Keep this draft');
    expect(state.draftGroup?.categoryMappings[0]?.modelRef).toBe('draft/model');
    expect(state.draftGroup?.openCodeAgentOverrides[0]?.modelRef).toBe('anthropic/claude-opus-4');
    expect(state.openCodePresentation.isReadOnly).toBe(true);
  });

  it('Given a copied group When mutation completes Then OpenCode presentation refreshes for the copied draft', async () => {
    const discoverOpenCodeAgents = vi.fn<SettingsCommandClient['discoverOpenCodeAgents']>()
      .mockImplementation(async (savedOverrides) => fakeSettingsClient().discoverOpenCodeAgents(savedOverrides));
    const store = createSettingsStore(fakeSettingsClient({ discoverOpenCodeAgents }));

    await store.load();
    await store.copySelectedGroup();

    expect(discoverOpenCodeAgents).toHaveBeenCalledTimes(2);
    expect(get(store).openCodePresentation.discoveredRows.find((row) => row.agentName === 'reviewer')?.modelRef).toBe('anthropic/claude-opus-4');
  });

});

const discoveryResponse = (agentName: string, modelRef: string): DiscoverOpenCodeAgentsResponse => ({
  agentNames: [agentName],
  error: null,
  presentation: {
    discoveredRows: [{ id: `discovered:${agentName}`, agentName, modelRef, isEditable: true }],
    staleOverrides: [],
    preservedOverrides: [],
    discoveryError: null,
    isReadOnly: false,
    allowsCustomAgentCreation: false
  }
});
