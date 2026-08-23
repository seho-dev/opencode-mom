<script lang="ts">
  import type { Snippet } from 'svelte';

  let { label, children }: { label: string; children: Snippet } = $props();
</script>

<section class="panel table-wrap" aria-label={label}>
  <table class="data-table">{@render children()}</table>
</section>
