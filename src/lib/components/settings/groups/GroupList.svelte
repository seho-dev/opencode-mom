<script lang="ts">
  import Copy from '@lucide/svelte/icons/copy';
  import Plus from '@lucide/svelte/icons/plus';
  import { Badge } from '$lib/components/ui/badge';
  import { Button } from '$lib/components/ui/button';
  import * as Tooltip from '$lib/components/ui/tooltip';
  import type { SettingsState, SettingsStore } from '../../../settingsStore';

  type Props = Readonly<{ store: SettingsStore; state: SettingsState }>;
  let { store, state: viewState }: Props = $props();
  const currentGroupId = $derived(viewState.appState?.selectedGroupID ?? null);
</script>

<aside class="group-list" aria-label="Groups">
  <Tooltip.Provider delayDuration={300}>
  <div class="group-list-toolbar">
    <h2 id="group-settings-title">Group Settings</h2>
    <Tooltip.Root>
      <Tooltip.Trigger>
        {#snippet child({ props })}
          <Button variant="ghost" size="icon" aria-label="New Group" {...props} onclick={() => store.createGroup()}>
            <Plus size={16} aria-hidden="true" />
          </Button>
        {/snippet}
      </Tooltip.Trigger>
      <Tooltip.Content>New Group</Tooltip.Content>
    </Tooltip.Root>
  </div>

  {#if viewState.groups.length === 0}
    <div class="empty-state">
      <p>No groups yet.</p>
      <Button variant="secondary" onclick={() => store.createGroup()}>New Group</Button>
    </div>
  {:else}
    <div class="group-list-scroll">
      {#each viewState.groups as group}
        <div class="group-row" class:selected={viewState.selectedGroupId === group.id}>
          <Button variant="ghost" class="group-select" aria-current={viewState.selectedGroupId === group.id ? 'page' : undefined} onclick={() => store.selectGroup(group.id)}>
            <span class:enabled={group.isEnabled} class="status-dot" aria-hidden="true"></span>
            <span class="group-row-main">
              <span class="group-name">{group.name}</span>
              {#if group.id === currentGroupId}<Badge variant="success">Active</Badge>{/if}
              {#if group.id === viewState.draftGroup?.id && !viewState.draftIsPersisted}<Badge variant="warning">Draft</Badge>{/if}
            </span>
          </Button>
          <Tooltip.Root>
            <Tooltip.Trigger>
              {#snippet child({ props })}
                <Button variant="ghost" size="icon-sm" aria-label="Copy group" {...props} onclick={() => { void store.selectGroup(group.id).then(() => store.copySelectedGroup()); }}>
                  <Copy size={14} aria-hidden="true" />
                </Button>
              {/snippet}
            </Tooltip.Trigger>
            <Tooltip.Content>Copy group</Tooltip.Content>
          </Tooltip.Root>
        </div>
      {/each}
    </div>
  {/if}
  </Tooltip.Provider>
</aside>

<style>
  .group-list {
    display: flex;
    min-width: 0;
    min-height: 0;
    flex-direction: column;
    border-right: var(--border-width) solid var(--border-default);
    background: var(--surface-muted);
  }

  .group-list-toolbar {
    display: flex;
    height: 44px;
    flex: 0 0 44px;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    border-bottom: var(--border-width) solid var(--border-default);
    padding: var(--space-2) var(--space-3);
  }

  h2,
  p {
    margin: 0;
  }

  h2 {
    font-size: var(--font-section-size);
    font-weight: var(--font-section-weight);
    line-height: var(--font-section-line);
  }

  .group-list-scroll {
    display: grid;
    min-width: 0;
    min-height: 0;
    flex: 1;
    gap: var(--space-1);
    overflow: auto;
    padding: var(--space-2);
  }

  .group-row {
    display: flex;
    min-height: 34px;
    align-items: center;
    gap: var(--space-1);
    border-radius: var(--radius-md);
    padding: var(--space-1);
    color: var(--text-primary);
  }

  .group-row:hover,
  .group-row.selected {
    background: var(--surface-active);
  }

  :global(.group-select) {
    min-width: 0;
    flex: 1;
    justify-content: flex-start;
    overflow: hidden;
    text-align: left;
  }

  .group-row-main {
    display: flex;
    min-width: 0;
    flex: 1;
    align-items: center;
    gap: var(--space-2);
  }

  .group-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .status-dot {
    width: var(--icon-sm);
    height: var(--icon-sm);
    flex: 0 0 auto;
    border-radius: var(--radius-pill);
    background: var(--text-muted);
  }

  .status-dot.enabled {
    background: var(--status-success);
  }

  .empty-state {
    display: grid;
    min-height: 0;
    flex: 1;
    place-content: center;
    gap: var(--space-2);
    padding: var(--space-8);
    color: var(--text-secondary);
    text-align: center;
  }

  @container groups-pane (width < 1040px) {
    .group-list {
      border-right: 0;
      border-bottom: var(--border-width) solid var(--border-default);
    }
  }
</style>
