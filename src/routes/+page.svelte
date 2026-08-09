<script lang="ts">
  import { page } from '$app/state';

  import { createQuickSwitchStore } from '$lib/appStore';
  import QuickSwitch from '$lib/components/QuickSwitch.svelte';
  import { quickSwitchFixtureClient } from '$lib/quickSwitchFixture';

  const useQuickSwitchFixture = $derived(page.url.searchParams.get('fixture') === 'quick-switch');
  const fixtureStore = $derived(useQuickSwitchFixture
    ? createQuickSwitchStore(quickSwitchFixtureClient)
    : null);
</script>

<svelte:head>
  <title>Quick Switch - omo-switch</title>
  <link rel="icon" href="/favicon.svg" type="image/svg+xml" />
  <meta
    name="description"
    content="Quick switch surface for omo-switch model groups."
  />
</svelte:head>

<div class="route-shell" data-fixture-root={useQuickSwitchFixture ? 'quick-switch' : undefined}>
  {#if fixtureStore}
    <QuickSwitch store={fixtureStore} />
  {:else}
    <QuickSwitch />
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
