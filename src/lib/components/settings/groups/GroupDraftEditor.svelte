<script lang="ts">
  import Plus from '@lucide/svelte/icons/plus';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
  import Save from '@lucide/svelte/icons/save';
  import Zap from '@lucide/svelte/icons/zap';
  import { Button } from '$lib/components/ui/button';
  import { Separator } from '$lib/components/ui/separator';
  import type { ModelGroup, ModelGroupAgentOverride, ModelGroupCategoryMapping } from '../../../contracts';
  import { updateAgentOverride, updateCategoryMapping } from '../../../settingsGroupDraft';
  import type { SettingsState, SettingsStore } from '../../../settingsStore';
  import DeleteGroupDialog from './DeleteGroupDialog.svelte';
  import ExactModelReplacementRegion from './ExactModelReplacementRegion.svelte';
  import GroupMetadataRegion from './GroupMetadataRegion.svelte';
  import OpenCodeOverridesRegion from './OpenCodeOverridesRegion.svelte';
  import StandardMappingsRegion from './StandardMappingsRegion.svelte';

  type Props = Readonly<{ store: SettingsStore; state: SettingsState }>;
  let { store, state: viewState }: Props = $props();
  const currentGroupId = $derived(viewState.appState?.selectedGroupID ?? null);
  const draft = $derived(viewState.draftGroup);

  const patchDraft = (patch: Partial<ModelGroup>) => {
    void store.updateDraftGroup(patch);
  };

  const updateCategory = (index: number, patch: Partial<ModelGroupCategoryMapping>) => {
    if (draft) {
      patchDraft({ categoryMappings: updateCategoryMapping(draft.categoryMappings, index, patch) });
    }
  };

  const updateAgent = (index: number, patch: Partial<ModelGroupAgentOverride>) => {
    if (draft) {
      patchDraft({ agentOverrides: updateAgentOverride(draft.agentOverrides, index, patch) });
    }
  };

  const appendCategory = () => {
    if (draft) {
      patchDraft({ categoryMappings: [...draft.categoryMappings, { categoryName: '', modelRef: '' }] });
    }
  };

  const appendAgent = () => {
    if (draft) {
      patchDraft({ agentOverrides: [...draft.agentOverrides, { agentName: '', modelRef: '' }] });
    }
  };
</script>

<section class="editor">
  <div class="toolbar">
    <Button variant="secondary" onclick={() => store.createGroup()}><Plus size={16} aria-hidden="true" />New Group</Button>
    <Button disabled={!viewState.canSaveDraft} onclick={() => store.saveDraftGroup()}><Save size={16} aria-hidden="true" />Save</Button>
    <Button variant="secondary" disabled={!draft} onclick={() => store.cancelDraftGroup()}><RotateCcw size={16} aria-hidden="true" />Cancel</Button>
    {#if draft}
      <DeleteGroupDialog groupName={draft.name} disabled={!draft} onDelete={() => store.deleteSelectedGroup()} />
    {:else}
      <Button variant="destructive" disabled>Delete</Button>
    {/if}
    {#if draft && draft.id !== currentGroupId}
      <Button
        variant="secondary"
        disabled={!viewState.canSwitchDraft}
        onclick={() => store.switchToSelectedGroup()}
      >
        <Zap size={16} aria-hidden="true" />
        Switch To This Group
      </Button>
    {/if}
  </div>

  {#if !draft}
    <div class="empty-detail">
      <h2>Select a group</h2>
      <p>Choose a group to edit or create a new one.</p>
      <Button variant="secondary" onclick={() => store.createGroup()}>New Group</Button>
    </div>
  {:else}
    <div class="editor-scroll">
      <GroupMetadataRegion draft={draft} validationMessage={viewState.groupValidationMessage} onPatch={patchDraft} />
      <ExactModelReplacementRegion search={viewState.matchSearch} replace={viewState.matchReplace} matchCounts={viewState.matchCounts} onSearch={(value) => void store.setMatchSearch(value)} onReplace={store.setMatchReplace} onReplaceExactMatches={() => void store.replaceExactMatches()} />
      <StandardMappingsRegion categoryMappings={draft.categoryMappings} agentOverrides={draft.agentOverrides} onCategoryPatch={updateCategory} onAgentPatch={updateAgent} onAppendCategory={appendCategory} onAppendAgent={appendAgent} />
      <OpenCodeOverridesRegion draft={draft} presentation={viewState.openCodePresentation} onPatch={patchDraft} />
      <Separator />
    </div>
  {/if}
</section>

<style>
  .editor {
    display: flex;
    min-width: 0;
    min-height: 0;
    flex-direction: column;
  }

  .toolbar {
    display: flex;
    min-height: 44px;
    flex: 0 0 auto;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-2);
    border-bottom: var(--border-width) solid var(--border-default);
    padding: var(--space-2) var(--space-3);
    background: var(--surface-muted);
  }

  .editor-scroll {
    display: grid;
    min-width: 0;
    min-height: 0;
    align-content: start;
    gap: var(--space-4);
    overflow: auto;
    padding: var(--space-5);
  }


  .empty-detail {
    display: grid;
    min-height: 0;
    flex: 1;
    place-content: center;
    gap: var(--space-2);
    padding: var(--space-8);
    color: var(--text-secondary);
    text-align: center;
  }

  @container groups-pane (width < 520px) {
    .toolbar { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); align-items: stretch; }

    .toolbar :global(button) { width: 100%; min-width: 0; height: auto; overflow-wrap: anywhere; white-space: normal; }

    .toolbar :global(button:nth-child(5)) { grid-column: 1 / -1; }

  }
</style>
