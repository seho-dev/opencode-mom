<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';
  import { cn, type WithElementRef } from '$lib/utils.js';

  let {
    ref = $bindable(null),
    children,
    class: className,
    variant = 'default',
    type = 'button',
    ...restProps
  }: WithElementRef<HTMLButtonAttributes> & { children?: Snippet; variant?: 'default' | 'destructive' } = $props();
</script>

<button
  bind:this={ref}
  data-slot="context-menu-item"
  role="menuitem"
  {type}
  class={cn(
    'flex h-8 w-full items-center rounded-[2px] px-2 text-left outline-none transition-colors hover:bg-[#1e1e1e] focus-visible:bg-[#1e1e1e] focus-visible:outline-none disabled:pointer-events-none disabled:opacity-[var(--disabled-opacity)]',
    variant === 'destructive' &&
      'text-[var(--status-error)] hover:bg-[var(--status-error)]/10 focus-visible:bg-[var(--status-error)]/10',
    className,
  )}
  {...restProps}
>
  {@render children?.()}
</button>
