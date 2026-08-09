<script lang="ts">
  import { onMount } from 'svelte';
  import AlertCircle from '@lucide/svelte/icons/alert-circle';
  import CheckCircle2 from '@lucide/svelte/icons/check-circle-2';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
  import * as Alert from '$lib/components/ui/alert';
  import { Button } from '$lib/components/ui/button';
  import { Spinner } from '$lib/components/ui/spinner';

  import { settingsStore, type SettingsStore } from '../../settingsStore';
  import GroupSettingsPane from './groups/GroupSettingsPane.svelte';

  type Props = Readonly<{
    store?: SettingsStore;
  }>;

  const { store = settingsStore }: Props = $props();

  onMount(() => {
    void store.load();
  });
</script>

<main class="settings-shell" data-surface="settings" aria-label="Model group settings">
  <section class="settings-detail" aria-live="polite">
    {#if $store.status === 'loading' && !$store.appState}
      <div class="state-block" aria-busy="true">
        <Spinner size="lg" />
        <p>Loading settings...</p>
      </div>
    {:else if $store.status === 'error' && !$store.appState}
      <div class="state-block initialization-error" role="alert">
        <AlertCircle class="initialization-error-icon" size={32} aria-hidden="true" />
        <div class="initialization-error-copy">
          <h2>Unable to load settings</h2>
          <p>{$store.message?.text ?? 'Settings could not be loaded.'}</p>
        </div>
        <Button type="button" variant="secondary" onclick={() => void store.load()}>
          <RotateCcw size={16} aria-hidden="true" />
          Retry
        </Button>
      </div>
    {:else}
      {#if $store.message}
        <Alert.Root
          class="status-message"
          variant={$store.message.tone === 'success' ? 'success' : $store.message.tone === 'warning' ? 'warning' : $store.message.tone === 'error' ? 'destructive' : 'default'}
          role={$store.message.tone === 'error' ? 'alert' : 'status'}
        >
          {#if $store.message.tone === 'success'}
            <CheckCircle2 size={16} aria-hidden="true" />
          {:else}
            <AlertCircle size={16} aria-hidden="true" />
          {/if}
          <span>{$store.message.text}</span>
        </Alert.Root>
      {/if}

      <GroupSettingsPane store={store} state={$store} />
    {/if}
  </section>
</main>

<style>
  .settings-shell {
    display: flex;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 100dvh;
    overflow: hidden;
    background: var(--surface-panel);
    color: var(--text-primary);
    font-family: var(--font-primary);
    font-size: var(--font-body-size);
  }

  .settings-detail {
    display: flex;
    flex: 1;
    min-width: 0;
    min-height: 0;
    flex-direction: column;
    overflow: hidden;
    background: var(--surface-panel);
  }

  :global(.status-message) {
    align-items: flex-start;
    border-left: 0;
    border-right: 0;
    border-top: 0;
    border-radius: 0;
    margin: 0;
    padding: var(--space-2) var(--space-5);
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

  .initialization-error {
    max-width: 400px;
    color: var(--text-primary);
  }

  :global(.initialization-error-icon) { color: var(--status-error); }

  .initialization-error-copy {
    display: grid;
    gap: var(--space-1);
  }

  .initialization-error h2,
  .initialization-error p { margin: 0; }

  .initialization-error h2 {
    font-size: var(--font-section-size);
    font-weight: var(--font-section-weight);
    line-height: var(--font-section-line);
  }

  .initialization-error p { color: var(--text-secondary); }

</style>
