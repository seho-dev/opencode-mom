<script lang="ts">
import { KeyRound, RefreshCw, ShieldAlert } from '@lucide/svelte';
import { onDestroy, onMount, untrack } from 'svelte';
import { Button } from '$src/components/button/index.js';
import { readError } from '$src/components/configuration/read-error.js';
import { Input } from '$src/components/input/index.js';
import { Switch } from '$src/components/switch/index.js';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import { toast } from '$src/shell/toast.svelte.js';
import type { McpProxySettingsDto, McpProxyStateDto, MomHttpSettings } from '$src/types/mcp-proxy.js';
import RotateKeyDialog from './RotateKeyDialog.svelte';

let { active = true, http = false }: { active?: boolean; http?: boolean } = $props();

const config = getConfig();
const i18n = getI18n();

let runtimeState = $state<McpProxyStateDto | null>(null);
let settings = $state<McpProxySettingsDto & MomHttpSettings>({
  enabled: false,
  port: 18765,
  allowLan: false,
  idleTimeout: 300,
  maxUnits: 16,
  revision: 0,
});

let known = $state(false);
let savedSettings = $state<(McpProxySettingsDto & MomHttpSettings) | null>(null);
let loading = $state(true);
let querying = $state(false);
let refreshing = $state(false);
let saving = $state(false);
let error = $state('');
let rotateDialogOpen = $state(false);

let mounted = $state(false);
let request = 0;
let timer: ReturnType<typeof setTimeout> | undefined;

const isDirty = $derived(savedSettings !== null && JSON.stringify(settings) !== JSON.stringify(savedSettings));
const validPort = $derived(Number.isInteger(settings.port) && settings.port >= 1 && settings.port <= 65535);

function schedule() {
  clearTimeout(timer);
  if (mounted && active && !querying && !saving) {
    timer = setTimeout(() => void readStatus(), 2000);
  }
}

async function readStatus(manual = false) {
  if (!mounted || !active || querying || saving) return;
  clearTimeout(timer);
  const seq = ++request;
  querying = true;
  refreshing = manual;
  try {
    const [fetchedSettings, fetchedState] = await Promise.all([
      http ? config.getMomHttpSettings() : config.getMcpProxySettings(),
      config.getMcpProxyState(),
    ]);
    if (!mounted || seq !== request) return;
    if (!isDirty) {
      settings = { ...settings, ...fetchedSettings };
      savedSettings = { ...settings };
    }
    runtimeState = fetchedState;
    known = true;
    error = '';
  } catch (cause) {
    if (mounted && seq === request) {
      error = readError(cause);
      runtimeState = null;
    }
  } finally {
    if (mounted) {
      querying = false;
      refreshing = false;
      loading = false;
      if (!saving && !querying) schedule();
    }
  }
}

async function save() {
  if (!mounted || !active || saving || querying || !known || (http && !validPort)) return;
  clearTimeout(timer);
  const seq = ++request;
  saving = true;
  error = '';
  try {
    if (http) {
      await config.setMomHttpSettings({ port: settings.port, allowLan: settings.allowLan });
    } else {
      const { enabled, idleTimeout, maxUnits, revision } = settings;
      await config.saveMcpProxySettings({ enabled, idleTimeout, maxUnits, revision }, revision);
    }
    if (!mounted || seq !== request) return;
    savedSettings = { ...settings };
    toast({
      variant: 'success',
      description: i18n.t(http ? 'settings.http.saveSuccess' : 'settings.proxy.saveSuccess'),
    });
    saving = false;
    await readStatus(true);
  } catch (cause) {
    if (mounted && seq === request) {
      error = readError(cause);
    }
  } finally {
    if (mounted) {
      saving = false;
      schedule();
    }
  }
}

onMount(() => {
  mounted = true;
});

$effect(() => {
  if (!mounted) return;
  if (active) {
    untrack(() => void readStatus());
  } else {
    clearTimeout(timer);
  }
});

