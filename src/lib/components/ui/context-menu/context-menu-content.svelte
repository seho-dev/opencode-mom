<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';
  import { cn, type WithElementRef } from '$lib/utils.js';

  let {
    ref = $bindable(null),
    children,
    class: className,
    ...restProps
  }: WithElementRef<HTMLAttributes<HTMLDivElement>> & { children?: Snippet } = $props();
</script>

<div
  bind:this={ref}
  data-slot="context-menu-content"
  role="menu"
  tabindex="-1"
  class={cn(
    'min-w-40 rounded-[2px] border border-[var(--border-default)] bg-[#121212] p-1 text-[12px] text-[var(--text-primary)]',
    className,
  )}
  {...restProps}
>
  {@render children?.()}
</div>
