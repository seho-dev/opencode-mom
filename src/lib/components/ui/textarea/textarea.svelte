<script lang="ts">
  import { cn, type WithElementRef, type WithoutChildren } from '$lib/utils.js';
  import type { HTMLTextareaAttributes } from 'svelte/elements';

  let {
    ref = $bindable(null),
    value = $bindable(),
    class: className,
    'data-slot': dataSlot = 'textarea',
    ...restProps
  }: WithoutChildren<WithElementRef<HTMLTextareaAttributes>> = $props();
</script>

<textarea
  bind:this={ref}
  data-slot={dataSlot}
  class={cn(
    'flex min-h-24 w-full resize-y rounded-[2px] border border-[var(--border-default)] bg-[var(--surface-input)] px-3 py-2 text-[12px] leading-5 text-[var(--text-primary)] outline-none transition-[border-color,box-shadow] duration-150 placeholder:text-[var(--text-muted)] focus-visible:border-[var(--accent-primary)] focus-visible:outline-none focus-visible:shadow-[0_0_12px_rgba(0,229,255,.2)] aria-invalid:border-[var(--status-error)] disabled:cursor-not-allowed disabled:opacity-[var(--disabled-opacity)]',
    className,
  )}
  bind:value
  {...restProps}></textarea>
