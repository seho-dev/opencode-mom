<script lang="ts">
import type { HTMLAttributes } from 'svelte/elements';
import { cn, type WithElementRef } from '$lib/utils.js';

let {
  ref = $bindable(null),
  class: className,
  ...restProps
}: WithElementRef<HTMLAttributes<HTMLDivElement>> = $props();
</script>

<!-- biome-ignore lint/a11y/useSemanticElements: styled div with an explicit separator role keeps the headless primitive's div ref type and attribute contract stable -->
<div
  bind:this={ref}
  data-slot="context-menu-separator"
  role="separator"
  class={cn('my-1 h-px bg-[var(--border-subtle)]', className)}
  {...restProps}
></div>
