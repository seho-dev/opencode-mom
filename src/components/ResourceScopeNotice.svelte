<script lang="ts">
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';

let { projectId = null, readonly = false }: { projectId?: string | null; readonly?: boolean } = $props();
const config = getConfig();
const i18n = getI18n();
const path = $derived((config.projects ?? []).find((project) => project.id === projectId)?.path ?? projectId);
</script>
{#if projectId !== null || readonly}
  <div class="state-banner mb-4 min-w-0 flex-col items-start gap-1" role="status">
    <span class="text-xs text-[var(--text-muted)]"
      >{i18n.t(projectId !== null ? 'scope.projectReadOnly' : 'scope.globalReadOnly')}</span
    >
    {#if projectId !== null}
      <span class="min-w-0 break-words text-xs [overflow-wrap:anywhere]">{path}</span>
    {/if}
  </div>
{/if}
