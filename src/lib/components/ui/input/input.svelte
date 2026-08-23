<script lang="ts">
  import type { HTMLInputAttributes, HTMLInputTypeAttribute } from 'svelte/elements';
  import { cn, type WithElementRef } from '$lib/utils.js';

  type InputType = Exclude<HTMLInputTypeAttribute, 'file'>;

  type Props = WithElementRef<
    Omit<HTMLInputAttributes, 'type'> & ({ type: 'file'; files?: FileList } | { type?: InputType; files?: undefined })
  >;

  let {
    ref = $bindable(null),
    value = $bindable(),
    type,
    files = $bindable(),
    class: className,
    'data-slot': dataSlot = 'input',
    ...restProps
  }: Props = $props();
</script>

{#if type === 'file'}
  <input
    bind:this={ref}
    data-slot={dataSlot}
    class={cn(
      'h-8 w-full min-w-0 rounded-[2px] border border-[var(--border-default)] bg-[var(--surface-input)] px-3 py-1.5 text-[12px] leading-4 text-[var(--text-primary)] outline-none transition-[border-color,box-shadow] duration-150 placeholder:text-[var(--text-muted)] focus-visible:border-[var(--accent-primary)] focus-visible:outline-none focus-visible:shadow-[0_0_12px_rgba(0,229,255,.2)] aria-invalid:border-[var(--status-error)] disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-[var(--disabled-opacity)] file:mr-3 file:h-6 file:border-0 file:bg-transparent file:text-[11px] file:font-medium',
      className,
    )}
    type="file"
    bind:files
    bind:value
    {...restProps}
  />
{:else}
  <input
    bind:this={ref}
    data-slot={dataSlot}
    class={cn(
      'h-8 w-full min-w-0 rounded-[2px] border border-[var(--border-default)] bg-[var(--surface-input)] px-3 py-1.5 text-[12px] leading-4 text-[var(--text-primary)] outline-none transition-[border-color,box-shadow] duration-150 placeholder:text-[var(--text-muted)] focus-visible:border-[var(--accent-primary)] focus-visible:outline-none focus-visible:shadow-[0_0_12px_rgba(0,229,255,.2)] aria-invalid:border-[var(--status-error)] disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-[var(--disabled-opacity)]',
      className,
    )}
    {type}
    bind:value
    {...restProps}
  />
{/if}
