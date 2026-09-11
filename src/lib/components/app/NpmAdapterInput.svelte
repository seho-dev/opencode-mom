<script lang="ts">
  import Combobox from './Combobox.svelte';

  let { value = $bindable(''), disabled = false } = $props();

  const NPM_ADAPTERS = [
    { value: '@ai-sdk/openai-compatible', label: 'openai-compatible' },
    { value: '@ai-sdk/openai', label: 'openai-response' },
  ];
</script>

<Combobox bind:value options={NPM_ADAPTERS} placeholder="@ai-sdk/openai-compatible" {disabled} />
