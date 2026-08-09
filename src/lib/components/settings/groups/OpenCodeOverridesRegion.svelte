<script lang="ts">
  import { Alert } from '$lib/components/ui/alert';
  import { Input } from '$lib/components/ui/input';
  import type { ModelGroup, OpenCodeAgentMappingPresentation } from '../../../contracts';

  type Props = Readonly<{
    draft: ModelGroup;
    presentation: OpenCodeAgentMappingPresentation;
    onPatch: (patch: Partial<ModelGroup>) => void;
  }>;

  const { draft, presentation, onPatch }: Props = $props();
  const count = $derived(presentation.isReadOnly ? presentation.preservedOverrides.length : presentation.discoveredRows.length);
  const updateOverride = (agentName: string, modelRef: string): void => {
    const trimmed = modelRef.trim();
    const remaining = draft.openCodeAgentOverrides.filter((override) => override.agentName !== agentName);
    onPatch({ openCodeAgentOverrides: trimmed ? [...remaining, { agentName, modelRef: trimmed }] : remaining });
  };
</script>

<section class:degraded={presentation.isReadOnly} class="field-section" data-group-region="opencode-overrides" data-opencode-mode={presentation.isReadOnly ? 'degraded' : 'editable'}>
  <div class="section-heading"><h3>OpenCode Agent Overrides</h3><span aria-label="OpenCode override count">{count}</span></div>
  {#if presentation.isReadOnly}
    <Alert variant="warning"><strong>OpenCode agent discovery warning: {presentation.discoveryError}</strong><span>OpenCode agent overrides are read-only until discovery succeeds. Saved overrides are preserved.</span></Alert>
    {#each presentation.preservedOverrides as row}
      <div class="preserved-row">
        <label>Agent<Input readonly value={row.agentName || 'Unnamed OpenCode agent'} /></label>
        <label>Model<Input readonly class="code-value" value={row.modelRef || 'Optional'} /></label>
        <p><strong>{row.status}</strong><span>{row.message}</span></p>
      </div>
    {/each}
  {:else if presentation.discoveredRows.length === 0}
    <p class="helper">No OpenCode agents discovered.</p>
  {:else}
    {#each presentation.discoveredRows as row}
      <div class="mapping-row"><span>{row.agentName}</span><Input aria-label={`OpenCode model ${row.agentName}`} value={draft.openCodeAgentOverrides.find((override) => override.agentName === row.agentName)?.modelRef ?? ''} placeholder="Model Ref" oninput={(event) => updateOverride(row.agentName, event.currentTarget.value)} /></div>
    {/each}
  {/if}
</section>

<style>
  .field-section { display: grid; gap: var(--space-3); border-bottom: var(--border-width) solid var(--border-subtle); padding-bottom: var(--space-4); }
  .field-section.degraded { border: var(--border-width) solid var(--status-warning); border-radius: var(--radius-lg); padding: var(--space-3); background: var(--surface-muted); }
  .section-heading { display: flex; min-width: 0; align-items: flex-start; justify-content: space-between; flex-wrap: wrap; gap: var(--space-2) var(--space-3); color: var(--text-secondary); }
  h3, p { margin: 0; } h3 { font-size: var(--font-section-size); font-weight: var(--font-section-weight); line-height: var(--font-section-line); }
  label { display: grid; gap: var(--space-1); color: var(--text-primary); font-size: var(--font-row-size); font-weight: var(--font-row-weight); }
  .mapping-row, .preserved-row { display: grid; min-width: 0; grid-template-columns: minmax(120px, 1fr) minmax(160px, 1fr) auto; align-items: end; gap: var(--space-2); }
  .preserved-row p { display: grid; grid-column: 1 / -1; gap: var(--space-1); color: var(--text-secondary); }
  .helper { color: var(--text-secondary); font-size: var(--font-small-size); line-height: var(--font-small-line); }
  @container groups-pane (width < 1040px) { .mapping-row { grid-template-columns: 1fr; } }
  @container groups-pane (width < 520px) { .preserved-row { grid-template-columns: 1fr; } }
</style>
