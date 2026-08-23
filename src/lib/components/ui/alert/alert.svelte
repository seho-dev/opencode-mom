<script lang="ts" module>
  import { type VariantProps, tv } from 'tailwind-variants';

  export const alertVariants = tv({
    base: 'group/alert grid w-full grid-cols-[minmax(0,1fr)_auto] items-start gap-x-3 gap-y-1 rounded-[2px] border bg-[#121212] p-3 text-left text-[12px] leading-5 has-[>svg]:grid-cols-[auto_minmax(0,1fr)_auto] [&>svg]:size-4 [&>svg]:text-current',
    variants: {
      variant: {
        default: 'border-[var(--border-default)] text-[var(--text-primary)]',
        info: 'border-[#00e5ff]/45 bg-[#00e5ff]/10 text-[#00e5ff]',
        success: 'border-[#39ff14]/45 bg-[#39ff14]/10 text-[#39ff14]',
        warning: 'border-[#f0b429]/45 bg-[#f0b429]/10 text-[#f0b429]',
        error: 'border-[var(--status-error)]/50 bg-[var(--status-error)]/10 text-[var(--status-error)]',
        destructive: 'border-[var(--status-error)]/50 bg-[var(--status-error)]/10 text-[var(--status-error)]',
      },
    },
    defaultVariants: {
      variant: 'default',
    },
  });

  export type AlertVariant = VariantProps<typeof alertVariants>['variant'];
</script>

<script lang="ts">
  import type { HTMLAttributes } from 'svelte/elements';
  import { cn, type WithElementRef } from '$lib/utils.js';

  let {
    ref = $bindable(null),
    class: className,
    variant = 'default',
    children,
    ...restProps
  }: WithElementRef<HTMLAttributes<HTMLDivElement>> & {
    variant?: AlertVariant;
  } = $props();
</script>

<div bind:this={ref} data-slot="alert" role="alert" class={cn(alertVariants({ variant }), className)} {...restProps}>
  {@render children?.()}
</div>
