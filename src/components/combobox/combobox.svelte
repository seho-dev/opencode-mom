<script lang="ts">
import { Popover } from 'bits-ui';
import type { HTMLInputAttributes } from 'svelte/elements';
import { getI18n } from '$src/i18n/context.js';

const i18n = getI18n();

type ComboboxOption = { value: string; label?: string; hint?: string };

interface ComboboxProps
  extends Omit<HTMLInputAttributes, 'value' | 'oninput' | 'onchange' | 'onfocus' | 'onblur' | 'onkeydown'> {
  value?: string;
  options?: ComboboxOption[];
  placeholder?: string;
  disabled?: boolean;
  onchange?: (value: string) => void;
}

let {
  value = $bindable(''),
  options = [],
  placeholder = '',
  disabled = false,
  onchange,
  id,
  ...rest
}: ComboboxProps = $props();

const generatedId = $props.id();
const inputId = $derived(id ?? generatedId);
const listId = $derived(`${inputId}-list`);
const optionId = (index: number) => `${inputId}-option-${index}`;

let open = $state(false);
// Filtering uses the typed query, not the committed value, so reopening shows all options.
let query = $state('');
let activeIndex = $state(-1);
let input = $state<HTMLInputElement | null>(null);
let list = $state<HTMLDivElement | null>(null);

const filtered = $derived(
  query.trim()
    ? options.filter((option) =>
        `${option.label ?? ''} ${option.value}`.toLowerCase().includes(query.trim().toLowerCase()),
      )
    : options,
);

function close() {
  open = false;
  query = '';
  activeIndex = -1;
}

function pick(option: ComboboxOption) {
  value = option.value;
  onchange?.(value);
  close();
}

function onkeydown(event: KeyboardEvent) {
  if (event.key === 'ArrowDown') {
    event.preventDefault();
    if (!open) {
      open = true;
      return;
    }
    activeIndex = Math.min(activeIndex + 1, filtered.length - 1);
  } else if (event.key === 'ArrowUp') {
    event.preventDefault();
    activeIndex = Math.max(activeIndex - 1, 0);
  } else if (event.key === 'Enter') {
    const option = filtered[activeIndex];
    if (open && activeIndex >= 0 && option) {
      event.preventDefault();
      pick(option);
    }
  } else if (event.key === 'Escape' || event.key === 'Tab') {
    close();
  }
}

$effect(() => {
  if (open && activeIndex >= 0) {
    list?.children[activeIndex]?.scrollIntoView({ block: 'nearest' });
  }
});
</script>

<Popover.Root
  bind:open
  onOpenChange={(isOpen) => {
    if (!isOpen) close();
  }}
>
  <div class="combo">
    <input
      {...rest}
      id={inputId}
      type="text"
      role="combobox"
      aria-expanded={open}
      aria-controls={listId}
      aria-autocomplete="list"
      aria-activedescendant={open && activeIndex >= 0 ? optionId(activeIndex) : undefined}
      {placeholder}
      {disabled}
      autocomplete="off"
      {value}
      bind:this={input}
      oninput={(event) => {
        value = event.currentTarget.value;
        query = value;
        open = true;
        activeIndex = -1;
        onchange?.(value);
      }}
      onfocus={() => (open = true)}
      onblur={(event) => {
        if (!(event.relatedTarget instanceof Node && list?.contains(event.relatedTarget))) close();
      }}
      {onkeydown}
    >
    <Popover.Portal>
      <Popover.Content
        role="listbox"
        customAnchor={input}
        align="start"
        sideOffset={4}
        collisionPadding={8}
        strategy="fixed"
        trapFocus={false}
        onOpenAutoFocus={(event) => event.preventDefault()}
        onCloseAutoFocus={(event) => event.preventDefault()}
        onFocusOutside={(event) => {
          if (event.target !== input) close();
        }}
      >
        {#snippet child({ props, wrapperProps })}
          <div {...wrapperProps}>
            <div {...props} id={listId} class="combo-list" bind:this={list}>
              {#if filtered.length === 0}
                <div class="combo-empty">{i18n.t('common.noMatches')}</div>
              {:else}
                {#each filtered as option, index (option.value)}
                  <button
                    type="button"
                    tabindex="-1"
                    id={optionId(index)}
                    role="option"
                    aria-selected={index === activeIndex}
                    class:active={index === activeIndex}
                    onmouseenter={() => (activeIndex = index)}
                    onmousedown={(event) => event.preventDefault()}
                    onclick={() => pick(option)}
                  >
                    <span>{option.label ?? option.value}</span>
                    {#if option.hint}
                      <small>{option.hint}</small>
                    {/if}
                  </button>
                {/each}
              {/if}
            </div>
          </div>
        {/snippet}
      </Popover.Content>
    </Popover.Portal>
  </div>
</Popover.Root>

<style>
.combo {
  position: relative;
}
.combo input {
  width: 100%;
  min-width: 0;
  border: 1px solid var(--border-default);
  border-radius: 2px;
  background: var(--surface-input);
  color: var(--text-primary);
  min-height: 36px;
  padding: 8px 10px;
}
.combo input:disabled {
  cursor: not-allowed;
  opacity: var(--disabled-opacity);
}
.combo-list {
  z-index: 50;
  width: var(--bits-popover-anchor-width);
  max-width: var(--bits-popover-content-available-width);
  max-height: min(264px, var(--bits-popover-content-available-height));
  overflow-y: auto;
  overflow-x: hidden;
  background: var(--surface-panel);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-sm);
}
.combo-list button {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  width: 100%;
  text-align: left;
  padding: 8px 10px;
  background: transparent;
  border: 0;
  font: inherit;
  color: var(--text-primary);
  font-family: var(--font-primary);
  font-size: 12px;
}
.combo-list button:hover,
.combo-list button.active {
  background: var(--surface-hover);
}
.combo-list small {
  margin-left: auto;
  color: var(--text-muted);
  font-size: 11px;
}
.combo-empty {
  padding: 8px 10px;
  color: var(--text-muted);
  font-family: var(--font-primary);
  font-size: 12px;
}
</style>
