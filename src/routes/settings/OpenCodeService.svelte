<script lang="ts">
import { RefreshCw } from '@lucide/svelte';
import { onMount, untrack } from 'svelte';
import { Button } from '$src/components/button/index.js';
import { Input } from '$src/components/input/index.js';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import type { OpenCodeServiceStatus, OpenCodeServiceTarget } from '$src/types/app.js';

let {
  ready,
  blocked = false,
  busy = $bindable(false),
}: { ready: boolean; blocked?: boolean; busy?: boolean } = $props();
const config = getConfig();
const i18n = getI18n();
let status = $state.raw<OpenCodeServiceStatus | null>(null);
let mode = $state<OpenCodeServiceTarget['mode']>('shared');
let url = $state('');
let password = $state('');
let clearPassword = $state(false);
let edited = false;
let initialized = false;
let pending = $state<'read' | 'save' | null>(null);
let error = $state<'read' | 'save' | 'busy' | 'validation' | null>(null);
let saved = $state(false);
let connectionUnknown = $state(false);
let mounted = false;
let request = 0;
const modes = ['shared', 'explicit'] as const;
const disabled = $derived(!ready || blocked || pending !== null);
const connected = $derived(!connectionUnknown && !status?.error && status?.connected === true);

function applyDraft(target: OpenCodeServiceTarget) {
  mode = target.mode;
  url = target.url ?? '';
}
export async function refresh() {
  if (!mounted || !ready || pending) return;
  const seq = ++request;
  pending = 'read';
  busy = true;
  error = null;
  saved = false;
  try {
    const result = await config.getOpenCodeServiceStatus();
    if (!mounted || seq !== request) return;
    status = result;
    connectionUnknown = false;
    if (!initialized && !edited) applyDraft(result.target);
    initialized = true;
  } catch {
    if (mounted && seq === request) {
      error = 'read';
      connectionUnknown = true;
    }
  } finally {
    if (mounted && seq === request) {
      pending = null;
      busy = false;
    }
  }
}
async function save(event: SubmitEvent) {
  event.preventDefault();
  if (!mounted || disabled || (mode === 'explicit' && !url.trim())) return;
  const seq = ++request;
  pending = 'save';
  busy = true;
  error = null;
  saved = false;
  try {
    const targetUrl = mode === 'shared' ? null : url.trim();
    const result =
      mode === 'explicit' && (password || clearPassword)
        ? await config.setOpenCodeServiceTarget(targetUrl, clearPassword ? '' : password)
        : await config.setOpenCodeServiceTarget(targetUrl);
    if (!mounted || seq !== request) return;
    status = result;
    connectionUnknown = false;
    applyDraft(result.target);
    password = '';
    clearPassword = false;
    edited = false;
    initialized = true;
    saved = true;
  } catch (cause) {
    if (mounted && seq === request) {
      const code = cause && typeof cause === 'object' && 'code' in cause ? cause.code : null;
      error = code === 'busy' ? 'busy' : code === 'validation_failed' ? 'validation' : 'save';
    }
  } finally {
    if (mounted && seq === request) {
      pending = null;
      busy = false;
    }
  }
}
onMount(() => {
  mounted = true;
  return () => {
    mounted = false;
    request += 1;
    password = '';
  };
});
$effect(() => {
  if (ready) untrack(() => void refresh());
});
</script>

<section
  class="mt-4 border-t border-[var(--border-subtle)] pt-4"
  aria-labelledby="opencode-service-title"
  aria-busy={pending !== null}
