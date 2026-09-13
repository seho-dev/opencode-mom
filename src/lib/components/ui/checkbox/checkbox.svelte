<script lang="ts">
  import { Checkbox as CheckboxPrimitive } from 'bits-ui';
  import { cn, type WithoutChildrenOrChild } from '$lib/utils.js';
  import CheckIcon from '@lucide/svelte/icons/check';
  import MinusIcon from '@lucide/svelte/icons/minus';

  let {
    ref = $bindable(null),
    checked = $bindable(false),
    indeterminate = $bindable(false),
    class: className,
    ...restProps
  }: WithoutChildrenOrChild<CheckboxPrimitive.RootProps> = $props();
</script>

<CheckboxPrimitive.Root
  bind:ref
  data-slot="checkbox"
  class={cn(
    'relative flex size-4 shrink-0 items-center justify-center rounded-[2px] border border-[var(--border-default)] bg-[var(--surface-input)] text-[var(--text-on-accent)] outline-none transition-[background-color,border-color,box-shadow] duration-100 data-[state=checked]:border-[var(--accent-solid)] data-[state=checked]:bg-[var(--accent-solid)] data-[state=indeterminate]:border-[var(--accent-solid)] data-[state=indeterminate]:bg-[var(--accent-solid)] hover:bg-[var(--surface-hover)] focus-visible:border-[var(--accent-primary)] focus-visible:outline-none focus-visible:shadow-[var(--focus-glow)] disabled:cursor-not-allowed disabled:opacity-[var(--disabled-opacity)] motion-reduce:transition-none',
    className,
  )}
  bind:checked
  bind:indeterminate
  {...restProps}
>
  {#snippet children({ checked, indeterminate })}
    <div
      data-slot="checkbox-indicator"
      class="grid place-content-center text-current transition-[opacity,transform] duration-100 motion-reduce:transition-none [&>svg]:size-3"
    >
      {#if checked}
        <CheckIcon />
      {:else if indeterminate}
        <MinusIcon />
      {/if}
    </div>
  {/snippet}
</CheckboxPrimitive.Root>
