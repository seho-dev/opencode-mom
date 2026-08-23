<script lang="ts">
  import type { HTMLInputAttributes } from 'svelte/elements';
  import { cn } from '$lib/utils.js';

  let {
    checked = $bindable(false),
    class: className,
    id,
    disabled = false,
    ...restProps
  }: Omit<HTMLInputAttributes, 'type' | 'checked' | 'class' | 'id' | 'disabled'> & {
    checked?: boolean;
    class?: string;
    id?: string;
    disabled?: boolean;
  } = $props();
</script>

<label
  class={cn(
    'relative inline-flex h-5 w-10 shrink-0 cursor-pointer items-center rounded-[2px] outline-none has-[:focus-visible]:ring-1 has-[:focus-visible]:ring-[var(--focus-ring)] has-[:focus-visible]:ring-offset-2 has-[:focus-visible]:ring-offset-[var(--surface-app)] has-[:disabled]:cursor-not-allowed has-[:disabled]:opacity-[var(--disabled-opacity)]',
    className,
  )}
>
  <input {id} type="checkbox" class="peer sr-only" bind:checked {disabled} {...restProps} />
  <span
    class="absolute inset-0 rounded-[2px] border border-[var(--border-default)] bg-[var(--surface-input)] transition-colors peer-checked:border-[#00c8df] peer-checked:bg-[#07343b]"
  ></span>
  <span
    class="relative ml-0.5 size-4 rounded-[2px] bg-[var(--text-muted)] transition-transform peer-checked:translate-x-5 peer-checked:bg-[#00c8df]"
  ></span>
</label>
