<script lang="ts">
  import { CheckCircle2, ChevronRight } from '@lucide/svelte';
  import { tick } from 'svelte';

  import { Button } from '$lib/components/ui/button/index.js';
  import { Spinner } from '$lib/components/ui/spinner/index.js';
  import type { Uuid } from '../../contracts';
  import type { SwitchTarget } from '../../quickSwitchView';

  type Props = Readonly<{
    targets: readonly SwitchTarget[];
    hasEnabledGroups: boolean;
    emptyText: string;
    switchingGroupId: Uuid | null;
    onSwitch: (id: Uuid) => Promise<void>;
  }>;

  const { targets, hasEnabledGroups, emptyText, switchingGroupId, onSwitch }: Props = $props();

  const switchAndRestoreFocus = async (id: Uuid): Promise<void> => {
    await onSwitch(id);
    await tick();
    document.querySelector<HTMLElement>(`[data-group-id="${id}"] .focus-target`)?.focus();
  };
</script>

<section class="switch-section" aria-labelledby="switch-targets-title">
  <div class="section-heading">
    <p id="switch-targets-title" class="section-label">Switch To</p>
    {#if switchingGroupId}
      <span class="caption">Switching...</span>
    {/if}
  </div>

  {#if !hasEnabledGroups}
    <div class="empty-state">{emptyText}</div>
  {:else}
    <div class="target-list">
      {#each targets as target (target.id)}
        <article class:active={target.active} class="target-row" data-group-id={target.id}>
          {#if target.active}
            <span class="focus-target" tabindex="-1" aria-label={`${target.name}, active group`}></span>
          {/if}
          <div class="target-copy">
            <div class="target-title">
              {#if target.active}
                <CheckCircle2 size={12} aria-hidden="true" />
                <span class="active-label">Active</span>
              {/if}
              <span title={target.name}>{target.name}</span>
            </div>
            {#if target.description}
              <p title={target.description}>{target.description}</p>
            {/if}
          </div>
          {#if !target.active}
            <Button
              size="sm"
              variant="secondary"
              disabled={switchingGroupId !== null}
              aria-label={`Switch to ${target.name}`}
              aria-busy={switchingGroupId === target.id}
              onclick={() => switchAndRestoreFocus(target.id)}
            >
              {#if switchingGroupId === target.id}
                <Spinner size="sm" />
              {:else}
                <ChevronRight size={12} aria-hidden="true" />
              {/if}
              <span>Switch</span>
            </Button>
          {/if}
        </article>
      {/each}
    </div>
  {/if}
</section>

<style>
  .section-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
  }

  .section-label,
  .caption {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--font-caption-size);
    font-weight: var(--font-caption-weight);
    line-height: var(--font-caption-line);
  }

  .target-list {
    display: grid;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }

  .target-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: var(--target-row-min-height);
    gap: var(--space-2);
    border-radius: var(--radius-md);
    padding: var(--space-1) var(--space-2);
  }

  .target-row.active {
    background: var(--surface-active);
  }

  .target-row:not(.active):hover {
    background: var(--surface-hover);
  }

  .target-copy {
    flex: 1 1 auto;
    min-width: 0;
  }

  .target-title {
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }

  .target-title :global(svg) {
    color: var(--status-success);
  }

  .target-title span:not(.active-label) {
    min-width: 0;
    overflow: hidden;
    font-size: var(--font-row-size);
    font-weight: var(--font-row-weight);
    line-height: var(--font-row-line);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .active-label {
    position: absolute;
    overflow: hidden;
    width: var(--screen-reader-box);
    height: var(--screen-reader-box);
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }

  .focus-target {
    position: absolute;
    width: var(--screen-reader-box);
    height: var(--screen-reader-box);
    overflow: hidden;
  }

  .target-copy p {
    margin: 0;
    overflow: hidden;
    color: var(--text-secondary);
    font-size: var(--font-caption-size);
    line-height: var(--font-caption-line);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .empty-state {
    border: var(--border-width) solid var(--border-subtle);
    border-radius: var(--radius-md);
    background: var(--surface-muted);
    padding: var(--space-2);
    color: var(--text-secondary);
    font-size: var(--font-small-size);
    line-height: var(--font-small-line);
  }

</style>
