<script lang="ts">
import { Check, Copy } from '@lucide/svelte';
import { onDestroy } from 'svelte';
import { Button } from '$src/components/button/index.js';
import { readError } from '$src/components/configuration/read-error.js';
import * as Dialog from '$src/components/dialog/index.js';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';

let {
  open = $bindable(false),
  active = true,
  onRotated,
}: { open: boolean; active?: boolean; onRotated?: () => void | Promise<void> } = $props();

const config = getConfig();
const i18n = getI18n();

let pending = $state(false);
let error = $state('');
let newKey = $state('');
let copied = $state(false);
let mounted = true;
let displayGeneration = 0;

$effect(() => {
  if (!open || !active) {
    displayGeneration += 1;
    newKey = '';
    error = '';
    copied = false;
  }
});

onDestroy(() => {
  mounted = false;
  newKey = '';
});

function close() {
  if (pending) return;
  const hadKey = newKey.length > 0;
  newKey = '';
  error = '';
  copied = false;
  open = false;
  if (hadKey && onRotated) {
    void onRotated();
  }
}

async function rotate() {
  if (!mounted || !active || !open || pending) return;
  const generation = displayGeneration;
  pending = true;
  error = '';
  copied = false;
  try {
    const key = await config.rotateMcpProxyKey();
    if (!mounted || !active || !open || generation !== displayGeneration) return;
    newKey = key;
  } catch (cause) {
    if (mounted && active && open && generation === displayGeneration) {
      error = readError(cause);
    }
  } finally {
    if (mounted) {
      pending = false;
    }
  }
}

async function copy() {
  if (!mounted || !active || !newKey) return;
  try {
    await navigator.clipboard.writeText(newKey);
    copied = true;
    setTimeout(() => {
      copied = false;
    }, 2000);
  } catch {
    // fallback if clipboard API fails
  }
}
</script>

<Dialog.Root open={open && active} onOpenChange={(val) => { if (!val) close(); }}>
  <Dialog.Content
    variant="destructive"
    aria-busy={pending}
    onEscapeKeydown={(event) => { if (pending) event.preventDefault(); }}
    onInteractOutside={(event) => { if (pending) event.preventDefault(); }}
  >
    <Dialog.Header>
      <Dialog.Title>{i18n.t('settings.proxy.rotateKeyTitle')}</Dialog.Title>
      <Dialog.Description class="break-words [overflow-wrap:anywhere]">
        {i18n.t('settings.proxy.rotateKeyDescription')}
      </Dialog.Description>
    </Dialog.Header>

    {#if error}
      <div class="state-banner error" role="alert">
        <span class="min-w-0 break-words [overflow-wrap:anywhere]">{error}</span>
      </div>
    {/if}

    {#if newKey}
      <div class="space-y-3">
        <div class="state-banner warning text-xs leading-5" role="alert">
          {i18n.t('settings.proxy.rotateKeyWarning')}
        </div>
        <div class="flex items-center gap-2">
          <input
            type="text"
            name="mcp-proxy-new-token"
            readonly
            value={newKey}
            class="input-field font-mono text-xs flex-1 select-all break-all"
            aria-label={i18n.t('settings.proxy.rotateKeySuccess')}
          >
          <Button variant="outline" size="sm" onclick={copy}>
            {#if copied}
              <Check size={14} class="text-[var(--status-success)]" />
              {i18n.t('settings.proxy.copied')}
            {:else}
              <Copy size={14} />
              {i18n.t('settings.proxy.copyKey')}
            {/if}
          </Button>
        </div>
      </div>
    {/if}

    <Dialog.Footer>
      {#if newKey}
        <Button variant="default" onclick={close}>{i18n.t('settings.proxy.close')}</Button>
      {:else}
        <Button variant="outline" disabled={pending} onclick={close}>{i18n.t('common.cancel')}</Button>
        <Button variant="destructive" disabled={pending} onclick={rotate}>
          {pending ? i18n.t('settings.saving') : i18n.t('settings.proxy.rotateKeyConfirm')}
        </Button>
      {/if}
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
