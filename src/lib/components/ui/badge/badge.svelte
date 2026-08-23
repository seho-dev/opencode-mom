<script lang="ts" module>
  import { type VariantProps, tv } from 'tailwind-variants';

  export const badgeVariants = tv({
    base: 'inline-flex h-5 w-fit shrink-0 items-center justify-center gap-1 overflow-hidden whitespace-nowrap rounded-[2px] border px-2 text-[11px] font-medium leading-4 [&>svg]:size-3 [&>svg]:shrink-0',
    variants: {
      variant: {
        active: 'border-[#00c8df] bg-[#00c8df]/15 text-[#00e5ff]',
        neutral: 'border-[var(--border-default)] bg-[#121212] text-[var(--text-secondary)]',
        success: 'border-[#39ff14]/35 bg-[#39ff14]/10 text-[#39ff14]',
        warning: 'border-[#f0b429]/40 bg-[#f0b429]/10 text-[#f0b429]',
        error: 'border-[var(--status-error)]/45 bg-[var(--status-error)]/10 text-[var(--status-error)]',
        secondary: 'border-[var(--border-default)] bg-[#121212] text-[var(--text-secondary)]',
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
