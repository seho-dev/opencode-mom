<script lang="ts">
import Select, { type SelectOption } from '$src/components/Select.svelte';
import { setI18n } from '$src/i18n/context.js';
import { createI18n } from '$src/i18n/i18n.svelte.js';
import Combobox from '../combobox.svelte';

setI18n(createI18n(() => 'en'));
let {
  options = Array.from({ length: 20 }, (_, index) => ({ value: `Option ${index + 1}` })),
  onAgentChange,
  onModelChange,
}: {
  options?: SelectOption[];
  onAgentChange?: (value: string) => void;
  onModelChange?: (value: string) => void;
} = $props();
let agent = $state('');
let model = $state('');
</script>

<main class="content">
  <form class="form-panel" onsubmit={(event) => event.preventDefault()}>
    <div class="form-grid">
      <div class="spacer"></div>
      <label for="agent">Agent</label>
      <Combobox id="agent" bind:value={agent} {options} onchange={onAgentChange} />
      <Select ariaLabel="Model" bind:value={model} {options} onchange={onModelChange} />
      <Combobox aria-label="Disabled agent" disabled {options} />
      <Select ariaLabel="Disabled model" disabled {options} />
    </div>
    <div class="form-actions">
      <button type="button">Cancel</button>
      <button type="submit">Save</button>
    </div>
  </form>
</main>

<style>
.content {
  width: min(400px, calc(100vw - 32px));
  height: calc(100vh - 32px);
  margin: 16px;
}
.form-grid {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.spacer {
  min-height: 300px;
}
</style>
