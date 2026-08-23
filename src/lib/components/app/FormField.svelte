<script lang="ts">
  import type { Snippet } from 'svelte';

  let {
    label,
    for: labelFor,
    control,
    hint,
    full = false,
  }: { label: string; for: string; control: Snippet; hint?: Snippet; full?: boolean } = $props();
</script>

<div class:full class="field">
  <label for={labelFor}>{label}</label>
  {@render control()}
  {#if hint}<div class="field-hint">{@render hint()}</div>{/if}
</div>
