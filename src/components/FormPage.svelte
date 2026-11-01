<script lang="ts">
import type { Snippet } from 'svelte';

let { active = true, children }: { active?: boolean; children: Snippet } = $props();
</script>

<div class:form-page={active}>{@render children()}</div>
