<script lang="ts">
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';

let {
  projectId = null,
  readonly = false,
  writable = false,
}: { projectId?: string | null; readonly?: boolean; writable?: boolean } = $props();
const config = getConfig();
const i18n = getI18n();
const path = $derived((config.projects ?? []).find((project) => project.id === projectId)?.path ?? projectId);
const notice = $derived(
  readonly
    ? projectId !== null
      ? 'scope.projectReadOnly'
      : 'scope.globalReadOnly'
    : writable
      ? projectId !== null
        ? 'scope.projectWritable'
        : 'scope.globalDestination'
      : 'scope.projectReadOnly',
);
</script>
{#if projectId !== null || readonly || writable}
  <div class="state-banner mb-4 min-w-0 flex-col items-start gap-1" role="status">
    <span class="text-xs text-[var(--text-muted)]">{i18n.t(notice)}</span>
    {#if projectId !== null}
      <span class="min-w-0 break-words text-xs [overflow-wrap:anywhere]">{path}</span>
    {/if}
  </div>
{/if}
