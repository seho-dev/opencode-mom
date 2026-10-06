<script lang="ts">
import { ExternalLink, Power, RefreshCw, Settings } from '@lucide/svelte';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { onMount } from 'svelte';
import { Button } from '$src/components/button/index.js';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import type { Group } from '$src/types/groups.js';

const config = getConfig();
const i18n = getI18n();
const typeLabels = $derived({
  native: i18n.t('tray.typeNative'),
  slim: i18n.t('groupForm.typeSlim'),
  omo: i18n.t('groupForm.typeOmo'),
});
const statusHelp = $derived(i18n.t('tray.statusHelp'));
let desktop = $state(false);
let health = $state<'unchecked' | 'checking' | 'online' | 'unavailable'>('unchecked');
let healthSessions = $state(0);
let healthError = $state<unknown>(null);
let action = $state<'reload' | 'switch' | 'open' | 'settings' | 'quit' | null>(null);
let actionError = $state<{ cause: unknown; updates?: boolean } | null>(null);
let notice = $state(false);
let visible = false;
let disposed = false;
let healthRequest = 0;
const busy = $derived(action !== null || config.loading || config.saving || config.switching || config.reloading);
const error = $derived(
  actionError
    ? actionError.updates
      ? i18n.t('tray.updatesFailed', { message: errorMessage(actionError.cause) })
      : errorMessage(actionError.cause)
    : config.error?.message || '',
);
const healthLabel = $derived(i18n.t(`tray.status.${health}`));
const healthDetail = $derived(
  health === 'online'
    ? i18n.t(healthSessions === 1 ? 'tray.statusOnlineOne' : 'tray.statusOnlineMany', {
        count: healthSessions,
        help: statusHelp,
      })
    : health === 'unavailable'
      ? desktop
        ? i18n.t('tray.statusUnavailable', { message: errorMessage(healthError), help: statusHelp })
        : i18n.t('tray.desktopOnly')
      : statusHelp,
);

function errorMessage(cause: unknown) {
  if (cause && typeof cause === 'object' && 'message' in cause) return String(cause.message);
  return typeof cause === 'string' ? cause : i18n.t('tray.operationFailed');
}

async function checkHealth() {
  const request = ++healthRequest;
  health = 'checking';
  try {
    const sessions = await invoke<number>('opencode_active_sessions');
    if (disposed || request !== healthRequest || !visible) return;
    health = 'online';
    healthSessions = sessions;
  } catch (cause) {
    if (disposed || request !== healthRequest || !visible) return;
    health = 'unavailable';
    healthError = cause;
  }
}

function shown() {
  if (disposed || visible) return;
  visible = true;
  actionError = null;
  notice = false;
  void config.refresh().catch((cause: unknown) => {
    actionError = { cause };
  });
  void checkHealth();
}

function hidden() {
  visible = false;
  healthRequest += 1;
  health = 'unchecked';
}

async function runAction(kind: NonNullable<typeof action>, perform: () => Promise<unknown>) {
  if (!desktop || busy) return;
  action = kind;
  actionError = null;
  notice = false;
  try {
    await perform();
  } catch (cause) {
    actionError = { cause };
  } finally {
    action = null;
  }
}

function switchGroup(group: Group) {
  if (!group.isEnabled || group.id === config.selectedGroupId) return;
  return runAction('switch', async () => {
    await config.switchGroup(group.id);
    if (group.type !== 'native') notice = true;
  });
}

function reload() {
  return runAction('reload', async () => {
    healthRequest += 1;
    health = 'unchecked';
    await config.reloadOpencode();
    if (visible) await checkHealth();
  });
}

async function dismiss(event: KeyboardEvent) {
  if (event.key !== 'Escape' || !desktop) return;
  event.preventDefault();
  try {
    await invoke('hide_tray_window');
    hidden();
  } catch (cause) {
    actionError = { cause };
  }
}

onMount(() => {
  desktop = isTauri();
  if (!desktop) {
    health = 'unavailable';
    return;
  }
  const unlisteners: UnlistenFn[] = [];
  const target = { target: { kind: 'WebviewWindow' as const, label: 'tray' } };
  const track = async (pending: Promise<UnlistenFn>) => {
    const unlisten = await pending;
    if (disposed) unlisten();
    else unlisteners.push(unlisten);
  };
  void Promise.all([
    track(
      listen<unknown>(
        'tauri://focus',
        ({ payload }) => {
          if (payload === false) hidden();
          else shown();
        },
        target,
      ),
    ),
    track(listen('tauri://blur', hidden, target)),
    track(listen('tray:shown', shown, target)),
  ])
    .then(() => {
      if (!disposed && document.hasFocus()) shown();
    })
    .catch((cause: unknown) => {
      if (!disposed) actionError = { cause, updates: true };
    });
  return () => {
    disposed = true;
    healthRequest += 1;
    for (const unlisten of unlisteners) unlisten();
  };
});
</script>

