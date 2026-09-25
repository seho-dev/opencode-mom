<script lang="ts">
import Combobox from './Combobox.svelte';

let { value = $bindable(''), disabled = false } = $props();

// V2 `providers.<id>.package` values: native `@opencode/ai/providers/*` runtimes
// or AI SDK packages carrying the `aisdk:` prefix.
const PACKAGE_OPTIONS = [
  { value: 'aisdk:@ai-sdk/openai-compatible', label: 'aisdk:@ai-sdk/openai-compatible' },
  { value: 'aisdk:@ai-sdk/openai', label: 'aisdk:@ai-sdk/openai' },
  { value: '@opencode/ai/providers/openai-compatible', label: '@opencode/ai/providers/openai-compatible' },
  { value: '@opencode/ai/providers/openai', label: '@opencode/ai/providers/openai' },
  { value: '@opencode/ai/providers/anthropic', label: '@opencode/ai/providers/anthropic' },
];
</script>

<Combobox bind:value options={PACKAGE_OPTIONS} placeholder="aisdk:@ai-sdk/openai-compatible" {disabled} />
