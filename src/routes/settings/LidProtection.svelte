<script lang="ts">
import { RefreshCw } from '@lucide/svelte';
import { onMount, tick } from 'svelte';
import { Button } from '$src/components/button/index.js';
import { readError } from '$src/components/configuration/read-error.js';
import { Switch } from '$src/components/switch/index.js';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import type { LidState } from '$src/types/power.js';

const config = getConfig();
const i18n = getI18n();
let state = $state<LidState>({ enabled: false, phase: 'disabled' });
let known = $state(false);
let loading = $state(true);
let querying = $state(false);
let refreshing = $state(false);
let saving = $state(false);
let savingEnabled = $state(false);
let queryError = $state('');
let saveError = $state('');
let mounted = false;
let request = 0;
let readGeneration = 0;
let timer: ReturnType<typeof setTimeout> | undefined;
const failure = $derived(saveError || queryError || state.error || '');
const uncertain = $derived(!known || !!failure || state.phase === 'unknown' || state.phase === 'error');
const protectedNow = $derived(!saving && !loading && !refreshing && !uncertain && state.phase === 'protected');
const status = $derived.by(() => {
  if (saving) return i18n.t(savingEnabled ? 'settings.lidAuthorizing' : 'settings.lidRestoring');
  if (loading || refreshing) return i18n.t('settings.lidLoading');
  if (failure || state.phase === 'error') return i18n.t('settings.lidError');
  if (!known || state.phase === 'unknown') return i18n.t('settings.lidUnknown');
  if (state.phase === 'protected') {
    if (state.activeSessions === undefined) return i18n.t('settings.lidProtected');
    return i18n.t(state.activeSessions === 1 ? 'settings.lidProtectedOne' : 'settings.lidProtectedCount', {
      count: state.activeSessions,
    });
  }
  if (state.phase === 'checking') return i18n.t('settings.lidChecking');
  return i18n.t(state.phase === 'idle' ? 'settings.lidIdle' : 'settings.lidDisabled');
});

function schedule() {
  clearTimeout(timer);
  if (mounted && !querying && !saving) timer = setTimeout(() => void readStatus(), 2000);
}
async function readStatus(manual = false) {
  if (!mounted || querying || saving) return;
  clearTimeout(timer);
  const seq = ++request;
  const readSeq = ++readGeneration;
  querying = true;
  refreshing = manual;
  try {
    const result = await config.getLidProtection();
    if (!mounted || seq !== request) return;
    state = result;
    known = true;
    queryError = '';
    if (manual) saveError = '';
  } catch (cause) {
    if (mounted && seq === request) queryError = readError(cause);
  } finally {
    if (mounted) {
      if (readSeq === readGeneration) {
        querying = false;
        refreshing = false;
        loading = false;
      }
      if (!saving && !querying) schedule();
    }
  }
}
async function setEnabled(enabled: boolean) {
  if (!mounted || saving) return;
  const control = document.getElementById('lid-closed');
  const restoreFocus = document.activeElement === control;
  clearTimeout(timer);
  // A pending status read must never overwrite a newer native setting result.
  const seq = ++request;
  saving = true;
  savingEnabled = enabled;
  refreshing = false;
  saveError = '';
  try {
    const result = await config.setLidProtection(enabled);
    if (!mounted || seq !== request) return;
    state = result;
    known = true;
    queryError = '';
  } catch (cause) {
    if (mounted && seq === request) saveError = readError(cause);
  } finally {
    if (mounted && seq === request) {
      saving = false;
      loading = false;
      schedule();
      if (restoreFocus) {
        await tick();
        if (mounted && seq === request && document.activeElement === document.body) control?.focus();
      }
    }
  }
}
onMount(() => {
  mounted = true;
  void readStatus();
  return () => {
    mounted = false;
    request += 1;
    clearTimeout(timer);
  };
});
</script>

<div class="px-4 py-3 sm:px-5 sm:py-4">
  <div class="flex items-start justify-between gap-6">
    <div class="min-w-0">
      <label for="lid-closed" class="font-[var(--font-heading)] text-[13px] leading-5 font-semibold"
        >{i18n.t('settings.lid')}</label
      >
      <p id="lid-description" class="mt-1 mb-0 text-xs leading-5 text-[var(--text-secondary)]">
        {i18n.t('settings.lidHint')}
      </p>
    </div>
    <Switch
      id="lid-closed"
      checked={state.enabled}
      disabled={!known || saving}
      aria-describedby={`lid-description lid-status lid-close-note lid-authorization${failure || state.phase === 'error' ? ' lid-error' : ''}`}
      aria-busy={loading || saving}
      class="mt-0.5"
      onclick={(event) => { event.preventDefault(); void setEnabled(!state.enabled); }}
    />
  </div>
  <div class="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1">
    <p
      id="lid-status"
      class={`m-0 min-w-0 text-xs leading-5 ${protectedNow ? 'text-[var(--accent-primary)]' : 'text-[var(--text-secondary)]'}`}
      role="status"
      aria-atomic="true"
    >
      {status}
    </p>
    <button
      type="button"
      class="inline-flex shrink-0 items-center gap-1 text-xs leading-5 text-[var(--text-muted)] hover:text-[var(--text-primary)] focus-visible:outline-1 focus-visible:outline-[var(--focus-ring)] disabled:cursor-not-allowed disabled:opacity-[var(--disabled-opacity)]"
      disabled={querying || saving}
      onclick={() => void readStatus(true)}
    >
      <RefreshCw size={12} aria-hidden="true" />{i18n.t('settings.lidRefresh')}
    </button>
  </div>
  <p id="lid-close-note" class="mt-1 mb-0 text-xs leading-5 text-[var(--text-secondary)]">
    {i18n.t('settings.lidWait')}
  </p>
  {#if failure || state.phase === 'error'}
    <div class="mt-2 flex flex-wrap items-center gap-2 text-xs leading-5">
      <p
        id="lid-error"
        class="m-0 min-w-0 basis-full break-words text-[var(--status-error)] [overflow-wrap:anywhere]"
        role="alert"
      >
        {i18n.t(saveError ? 'settings.lidSaveFailed' : 'settings.lidLoadFailed')} {failure}
      </p>
      <Button variant="outline" size="sm" disabled={querying || saving} onclick={() => void readStatus(true)}
        >{i18n.t('settings.lidRetry')}</Button
      >
    </div>
  {/if}
  {#if uncertain && !loading}
    <Button class="mt-2" variant="outline" size="sm" disabled={saving} onclick={() => void setEnabled(false)}
      >{i18n.t('settings.lidTurnOff')}</Button
    >
  {/if}
  <details class="mt-2 text-xs leading-5 text-[var(--text-muted)]">
    <summary
      class="w-fit cursor-pointer hover:text-[var(--text-primary)] focus-visible:outline-1 focus-visible:outline-[var(--focus-ring)]"
    >
      {i18n.t('settings.lidDetails')}
    </summary>
    <div class="mt-2 space-y-2 text-[var(--text-secondary)]">
      <p id="lid-authorization" class="m-0">{i18n.t('settings.lidAuthorization')}</p>
      <p class="m-0">{i18n.t('settings.lidScope')}</p>
      <p class="m-0">{i18n.t('settings.lidRestore')}</p>
      <p class="m-0">{i18n.t('settings.lidSafety')}</p>
      <p class="m-0">{i18n.t('settings.lidRecovery')}</p>
    </div>
  </details>
</div>
