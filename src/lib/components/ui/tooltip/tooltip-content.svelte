<script lang="ts">
  import { Tooltip as TooltipPrimitive } from 'bits-ui';
  import { cn } from '$lib/utils.js';
  import TooltipPortal from './tooltip-portal.svelte';
  import type { ComponentProps } from 'svelte';
  import type { WithoutChildrenOrChild } from '$lib/utils.js';

  let {
    ref = $bindable(null),
    class: className,
    role = 'tooltip',
    sideOffset = 4,
    side = 'top',
    children,
    portalProps,
    ...restProps
  }: TooltipPrimitive.ContentProps & {
    portalProps?: WithoutChildrenOrChild<ComponentProps<typeof TooltipPortal>>;
  } = $props();
</script>

<TooltipPortal {...portalProps}>
  <TooltipPrimitive.Content
    bind:ref
    data-slot="tooltip-content"
    {role}
    {sideOffset}
    {side}
    class={cn(
      'relative z-50 inline-flex w-fit max-w-xs origin-(--bits-tooltip-content-transform-origin) items-center rounded-[2px] border border-[var(--border-default)] bg-[#0e0e0e] px-2 py-1 text-[11px] leading-4 text-[var(--text-primary)] after:absolute after:size-2 after:rotate-45 after:border-r after:border-b after:border-[var(--border-default)] after:bg-[#0e0e0e] data-[side=top]:after:-bottom-1 data-[side=bottom]:after:-top-1 data-[side=left]:after:-right-1 data-[side=right]:after:-left-1 data-open:animate-in data-open:fade-in-0 data-open:zoom-in-95 data-closed:animate-out data-closed:fade-out-0 data-closed:zoom-out-95 duration-100 motion-reduce:animate-none motion-reduce:transition-none',
      className,
    )}
    {...restProps}
  >
    {@render children?.()}
  </TooltipPrimitive.Content>
</TooltipPortal>
