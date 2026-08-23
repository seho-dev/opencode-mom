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
    'h-5 w-full cursor-pointer appearance-none bg-transparent accent-[#00c8df] disabled:cursor-not-allowed disabled:opacity-[var(--disabled-opacity)] [&::-webkit-slider-runnable-track]:h-1 [&::-webkit-slider-runnable-track]:rounded-[2px] [&::-webkit-slider-runnable-track]:bg-[var(--border-default)] [&::-webkit-slider-thumb]:mt-[-5px] [&::-webkit-slider-thumb]:size-3 [&::-webkit-slider-thumb]:appearance-none [&::-webkit-slider-thumb]:rounded-[2px] [&::-webkit-slider-thumb]:border [&::-webkit-slider-thumb]:border-[#020708] [&::-webkit-slider-thumb]:bg-[#00c8df] [&::-moz-range-track]:h-1 [&::-moz-range-track]:rounded-[2px] [&::-moz-range-track]:bg-[var(--border-default)] [&::-moz-range-thumb]:size-3 [&::-moz-range-thumb]:rounded-[2px] [&::-moz-range-thumb]:border [&::-moz-range-thumb]:border-[#020708] [&::-moz-range-thumb]:bg-[#00c8df] focus-visible:outline-none focus-visible:[&::-webkit-slider-thumb]:shadow-[0_0_12px_rgba(0,229,255,.35)] focus-visible:[&::-moz-range-thumb]:shadow-[0_0_12px_rgba(0,229,255,.35)]',
    className,
  )}
  bind:value
  {...restProps}
/>
