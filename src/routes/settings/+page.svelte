<script lang="ts">
  import { page } from '$app/state';

  import SettingsShell from '$lib/components/settings/SettingsShell.svelte';
  import { createSettingsStore } from '$lib/settingsStore';
  import { degradedSettingsFixtureClient, settingsFixtureClient } from '$lib/settingsFixture';

  const useSettingsFixture = $derived(page.url.searchParams.get('fixture') === 'settings');
  const useDegradedSettingsFixture = $derived(useSettingsFixture && page.url.searchParams.get('degraded') === 'opencode');
  const fixtureStore = $derived(useSettingsFixture
    ? createSettingsStore(useDegradedSettingsFixture ? degradedSettingsFixtureClient : settingsFixtureClient)
    : null);
</script>

<svelte:head>
  <title>omo-switch</title>
  <link rel="icon" href="/favicon.svg" type="image/svg+xml" />
  <meta
    name="description"
    content="Settings surface for omo-switch model groups."
  />
</svelte:head>

<div class="route-shell" data-fixture-root={useSettingsFixture ? 'settings' : undefined}>
  {#if fixtureStore}
    <SettingsShell store={fixtureStore} />
  {:else}
    <SettingsShell />
  {/if}
</div>

<style>
  .route-shell {
    display: flex;
    width: 100%;
    height: 100%;
    min-height: 100dvh;
    min-width: 0;
    overflow: hidden;
  }
</style>
