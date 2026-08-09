<script lang="ts">
  import { onMount, tick } from 'svelte';
  import AlertCircle from '@lucide/svelte/icons/alert-circle';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';

  import { quickSwitchStore, type QuickSwitchStore } from '../appStore';
  import * as Alert from './ui/alert';
  import { Button } from './ui/button';
  import { Spinner } from './ui/spinner/index.js';
  import CurrentGroupSummary from './quick-switch/CurrentGroupSummary.svelte';
  import SwitchTargets from './quick-switch/SwitchTargets.svelte';

  type Props = Readonly<{
    store?: QuickSwitchStore;
  }>;

  const { store = quickSwitchStore }: Props = $props();
  let quickSwitchElement: HTMLElement;

  onMount(() => {
    const loadAndFocus = async (): Promise<void> => {
      await store.load();
      await tick();
      quickSwitchElement.querySelector<HTMLElement>('[aria-label="Current group summary"]')?.focus();
    };
    void loadAndFocus();
  });
</script>

<main bind:this={quickSwitchElement} class="quick-switch" data-surface="quick-switch" aria-labelledby="quick-switch-title">
  {#if $store.status === 'loading' && !$store.response}
    <section class="state-block" aria-live="polite" aria-busy="true">
      <Spinner size="lg" />
      <p>Loading groups...</p>
    </section>
  {:else if $store.status === 'error' && !$store.response}
    <section class="state-block initialization-error" role="alert">
      <AlertCircle class="initialization-error-icon" size={32} aria-hidden="true" />
      <div class="initialization-error-copy">
        <h2>Unable to load groups</h2>
        <p>{$store.error ?? 'Groups could not be loaded.'}</p>
      </div>
      <Button type="button" variant="secondary" onclick={() => void store.load()}>
        <RotateCcw size={16} aria-hidden="true" />
        Retry
      </Button>
    </section>
  {:else}
    <section class="feedback" aria-live="polite">
      {#if $store.error}
        <Alert.Root class="status-message" variant="destructive" role="alert">
          <AlertCircle size={16} aria-hidden="true" />
          <span>{$store.error}</span>
        </Alert.Root>
      {:else if $store.view.error}
        <Alert.Root class="status-message" variant="destructive" role="alert">
          <AlertCircle size={16} aria-hidden="true" />
          <span>{$store.view.error}</span>
        </Alert.Root>
      {/if}

      {#if $store.view.warning}
        <Alert.Root class="status-message" variant="warning" role="status">
          <AlertCircle size={16} aria-hidden="true" />
          <span>{$store.view.warning}</span>
        </Alert.Root>
      {/if}

      {#if $store.view.discoveryError}
        <Alert.Root class="status-message" variant="warning" role="status">
          <AlertCircle size={16} aria-hidden="true" />
          <span>OpenCode agent discovery warning: {$store.view.discoveryError}</span>
        </Alert.Root>
      {/if}
    </section>

    <CurrentGroupSummary group={$store.view.currentGroup} />
    <SwitchTargets
      targets={$store.view.switchTargets}
      hasEnabledGroups={$store.view.hasEnabledGroups}
      emptyText={$store.view.emptySwitchTargetText}
      switchingGroupId={$store.switchingGroupId}
      onSwitch={(id) => store.switchTo(id)}
    />
  {/if}
</main>

<style>
  .quick-switch {
    display: flex;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 100dvh;
    box-sizing: border-box;
    flex-direction: column;
    gap: var(--space-3);
    overflow-y: auto;
    background: var(--surface-panel);
    color: var(--text-primary);
    padding: var(--space-4);
  }

  .state-block {
    display: grid;
    min-height: 0;
    flex: 1;
    place-content: center;
    gap: var(--space-2);
    color: var(--text-secondary);
    text-align: center;
  }

  .state-block p {
    margin: 0;
    font-size: var(--font-small-size);
    line-height: var(--font-small-line);
  }

  .feedback {
    display: grid;
    gap: var(--space-2);
  }

  :global(.status-message) {
    align-items: flex-start;
  }

  .initialization-error {
    max-width: 400px;
    color: var(--text-primary);
  }

  :global(.initialization-error-icon) {
    color: var(--status-error);
  }

  .initialization-error-copy {
    display: grid;
    gap: var(--space-1);
  }

  .initialization-error h2,
  .initialization-error p {
    margin: 0;
  }

  .initialization-error h2 {
    font-size: var(--font-section-size);
    font-weight: var(--font-section-weight);
    line-height: var(--font-section-line);
  }

  .initialization-error p {
    color: var(--text-secondary);
  }

</style>
