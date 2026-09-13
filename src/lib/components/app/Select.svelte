<script lang="ts" module>
  export type SelectOption = { value: string; label?: string; hint?: string; disabled?: boolean };
</script>

<script lang="ts">
  import { ChevronDown } from '@lucide/svelte';
  import { getI18n } from '$lib/features/i18n/context.js';

  const i18n = getI18n();

  let {
    value = $bindable(''),
    options = [] as SelectOption[],
    placeholder = i18n.t('common.selectPlaceholder'),
    disabled = false,
    id,
    ariaLabel,
    onchange,
    class: className = '',
  }: {
    value?: string;
    options?: SelectOption[];
    placeholder?: string;
    disabled?: boolean;
    id?: string;
    ariaLabel?: string;
    onchange?: (value: string) => void;
    class?: string;
  } = $props();

  // Long lists (e.g. model pickers) get an inline search field; short selects stay compact.
  const SEARCH_THRESHOLD = 8;
  const uid = $props.id();
  const listboxId = `${uid}-listbox`;
  const optionId = (index: number) => `${uid}-option-${index}`;

  let open = $state(false);
  let query = $state('');
  let highlight = $state(-1);
  let root = $state<HTMLElement | null>(null);
  let trigger = $state<HTMLButtonElement | null>(null);
  let list = $state<HTMLElement | null>(null);
  let searchInput = $state<HTMLInputElement | null>(null);

  const searchable = $derived(options.length > SEARCH_THRESHOLD);
  const selected = $derived(options.find((option) => option.value === value));
  const label = $derived(selected ? (selected.label ?? selected.value) : placeholder);
  const placeholderVisible = $derived(!selected);
  const filtered = $derived(query.trim() ? options.filter((option) => matches(option, query.trim())) : options);

  function matches(option: SelectOption, needle: string) {
    return `${option.label ?? ''} ${option.value}`.toLowerCase().includes(needle.toLowerCase());
  }

  function openList() {
    if (disabled) return;
    query = '';
    open = true;
    const current = options.findIndex((option) => option.value === value && !option.disabled);
    highlight = current >= 0 ? current : options.findIndex((option) => !option.disabled);
  }

  function close(focusTrigger = false) {
    open = false;
    query = '';
    if (focusTrigger) trigger?.focus();
  }

  function choose(option: SelectOption) {
    if (option.disabled) return;
    const changed = option.value !== value;
    value = option.value;
    close(true);
    if (changed) onchange?.(option.value);
  }

  function move(step: number) {
    if (!filtered.length) return;
    let index = highlight < 0 ? (step > 0 ? -1 : 0) : highlight;
    for (let count = 0; count < filtered.length; count++) {
      index = (index + step + filtered.length) % filtered.length;
      if (!filtered[index]?.disabled) {
        highlight = index;
        return;
      }
    }
  }

  function onTriggerKeydown(event: KeyboardEvent) {
    if (disabled) return;
    const key = event.key;
    if (key === 'Escape') {
      if (open) {
        event.preventDefault();
        close(true);
      }
      return;
    }
    if (key === 'Tab') {
      if (open) close();
      return;
    }
    if (key === 'ArrowDown' || key === 'ArrowUp') {
      event.preventDefault();
      if (!open) openList();
      else move(key === 'ArrowDown' ? 1 : -1);
      return;
    }
    if (key === 'Enter' || key === ' ') {
      event.preventDefault();
      if (!open) openList();
      else if (highlight >= 0) {
        const option = filtered[highlight];
        if (option) choose(option);
      }
    }
  }

  function onSearchInput(event: Event) {
    query = (event.currentTarget as HTMLInputElement).value;
    // Read the derived list after updating the query so the highlight lands on the first enabled match.
    highlight = filtered.findIndex((option) => !option.disabled);
  }

  function onSearchKeydown(event: KeyboardEvent) {
    const key = event.key;
    if (key === 'Escape') {
      event.preventDefault();
      close(true);
      return;
    }
    if (key === 'Tab') {
      close();
      return;
    }
    if (key === 'ArrowDown' || key === 'ArrowUp') {
      event.preventDefault();
      move(key === 'ArrowDown' ? 1 : -1);
      return;
    }
    if (key === 'Enter') {
      event.preventDefault();
      const option = filtered[highlight];
      if (option) choose(option);
    }
  }

  // Close when clicking or focusing outside the component.
  $effect(() => {
    if (!open) return;
    const onPointerDown = (event: PointerEvent) => {
      if (root && event.target instanceof Node && !root.contains(event.target)) close();
    };
    const onFocusIn = (event: FocusEvent) => {
      if (root && event.target instanceof Node && !root.contains(event.target)) close();
    };
    document.addEventListener('pointerdown', onPointerDown);
    document.addEventListener('focusin', onFocusIn);
    return () => {
      document.removeEventListener('pointerdown', onPointerDown);
      document.removeEventListener('focusin', onFocusIn);
    };
  });

  // Keep the highlighted option visible while navigating with the keyboard.
  $effect(() => {
    if (!open || highlight < 0) return;
    list?.querySelector<HTMLElement>(`[data-index="${highlight}"]`)?.scrollIntoView({ block: 'nearest' });
  });

  // Opening a searchable select moves focus into the search field so typing filters immediately.
  $effect(() => {
    if (open && searchable) searchInput?.focus();
  });
