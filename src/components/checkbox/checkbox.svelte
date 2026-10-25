<script lang="ts" module>
import type { Checkbox as CheckboxPrimitive } from 'bits-ui';
import type { WithoutChildrenOrChild } from '$src/utils/index.js';

export type CheckboxProps = WithoutChildrenOrChild<CheckboxPrimitive.RootProps>;
</script>

<script lang="ts">
import CheckIcon from '@lucide/svelte/icons/check';
import MinusIcon from '@lucide/svelte/icons/minus';
import { Checkbox as CheckboxPrimitive } from 'bits-ui';
import { cn } from '$src/utils/index.js';

let {
  ref = $bindable(null),
  checked = $bindable(false),
  indeterminate = $bindable(false),
  class: className,
  ...restProps
}: CheckboxProps = $props();
</script>

<CheckboxPrimitive.Root
  bind:ref
  data-slot="checkbox"
  class={cn(
    'relative inline-flex size-4 shrink-0 items-center justify-center rounded-[2px] border border-[var(--border-default)] bg-[var(--surface-input)] p-0 text-[var(--text-on-accent)] outline-none transition-[background-color,border-color,box-shadow] duration-100 data-[state=checked]:border-[var(--accent-solid)] data-[state=checked]:bg-[var(--accent-solid)] data-[state=checked]:hover:border-[var(--accent-solid-hover)] data-[state=checked]:hover:bg-[var(--accent-solid-hover)] data-[state=indeterminate]:border-[var(--accent-solid)] data-[state=indeterminate]:bg-[var(--accent-solid)] data-[state=indeterminate]:hover:border-[var(--accent-solid-hover)] data-[state=indeterminate]:hover:bg-[var(--accent-solid-hover)] hover:border-[var(--accent-primary)] hover:bg-[var(--surface-hover)] focus-visible:border-[var(--accent-primary)] focus-visible:outline-1 focus-visible:outline-[var(--focus-ring)] focus-visible:outline-offset-2 focus-visible:shadow-[var(--focus-glow)] disabled:cursor-not-allowed disabled:opacity-[var(--disabled-opacity)] disabled:hover:border-[var(--border-default)] disabled:hover:bg-[var(--surface-input)] disabled:data-[state=checked]:hover:border-[var(--accent-solid)] disabled:data-[state=checked]:hover:bg-[var(--accent-solid)] disabled:data-[state=indeterminate]:hover:border-[var(--accent-solid)] disabled:data-[state=indeterminate]:hover:bg-[var(--accent-solid)] motion-reduce:transition-none',
    className,
  )}
  bind:checked
  bind:indeterminate
  {...restProps}
>
  {#snippet children({ checked, indeterminate })}
    <span
      data-slot="checkbox-indicator"
      class="pointer-events-none grid place-content-center text-current transition-[opacity,transform] duration-100 motion-reduce:transition-none [&>svg]:size-3 [&>svg]:stroke-[2.5]"
    >
      {#if indeterminate}
        <MinusIcon />
      {:else if checked}
        <CheckIcon />
      {/if}
    </span>
  {/snippet}
</CheckboxPrimitive.Root>
