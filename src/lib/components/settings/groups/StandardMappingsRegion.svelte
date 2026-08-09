<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import type { ModelGroupAgentOverride, ModelGroupCategoryMapping } from '../../../contracts';

  type Props = Readonly<{
    categoryMappings: readonly ModelGroupCategoryMapping[];
    agentOverrides: readonly ModelGroupAgentOverride[];
    onCategoryPatch: (index: number, patch: Partial<ModelGroupCategoryMapping>) => void;
    onAgentPatch: (index: number, patch: Partial<ModelGroupAgentOverride>) => void;
    onAppendCategory: () => void;
    onAppendAgent: () => void;
  }>;

  const { categoryMappings, agentOverrides, onCategoryPatch, onAgentPatch, onAppendCategory, onAppendAgent }: Props = $props();
</script>

<section class="field-section" data-group-region="standard-mappings">
  <div class="section-heading"><h3>Category Mappings</h3><span>{categoryMappings.length}</span></div>
  {#each categoryMappings as mapping, index}
    <div class="mapping-row">
      <Input aria-label={`Category name ${index + 1}`} value={mapping.categoryName} placeholder="Category Name" oninput={(event) => onCategoryPatch(index, { categoryName: event.currentTarget.value })} />
      <Input aria-label={`Category model ${index + 1}`} value={mapping.modelRef} placeholder="Model Ref" oninput={(event) => onCategoryPatch(index, { modelRef: event.currentTarget.value })} />
    </div>
  {/each}
  <Button variant="secondary" onclick={onAppendCategory}>Add Custom Category</Button>
</section>

<section class="field-section" data-group-region="standard-mappings">
  <div class="section-heading"><h3>Agent Overrides</h3><span>{agentOverrides.length}</span></div>
  {#each agentOverrides as override, index}
    <div class="mapping-row">
      <Input aria-label={`Agent name ${index + 1}`} value={override.agentName} placeholder="Agent Name" oninput={(event) => onAgentPatch(index, { agentName: event.currentTarget.value })} />
      <Input aria-label={`Agent model ${index + 1}`} value={override.modelRef} placeholder="Model Ref" oninput={(event) => onAgentPatch(index, { modelRef: event.currentTarget.value })} />
    </div>
  {/each}
  <Button variant="secondary" onclick={onAppendAgent}>Add Custom Agent</Button>
</section>

<style>
  .field-section { display: grid; gap: var(--space-3); border-bottom: var(--border-width) solid var(--border-subtle); padding-bottom: var(--space-4); }
  .section-heading { display: flex; min-width: 0; align-items: flex-start; justify-content: space-between; flex-wrap: wrap; gap: var(--space-2) var(--space-3); color: var(--text-secondary); }
  h3 { margin: 0; font-size: var(--font-section-size); font-weight: var(--font-section-weight); line-height: var(--font-section-line); }
  .mapping-row { display: grid; min-width: 0; grid-template-columns: minmax(120px, 1fr) minmax(160px, 1fr) auto; align-items: end; gap: var(--space-2); }
  @container groups-pane (width < 1040px) { .mapping-row { grid-template-columns: 1fr; } .section-heading > :last-child { text-align: left; } }
</style>
