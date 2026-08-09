<script lang="ts">
  import type { ModelGroup, ModelGroupAgentOverride, ModelGroupCategoryMapping } from '../../contracts';

  type Props = Readonly<{
    group: ModelGroup;
  }>;

  const { group }: Props = $props();
</script>

<div class="value-sections">
  {@render AgentValueSection('Agent Overrides', group.agentOverrides)}
  {@render AgentValueSection('OpenCode Overrides', group.openCodeAgentOverrides)}
  {@render MappingValueSection(group.categoryMappings)}
</div>

{#snippet AgentValueSection(title: string, rows: readonly ModelGroupAgentOverride[])}
  {#if rows.length > 0}
    <section class="value-section" aria-label={title}>
      <p class="section-label">{title}</p>
      <div class="value-list">
        {#each rows as row (row.agentName)}
          <div class="value-row">
            <span title={row.agentName}>{row.agentName}</span>
            <code title={row.modelRef}>{row.modelRef}</code>
          </div>
        {/each}
      </div>
    </section>
  {/if}
{/snippet}

{#snippet MappingValueSection(rows: readonly ModelGroupCategoryMapping[])}
  {#if rows.length > 0}
    <section class="value-section" aria-label="Category Mappings">
      <p class="section-label">Category Mappings</p>
      <div class="value-list">
        {#each rows as row (row.categoryName)}
          <div class="value-row">
            <span title={row.categoryName}>{row.categoryName}</span>
            <code title={row.modelRef}>{row.modelRef}</code>
          </div>
        {/each}
      </div>
    </section>
  {/if}
{/snippet}

<style>
  .value-sections,
  .value-list {
    display: grid;
    gap: var(--space-2);
  }

  .value-sections {
    margin-top: var(--space-2);
  }

  .value-section {
    display: grid;
    gap: var(--space-1);
  }

  .section-label {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--font-caption-size);
    font-weight: var(--font-caption-weight);
    line-height: var(--font-caption-line);
  }

  .value-row {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    min-height: var(--value-row-min-height);
    gap: var(--space-2);
  }

  .value-row span {
    flex: 1 1 40%;
    min-width: 0;
    overflow: hidden;
    font-size: var(--font-row-size);
    font-weight: var(--font-row-weight);
    line-height: var(--font-row-line);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  code {
    display: block;
    flex: 1 1 60%;
    min-width: 0;
    max-width: min(var(--model-ref-max-width), 60%);
    overflow: hidden;
    color: var(--text-secondary);
    font-family: var(--font-mono);
    font-size: var(--font-code-size);
    line-height: var(--font-code-line);
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
