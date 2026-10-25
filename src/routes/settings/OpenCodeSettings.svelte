<script lang="ts">
import { ArrowUp, Download, RefreshCw } from '@lucide/svelte';
import { isTauri } from '@tauri-apps/api/core';
import { onMount, untrack } from 'svelte';
import { Button } from '$src/components/button/index.js';
import { readError } from '$src/components/configuration/read-error.js';
import * as Dialog from '$src/components/dialog/index.js';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import type { AppInfo, OpenCodeInstallation, OpenCodeUpdateCheck } from '$src/types/app.js';
import OpenCodeService from './OpenCodeService.svelte';

let { active = true }: { active?: boolean } = $props();
const config = getConfig();
const i18n = getI18n();
let native = $state(false);
let appInfo = $state.raw<AppInfo | null>(null);
let installation = $state.raw<OpenCodeInstallation | null>(null);
let result = $state.raw<OpenCodeUpdateCheck | null>(null);
let pending = $state<'inspect' | 'check' | 'upgrade' | null>(null);
let error = $state('');
let errorKind = $state<'inspect' | 'check' | 'upgrade' | 'readback'>('inspect');
let needsRefresh = $state(false);
let upgradedVersion = $state<string | null>(null);
let confirmation = $state.raw<{ binaryPath: string; version: string; targetVersion: string } | null>(null);
let mounted = $state(false);
let request = 0;
let servicePanel = $state<ReturnType<typeof OpenCodeService>>();
let serviceBusy = $state(false);
const canUpgrade = $derived(
  native &&
    !serviceBusy &&
    appInfo !== null &&
    !needsRefresh &&
    installation?.canUpgrade &&
    installation.binaryPath &&
    installation.version &&
    result?.updateAvailable,
);

async function load() {
  if (!mounted || !active || !native || pending || serviceBusy || confirmation) return;
  const seq = ++request;
  pending = 'inspect';
  error = '';
  result = null;
  upgradedVersion = null;
  try {
    if (!appInfo) {
      const info = await config.getAppInfo();
      if (!mounted || !active || seq !== request) return;
      appInfo = info;
    }
    if (!active) return;
    const info = await config.getOpenCodeInstallation();
    if (!mounted || seq !== request) return;
    installation = info;
    needsRefresh = false;
  } catch (cause) {
    if (mounted && seq === request) {
      errorKind = 'inspect';
      error = readError(cause);
      needsRefresh = true;
    }
  } finally {
    if (mounted && seq === request) pending = null;
  }
}

async function check() {
  if (
    !mounted ||
    !active ||
    !native ||
    !appInfo ||
    !installation ||
    pending ||
    serviceBusy ||
    confirmation ||
    needsRefresh
  )
    return;
  const seq = ++request;
  pending = 'check';
  error = '';
  result = null;
  upgradedVersion = null;
  try {
    const update = await config.checkOpenCodeUpdate();
    if (!mounted || seq !== request) return;
    installation = update.installation;
    result = update;
  } catch (cause) {
    if (mounted && seq === request) {
      errorKind = 'check';
      error = readError(cause);
    }
  } finally {
    if (mounted && seq === request) pending = null;
  }
}

function confirmUpgrade() {
  if (!mounted || !active || !canUpgrade || pending || !installation?.binaryPath || !installation.version || !result)
    return;
  confirmation = {
    binaryPath: installation.binaryPath,
    version: installation.version,
    targetVersion: result.latestVersion,
  };
}
function closeConfirmation() {
  if (!pending) confirmation = null;
}
async function upgrade() {
  if (!mounted || !active || !canUpgrade || pending || !confirmation) return;
  const expected = confirmation;
  const seq = ++request;
  pending = 'upgrade';
  error = '';
  let completed = false;
  try {
    await config.upgradeOpenCode(expected.binaryPath, expected.version, expected.targetVersion);
    if (!mounted || seq !== request) return;
    completed = true;
    result = null;
    needsRefresh = true;
    if (!active) return;
    const info = await config.getOpenCodeInstallation();
    if (!mounted || seq !== request) return;
    installation = info;
    needsRefresh = false;
    upgradedVersion = info.version;
  } catch (cause) {
    if (mounted && seq === request) {
      errorKind = completed ? 'readback' : 'upgrade';
      error = readError(cause);
      needsRefresh = true;
      result = null;
    }
  } finally {
    if (mounted && seq === request) {
      if (completed && active) await servicePanel?.refresh();
      if (mounted && seq === request) {
        confirmation = null;
        pending = null;
      }
    }
  }
}

