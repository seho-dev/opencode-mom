<script lang="ts">
import { isTauri } from '@tauri-apps/api/core';
import { onMount, untrack } from 'svelte';
import { Button } from '$src/components/button/index.js';
import { readError } from '$src/components/configuration/read-error.js';
import { Switch } from '$src/components/switch/index.js';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import type { IslandState } from '$src/types/island.js';

const config = getConfig();
const i18n = getI18n();
let { active = true }: { active?: boolean } = $props();
let state = $state.raw<IslandState | null>(null);
let macos = $state(false);
let loading = $state(false);
let saving = $state(false);
let error = $state('');
let mounted = $state(false);
let platformChecked = false;
let request = 0;
let targetVersion = config.serviceTargetVersion;
let reloadQueued = false;

$effect(() => {
  const version = config.serviceTargetVersion;
  if (version === targetVersion) return;
  targetVersion = version;
  untrack(() => {
    reloadQueued = true;
    if (active && macos) void load();
  });
});

function reloadLatest() {
  if (reloadQueued && active && macos) void load();
}

async function load() {
  if (!mounted || !active || loading || saving) return;
  const seq = ++request;
  const version = config.serviceTargetVersion;
  reloadQueued = false;
  loading = true;
  error = '';
  try {
    const result = await config.getIslandState();
    if (mounted && seq === request && version === config.serviceTargetVersion) state = result;
  } catch (cause) {
    if (mounted && seq === request && version === config.serviceTargetVersion) error = readError(cause);
  } finally {
    if (mounted && seq === request) {
      loading = false;
      reloadLatest();
    }
  }
}
async function toggle() {
  if (!mounted || !active || !state?.supported || loading || saving) return;
  const seq = ++request;
  const version = config.serviceTargetVersion;
  saving = true;
  error = '';
  try {
    const result = await config.setIslandEnabled(!state.enabled);
    if (mounted && seq === request && version === config.serviceTargetVersion) state = result;
  } catch (cause) {
    if (mounted && seq === request && version === config.serviceTargetVersion) error = readError(cause);
  } finally {
    if (mounted && seq === request) {
      saving = false;
      reloadLatest();
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
    if (macos) {
      reloadQueued = true;
      void load();
      return;
    }
    if (platformChecked || !isTauri()) return;
    platformChecked = true;
    void config
      .getAppInfo()
      .then((info) => {
        if (!mounted) return;
        macos = info.platform === 'macos';
        if (macos && active) void load();
      })
      .catch(() => {
        if (mounted) platformChecked = false;
      });
  });
});
</script>

{#if macos && state?.supported !== false}
  <div id="settings-island" tabindex="-1" class="px-4 py-3 sm:px-5 sm:py-4" aria-busy={loading || saving}>
    <div class="flex items-start justify-between gap-6">
      <div class="min-w-0">
        <label for="mini-island" class="font-[var(--font-heading)] text-[13px] leading-5 font-semibold"
          >{i18n.t('settings.island')}</label
        >
        <p id="island-description" class="field-hint mt-1">
          {i18n.t('settings.islandHint')}
        </p>
        <p class="field-hint mt-1">{i18n.t('island.closeHint')}</p>
        {#if loading || saving}
          <p class="mt-1 mb-0 text-xs text-[var(--text-muted)]" role="status">
            {i18n.t(saving ? 'settings.islandSaving' : 'settings.islandLoading')}
          </p>
        {/if}
      </div>
      <Switch
        id="mini-island"
        name="mini-island"
        checked={state?.enabled ?? false}
        disabled={!state || loading || saving}
        aria-describedby="island-description"
        onclick={(event) => { event.preventDefault(); void toggle(); }}
      />
    </div>
    {#if error || state?.error}
      <div class="mt-3 flex flex-wrap items-center gap-2 text-xs leading-5 text-[var(--status-error)]" role="alert">
        <span class="min-w-0 break-words">{i18n.t('settings.islandFailed')} {error || state?.error}</span
        ><Button variant="outline" size="sm" disabled={loading || saving} onclick={load}
          >{i18n.t('settings.islandRetry')}</Button
        >
      </div>
    {/if}
  </div>
{/if}