>
  <div class="flex items-start justify-between gap-3 sm:gap-6">
    <div class="min-w-0">
      <h3 id="opencode-service-title" class="m-0 text-xs leading-5 font-semibold">{i18n.t('settings.serviceTitle')}</h3>
      <p class="mt-1 mb-0 text-xs leading-5 text-[var(--text-muted)]">{i18n.t('settings.serviceReadHint')}</p>
    </div>
    <Button
      variant="outline"
      size="icon"
      {disabled}
      aria-label={i18n.t('settings.serviceRefresh')}
      title={i18n.t('settings.serviceRefresh')}
      aria-busy={pending === 'read'}
      onclick={() => { if (!disabled) void refresh(); }}
    >
      <RefreshCw
        size={14}
        class={pending === 'read' ? 'animate-spin motion-reduce:animate-none' : ''}
        aria-hidden="true"
      />
    </Button>
  </div>
  <div class="mt-3 border border-[var(--border-subtle)] bg-[var(--surface-input)] px-3 py-2.5">
    <div class="flex items-center gap-2 text-xs leading-5" role="status" aria-live="polite">
      <span
        class={['h-1.5 w-1.5 shrink-0 rounded-full', connected ? 'bg-[var(--status-success)]' : 'bg-[var(--text-muted)]']}
        aria-hidden="true"
      ></span>
      <span
        >{i18n.t(pending === 'read' ? 'settings.serviceReading' : connected ? 'settings.serviceConnected' : connectionUnknown || !status ? 'settings.serviceUnknown' : 'settings.serviceDisconnected')}</span
      >
    </div>
    <dl class="mt-2 mb-0 grid grid-cols-[auto_minmax(0,1fr)] gap-x-4 gap-y-1 text-xs leading-5">
      <dt class="text-[var(--text-muted)]">{i18n.t('settings.serviceConfirmedTarget')}</dt>
      <dd class="m-0 min-w-0 select-text break-words [overflow-wrap:anywhere]">
        {status ? status.target.mode === 'shared' ? i18n.t('settings.serviceShared') : status.target.url : i18n.t('settings.serviceUnknown')}
      </dd>
      <dt class="text-[var(--text-muted)]">{i18n.t('settings.serviceConnectedUrl')}</dt>
      <dd class="m-0 min-w-0 select-text break-words [overflow-wrap:anywhere]">
        {connected ? status?.url ?? i18n.t('settings.serviceUnknown') : '—'}
      </dd>
      <dt class="text-[var(--text-muted)]">{i18n.t('settings.servicePid')}</dt>
      <dd class="m-0 select-text">{connected ? status?.pid ?? i18n.t('settings.serviceUnknown') : '—'}</dd>
      <dt class="text-[var(--text-muted)]">{i18n.t('settings.serviceVersion')}</dt>
      <dd class="m-0 select-text break-words">
        {connected ? status?.version ?? i18n.t('settings.serviceUnknown') : '—'}
      </dd>
    </dl>
  </div>
  <form class="mt-4 flex flex-col gap-3" onsubmit={save} oninput={() => { edited = true; saved = false; }}>
    <fieldset class="m-0 min-w-0 border-0 p-0" {disabled}>
      <legend class="mb-2 text-xs leading-5 font-medium">{i18n.t('settings.serviceMode')}</legend>
      <div class="grid gap-2 sm:grid-cols-2">
        {#each modes as choice}
          <label
            class={['flex cursor-pointer items-start gap-2.5 rounded-[2px] border px-3 py-2.5 text-xs leading-5', mode === choice ? 'border-[var(--accent-primary)] bg-[var(--surface-active)]' : 'border-[var(--border-default)]', disabled && 'cursor-default opacity-[var(--disabled-opacity)]']}
          >
            <input
              type="radio"
              name="opencode-service-mode"
              value={choice}
              bind:group={mode}
              class="mt-1 shrink-0 accent-[var(--accent-primary)]"
            >
            <span class="min-w-0"
              ><span class="block font-medium"
                >{i18n.t(choice === 'shared' ? 'settings.serviceShared' : 'settings.serviceExplicit')}</span
              ><span class="block text-[var(--text-muted)]"
                >{i18n.t(choice === 'shared' ? 'settings.serviceSharedHint' : 'settings.serviceExplicitHint')}</span
              ></span
            >
          </label>
        {/each}
      </div>
    </fieldset>
    {#if mode === 'explicit'}
      <div>
        <label for="opencode-service-url" class="mb-1 block text-xs leading-5 font-medium"
          >{i18n.t('settings.serviceUrl')}</label
        >
        <Input
          id="opencode-service-url"
          type="url"
          bind:value={url}
          {disabled}
          required
          placeholder="http://127.0.0.1:4096"
          autocomplete="off"
          spellcheck={false}
          aria-describedby="opencode-service-url-hint"
        />
        <p id="opencode-service-url-hint" class="mt-1 mb-0 text-xs leading-5 text-[var(--text-muted)]">
          {i18n.t('settings.serviceUrlHint')}
        </p>
      </div>
      <div>
        <label for="opencode-service-password" class="mb-1 block text-xs leading-5 font-medium"
          >{i18n.t('settings.servicePassword')}</label
        >
        <Input
          id="opencode-service-password"
          type="password"
          bind:value={password}
          disabled={disabled || clearPassword}
          autocomplete="new-password"
          aria-describedby="opencode-service-password-hint"
        />
        <p id="opencode-service-password-hint" class="mt-1 mb-0 text-xs leading-5 text-[var(--text-muted)]">
          {i18n.t('settings.servicePasswordHint')}
        </p>
        {#if status?.target.hasPassword}
          <p class="mt-1 mb-0 text-xs leading-5 text-[var(--text-secondary)]">
            {i18n.t('settings.servicePasswordSaved')}
          </p>
          <label class="mt-2 flex items-center gap-2 text-xs leading-5">
            <input type="checkbox" bind:checked={clearPassword} {disabled} class="accent-[var(--accent-primary)]">
            {i18n.t('settings.serviceClearPassword')}
          </label>
        {/if}
        <p class="mt-2 mb-0 text-xs leading-5 text-[var(--status-warning)]">
          {i18n.t('settings.serviceCredentialWarning')}
        </p>
      </div>
    {/if}
    <Button type="submit" size="sm" class="self-start" disabled={disabled || (mode === 'explicit' && !url.trim())}
      >{i18n.t(pending === 'save' ? 'settings.serviceSaving' : 'settings.serviceSave')}</Button
    >
  </form>
  {#if error || status?.error}
    <p class="mt-3 mb-0 text-xs leading-5 text-[var(--status-error)]" role="alert">
      {i18n.t(error === 'busy' ? 'settings.serviceSaveBusy' : error === 'validation' ? 'settings.serviceSaveValidation' : error === 'save' ? 'settings.serviceSaveFailed' : 'settings.serviceReadFailed')}
    </p>
  {/if}
  {#if saved}
    <p class="mt-3 mb-0 text-xs leading-5 text-[var(--accent-primary)]" role="status">
      {i18n.t(connected ? 'settings.serviceSaved' : 'settings.serviceSavedOffline')}
    </p>
  {/if}
</section>
