<script lang="ts">
  import type { HTMLInputAttributes } from 'svelte/elements';
  import { cn } from '$lib/utils.js';

  let {
    value = $bindable(),
    class: className,
    ...restProps
  }: Omit<HTMLInputAttributes, 'type' | 'value' | 'class'> & {
    value?: number | string;
    class?: string;
  } = $props();
</script>

<input
  type="range"
  data-slot="slider"
  class={cn(
    'h-5 w-full cursor-pointer appearance-none bg-transparent accent-[var(--accent-solid)] disabled:cursor-not-allowed disabled:opacity-[var(--disabled-opacity)] [&::-webkit-slider-runnable-track]:h-1 [&::-webkit-slider-runnable-track]:rounded-[2px] [&::-webkit-slider-runnable-track]:bg-[var(--border-default)] [&::-webkit-slider-thumb]:mt-[-5px] [&::-webkit-slider-thumb]:size-3 [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:rounded-[2px] [&::-webkit-slider-thumb]:border [&::-webkit-slider-thumb]:border-[var(--text-on-accent)] [&::-webkit-slider-thumb]:bg-[var(--accent-solid)] [&::-moz-range-track]:h-1 [&::-moz-range-track]:rounded-[2px] [&::-moz-range-track]:bg-[var(--border-default)] [&::-moz-range-thumb]:size-3 [&::-moz-range-thumb]:rounded-[2px] [&::-moz-range-thumb]:border [&::-moz-range-thumb]:border-[var(--text-on-accent)] [&::-moz-range-thumb]:bg-[var(--accent-solid)] focus-visible:outline-none focus-visible:[&::-webkit-slider-thumb]:shadow-[var(--focus-glow-strong)] focus-visible:[&::-moz-range-thumb]:shadow-[var(--focus-glow-strong)]',
    className,
  )}
  bind:value
  {...restProps}
/>
