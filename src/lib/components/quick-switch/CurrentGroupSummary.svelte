<script lang="ts">
  import type { ModelGroup } from '../../contracts';
  import CurrentValues from './CurrentValues.svelte';

  type Props = Readonly<{
    group: ModelGroup | null;
  }>;

  const { group }: Props = $props();
</script>

<section class="current-section" aria-labelledby="quick-switch-title" aria-label="Current group summary" tabindex="-1">
  <p class="section-label">Current Group</p>
  {#if group}
    <h1 id="quick-switch-title">{group.name}</h1>
    {#if group.description}
      <p class="description">{group.description}</p>
    {/if}
    <CurrentValues {group} />
  {:else}
    <h1 id="quick-switch-title" class="muted-title">None</h1>
    <p class="description">No group is selected.</p>
  {/if}
</section>

<style>
  .current-section {
    display: grid;
    min-width: 0;
    gap: var(--space-1);
  }

  .section-label {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--font-caption-size);
    font-weight: var(--font-caption-weight);
    line-height: var(--font-caption-line);
  }

  h1 {
    margin: 0;
    font-size: var(--font-section-size);
    font-weight: var(--font-section-weight);
    line-height: var(--font-section-line);
  }

  .muted-title,
  .description {
    color: var(--text-secondary);
  }

  .description {
    display: -webkit-box;
    margin: 0;
    overflow: hidden;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    font-size: var(--font-small-size);
    line-height: var(--font-small-line);
  }
</style>
