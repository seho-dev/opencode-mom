<script lang="ts">
import { RefreshCw } from '@lucide/svelte';
import { onDestroy, onMount } from 'svelte';
import { Button } from '$src/components/button/index.js';
import { readError } from '$src/components/configuration/read-error.js';
import PageHead from '$src/components/PageHead.svelte';
import { Switch } from '$src/components/switch/index.js';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import { toast } from '$src/shell/toast.svelte.js';
import AppUpdates from './AppUpdates.svelte';
import LidProtection from './LidProtection.svelte';

const config = getConfig();
const i18n = getI18n();
let enabled = $state(false);
let known = $state(false);
let loading = $state(true);
let saving = $state(false);
let error = $state('');
let failedSave = $state(false);
let request = 0;

async function load() {
  const seq = ++request;
  loading = true;
  error = '';
  failedSave = false;
  try {
    const status = await config.getAutostart();
    if (seq !== request) return;
    enabled = status;
    known = true;
  } catch (cause) {
    if (seq === request) {
      error = readError(cause);
      known = false;
    }
  } finally {
    if (seq === request) loading = false;
  }
}
async function toggle() {
  if (!known || loading || saving) return;
  const next = !enabled;
  const seq = ++request;
  saving = true;
  error = '';
  failedSave = false;
  try {
    const status = await config.setAutostart(next);
    if (seq !== request) return;
    enabled = status;
    toast({ variant: 'success', description: i18n.t('settings.autostartSaved') });
  } catch (cause) {
    if (seq === request) {
      error = readError(cause);
      failedSave = true;
    }
  } finally {
    if (seq === request) saving = false;
  }
}
onMount(() => {
  void load();
});
onDestroy(() => {
  request += 1;
});
</script>

<svelte:head><title>{i18n.t('settings.metaTitle')}</title></svelte:head>
<div class="[&_h1]:text-2xl [&_h1]:leading-8 max-[760px]:[&_h1]:text-[22px] max-[760px]:[&_h1]:leading-7">
  <PageHead eyebrow={i18n.t('settings.eyebrow')} title={i18n.t('settings.title')}>
    <Button variant="outline" disabled={loading || saving} onclick={load}
      ><RefreshCw size={14} />{i18n.t('configuration.refresh')}</Button
    >
  </PageHead>
</div>
<p class="page-description">{i18n.t('settings.description')}</p>
{#if error}
  <div class="state-banner error" role="alert">
    <span class="min-w-0 break-words [overflow-wrap:anywhere]"
      >{i18n.t(failedSave ? 'settings.saveFailed' : 'settings.loadFailed')} {error}</span
    >
    <Button variant="outline" size="sm" disabled={loading || saving} onclick={load}
      >{i18n.t('configuration.retry')}</Button
    >
  </div>
{/if}
<section class="panel max-w-3xl divide-y divide-[var(--border-default)]" aria-label={i18n.t('settings.title')}>
  <div class="flex items-center justify-between gap-6 px-4 py-3 sm:px-5 sm:py-4" aria-busy={loading || saving}>
    <div class="min-w-0">
      <label for="launch-at-login" class="font-[var(--font-heading)] text-[13px] leading-5 font-semibold"
        >{i18n.t('settings.autostart')}</label
      >
      <p id="autostart-description" class="mt-1 mb-0 text-xs leading-5 text-[var(--text-secondary)]">
        {i18n.t('settings.autostartHint')}
      </p>
      {#if loading || saving}
        <p class="mt-1 mb-0 text-xs text-[var(--text-muted)]" role="status">
          {i18n.t(saving ? 'settings.saving' : 'settings.loading')}
        </p>
      {/if}
    </div>
    <Switch
      id="launch-at-login"
      checked={enabled}
      disabled={!known || loading || saving}
      aria-describedby="autostart-description"
      onclick={(event) => { event.preventDefault(); void toggle(); }}
    />
  </div>
  <LidProtection />
  <AppUpdates />
</section>
