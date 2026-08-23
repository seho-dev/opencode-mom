<script lang="ts">
  import type { Snippet } from 'svelte';

  let { variant = 'neutral', children }: { variant?: 'success' | 'neutral' | 'warning' | 'error'; children: Snippet } =
    $props();
</script>

<span class={`status-badge ${variant}`}>{@render children()}</span>
