<script lang="ts">
import { isTauri } from '@tauri-apps/api/core';
import { onMount } from 'svelte';
import { getConfig } from '$src/config/context.js';
import AppHeader from './AppHeader.svelte';
import AppSidebar from './AppSidebar.svelte';
import ConfigFeedback from './ConfigFeedback.svelte';
import Toaster from './Toaster.svelte';

let { children } = $props();
const config = getConfig();
let macos = $state(false);

onMount(() => {
  if (!isTauri()) return;
  let mounted = true;
  let updateTimer = setTimeout(() => void checkUpdates(), 5_000);

  async function checkUpdates() {
    try {
      await config.checkForUpdates();
    } catch {
      // Background checks stay silent; manual checks report their own errors.
    } finally {
      if (mounted) updateTimer = setTimeout(() => void checkUpdates(), 2 * 60 * 60 * 1_000);
    }
  }

  void config
    .getAppInfo()
    .then((info) => {
      if (mounted) macos = info.platform === 'macos';
    })
    .catch(() => {});
  return () => {
    mounted = false;
    clearTimeout(updateTimer);
  };
});
</script>

<div class="app-shell" class:macos>
  {#if macos}
    <div class="window-drag-region" data-tauri-drag-region aria-hidden="true"></div>
  {/if}
  <AppSidebar />
  <div class="workspace">
    <AppHeader {macos} />
    <main class="content">{@render children()}</main>
  </div>
  <ConfigFeedback />
  <Toaster />
</div>
