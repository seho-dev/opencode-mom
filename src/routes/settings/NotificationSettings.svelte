<script lang="ts">
import { isTauri } from '@tauri-apps/api/core';
import { onMount, tick, untrack } from 'svelte';
import { Button } from '$src/components/button/index.js';
import { Switch } from '$src/components/switch/index.js';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import type { NotificationSettings } from '$src/types/notifications.js';

let { active = true }: { active?: boolean } = $props();
const config = getConfig();
const i18n = getI18n();
const categories = [
  { key: 'agentCompleted', label: 'settings.notificationsAgentCompleted', hint: 'settings.notificationsMainOnly' },
  { key: 'approvalRequired', label: 'settings.notificationsApprovalRequired', hint: 'settings.notificationsAllAgents' },
  { key: 'agentError', label: 'settings.notificationsAgentError', hint: 'settings.notificationsAllAgents' },
] as const;
let state = $state.raw<NotificationSettings | null>(null);
let known = $state(false);
let supported = $state<boolean | null>(null);
let windows = $state(false);
let permission = $state<Awaited<ReturnType<typeof config.getNotificationPermission>> | null>(null);
let pending = $state<'read' | 'save' | 'permission' | 'settings' | null>(null);
let error = $state<'read' | 'save' | 'permission' | null>(null);
let mounted = $state(false);
let section = $state<HTMLDivElement | null>(null);
let generation = 0;
let reloadQueued = false;
const blocked = $derived(!active || supported !== true || !known || !state || pending !== null);

export async function refresh() {
  if (!mounted || !active || pending) return;
  const version = generation;
  reloadQueued = false;
  pending = 'read';
  known = false;
  permission = null;
  error = null;
  try {
    if (supported === null) {
      if (!isTauri()) {
        supported = false;
        return;
      }
      const info = await config.getAppInfo();
      if (!mounted || !active || version !== generation) return;
      supported = info.platform === 'macos' || info.platform === 'windows';
      windows = info.platform === 'windows';
    }
    if (!supported) return;
    const result = await config.getNotificationSettings();
    if (mounted && active && version === generation) {
      state = result;
      known = true;
    }
    if (!mounted || !active || version !== generation) return;
    try {
      const result = await config.getNotificationPermission();
      if (mounted && active && version === generation) permission = result;
    } catch {
      if (mounted && active && version === generation) error = 'permission';
    }
  } catch {
    if (mounted && active && version === generation) error = 'read';
  } finally {
    if (mounted) {
      pending = null;
      if (active && reloadQueued) void refresh();
    }
  }
}

async function toggle(key: keyof NotificationSettings, control: HTMLInputElement) {
  if (!mounted || blocked || !state || (key !== 'enabled' && !state.enabled)) return;
  const version = generation;
  const next = { ...state, [key]: !state[key] };
  const restoreFocus = document.activeElement === control;
  pending = 'save';
  error = null;
  try {
    // Let the canceled native click restore checked before committing confirmed state.
    await tick();
    if (!mounted || !active || version !== generation) return;
    const result = await config.setNotificationSettings(next);
    if (mounted && active && version === generation) {
      state = result;
      if (result[key] !== next[key]) error = 'save';
      if (key === 'enabled' && next.enabled && result.enabled) {
        pending = 'permission';
        permission = null;
        try {
          const result = await config.requestNotificationPermission();
          if (mounted && active && version === generation) permission = result;
        } catch {
          if (mounted && active && version === generation) error = 'permission';
        }
      }
    }
  } catch {
    if (!mounted || !active || version !== generation) return;
    error = 'save';
    known = false;
    pending = 'read';
    try {
      const result = await config.getNotificationSettings();
      if (mounted && active && version === generation) {
        state = result;
        known = true;
      }
    } catch {
      if (mounted && active && version === generation) error = 'save';
    }
  } finally {
    if (mounted) {
      pending = null;
      if (active && reloadQueued) void refresh();
      if (restoreFocus) {
        await tick();
        if (mounted && active && version === generation && document.activeElement === document.body) {
          (control.isConnected && !control.disabled ? control : section)?.focus();
        }
      }
    }
  }
}

