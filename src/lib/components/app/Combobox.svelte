<script lang="ts">
  type ComboboxOption = { value: string; label?: string; hint?: string };

  let { value = $bindable(''), options = [] as ComboboxOption[], placeholder = '', disabled = false } = $props();
  let open = $state(false);

  function pick(option: ComboboxOption) {
    value = option.value;
    open = false;
  }
</script>

<div class="combo">
  <input
    bind:value
    {placeholder}
    {disabled}
    autocomplete="off"
    onfocus={() => (open = true)}
    onblur={() => setTimeout(() => (open = false), 100)}
    onkeydown={(event) => {
      if (event.key === 'Escape') open = false;
    }}
  />
  {#if open}
    <div class="combo-list" role="listbox">
      {#each options as option (option.value)}
        <button type="button" role="option" onclick={() => pick(option)}>
          <span>{option.label ?? option.value}</span>
          {#if option.hint}<small>{option.hint}</small>{/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

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
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    z-index: 50;
    background: var(--surface-panel);
    border: 1px solid var(--border-default);
    border-radius: var(--radius-sm);
    overflow: hidden;
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
  .combo-list button:hover {
    background: var(--surface-hover);
  }
  .combo-list small {
    margin-left: auto;
    color: var(--text-muted);
    font-size: 11px;
  }
</style>
