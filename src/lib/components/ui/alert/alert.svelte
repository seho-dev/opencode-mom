<script lang="ts" module>
  import { type VariantProps, tv } from 'tailwind-variants';

  export const alertVariants = tv({
    base: 'group/alert grid w-full grid-cols-[minmax(0,1fr)_auto] items-start gap-x-3 gap-y-1 rounded-[2px] border bg-[var(--surface-input)] p-3 text-left text-[12px] leading-5 has-[>svg]:grid-cols-[auto_minmax(0,1fr)_auto] [&>svg]:size-4 [&>svg]:text-current',
    variants: {
      variant: {
        default: 'border-[var(--border-default)] text-[var(--text-primary)]',
        info: 'border-[var(--accent-primary)]/45 bg-[var(--accent-primary)]/10 text-[var(--accent-primary)]',
        success: 'border-[var(--status-success)]/45 bg-[var(--status-success)]/10 text-[var(--status-success)]',
        warning: 'border-[var(--status-warning)]/45 bg-[var(--status-warning)]/10 text-[var(--status-warning)]',
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
