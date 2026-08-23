<script lang="ts" module>
  import { cn, type WithElementRef } from '$lib/utils.js';
  import type { HTMLAnchorAttributes, HTMLButtonAttributes } from 'svelte/elements';
  import { type VariantProps, tv } from 'tailwind-variants';

  export const buttonVariants = tv({
    base: 'group/button inline-flex shrink-0 items-center justify-center whitespace-nowrap rounded-[2px] border font-[var(--font-primary)] text-[11px] font-semibold leading-4 tracking-normal transition-[background-color,border-color,color,box-shadow,transform] duration-150 outline-none select-none focus-visible:border-[var(--accent-primary)] focus-visible:outline-1 focus-visible:outline-[var(--focus-ring)] focus-visible:outline-offset-2 focus-visible:shadow-[0_0_12px_rgba(0,229,255,.2)] disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-[var(--disabled-opacity)] disabled:shadow-none [&_svg]:pointer-events-none [&_svg]:size-[var(--icon-md)] [&_svg]:shrink-0',
    variants: {
      variant: {
        default:
          'border-[#00c8df] bg-[#00c8df] text-[#020708] hover:border-[var(--accent-primary)] hover:bg-[var(--accent-primary)] hover:shadow-[0_0_12px_rgba(0,229,255,.2)] active:translate-y-px active:brightness-95 active:shadow-none',
        secondary:
          'border-[var(--border-default)] bg-[#121212] text-[var(--text-primary)] hover:border-[var(--accent-primary)] hover:text-[var(--accent-primary)] active:translate-y-px active:bg-[#1e1e1e]',
        outline:
          'border-[var(--border-default)] bg-transparent text-[var(--text-primary)] hover:border-[var(--accent-primary)] hover:text-[var(--accent-primary)] active:translate-y-px active:bg-[#1e1e1e]',
        destructive:
          'border-[var(--status-error)] bg-[#121212] text-[var(--status-error)] hover:bg-[rgb(255_92_92_/_0.1)] active:translate-y-px active:bg-[rgb(255_92_92_/_0.16)]',
        ghost:
          'border-transparent bg-transparent text-[var(--text-primary)] hover:bg-[#1e1e1e] active:translate-y-px active:bg-[var(--surface-active)] aria-current:bg-[var(--surface-active)]',
      },
      size: {
        default: 'h-8 gap-1.5 px-4',
        sm: 'h-7 gap-1.5 px-3',
        icon: 'size-8 p-0',
        'icon-sm': 'size-7 p-0 [&_svg]:size-[var(--icon-sm)]',
      },
    },
    defaultVariants: {
      variant: 'default',
      size: 'default',
    },
  });

  export type ButtonVariant = VariantProps<typeof buttonVariants>['variant'];
  export type ButtonSize = VariantProps<typeof buttonVariants>['size'];

  export type ButtonProps = WithElementRef<HTMLButtonAttributes> &
    WithElementRef<HTMLAnchorAttributes> & {
      variant?: ButtonVariant;
      size?: ButtonSize;
    };
</script>

<script lang="ts">
  let {
    class: className,
    variant = 'default',
    size = 'default',
    ref = $bindable(null),
    href = undefined,
    type = 'button',
    disabled,
    children,
    ...restProps
  }: ButtonProps = $props();
</script>

{#if href}
  <a
    bind:this={ref}
    data-slot="button"
    class={cn(buttonVariants({ variant, size }), className)}
    href={disabled ? undefined : href}
    aria-disabled={disabled}
    role={disabled ? 'link' : undefined}
    tabindex={disabled ? -1 : undefined}
    {...restProps}
  >
    {@render children?.()}
  </a>
{:else}
  <button
    bind:this={ref}
    data-slot="button"
    class={cn(buttonVariants({ variant, size }), className)}
    {type}
    {disabled}
    {...restProps}
  >
    {@render children?.()}
  </button>
{/if}

<style>
  :global([data-slot='button']) {
    font-family: var(--font-primary) !important;
    font-size: 11px !important;
    font-weight: 600 !important;
  }
</style>