<svelte:head><title>{i18n.t('tray.metaTitle')}</title></svelte:head>
<svelte:window onkeydown={dismiss} />

<div
  class="tray-popup flex h-dvh w-full flex-col overflow-hidden border border-[var(--border-subtle)] bg-[var(--surface-app)] text-[var(--text-primary)]"
>
  <header
    class="flex h-14 shrink-0 items-center justify-between gap-2 border-b border-[var(--border-subtle)] bg-[var(--surface-panel)]/40 px-3.5"
  >
    <div class="flex min-w-0 items-center gap-2">
      <span
        aria-hidden="true"
        class="size-2 shrink-0 rounded-full"
        class:healthy={health === 'online'}
        class:unhealthy={health === 'unavailable'}
      ></span>
      <h1 class="m-0 truncate text-xs font-bold tracking-tight">{i18n.t('tray.brand')}</h1>
    </div>
    <div class="flex shrink-0 items-center gap-1.5">
      <Button
        variant="outline"
        size="sm"
        class="h-7 gap-1 rounded-lg border-[var(--border-subtle)] bg-[var(--surface-panel)]/80 px-2 text-[var(--text-secondary)] hover:border-[var(--accent-primary)]/40 hover:bg-[var(--surface-hover)] hover:text-[var(--accent-primary)]"
        disabled={!desktop || busy}
        onclick={reload}
        title={i18n.t('tray.reloadTitle')}
      >
        <RefreshCw size={14} aria-hidden="true" class={action === 'reload' ? 'motion-safe:animate-spin' : ''} />
        <span>{i18n.t('tray.reload')}</span>
      </Button>
      <Button
        size="sm"
        class="h-7 gap-1.5 rounded-lg px-2.5 shadow-[var(--focus-glow)]"
        disabled={!desktop || busy}
        onclick={() => runAction('open', () => invoke('open_main_window', { section: null }))}
        title={i18n.t('tray.openTitle')}
      >
        <ExternalLink size={14} aria-hidden="true" /><span>{i18n.t('tray.open')}</span>
      </Button>
    </div>
  </header>

  <main class="flex min-h-0 flex-1 flex-col gap-2 p-3">
    <div class="flex shrink-0 items-center justify-between gap-2 px-1">
      <h2
        id="tray-groups"
        class="m-0 text-[10px] font-normal uppercase leading-4 tracking-wider text-[var(--text-secondary)]"
      >
        {i18n.t('tray.runtimeGroup')}
      </h2>
      <span
        class="runtime-status flex items-center gap-1 text-[9px] leading-4"
        class:healthy={health === 'online'}
        class:unhealthy={health === 'unavailable'}
        role="status"
        title={healthDetail}
        aria-label={i18n.t('tray.runtimeStatus', { status: healthLabel, detail: healthDetail })}
      >
        <span aria-hidden="true" class="size-1.5 rounded-full bg-current"></span>
        {healthLabel}
      </span>
    </div>

    <div
      class="group-scroll min-h-0 max-h-[220px] flex-1 overflow-y-auto pr-1"
      aria-busy={config.loading || config.switching}
    >
      {#if config.loading && config.groups.length === 0}
        <p class="m-0 flex h-full items-center justify-center text-[11px] text-[var(--text-secondary)]" role="status">
          {i18n.t('tray.loadingGroups')}
        </p>
      {:else if config.groups.length === 0}
        <div class="flex h-full flex-col items-center justify-center gap-3 px-4 text-center">
          <p class="m-0 text-xs text-[var(--text-secondary)]">
            {i18n.t(error ? 'tray.groupsFailed' : 'tray.noGroups')}
          </p>
          <Button
            variant="outline"
            size="sm"
            class="rounded-lg"
            disabled={!desktop || busy}
            onclick={() => runAction('open', () => invoke('open_main_window', { section: null }))}
            >{i18n.t('tray.manageGroups')}</Button
          >
        </div>
      {:else}
        <ul class="m-0 flex list-none flex-col gap-1.5 p-0" aria-labelledby="tray-groups">
          {#each config.groups as group (group.id)}
            {@const active = group.id === config.selectedGroupId}
            <li>
              <button
                type="button"
                class="group-row flex h-[38px] w-full items-center justify-between gap-2 rounded-lg border border-[var(--border-subtle)] bg-[var(--surface-panel)]/40 p-2.5 text-left transition-colors duration-150 enabled:hover:border-[var(--border-default)] enabled:hover:bg-[var(--surface-hover)] disabled:cursor-default"
                class:active
                class:group-disabled={!group.isEnabled}
                disabled={!desktop || busy || !group.isEnabled}
                aria-label={i18n.t(group.isEnabled ? 'tray.groupLabel' : 'tray.disabledGroupLabel', {
                  name: group.name,
                  type: typeLabels[group.type],
                })}
                aria-pressed={active}
                title={!group.isEnabled ? i18n.t('tray.disabledGroupTitle', { name: group.name }) : group.name}
                onclick={() => switchGroup(group)}
              >
                <span class="flex min-w-0 flex-1 items-center gap-2"
                  ><span
                    aria-hidden="true"
                    class="group-pip size-2 shrink-0 rounded-full bg-[var(--border-default)]"
                  ></span><span class="truncate text-xs leading-4" class:font-semibold={active}
                    >{group.name}</span
                  ></span
                >
                {#if active}
                  <span
                    class="shrink-0 rounded border border-[var(--accent-primary)]/40 bg-[var(--accent-primary)]/20 px-1.5 py-0.5 text-[9px] font-semibold leading-3 text-[var(--accent-primary)]"
                    >{i18n.t('tray.active')}</span
                  >
                {:else}
                  <span class="shrink-0 text-[10px] leading-4 text-[var(--text-secondary)]"
                    >{group.isEnabled ? typeLabels[group.type] : i18n.t('groups.disabled')}</span
                  >
                {/if}
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
    {#if error}
      <p
        class="m-0 max-h-10 shrink-0 overflow-y-auto px-1 text-[10px] leading-[14px] text-[var(--status-error)]"
        role="alert"
      >
        {error}
      </p>
    {:else if action === 'switch' || config.switching}
      <p class="m-0 shrink-0 px-1 text-[10px] leading-[14px] text-[var(--text-secondary)]" role="status">
        {i18n.t('tray.switching')}
      </p>
    {:else if action === 'reload' || config.reloading}
      <p class="m-0 shrink-0 px-1 text-[10px] leading-[14px] text-[var(--text-secondary)]" role="status">
        {i18n.t('tray.reloading')}
      </p>
    {:else if notice}
      <p class="m-0 shrink-0 px-1 text-[10px] leading-[14px] text-[var(--text-secondary)]" role="status">
        {i18n.t('tray.groupSwitched')}
      </p>
    {/if}
  </main>

  <footer
    class="flex h-11 shrink-0 items-center justify-between border-t border-[var(--border-subtle)] bg-[var(--surface-panel)]/80 px-2.5"
  >
    <Button
      variant="ghost"
      size="sm"
      class="gap-1.5 rounded px-2 text-[var(--text-secondary)] hover:bg-[var(--surface-hover)] hover:text-[var(--text-primary)]"
      disabled={!desktop || busy}
      onclick={() => runAction('settings', () => invoke('open_main_window', { section: 'settings' }))}
      ><Settings size={14} aria-hidden="true" /><span>{i18n.t('nav.settings')}</span></Button
    >
    <Button
      variant="ghost"
      size="sm"
      class="gap-1.5 rounded px-2 text-[var(--status-error)] hover:bg-[var(--status-error)]/10 hover:text-[var(--status-error)]"
      disabled={!desktop || busy}
      onclick={() => runAction('quit', () => invoke('quit_app'))}
      ><Power size={14} aria-hidden="true" /><span>{i18n.t('tray.quit')}</span></Button
    >
  </footer>
</div>

<style>
@font-face {
  font-family: "JetBrains Mono";
  src: url("/fonts/jetbrains-mono-semibold.woff2") format("woff2");
  font-weight: 600 700;
  font-display: swap;
}
:global(html.tray-window),
:global(html.tray-window body) {
  height: 100%;
  min-height: 0;
  overflow: hidden;
  background: var(--surface-app);
  color: var(--text-primary);
}
.tray-popup {
  font-family: var(--font-primary);
}
header > div > span,
.runtime-status {
  color: var(--text-secondary);
}
header > div > span {
  background: var(--text-muted);
}
header > div > span.healthy {
  background: var(--accent-primary);
  box-shadow: var(--focus-glow-strong);
}
.runtime-status.healthy {
  color: var(--status-success);
}
.runtime-status.unhealthy {
  color: var(--status-warning);
}
.group-row {
  color: var(--text-secondary);
}
.group-row.active {
  height: 40px;
  border-color: var(--accent-primary);
  background: var(--surface-active);
  color: var(--text-primary);
  box-shadow: inset 0 0 12px color-mix(in srgb, var(--accent-primary) 6%, transparent);
}
.group-row.active .group-pip {
  background: var(--accent-primary);
  box-shadow: var(--focus-glow-strong);
}
.group-row.group-disabled {
  opacity: var(--disabled-opacity);
}
.group-row:focus-visible {
  outline-offset: -3px;
}
.group-scroll {
  scrollbar-width: thin;
  scrollbar-color: var(--scrollbar-thumb) var(--scrollbar-track-raised);
}
.group-scroll::-webkit-scrollbar {
  width: 4px;
}
.group-scroll::-webkit-scrollbar-track {
  background: var(--scrollbar-track-raised);
}
.group-scroll::-webkit-scrollbar-thumb {
  background: var(--scrollbar-thumb);
  border-radius: 4px;
}
.group-scroll::-webkit-scrollbar-thumb:hover {
  background: var(--scrollbar-thumb-hover);
}
</style>
