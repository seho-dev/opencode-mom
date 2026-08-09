import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';

import type { ModelGroup, OpenCodeAgentMappingPresentation } from '../../../contracts';
import { baseGroups } from '../../../settingsTestSupport';
import ExactModelReplacementRegion from './ExactModelReplacementRegion.svelte';
import GroupMetadataRegion from './GroupMetadataRegion.svelte';
import OpenCodeOverridesRegion from './OpenCodeOverridesRegion.svelte';
import StandardMappingsRegion from './StandardMappingsRegion.svelte';

const draft = baseGroups[0] as ModelGroup;
const editablePresentation: OpenCodeAgentMappingPresentation = {
  discoveredRows: [{ id: 'reviewer', agentName: 'reviewer', modelRef: '', isEditable: true }],
  staleOverrides: [], preservedOverrides: [], discoveryError: null, isReadOnly: false, allowsCustomAgentCreation: false
};

describe('Group draft regions', () => {
  afterEach(() => cleanup());

  it('emits metadata and exact replacement callbacks without owning store behavior', async () => {
    const onPatch = vi.fn();
    const onSearch = vi.fn();
    const onReplace = vi.fn();
    const onReplaceExactMatches = vi.fn();
    render(GroupMetadataRegion, { props: { draft, validationMessage: null, onPatch } });
    await fireEvent.input(screen.getByLabelText('Name'), { target: { value: 'Focused' } });
    expect(onPatch).toHaveBeenCalledWith({ name: 'Focused' });
    cleanup();
    render(ExactModelReplacementRegion, { props: { search: '', replace: '', matchCounts: { categoryMappings: 1, agentOverrides: 0, openCodeAgentOverrides: 0 }, onSearch, onReplace, onReplaceExactMatches } });
    await fireEvent.input(screen.getByLabelText('Find exact model'), { target: { value: 'source/model' } });
    await fireEvent.input(screen.getByLabelText('Replace with'), { target: { value: 'target/model' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Replace All Exact Matches' }));
    expect(onSearch).toHaveBeenCalledWith('source/model');
    expect(onReplace).toHaveBeenCalledWith('target/model');
    expect(onReplaceExactMatches).toHaveBeenCalledTimes(1);
  });

  it('emits standard mapping and editable OpenCode patches, while degraded values stay readable', async () => {
    const onCategoryPatch = vi.fn();
    const onAgentPatch = vi.fn();
    const onAppendCategory = vi.fn();
    const onAppendAgent = vi.fn();
    const onPatch = vi.fn();
    render(StandardMappingsRegion, { props: { categoryMappings: draft.categoryMappings, agentOverrides: draft.agentOverrides, onCategoryPatch, onAgentPatch, onAppendCategory, onAppendAgent } });
    await fireEvent.input(screen.getByLabelText('Category model 1'), { target: { value: 'next/model' } });
    await fireEvent.click(screen.getByRole('button', { name: 'Add Custom Agent' }));
    expect(onCategoryPatch).toHaveBeenCalledWith(0, { modelRef: 'next/model' });
    expect(onAppendAgent).toHaveBeenCalledTimes(1);
    cleanup();
    render(OpenCodeOverridesRegion, { props: { draft, presentation: editablePresentation, onPatch } });
    await fireEvent.input(screen.getByLabelText('OpenCode model reviewer'), { target: { value: 'openai/gpt-5.2' } });
    expect(onPatch).toHaveBeenCalledWith(expect.objectContaining({ openCodeAgentOverrides: expect.arrayContaining([{ agentName: 'reviewer', modelRef: 'openai/gpt-5.2' }]) }));
    cleanup();
    render(OpenCodeOverridesRegion, { props: { draft, presentation: { ...editablePresentation, isReadOnly: true, discoveryError: 'Discovery failed.', discoveredRows: [], preservedOverrides: [{ id: 'preserved:reviewer', agentName: 'reviewer', modelRef: 'saved/model', status: 'Preserved', message: 'Unavailable' }] }, onPatch } });
    const preserved = screen.getByDisplayValue('saved/model') as HTMLInputElement;
    expect(preserved.readOnly).toBe(true);
    expect(preserved.disabled).toBe(false);
    expect(screen.getByRole('alert')).toBeTruthy();
  });
});
