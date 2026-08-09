<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { matchCountTotal, type ExactModelMatchCounts } from '../../../settingsGroupDraft';

  type Props = Readonly<{
    search: string;
    replace: string;
    matchCounts: ExactModelMatchCounts;
    onSearch: (value: string) => void;
    onReplace: (value: string) => void;
    onReplaceExactMatches: () => void;
  }>;

  const { search, replace, matchCounts, onSearch, onReplace, onReplaceExactMatches }: Props = $props();
  const canReplace = $derived(matchCountTotal(matchCounts) > 0);
</script>

<section class="field-section" data-group-region="exact-model-replacement">
  <div class="section-heading">
    <h3>Current Group Model Batch Replace</h3>
    <span>{matchCountTotal(matchCounts)} matches</span>
  </div>
  <div class="replace-row">
    <label>Find exact model<Input value={search} oninput={(event) => onSearch(event.currentTarget.value)} /></label>
    <label>Replace with<Input value={replace} oninput={(event) => onReplace(event.currentTarget.value)} /></label>
    <Button variant="secondary" disabled={!canReplace} onclick={onReplaceExactMatches}>Replace All Exact Matches</Button>
  </div>
  <p class="helper">Category {matchCounts.categoryMappings}, agent {matchCounts.agentOverrides}, OpenCode {matchCounts.openCodeAgentOverrides}</p>
</section>

<style>
  .field-section { display: grid; gap: var(--space-3); border-bottom: var(--border-width) solid var(--border-subtle); padding-bottom: var(--space-4); }
  .section-heading { display: flex; min-width: 0; align-items: flex-start; justify-content: space-between; flex-wrap: wrap; gap: var(--space-2) var(--space-3); color: var(--text-secondary); }
  h3, p { margin: 0; }
  h3 { font-size: var(--font-section-size); font-weight: var(--font-section-weight); line-height: var(--font-section-line); }
  label { display: grid; gap: var(--space-1); color: var(--text-primary); font-size: var(--font-row-size); font-weight: var(--font-row-weight); }
  .replace-row { display: grid; min-width: 0; grid-template-columns: minmax(120px, 1fr) minmax(160px, 1fr) auto; align-items: end; gap: var(--space-2); }
  .helper { color: var(--text-secondary); font-size: var(--font-small-size); line-height: var(--font-small-line); }
  @container groups-pane (width < 1040px) { .replace-row { grid-template-columns: 1fr; } .section-heading > :last-child { text-align: left; } }
</style>
