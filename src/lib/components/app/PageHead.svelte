<script lang="ts">
  import type { Snippet } from 'svelte';

  let { eyebrow, title, actions, children }: { eyebrow: string; title: string; actions?: Snippet; children?: Snippet } =
    $props();
</script>

<div class="page-head">
  <div>
    <p class="eyebrow">{eyebrow}</p>
    <h1>{title}</h1>
  </div>
  {#if actions}
    <div class="page-head-actions">{@render actions()}</div>
  {:else if children}
    <div class="page-head-actions">{@render children()}</div>
  {/if}
</div>
