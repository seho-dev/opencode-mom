<script lang="ts">
import { onMount, tick, untrack } from 'svelte';
import { Button } from '$src/components/button/index.js';
import { Switch } from '$src/components/switch/index.js';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import type { LoggingState } from '$src/types/logging.js';

let { active = true }: { active?: boolean } = $props();
const config = getConfig();
const i18n = getI18n();
let state = $state.raw<LoggingState | null>(null);
let pending = $state<'read' | 'save' | null>(null);
let error = $state<'read' | 'save' | null>(null);
let mounted = $state(false);
let section = $state<HTMLDivElement | null>(null);
let generation = 0;
let reloadQueued = false;
const needsAttention = $derived(!!state && (!!state.error || (state.enabled && !state.recording)));

export async function refresh() {
  if (!mounted || !active || pending) return;

  const version = generation;

  reloadQueued = false;
  pending = 'read';
  error = null;

  try {
    const result = await config.getLoggingState();

    if (mounted && active && version === generation) state = result;
  } catch {
    if (mounted && active && version === generation) error = 'read';
  } finally {
    if (mounted) {
      pending = null;
      if (active && reloadQueued) void refresh();
    }
  }
}

async function setEnabled(next: boolean, control: HTMLElement) {
  if (!mounted || !active || !state || pending) return;

  const version = generation;
  const restoreFocus = document.activeElement === control;

  pending = 'save';
  error = null;

  try {
    const result = await config.setLoggingEnabled(next);

    if (mounted && active && version === generation) {
      state = result;
      if (result.enabled !== next) error = 'save';
    }
  } catch {
    if (mounted && active && version === generation) error = 'save';
  } finally {
    if (mounted) {
      pending = null;
      if (active && reloadQueued) void refresh();
      if (restoreFocus) {
        await tick();
        if (mounted && active && version === generation && document.activeElement === document.body) {
          (control.isConnected ? control : section)?.focus();
        }
      }
    }
  }
}

onMount(() => {
  mounted = true;
  return () => {
    mounted = false;
    generation += 1;
    reloadQueued = false;
  };
});

$effect(() => {
  if (!mounted) return;

  const entered = active;

  untrack(() => {
    generation += 1;
    reloadQueued = entered;
    if (entered) void refresh();
  });
});
</script>

<div
  bind:this={section}
  id="settings-logging"
  tabindex="-1"
  class="scroll-mt-24 px-4 py-3 sm:px-5 sm:py-4"
  aria-busy={pending !== null}
>
  <div class="flex items-start justify-between gap-6">
    <div class="min-w-0">
      <label for="diagnostic-logging" class="font-[var(--font-heading)] text-[13px] leading-5 font-semibold"
        >{i18n.t('settings.logging')}</label
      >
      <p id="logging-description" class="field-hint mt-1">{i18n.t('settings.loggingHint')}</p>
      <p id="logging-privacy" class="field-hint mt-1">{i18n.t('settings.loggingPrivacy')}</p>
    </div>
    <Switch
      id="diagnostic-logging"
      name="diagnostic-logging"
      checked={state?.enabled ?? false}
      disabled={!active || !state || pending !== null}
      aria-describedby="logging-description logging-privacy logging-state"
      aria-busy={pending !== null}
      class="mt-0.5"
      onclick={(event) => { event.preventDefault(); if (state) void setEnabled(!state.enabled, event.currentTarget); }}
    />
  </div>
  <div class="mt-2 flex flex-wrap items-center justify-between gap-2">
    <p id="logging-state" class="m-0 text-xs leading-5 text-[var(--text-secondary)]" role="status">
      {#if pending}
        {i18n.t(pending === 'save' ? 'settings.saving' : 'settings.loading')}
      {:else}
        {i18n.t(error || !state ? 'common.unknownStatus' : state.recording ? 'logs.recording' : 'logs.recordingOff')}
        {#if !error && needsAttention}
          · {i18n.t('logs.recordingFailed')}
        {/if}
      {/if}
    </p>
    <Button variant="outline" size="sm" href="/logs">{i18n.t('settings.loggingView')}</Button>
  </div>
  {#if error || needsAttention}
    <div class="mt-2 flex flex-wrap items-center gap-2 text-xs leading-5 text-[var(--status-error)]" role="alert">
      <span class="min-w-0 break-words"
        >{i18n.t(error === 'save' ? 'settings.loggingSaveFailed' : error === 'read' ? 'settings.loggingLoadFailed' : 'logs.recordingFailedHint')}</span
      >
      <Button variant="outline" size="sm" disabled={!active || pending !== null} onclick={refresh}
        >{i18n.t('settings.loggingRefresh')}</Button
      >
      {#if state?.enabled && !state.recording}
        <Button
          variant="outline"
          size="sm"
          disabled={!active || pending !== null}
          aria-busy={pending === 'save'}
          onclick={(event) => { if (state?.enabled && !state.recording) void setEnabled(true, event.currentTarget); }}
          >{i18n.t('settings.loggingRetryRecording')}</Button
        >
      {/if}
    </div>
  {/if}
  {#if state && state.droppedRecords > 0}
    <p class="mt-2 mb-0 text-xs leading-5 text-[var(--status-warning)]">
      {i18n.t('logs.droppedRecords', { count: state.droppedRecords })}
    </p>
  {/if}
</div>