onDestroy(() => {
  mounted = false;
  request += 1;
  clearTimeout(timer);
});

export function refresh() {
  return readStatus(true);
}
</script>

{#if http}
  <section class="max-w-3xl" aria-labelledby="settings-http-title">
    <h2 id="settings-http-title" class="mt-0 mb-2 font-[var(--font-heading)] text-sm leading-5 font-semibold">
      {i18n.t('settings.http.title')}
    </h2>
    <div class="panel rounded-[var(--radius)] divide-y divide-[var(--border-default)]">
      {#if error}
        <div class="px-4 py-3 sm:px-5 sm:py-4">
          <div class="state-banner error" role="alert">
            <span class="min-w-0 break-words [overflow-wrap:anywhere]">{error}</span>
            <Button variant="outline" size="sm" disabled={saving || querying} onclick={() => readStatus(true)}>
              {i18n.t('configuration.retry')}
            </Button>
          </div>
        </div>
      {/if}

      <!-- Port Input -->
      <div id="settings-http-port" tabindex="-1" class="px-4 py-3 sm:px-5 sm:py-4">
        <p class="field-hint mb-4">{i18n.t('settings.http.description')}</p>
        <label for="mom-http-port" class="font-[var(--font-heading)] text-[13px] leading-5 font-semibold block">
          {i18n.t('settings.http.port')}
        </label>
        <p class="field-hint mt-1 mb-2">{i18n.t('settings.http.portHint')}</p>
        <Input
          id="mom-http-port"
          name="mom-http-port"
          type="number"
          min={1}
          max={65535}
          disabled={loading || saving || !known}
          bind:value={settings.port}
          class="max-w-xs"
        />
      </div>

      <!-- Allow LAN Switch + Warning -->
      <div id="settings-http-lan" tabindex="-1" class="px-4 py-3 sm:px-5 sm:py-4 space-y-3">
        <div class="flex items-center justify-between gap-6">
          <div class="min-w-0">
            <label for="mom-http-allow-lan" class="font-[var(--font-heading)] text-[13px] leading-5 font-semibold">
              {i18n.t('settings.http.allowLan')}
            </label>
            <p class="field-hint mt-1">{i18n.t('settings.http.allowLanHint')}</p>
          </div>
          <Switch
            id="mom-http-allow-lan"
            name="mom-http-allow-lan"
            bind:checked={settings.allowLan}
            disabled={loading || saving || !known}
          />
        </div>

        {#if settings.allowLan}
          <div class="state-banner warning text-xs leading-5" role="alert">
            {i18n.t('settings.http.allowLanWarning')}
          </div>
        {/if}
      </div>

      <!-- Save Button Footer -->
      <div class="flex justify-end gap-2 px-4 py-3 sm:px-5 sm:py-4">
        <Button variant="default" disabled={loading || saving || querying || !known || !validPort} onclick={save}>
          {saving ? i18n.t('settings.proxy.saving') : i18n.t('settings.http.saveSettings')}
        </Button>
      </div>
    </div>
  </section>
{:else}
  <div class="space-y-6">
    {#if error}
      <div class="state-banner error" role="alert">
        <span class="min-w-0 break-words [overflow-wrap:anywhere]">{error}</span>
        <Button variant="outline" size="sm" disabled={saving || querying} onclick={() => readStatus(true)}>
          {i18n.t('configuration.retry')}
        </Button>
      </div>
    {/if}

    {#if runtimeState?.securityErrors && runtimeState.securityErrors.length > 0}
      <div class="state-banner warning space-y-1" role="alert">
        <div class="flex items-center gap-2 font-medium">
          <ShieldAlert size={16} />
          {i18n.t('mcp.proxy.securityErrors')}
        </div>
        <ul class="list-disc pl-5 text-xs m-0 space-y-0.5">
          {#each runtimeState.securityErrors as secErr}
            <li>{secErr}</li>
          {/each}
        </ul>
      </div>
    {/if}

    <section
      class="panel max-w-3xl divide-y divide-[var(--border-default)]"
      aria-label={i18n.t('settings.proxy.title')}
    >
      <!-- Header with status and actions -->
      <div class="flex items-center justify-between gap-4 px-4 py-3 sm:px-5 sm:py-4">
        <div>
          <h2 class="font-[var(--font-heading)] text-sm font-semibold">
            {i18n.t('settings.proxy.title')}
          </h2>
          <p class="field-hint mt-1">
            {i18n.t('settings.proxy.description')}
          </p>
        </div>
        <div class="flex items-center gap-2">
          <Button
            variant="outline"
            size="icon-sm"
            disabled={loading || saving || querying}
            onclick={() => readStatus(true)}
            aria-label={i18n.t('mcp.proxy.refresh')}
            title={i18n.t('mcp.proxy.refresh')}
          >
            <RefreshCw size={14} class={refreshing ? 'animate-spin motion-reduce:animate-none' : ''} />
          </Button>
        </div>
      </div>

      <!-- Idle Timeout & Max Units -->
      <div class="grid grid-cols-1 sm:grid-cols-2 gap-4 px-4 py-3 sm:px-5 sm:py-4">
        <div id="settings-proxy-timeout" tabindex="-1">
          <label for="proxy-idle-timeout" class="font-[var(--font-heading)] text-[13px] leading-5 font-semibold block">
            {i18n.t('settings.proxy.idleTimeout')}
          </label>
          <p class="field-hint mt-1 mb-2">{i18n.t('settings.proxy.idleTimeoutHint')}</p>
          <Input
            id="proxy-idle-timeout"
            name="proxy-idle-timeout"
            type="number"
            min={5}
            max={86400}
            disabled={loading || saving || !known}
            bind:value={settings.idleTimeout}
          />
        </div>

        <div id="settings-proxy-capacity" tabindex="-1">
          <label for="proxy-max-units" class="font-[var(--font-heading)] text-[13px] leading-5 font-semibold block">
            {i18n.t('settings.proxy.maxUnits')}
          </label>
          <p class="field-hint mt-1 mb-2">{i18n.t('settings.proxy.maxUnitsHint')}</p>
          <Input
            id="proxy-max-units"
            name="proxy-max-units"
            type="number"
            min={1}
            max={256}
            disabled={loading || saving || !known}
            bind:value={settings.maxUnits}
          />
        </div>
      </div>

      <!-- Token Rotation -->
      <div
        id="settings-proxy-token"
        tabindex="-1"
        class="flex flex-col items-start justify-between gap-6 px-4 py-3 sm:flex-row sm:items-center sm:px-5 sm:py-4"
      >
        <div class="min-w-0">
          <span class="font-[var(--font-heading)] text-[13px] leading-5 font-semibold block">
            {i18n.t('settings.proxy.rotateKey')}
          </span>
          <p class="field-hint mt-1">
            {i18n.t('settings.proxy.rotateKeyDescription')}
          </p>
        </div>
        <Button
          variant="outline"
          size="sm"
          disabled={loading || saving || !known}
          onclick={() => (rotateDialogOpen = true)}
        >
          <KeyRound size={14} />
          {i18n.t('settings.proxy.rotateKey')}
        </Button>
      </div>

      <!-- Save Button Footer -->
      <div class="flex justify-end gap-2 px-4 py-3 sm:px-5 sm:py-4">
        <Button variant="default" disabled={loading || saving || querying || !known} onclick={save}>
          {saving ? i18n.t('settings.proxy.saving') : i18n.t('settings.proxy.saveSettings')}
        </Button>
      </div>
    </section>
  </div>

  <RotateKeyDialog {active} bind:open={rotateDialogOpen} onRotated={() => readStatus(true)} />
{/if}