</script>

<div class="select {className}" bind:this={root}>
  <button
    type="button"
    class="select-trigger"
    class:open
    class:placeholder={placeholderVisible}
    {id}
    aria-label={ariaLabel}
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-controls={listboxId}
    {disabled}
    bind:this={trigger}
    onclick={() => (open ? close() : openList())}
    onkeydown={onTriggerKeydown}
  >
    <span class="select-label">{label}</span>
    <span class="select-chevron" class:open><ChevronDown size={14} /></span>
  </button>
  {#if open}
    <div class="select-panel">
      {#if searchable}
        <input
          class="select-search"
          type="text"
          role="combobox"
          aria-label={i18n.t('common.searchOptions')}
          aria-expanded={open}
          aria-controls={listboxId}
          aria-autocomplete="list"
          aria-activedescendant={highlight >= 0 ? optionId(highlight) : undefined}
          placeholder={i18n.t('common.searchOptionsPlaceholder')}
          autocomplete="off"
          value={query}
          oninput={onSearchInput}
          onkeydown={onSearchKeydown}
          bind:this={searchInput}
        />
      {/if}
      <div class="select-options" role="listbox" id={listboxId} bind:this={list}>
        {#if filtered.length === 0}
          <div class="select-empty">{i18n.t('common.searchOptionsEmpty')}</div>
        {:else}
          {#each filtered as option, index (option.value)}
            {#if option.disabled}
              <div
                class="select-option disabled"
                role="option"
                tabindex="-1"
                aria-disabled="true"
                aria-selected="false"
                data-index={index}
              >
                <span>{option.label ?? option.value}</span>
                {#if option.hint}<small>{option.hint}</small>{/if}
              </div>
            {:else}
              <div
                class="select-option"
                id={optionId(index)}
                class:highlighted={highlight === index}
                role="option"
                tabindex="-1"
                aria-selected={option.value === value}
                data-index={index}
                onpointerenter={() => (highlight = index)}
                onclick={() => choose(option)}
                onkeydown={(event) => {
                  if (event.key === 'Enter' || event.key === ' ') {
                    event.preventDefault();
                    choose(option);
                  }
                }}
              >
                <span>{option.label ?? option.value}</span>
                {#if option.hint}<small>{option.hint}</small>{/if}
              </div>
            {/if}
          {/each}
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .select {
    position: relative;
  }
  .select-trigger {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    width: 100%;
    min-width: 0;
    border: 1px solid var(--border-default);
    border-radius: 2px;
    background: var(--surface-input);
    color: var(--text-primary);
    min-height: 36px;
    padding: 8px 10px;
    font-family: var(--font-primary);
    font-size: 12px;
    text-align: left;
  }
  .select-trigger.placeholder .select-label {
    color: var(--text-muted);
  }
  .select-trigger:disabled {
    cursor: not-allowed;
    opacity: var(--disabled-opacity);
  }
  .select-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .select-chevron {
    flex-shrink: 0;
    color: var(--text-muted);
    transition: transform 120ms ease;
  }
  .select-chevron.open {
    transform: rotate(180deg);
  }
  .select-panel {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    z-index: 50;
    display: flex;
    flex-direction: column;
    background: var(--surface-panel);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }
  .select-search {
    width: 100%;
    min-width: 0;
    border: 0;
    border-bottom: 1px solid var(--border-default);
    background: var(--surface-input);
    color: var(--text-primary);
    font-family: var(--font-primary);
    font-size: 12px;
    padding: 8px 10px;
  }
  .select-search:focus-visible {
    outline: 1px solid var(--accent-primary);
    outline-offset: -1px;
  }
  .select-options {
    max-height: 240px;
    overflow-y: auto;
    overflow-x: hidden;
  }
  .select-option {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 8px 10px;
    color: var(--text-primary);
    font-family: var(--font-primary);
    font-size: 12px;
    cursor: pointer;
  }
  .select-option.highlighted {
    background: var(--surface-hover);
  }
  .select-option[aria-selected='true'] {
    color: var(--accent-primary);
  }
  .select-option.disabled {
    cursor: not-allowed;
    opacity: var(--disabled-opacity);
  }
  .select-option small {
    margin-left: auto;
    color: var(--text-muted);
    font-size: 11px;
  }
  .select-empty {
    padding: 8px 10px;
    color: var(--text-muted);
    font-family: var(--font-primary);
    font-size: 12px;
  }
</style>
