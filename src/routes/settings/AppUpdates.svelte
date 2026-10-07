<script lang="ts">
import { ExternalLink, RefreshCw, Star } from '@lucide/svelte';
import { onDestroy, onMount } from 'svelte';
import { Button } from '$src/components/button/index.js';
import { readError } from '$src/components/configuration/read-error.js';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import type { AppInfo, ProjectPage, UpdateCheck } from '$src/types/app.js';

const config = getConfig();
const i18n = getI18n();
let appInfo = $state<AppInfo | null>(null);
let infoLoading = $state(true);
let infoError = $state('');
let checking = $state(false);
let result = $state<UpdateCheck | null>(null);
let checkError = $state('');
let opening = $state<ProjectPage | null>(null);
let openError = $state('');
let infoRequest = 0;
let checkRequest = 0;
let openRequest = 0;

async function loadInfo() {
  const seq = ++infoRequest;
  infoLoading = true;
  infoError = '';
  try {
    const info = await config.getAppInfo();
    if (seq === infoRequest) appInfo = info;
  } catch (cause) {
    if (seq === infoRequest) infoError = readError(cause);
  } finally {
    if (seq === infoRequest) infoLoading = false;
  }
}

async function check() {
  if (checking) return;
  const seq = ++checkRequest;
  checking = true;
  checkError = '';
  result = null;
  try {
    const update = await config.checkForUpdates();
    if (seq === checkRequest) result = update;
  } catch (cause) {
    if (seq === checkRequest) checkError = readError(cause);
  } finally {
    if (seq === checkRequest) checking = false;
  }
}

async function openProject(page: ProjectPage) {
  if (opening) return;
  const seq = ++openRequest;
  opening = page;
  openError = '';
  try {
    await config.openProjectPage(page);
  } catch (cause) {
    if (seq === openRequest) openError = readError(cause);
  } finally {
    if (seq === openRequest) opening = null;
  }
}

onMount(() => {
  void loadInfo();
});
onDestroy(() => {
  infoRequest += 1;
  checkRequest += 1;
  openRequest += 1;
});
</script>

<div class="px-4 py-3 sm:px-5 sm:py-4">
  <div class="flex flex-col items-start justify-between gap-3 sm:flex-row sm:gap-6">
    <div class="min-w-0">
      <h2 class="m-0 font-[var(--font-heading)] text-[13px] leading-5 font-semibold">
        {i18n.t('settings.updates')}
      </h2>
      <p class="mt-1 mb-0 break-words text-xs leading-5 text-[var(--text-secondary)]">
        {i18n.t('settings.installedVersion', {
          version: appInfo?.version ?? result?.currentVersion ?? i18n.t(infoLoading ? 'common.loading' : 'settings.versionUnavailable'),
        })}
      </p>
      <p class="mt-1 mb-0 text-xs leading-5 text-[var(--text-muted)]">{i18n.t('settings.updatesHint')}</p>
    </div>
    <Button variant="outline" disabled={checking} aria-busy={checking} onclick={check}>
      <RefreshCw size={14} class={checking ? 'animate-spin motion-reduce:animate-none' : ''} aria-hidden="true" />
      {i18n.t(checking ? 'settings.checkingUpdates' : checkError ? 'settings.retryUpdates' : 'settings.checkUpdates')}
    </Button>
  </div>

  {#if infoError}
    <div class="mt-3 flex flex-wrap items-center gap-2 text-xs leading-5 text-[var(--status-error)]" role="alert">
      <span class="min-w-0 break-words [overflow-wrap:anywhere]">{i18n.t('settings.appInfoFailed')} {infoError}</span>
      <Button variant="outline" size="sm" disabled={infoLoading} onclick={loadInfo}
        >{i18n.t('settings.retryAppInfo')}</Button
      >
    </div>
  {/if}
  <div role="status" aria-live="polite" aria-atomic="true" class="text-xs leading-5">
    {#if checking}
      <p class="mt-3 mb-0 text-[var(--text-secondary)]">{i18n.t('settings.checkingUpdates')}</p>
    {:else if result}
      <p class={['mt-3 mb-0 break-words', result.updateAvailable && 'text-[var(--accent-primary)]']}>
        {#if result.latestVersion === null}
          {i18n.t('settings.noRelease')}
        {:else if result.updateAvailable}
          {i18n.t('settings.updateAvailable', { version: result.latestVersion })}
        {:else}
          {i18n.t('settings.upToDate', { version: result.currentVersion })}
        {/if}
      </p>
    {/if}
  </div>
  {#if checkError}
    <p class="mt-3 mb-0 break-words text-xs leading-5 text-[var(--status-error)] [overflow-wrap:anywhere]" role="alert">
      {i18n.t('settings.updateCheckFailed')} {checkError}
    </p>
  {/if}
  {#if result?.updateAvailable}
    <Button
      variant="outline"
      size="sm"
      class="mt-3"
      disabled={opening !== null}
      aria-busy={opening === 'releases'}
      onclick={() => openProject('releases')}
    >
      <ExternalLink size={14} aria-hidden="true" />{i18n.t('settings.releaseDownloads')}
    </Button>
    {#if appInfo?.platform === 'macos'}
      <details
        open
        class="mt-3 border border-[var(--border-default)] px-3 py-2 text-xs leading-5 text-[var(--text-secondary)]"
      >
        <summary class="cursor-pointer font-semibold text-[var(--text-primary)]">
          {i18n.t('settings.macInstallGuide')}
        </summary>
        <div class="mt-3 space-y-3">
          <p class="m-0">{i18n.t('settings.macSigning')}</p>
          <p class="m-0 text-[var(--status-warning)]">{i18n.t('settings.macTrustWarning')}</p>
          <ol class="m-0 list-decimal space-y-1 pl-5">
            <li>{i18n.t('settings.macDownloadOfficial')}</li>
            <li>{i18n.t('settings.macReplaceApp')}</li>
            <li>{i18n.t('settings.macOpenAnyway')}</li>
          </ol>
          <p class="m-0">{i18n.t('settings.macManualQuarantine')}</p>
          <code
            class="block select-text border border-[var(--border-subtle)] bg-[var(--surface-input)] px-3 py-2 font-mono whitespace-pre-wrap break-words text-[var(--text-primary)] [overflow-wrap:anywhere]"
            >{i18n.t('settings.macQuarantineCommand')}</code
          >
          <p class="m-0 text-[var(--text-muted)]">{i18n.t('settings.macAuthorizationScope')}</p>
        </div>
      </details>
    {/if}
  {/if}

  <div
    class="mt-4 flex flex-col items-start justify-between gap-3 border-t border-[var(--border-subtle)] pt-4 sm:flex-row sm:gap-6"
  >
    <div class="min-w-0">
      <h3 class="m-0 font-[var(--font-heading)] text-xs leading-5 font-semibold">
        {i18n.t('settings.supportProject')}
      </h3>
      <p class="mt-1 mb-0 text-xs leading-5 text-[var(--text-secondary)]">{i18n.t('settings.starRequest')}</p>
    </div>
    <Button
      variant="outline"
      size="sm"
      disabled={opening !== null}
      aria-busy={opening === 'repository'}
      onclick={() => openProject('repository')}
    >
      <Star size={14} aria-hidden="true" />{i18n.t('settings.starProject')}
    </Button>
  </div>
  {#if openError}
    <p class="mt-3 mb-0 break-words text-xs leading-5 text-[var(--status-error)] [overflow-wrap:anywhere]" role="alert">
      {i18n.t('header.projectOpenFailed', { message: openError })}
    </p>
  {/if}
</div>
