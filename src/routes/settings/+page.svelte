<script lang="ts">
import { RefreshCw } from '@lucide/svelte';
import { onDestroy, onMount, tick } from 'svelte';
import { goto } from '$app/navigation';
import { page as route } from '$app/state';
import { Button } from '$src/components/button/index.js';
import { readError } from '$src/components/configuration/read-error.js';
import PageHead from '$src/components/PageHead.svelte';
import { Switch } from '$src/components/switch/index.js';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import { toast } from '$src/shell/toast.svelte.js';
import AppUpdates from './AppUpdates.svelte';
import Behavior from './Behavior.svelte';
import LidProtection from './LidProtection.svelte';
import McpProxySettings from './McpProxySettings.svelte';
import MiniIsland from './MiniIsland.svelte';
import OpenCodeSettings from './OpenCodeSettings.svelte';
import SettingsSearch, { type SettingsSearchTarget } from './SettingsSearch.svelte';

type SettingsTab = 'general' | 'mcp-proxy' | 'behavior';

const config = getConfig();
const i18n = getI18n();

const tabOrder: SettingsTab[] = ['general', 'mcp-proxy', 'behavior'];
let generalTabEl = $state<HTMLButtonElement | null>(null);
let proxyTabEl = $state<HTMLButtonElement | null>(null);
let behaviorTabEl = $state<HTMLButtonElement | null>(null);

let proxySettingsRef = $state<{ refresh: () => Promise<void> } | null>(null);
let httpSettingsRef = $state<{ refresh: () => Promise<void> } | null>(null);
let behaviorRef = $state<{ refresh: () => Promise<void> } | null>(null);

let activeTab = $state<SettingsTab>(
  route.url.searchParams.get('tab') === 'behavior'
    ? 'behavior'
    : route.url.searchParams.get('tab') === 'mcp-proxy'
      ? 'mcp-proxy'
      : 'general',
);

let enabled = $state(false);
let known = $state(false);
let loading = $state(true);
let saving = $state(false);
let error = $state('');
let failedSave = $state(false);
let request = 0;

function selectTab(tab: SettingsTab): boolean {
  if (activeTab === tab) return true;
  activeTab = tab;
  const url = new URL(route.url);
  if (tab === 'general') {
    url.searchParams.delete('tab');
  } else {
    url.searchParams.set('tab', tab);
  }
  void goto(url.toString(), { replaceState: true, keepFocus: true, noScroll: true });
  if (tab === 'general') {
    void load();
  }
  return true;
}

function onTabKeys(event: KeyboardEvent) {
  const current = tabOrder.indexOf(activeTab);
  let next: SettingsTab | undefined;
  if (event.key === 'ArrowRight') next = tabOrder[(current + 1) % tabOrder.length];
  else if (event.key === 'ArrowLeft') next = tabOrder[(current - 1 + tabOrder.length) % tabOrder.length];
  else if (event.key === 'Home') next = tabOrder[0];
  else if (event.key === 'End') next = tabOrder[tabOrder.length - 1];
  if (!next) return;
  event.preventDefault();
  if (!selectTab(next)) return;
  if (next === 'general') generalTabEl?.focus();
  else if (next === 'mcp-proxy') proxyTabEl?.focus();
  else if (next === 'behavior') behaviorTabEl?.focus();
}

async function navigateToSetting(entry: SettingsSearchTarget) {
  if (!selectTab(entry.tab)) return;
  await tick();
  const target =
    document.getElementById(entry.target) ??
    (entry.fallback ? document.getElementById(entry.fallback) : null) ??
    document.getElementById(`settings-tabpanel-${entry.tab}`);
  target?.scrollIntoView({ block: 'center' });
  target?.focus({ preventScroll: true });
}

