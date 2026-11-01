<script lang="ts">
import { onDestroy } from 'svelte';
import { Button } from '$src/components/button/index.js';
import * as Dialog from '$src/components/dialog/index.js';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import type { McpServer } from '$src/types/mcp.js';

let { server = $bindable(null), onDeleted }: { server: McpServer | null; onDeleted: () => void | Promise<void> } =
  $props();
const config = getConfig();
const i18n = getI18n();
let pending = $state(false);
let failed = $state(false);
let active = true;
onDestroy(() => {
  active = false;
});

function close() {
  if (pending) return;
  server = null;
  failed = false;
}
async function remove() {
  if (!server || pending) return;
  const name = server.name;
  pending = true;
  failed = false;
  try {
    await config.deleteMcp(name);
  } catch {
    if (active && server?.name === name) failed = true;
    pending = false;
    return;
  }
  pending = false;
  if (!active || server?.name !== name) return;
  close();
  await onDeleted();
}
</script>

<Dialog.Root open={server !== null} onOpenChange={(open) => { if (!open) close(); }}>
  <Dialog.Content
    variant="destructive"
    aria-busy={pending}
    onEscapeKeydown={(event) => { if (pending) event.preventDefault(); }}
    onInteractOutside={(event) => { if (pending) event.preventDefault(); }}
  >
    <Dialog.Header>
      <Dialog.Title>{i18n.t('mcp.deleteTitle')}</Dialog.Title>
      <Dialog.Description class="break-words [overflow-wrap:anywhere]">
        {i18n.t('mcp.deleteConfirm', { name: server?.name ?? '' })}
      </Dialog.Description>
    </Dialog.Header>
    <div class="min-w-0 text-xs text-[var(--text-secondary)]">
      <p class="mt-0 mb-2">{i18n.t('configuration.globalSources')}</p>
      <ul class="m-0 pl-4 space-y-1 break-words [overflow-wrap:anywhere]">
        {#each server?.sourcePaths.length ? server.sourcePaths : [server?.sourcePath ?? '—'] as path}
          <li>{path}</li>
        {/each}
      </ul>
      <p class="mb-0">{i18n.t('mcp.deleteHint')}</p>
    </div>
    {#if failed}
      <p class="m-0 text-[var(--status-error)]" role="alert">{i18n.t('mcp.deleteFailed')}</p>
    {/if}
    <Dialog.Footer>
      <Button variant="outline" disabled={pending} onclick={close}>{i18n.t('common.cancel')}</Button>
      <Button variant="destructive" disabled={pending} onclick={remove}>
        {i18n.t(pending ? 'configuration.deleting' : 'common.delete')}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
