<script lang="ts" module>
  import { type VariantProps, tv } from 'tailwind-variants';

  export const badgeVariants = tv({
    base: 'inline-flex h-5 w-fit shrink-0 items-center justify-center gap-1 overflow-hidden whitespace-nowrap rounded-[2px] border px-2 text-[11px] font-medium leading-4 [&>svg]:size-3 [&>svg]:shrink-0',
    variants: {
      variant: {
        active: 'border-[var(--accent-solid)] bg-[var(--accent-solid)]/15 text-[var(--accent-primary)]',
        neutral: 'border-[var(--border-default)] bg-[var(--surface-input)] text-[var(--text-secondary)]',
        success: 'border-[var(--status-success)]/35 bg-[var(--status-success)]/10 text-[var(--status-success)]',
        warning: 'border-[var(--status-warning)]/40 bg-[var(--status-warning)]/10 text-[var(--status-warning)]',
        error: 'border-[var(--status-error)]/45 bg-[var(--status-error)]/10 text-[var(--status-error)]',
        secondary: 'border-[var(--border-default)] bg-[var(--surface-input)] text-[var(--text-secondary)]',
        destructive: 'border-[var(--status-error)]/45 bg-[var(--status-error)]/10 text-[var(--status-error)]',
        outline: 'border-[var(--border-default)] bg-transparent text-[var(--text-primary)]',
      },
    },
    defaultVariants: {
      variant: 'neutral',
    },
  });

  export type BadgeVariant = VariantProps<typeof badgeVariants>['variant'];
</script>

<script lang="ts">
  import type { HTMLAttributes } from 'svelte/elements';
  import { cn, type WithElementRef } from '$lib/utils.js';

  let {
    ref = $bindable(null),
    class: className,
    variant = 'secondary',
    children,
    ...restProps
  }: WithElementRef<HTMLAttributes<HTMLSpanElement>> & {
    variant?: BadgeVariant;
  } = $props();
</script>

<span bind:this={ref} data-slot="badge" class={cn(badgeVariants({ variant }), className)} {...restProps}>
  {@render children?.()}
</span>
