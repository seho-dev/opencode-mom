<script lang="ts" module>
  import { type VariantProps, tv } from 'tailwind-variants';

  export const toastVariants = tv({
    base: 'grid w-full max-w-sm grid-cols-[minmax(0,1fr)_auto] gap-3 rounded-[2px] border bg-[#121212] p-3 text-[12px] leading-5',
    variants: {
      variant: {
        info: 'border-[#00e5ff]/45 text-[#00e5ff]',
        success: 'border-[#39ff14]/45 text-[#39ff14]',
        warning: 'border-[#f0b429]/45 text-[#f0b429]',
        error: 'border-[var(--status-error)]/50 text-[var(--status-error)]',
      },
    },
    defaultVariants: { variant: 'info' },
  });
  export type ToastVariant = VariantProps<typeof toastVariants>['variant'];
</script>

<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';
  import { cn, type WithElementRef } from '$lib/utils.js';

  let {
    ref = $bindable(null),
    class: className,
    variant = 'info',
    children,
    ...restProps
  }: WithElementRef<HTMLAttributes<HTMLDivElement>> & { variant?: ToastVariant; children?: Snippet } = $props();
</script>

<div bind:this={ref} data-slot="toast" role="status" class={cn(toastVariants({ variant }), className)} {...restProps}>
  {@render children?.()}
</div>
