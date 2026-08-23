<script lang="ts">
  import type { HTMLAttributes } from 'svelte/elements';
  import { cn, type WithElementRef } from '$lib/utils.js';

  let {
    ref = $bindable(null),
    class: className,
    children,
    ...restProps
  }: WithElementRef<HTMLAttributes<HTMLDivElement>> = $props();
</script>

<div
  bind:this={ref}
  data-slot="alert-description"
  class={cn('min-w-0 text-[12px] leading-5 text-[var(--text-secondary)] group-has-[>svg]/alert:col-start-2', className)}
  {...restProps}
>
  {@render children?.()}
</div>
