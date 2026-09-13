<script lang="ts" module>
  import { type VariantProps, tv } from 'tailwind-variants';

  export const toastVariants = tv({
    base: 'grid w-full max-w-sm grid-cols-[minmax(0,1fr)_auto] gap-3 rounded-[2px] border bg-[var(--surface-input)] p-3 text-[12px] leading-5',
    variants: {
      variant: {
        info: 'border-[var(--accent-primary)]/45 text-[var(--accent-primary)]',
        success: 'border-[var(--status-success)]/45 text-[var(--status-success)]',
        warning: 'border-[var(--status-warning)]/45 text-[var(--status-warning)]',
        error: 'border-[var(--status-error)]/50 text-[var(--status-error)]',
      },
    },
    defaultVariants: { variant: 'info' },
  });
  export type ToastVariant = VariantProps<typeof toastVariants>['variant'];
  export type ToastAction = { label: string; onclick: () => void };
</script>

<script lang="ts">
  import { X } from '@lucide/svelte';
  import type { HTMLAttributes } from 'svelte/elements';
  import { cn, type WithElementRef } from '$lib/utils.js';
  import { Button } from '$lib/components/ui/button/index.js';

  let {
    ref = $bindable(null),
    class: className,
    variant = 'info',
    title,
    description,
    action,
    actions,
    onClose,
    ...restProps
  }: WithElementRef<HTMLAttributes<HTMLDivElement>> & {
    variant?: ToastVariant;
    title?: string;
    description?: string;
    action?: ToastAction;
    actions?: ToastAction[];
    onClose?: () => void;
  } = $props();

  const buttons = $derived(actions ?? (action ? [action] : []));
</script>

<div
  bind:this={ref}
  data-slot="toast"
  role={variant === 'error' || variant === 'warning' ? 'alert' : 'status'}
  class={cn(toastVariants({ variant }), className)}
  {...restProps}
>
  <div class="min-w-0">
    {#if title}<p class="font-semibold text-[13px] leading-5">{title}</p>{/if}
    {#if description}<p class="break-words">{description}</p>{/if}
  </div>
  <div class="flex items-center gap-1">
    {#each buttons as button (button.label)}<Button size="sm" variant="outline" onclick={button.onclick}
        >{button.label}</Button
      >{/each}{#if onClose}<Button size="icon-sm" variant="ghost" aria-label="Dismiss" onclick={onClose}
        ><X size={13} /></Button
      >{/if}
  </div>
</div>
