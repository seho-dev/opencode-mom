<script lang="ts">
  import { Checkbox } from '$lib/components/ui/checkbox';
  import { Input } from '$lib/components/ui/input';
  import { Textarea } from '$lib/components/ui/textarea';
  import type { ModelGroup } from '../../../contracts';

  type Props = Readonly<{
    draft: ModelGroup;
    validationMessage: string | null;
    onPatch: (patch: Partial<ModelGroup>) => void;
  }>;

  const { draft, validationMessage, onPatch }: Props = $props();
</script>

<section class="field-section" data-group-region="metadata">
  <div class="section-heading">
    <h3>Group Metadata</h3>
    <span class="code-value">Updated {new Date(draft.updatedAt).toLocaleString()}</span>
  </div>
  <label>
    Name
    <Input value={draft.name} oninput={(event) => onPatch({ name: event.currentTarget.value })} />
  </label>
  {#if validationMessage}
    <p class="error-text">{validationMessage}</p>
  {/if}
  <label>
    Description
    <Textarea rows={2} value={draft.description ?? ''} oninput={(event) => onPatch({ description: event.currentTarget.value })} />
  </label>
  <label class="toggle-row">
    <Checkbox checked={draft.isEnabled} onCheckedChange={(checked) => onPatch({ isEnabled: checked === true })} />
    Enabled
  </label>
</section>

<style>
  .field-section { display: grid; gap: var(--space-3); border-bottom: var(--border-width) solid var(--border-subtle); padding-bottom: var(--space-4); }
  .section-heading { display: flex; min-width: 0; align-items: flex-start; justify-content: space-between; flex-wrap: wrap; gap: var(--space-2) var(--space-3); color: var(--text-secondary); }
  h3, p { margin: 0; }
  h3 { font-size: var(--font-section-size); font-weight: var(--font-section-weight); line-height: var(--font-section-line); }
  label { display: grid; gap: var(--space-1); color: var(--text-primary); font-size: var(--font-row-size); font-weight: var(--font-row-weight); }
  .toggle-row { display: flex; align-items: center; gap: var(--space-2); }
  .error-text { color: var(--status-error); font-size: var(--font-small-size); line-height: var(--font-small-line); }
  .code-value { overflow-wrap: anywhere; font-family: var(--font-mono); font-size: var(--font-code-size); line-height: var(--font-code-line); }
</style>
