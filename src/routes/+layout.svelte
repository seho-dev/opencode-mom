<script lang="ts">
import '../app.css';
import { isTauri } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { onMount, type Snippet } from 'svelte';
import { goto } from '$app/navigation';
import { page } from '$app/state';
import { createCommandAdapter } from '$src/config/adapter.js';
import { getConfig, setConfig } from '$src/config/context.js';
import { createConfigStore } from '$src/config/store.svelte.js';
import { setI18n } from '$src/i18n/context.js';
import { createI18n } from '$src/i18n/i18n.svelte.js';
import AppShell from '$src/shell/AppShell.svelte';
import SplashScreen from '$src/shell/SplashScreen.svelte';

let { children }: { children: Snippet } = $props();
const isTray = $derived(page.url.pathname === '/tray');
const isIsland = $derived(page.url.pathname === '/island');
setConfig(
  createConfigStore(
    createCommandAdapter(),
    page.url.pathname !== '/tray' && page.url.pathname !== '/island',
    page.url.pathname !== '/island',
  ),
);
const config = getConfig();
setI18n(createI18n(() => config.preferences.locale));

$effect(() => {
  document.documentElement.classList.toggle('tray-window', isTray);
  document.documentElement.classList.toggle('island-window', isIsland);
  return () => document.documentElement.classList.remove('tray-window', 'island-window');
});

onMount(() => {
  if (!isTauri() || isTray || isIsland) return;
  let disposed = false;
  const unlisteners: UnlistenFn[] = [];
  const track = (pending: Promise<UnlistenFn>) => {
    void pending
      .then((unlisten) => {
        if (disposed) unlisten();
        else unlisteners.push(unlisten);
      })
      .catch((error) => console.error('Could not listen for tray updates:', error));
  };
  const target = { target: 'main' };
  track(
    listen<unknown>(
      'tray:navigate',
      ({ payload }) => {
        if (!disposed && payload === '/settings') void goto('/settings').catch(console.error);
      },
      target,
    ),
  );
  track(
    listen(
      'tray:config-changed',
      () => {
        if (!disposed) void config.reloadKeepingDraft().catch(console.error);
      },
      target,
    ),
  );
  return () => {
    disposed = true;
    for (const unlisten of unlisteners) unlisten();
  };
});

onMount(() => {
  if (!isTauri() || !isTray) return;
  let disposed = false;
  let unlisten: UnlistenFn | undefined;
  const reload = () => {
    if (!disposed) void config.reloadKeepingDraft().catch(console.error);
  };
  void listen('app:preferences-changed', reload, { target: 'tray' })
    .then((stop) => {
      if (disposed) stop();
      else {
        unlisten = stop;
        reload();
      }
    })
    .catch((error) => console.error('Could not listen for preference updates:', error));
  return () => {
    disposed = true;
    unlisten?.();
  };
});
</script>

{#if isTray || isIsland}
  {@render children()}
{:else}
  <AppShell>{@render children()}</AppShell>
  <SplashScreen visible={config.splashLoading} />
{/if}

<style>
:global(html.island-window),
:global(html.island-window body) {
  background: transparent;
  overflow: hidden;
}
</style>