onMount(() => {
  mounted = true;
  return () => {
    mounted = false;
    request += 1;
  };
});

$effect(() => {
  if (!mounted || !active) return;
  untrack(() => {
    native = isTauri();
    if (native && !installation && !needsRefresh) void load();
  });
});
</script>

<section class="max-w-3xl" aria-labelledby="opencode-settings-title" aria-busy={pending !== null || serviceBusy}>
  <h2
    id="opencode-settings-title"
    tabindex="-1"
    class="mt-0 mb-2 font-[var(--font-heading)] text-sm leading-5 font-semibold"
  >
    {i18n.t('settings.opencodeTitle')}
  </h2>
  <div class="panel rounded-[var(--radius)] divide-y divide-[var(--border-default)]">
    {#if !native}
      <p class="m-0 px-4 py-3 text-xs leading-5 text-[var(--text-muted)] sm:px-5 sm:py-4">
        {i18n.t('settings.opencodeDesktopOnly')}
      </p>
    {:else}
      <OpenCodeService
        bind:this={servicePanel}
        {active}
        ready={appInfo !== null}
        blocked={pending !== null || confirmation !== null}
        bind:busy={serviceBusy}
      />
      <section class="px-4 py-3 sm:px-5 sm:py-4" aria-labelledby="opencode-updates-title">
        <div class="flex items-start justify-between gap-3 sm:gap-6">
          <div class="min-w-0">
            <h3 id="opencode-updates-title" class="m-0 text-xs leading-5 font-semibold">
              {i18n.t('settings.opencodeUpdates')}
            </h3>
            <p class="field-hint mt-1">{i18n.t('settings.opencodeHint')}</p>
            <p class="field-hint mt-1">{i18n.t('settings.opencodeCheckHint')}</p>
          </div>
          <div class="flex shrink-0 items-center gap-2">
            <Button
              variant="outline"
              size="icon"
              disabled={pending !== null || serviceBusy || confirmation !== null}
              aria-label={i18n.t('settings.opencodeRefresh')}
              title={i18n.t('settings.opencodeRefresh')}
              aria-busy={pending === 'inspect'}
              onclick={load}
            >
              <RefreshCw
                size={14}
                class={pending === 'inspect' ? 'animate-spin motion-reduce:animate-none' : ''}
                aria-hidden="true"
              />
            </Button>
            <Button
              variant="outline"
              size="icon"
              disabled={!installation || needsRefresh || pending !== null || serviceBusy || confirmation !== null}
              aria-label={i18n.t(error && errorKind === 'check' ? 'settings.opencodeRetryCheck' : 'settings.opencodeCheck')}
              title={i18n.t(error && errorKind === 'check' ? 'settings.opencodeRetryCheck' : 'settings.opencodeCheck')}
              aria-busy={pending === 'check'}
              onclick={check}
            >
              {#if pending === 'check'}
                <RefreshCw size={14} class="animate-spin motion-reduce:animate-none" aria-hidden="true" />
              {:else}
                <Download size={14} aria-hidden="true" />
              {/if}
            </Button>
          </div>
        </div>
        <div role="status" aria-live="polite" aria-atomic="true" class="text-xs leading-5">
          {#if pending}
            <p class="mt-3 mb-0 text-[var(--text-secondary)]">{i18n.t(`settings.opencodePending.${pending}`)}</p>
          {:else if upgradedVersion}
            <p class="mt-3 mb-0 text-[var(--accent-primary)]">
              {i18n.t('settings.opencodeUpgraded', { version: upgradedVersion })}
            </p>
            <p class="mt-1 mb-0 text-[var(--text-muted)]">{i18n.t('settings.opencodeRunningHint')}</p>
          {:else if result}
            <p class={['mt-3 mb-0 break-words', result.updateAvailable && 'text-[var(--accent-primary)]']}>
              {i18n.t(result.updateAvailable ? 'settings.opencodeAvailable' : 'settings.opencodeCurrent', { version: result.latestVersion })}
            </p>
            {#if result.updateAvailable && !result.installation.canUpgrade}
              <p class="mt-1 mb-0 text-[var(--text-muted)]">{i18n.t('settings.opencodeReason.unsupported')}</p>
            {/if}
          {/if}
        </div>
        {#if canUpgrade}
          <Button size="sm" class="mt-3" disabled={pending !== null || confirmation !== null} onclick={confirmUpgrade}>
            <ArrowUp size={14} aria-hidden="true" />{i18n.t('settings.opencodeUpgrade')}
          </Button>
        {/if}
        {#if error}
          <div class="mt-3 flex flex-wrap items-center gap-2 text-xs leading-5 text-[var(--status-error)]" role="alert">
            <span class="min-w-0 break-words [overflow-wrap:anywhere]"
              >{i18n.t(`settings.opencodeError.${errorKind}`)}</span
            >
            {#if errorKind !== 'check'}
              <Button
                variant="outline"
                size="icon"
                disabled={pending !== null || serviceBusy || confirmation !== null}
                aria-label={i18n.t('settings.opencodeRetry')}
                title={i18n.t('settings.opencodeRetry')}
                onclick={load}
              >
                <RefreshCw size={14} aria-hidden="true" />
              </Button>
            {/if}
          </div>
        {/if}
      </section>
    {/if}
  </div>
</section>

<Dialog.Root open={active && confirmation !== null} onOpenChange={(open) => { if (!open) closeConfirmation(); }}>
  <Dialog.Content
    aria-busy={pending === 'upgrade'}
    onEscapeKeydown={(event) => { if (pending) event.preventDefault(); }}
    onInteractOutside={(event) => { if (pending) event.preventDefault(); }}
  >
    <Dialog.Header>
      <Dialog.Title>{i18n.t('settings.opencodeConfirmTitle')}</Dialog.Title>
      <Dialog.Description>{i18n.t('settings.opencodeConfirmDescription')}</Dialog.Description>
    </Dialog.Header>
    <dl class="m-0 min-w-0 space-y-3 text-xs leading-5">
      <div>
        <dt class="text-[var(--text-muted)]">{i18n.t('settings.opencodePath')}</dt>
        <dd class="m-0">
          <code class="select-text font-mono break-words [overflow-wrap:anywhere]">{confirmation?.binaryPath}</code>
        </dd>
      </div>
      <div>
        <dt class="text-[var(--text-muted)]">{i18n.t('settings.opencodeVersionChange')}</dt>
        <dd class="m-0 select-text break-words">{confirmation?.version} → {confirmation?.targetVersion}</dd>
      </div>
    </dl>
    <p class="m-0 text-xs leading-5 text-[var(--status-warning)]">{i18n.t('settings.opencodeUpgradeWarning')}</p>
    <Dialog.Footer>
      <Button variant="outline" disabled={pending !== null} onclick={closeConfirmation}
        >{i18n.t('common.cancel')}</Button
      >
      <Button disabled={pending !== null} onclick={upgrade}>
        {i18n.t(pending === 'upgrade' ? 'settings.opencodePending.upgrade' : 'settings.opencodeConfirm')}
      </Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
