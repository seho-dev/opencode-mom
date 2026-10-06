<script lang="ts">
import { getI18n } from '$src/i18n/context.js';

let { messages }: { messages: string[] } = $props();
const i18n = getI18n();
</script>

{#if messages.length}
  <div class="state-banner !border-l-[var(--status-warning)]" role="status">
    <div class="min-w-0 w-full">
      <p class="m-0 text-[var(--status-warning)]">{i18n.t('configuration.diagnostics')}</p>
      <ul class="mt-2 mb-0 pl-5 space-y-1 break-words [overflow-wrap:anywhere]">
        {#each messages as message}
          <li>{message}</li>
        {/each}
      </ul>
    </div>
  </div>
{/if}