async function load() {
  if (activeTab !== 'general' || saving) return;
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
  if (activeTab !== 'general' || !known || loading || saving) return;
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

async function handleRefresh() {
  if (activeTab === 'general') {
    await Promise.all([load(), httpSettingsRef?.refresh()]);
  } else if (activeTab === 'mcp-proxy') {
    await proxySettingsRef?.refresh();
  } else if (activeTab === 'behavior') {
    await behaviorRef?.refresh();
  }
}

onMount(() => {
  if (activeTab === 'general') {
    void load();
  }
});

onDestroy(() => {
  request += 1;
});
</script>

<svelte:head><title>{i18n.t('settings.metaTitle')}</title></svelte:head>
<div class="[&_h1]:text-2xl [&_h1]:leading-8 max-[760px]:[&_h1]:text-[22px] max-[760px]:[&_h1]:leading-7">
  <PageHead eyebrow={i18n.t('settings.eyebrow')} title={i18n.t('settings.title')}>
    <Button
      variant="outline"
      size="sm"
      disabled={activeTab === 'general' ? loading || saving : false}
      aria-busy={activeTab === 'general' ? loading || saving : false}
      onclick={handleRefresh}
    >
      <RefreshCw
        size={14}
        class={activeTab === 'general' && (loading || saving) ? 'animate-spin motion-reduce:animate-none' : ''}
        aria-hidden="true"
      /> {i18n.t('configuration.refresh')}
    </Button>
  </PageHead>
</div>
<p class="page-description">{i18n.t('settings.description')}</p>

<SettingsSearch onselect={navigateToSetting} />

<!-- Tab navigation -->
<div class="flex flex-wrap gap-2 mb-4" role="tablist" aria-label={i18n.t('settings.title')}>
  <Button
    bind:ref={generalTabEl}
    id="settings-tab-general"
    role="tab"
    variant={activeTab === 'general' ? 'default' : 'outline'}
    size="sm"
    aria-selected={activeTab === 'general'}
    aria-controls="settings-tabpanel-general"
    tabindex={activeTab === 'general' ? 0 : -1}
    onclick={() => selectTab('general')}
    onkeydown={onTabKeys}
  >
    {i18n.t('settings.tabGeneral')}
  </Button>
  <Button
    bind:ref={proxyTabEl}
    id="settings-tab-mcp-proxy"
    role="tab"
    variant={activeTab === 'mcp-proxy' ? 'default' : 'outline'}
    size="sm"
    aria-selected={activeTab === 'mcp-proxy'}
    aria-controls="settings-tabpanel-mcp-proxy"
    tabindex={activeTab === 'mcp-proxy' ? 0 : -1}
    onclick={() => selectTab('mcp-proxy')}
    onkeydown={onTabKeys}
  >
    {i18n.t('settings.tabMcpProxy')}
  </Button>
  <Button
    bind:ref={behaviorTabEl}
    id="settings-tab-behavior"
    role="tab"
    variant={activeTab === 'behavior' ? 'default' : 'outline'}
    size="sm"
    aria-selected={activeTab === 'behavior'}
    aria-controls="settings-tabpanel-behavior"
    tabindex={activeTab === 'behavior' ? 0 : -1}
    onclick={() => selectTab('behavior')}
    onkeydown={onTabKeys}
  >
    {i18n.t('settings.behaviorTab')}
  </Button>
</div>

<div
  id="settings-tabpanel-general"
  role="tabpanel"
  tabindex="-1"
  aria-labelledby="settings-tab-general"
  class="max-w-3xl space-y-6"
  hidden={activeTab !== 'general'}
  aria-hidden={activeTab !== 'general'}
>
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
  <section aria-labelledby="settings-app-behavior">
    <h2 id="settings-app-behavior" class="mt-0 mb-2 font-[var(--font-heading)] text-sm leading-5 font-semibold">
      {i18n.t('settings.appBehavior')}
    </h2>
    <div class="panel rounded-[var(--radius)] divide-y divide-[var(--border-default)]">
      <div
        id="settings-autostart"
        tabindex="-1"
        class="flex items-center justify-between gap-6 px-4 py-3 sm:px-5 sm:py-4"
        aria-busy={loading || saving}
      >
        <div class="min-w-0">
          <label for="launch-at-login" class="font-[var(--font-heading)] text-[13px] leading-5 font-semibold"
            >{i18n.t('settings.autostart')}</label
          >
          <p id="autostart-description" class="field-hint mt-1">
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
          name="launch-at-login"
          checked={enabled}
          disabled={!known || loading || saving}
          aria-describedby="autostart-description"
          onclick={(event) => { event.preventDefault(); void toggle(); }}
        />
      </div>
      <MiniIsland active={activeTab === 'general'} />
      <LidProtection active={activeTab === 'general'} />
    </div>
  </section>
  <OpenCodeSettings active={activeTab === 'general'} />
  <McpProxySettings active={activeTab === 'general'} http bind:this={httpSettingsRef} />
  <AppUpdates active={activeTab === 'general'} />
</div>
<div
  id="settings-tabpanel-mcp-proxy"
  role="tabpanel"
  tabindex="-1"
  aria-labelledby="settings-tab-mcp-proxy"
  hidden={activeTab !== 'mcp-proxy'}
  aria-hidden={activeTab !== 'mcp-proxy'}
>
  <McpProxySettings active={activeTab === 'mcp-proxy'} bind:this={proxySettingsRef} />
</div>
<div
  id="settings-tabpanel-behavior"
  role="tabpanel"
  tabindex="-1"
  aria-labelledby="settings-tab-behavior"
  hidden={activeTab !== 'behavior'}
  aria-hidden={activeTab !== 'behavior'}
>
  <Behavior active={activeTab === 'behavior'} bind:this={behaviorRef} />
</div>