async function permissionAction(action: 'request' | 'settings', control: HTMLButtonElement) {
  if (!mounted || blocked || !state?.enabled) return;
  const version = generation;
  const restoreFocus = document.activeElement === control;
  pending = action === 'request' ? 'permission' : 'settings';
  error = null;
  try {
    if (action === 'settings') {
      await config.openNotificationSettings();
    } else {
      permission = null;
      const result = await config.requestNotificationPermission();
      if (mounted && active && version === generation) permission = result;
    }
  } catch {
    if (mounted && active && version === generation) error = 'permission';
  } finally {
    if (mounted) {
      pending = null;
      if (active && reloadQueued) void refresh();
      if (restoreFocus) {
        await tick();
        if (mounted && active && version === generation && document.activeElement === document.body) {
          (control.isConnected && !control.disabled ? control : section)?.focus();
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
    known = false;
    permission = null;
    reloadQueued = entered;
    if (entered) void refresh();
  });
});
</script>

<div
  bind:this={section}
  id="settings-notifications"
  tabindex="-1"
  class="scroll-mt-24 px-4 py-3 sm:px-5 sm:py-4"
  aria-busy={pending !== null}
>
  <div class="flex items-start justify-between gap-6">
    <div class="min-w-0">
      <label
        id="notifications-label"
        for="system-notifications"
        class="font-[var(--font-heading)] text-[13px] leading-5 font-semibold"
        >{i18n.t('settings.notifications')}</label
      >
      <p id="notifications-description" class="field-hint mt-1">{i18n.t('settings.notificationsHint')}</p>
    </div>
    <Switch
      id="system-notifications"
      name="system-notifications"
      checked={state?.enabled ?? false}
      disabled={blocked}
      aria-describedby="notifications-description notifications-os notifications-privacy notifications-state"
      aria-busy={pending !== null}
      class="mt-0.5"
      onclick={(event) => { event.preventDefault(); void toggle('enabled', event.currentTarget); }}
    />
  </div>
  <fieldset class="mt-3 min-w-0 space-y-3 border-0 p-0" aria-labelledby="notifications-label">
    {#each categories as category (category.key)}
      <div class="flex items-start justify-between gap-6">
        <div class="min-w-0">
          <label
            for={`notification-${category.key}`}
            class={`text-xs leading-5 font-medium ${state?.enabled ? 'text-[var(--text-primary)]' : 'text-[var(--text-muted)]'}`}
            >{i18n.t(category.label)}</label
          >
          <p id={`notification-${category.key}-hint`} class="field-hint mt-0.5">{i18n.t(category.hint)}</p>
        </div>
        <Switch
          id={`notification-${category.key}`}
          name={`notification-${category.key}`}
          checked={state?.[category.key] ?? false}
          disabled={blocked || !state?.enabled}
          aria-describedby={`notification-${category.key}-hint notifications-state`}
          aria-busy={pending !== null}
          class="mt-0.5"
          onclick={(event) => { event.preventDefault(); void toggle(category.key, event.currentTarget); }}
        />
      </div>
    {/each}
  </fieldset>
  <p id="notifications-os" class="field-hint mt-3">
    {i18n.t(windows ? 'settings.notificationsWindowsHint' : 'settings.notificationsOsHint')}
  </p>
  <p id="notifications-privacy" class="field-hint mt-1">{i18n.t('settings.notificationsPrivacy')}</p>
  <p id="notifications-state" class="mt-1 mb-0 text-xs leading-5 text-[var(--text-secondary)]" role="status">
    {#if pending}
      {i18n.t(pending === 'save' ? 'settings.saving' : 'settings.loading')}
    {:else if supported === false}
      {i18n.t('settings.notificationsNativeOnly')}
    {:else if !known || !state}
      {i18n.t('common.unknownStatus')}
    {:else if !state.enabled}
      {i18n.t('settings.notificationsDisabledHint')}
    {:else if permission === 'granted'}
      {i18n.t('settings.notificationsPermissionGranted')}
    {:else if permission === 'denied'}
      {i18n.t('settings.notificationsPermissionDenied')}
    {:else if permission === 'prompt'}
      {i18n.t(windows ? 'settings.notificationsPermissionPromptWindows' : 'settings.notificationsPermissionPrompt')}
    {:else if permission === 'unavailable'}
      {i18n.t('settings.notificationsPermissionUnavailable')}
    {:else if permission === 'unsupported'}
      {i18n.t('settings.notificationsNativeOnly')}
    {:else}
      {i18n.t('settings.notificationsPermissionUnknown')}
    {/if}
  </p>
  {#if known && state?.enabled && supported}
    <div class="mt-2 flex flex-wrap items-center gap-2">
      {#if permission === 'prompt' || permission === null}
        <Button
          variant="outline"
          size="sm"
          disabled={blocked}
          onclick={(event) => permissionAction('request', event.currentTarget)}
          >{i18n.t(windows ? 'settings.notificationsOpenSettings' : 'settings.notificationsRequestPermission')}</Button
        >
      {/if}
      {#if permission !== 'unsupported' && permission !== 'unavailable' && (!windows || (permission !== 'prompt' && permission !== null))}
        <Button
          variant="outline"
          size="sm"
          disabled={blocked}
          onclick={(event) => permissionAction('settings', event.currentTarget)}
          >{i18n.t('settings.notificationsOpenSettings')}</Button
        >
      {/if}
      {#if !error}
        <Button variant="outline" size="sm" disabled={blocked} onclick={refresh}
          >{i18n.t('settings.notificationsRefresh')}</Button
        >
      {/if}
    </div>
  {/if}
  {#if error}
    <div class="mt-2 flex flex-wrap items-center gap-2 text-xs leading-5 text-[var(--status-error)]" role="alert">
      <span class="min-w-0 break-words [overflow-wrap:anywhere]"
        >{i18n.t(error === 'save' ? 'settings.notificationsSaveFailed' : error === 'permission' ? 'settings.notificationsPermissionFailed' : 'settings.notificationsLoadFailed')}</span
      >
      <Button
        variant="outline"
        size="sm"
        disabled={!active || pending !== null}
        aria-busy={pending === 'read'}
        onclick={refresh}
        >{i18n.t('settings.notificationsRefresh')}</Button
      >
    </div>
  {/if}
</div>
