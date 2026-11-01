<script lang="ts">
import { onMount, tick, untrack } from 'svelte';
import { Switch } from '$src/components/switch/index.js';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';

const config = getConfig();
const i18n = getI18n();
let { active = true }: { active?: boolean } = $props();
let enabled = $state(false);
let known = $state(false);
let loading = $state(false);
let saving = $state(false);
let error = $state<'' | 'read' | 'save'>('');
let mounted = $state(false);
let request = 0;
let targetVersion = config.serviceTargetVersion;
let reloadQueued = false;

$effect(() => {
  const version = config.serviceTargetVersion;
  if (version === targetVersion) return;
  targetVersion = version;
  untrack(() => {
    known = false;
    error = '';
    reloadQueued = true;
    void load();
  });
});

async function load() {
  if (!mounted || !active || loading || saving) return;
  const seq = ++request;
  const version = config.serviceTargetVersion;
  reloadQueued = false;
  loading = true;
  error = '';
  try {
    const result = await config.getLidProtection();
    if (!mounted || seq !== request || version !== config.serviceTargetVersion) return;
    enabled = result.enabled;
    known = true;
  } catch {
    if (mounted && seq === request && version === config.serviceTargetVersion) error = 'read';
  } finally {
    if (mounted && seq === request) {
      loading = false;
      if (reloadQueued) void load();
    }
  }
}

async function setEnabled(next: boolean) {
  if (!mounted || !active || !known || loading || saving) return;
  const control = document.getElementById('lid-closed');
  const restoreFocus = document.activeElement === control;
  const seq = ++request;
  const version = config.serviceTargetVersion;
  saving = true;
  error = '';
  try {
    const result = await config.setLidProtection(next);
    if (!mounted || seq !== request || version !== config.serviceTargetVersion) return;
    enabled = result.enabled;
    known = true;
    if (result.enabled !== next || result.phase === 'error' || result.error) error = 'save';
  } catch {
    if (mounted && seq === request && version === config.serviceTargetVersion) error = 'save';
  } finally {
    if (mounted && seq === request) {
      saving = false;
      if (reloadQueued) void load();
      if (restoreFocus) {
        await tick();
        if (mounted && active && seq === request && document.activeElement === document.body) control?.focus();
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
    known = false;
    error = '';
    reloadQueued = true;
    void load();
  });
});
</script>

<div id="settings-lid" tabindex="-1" class="px-4 py-3 sm:px-5 sm:py-4">
  <div class="flex items-start justify-between gap-6">
    <div class="min-w-0">
      <label for="lid-closed" class="font-[var(--font-heading)] text-[13px] leading-5 font-semibold"
        >{i18n.t('settings.lid')}</label
      >
      <p id="lid-description" class="field-hint mt-1">{i18n.t('settings.lidHint')}</p>
    </div>
    <Switch
      id="lid-closed"
      name="lid-closed"
      checked={enabled}
      disabled={!active || !known || loading || saving}
      aria-describedby={`lid-description lid-authorization lid-safety${error ? ' lid-error' : ''}`}
      aria-busy={loading || saving}
      class="mt-0.5"
      onclick={(event) => { event.preventDefault(); void setEnabled(!enabled); }}
    />
  </div>
  <p id="lid-authorization" class="mt-2 mb-0 text-xs leading-5 text-[var(--text-secondary)]">
    {i18n.t('settings.lidAuthorization')}
  </p>
  <p id="lid-safety" class="mt-1 mb-0 text-xs leading-5 text-[var(--text-secondary)]">
    {i18n.t('settings.lidSafety')}
  </p>
  {#if error}
    <p id="lid-error" class="mt-2 mb-0 break-words text-xs leading-5 text-[var(--status-error)]" role="alert">
      {i18n.t(error === 'read' ? 'settings.lidLoadFailed' : 'settings.lidSaveFailed')}
    </p>
  {/if}
</div>
