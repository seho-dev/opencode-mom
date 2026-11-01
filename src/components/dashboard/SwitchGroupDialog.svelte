<script lang="ts">
import { ArrowLeftRight, Check, Plus, Search, X } from '@lucide/svelte';
import { untrack } from 'svelte';
import { Button } from '$src/components/button/index.js';
import * as Dialog from '$src/components/dialog/index.js';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import { GROUP_TYPE_LABELS, type GroupType } from '$src/utils/constants.js';
import { groupBindings } from '$src/utils/dashboard.js';

let { runtime = $bindable(null), trigger }: { runtime: GroupType | null; trigger: HTMLButtonElement | null } = $props();
const config = getConfig();
const i18n = getI18n();
let query = $state('');
let selected = $state('');
let submitting = $state(false);
let failed = $state(false);
let switchedGroup = $state<string | null>(null);
let search = $state<HTMLInputElement | null>(null);
const isBusy = () => submitting || config.switching || config.saving || config.loading || config.reloading;
const busy = $derived(isBusy());
const groups = $derived(config.groups.filter((group) => group.type === runtime));
const filtered = $derived(
  groups.filter((group) => `${group.name} ${group.description}`.toLowerCase().includes(query.trim().toLowerCase())),
);
const selectedGroup = $derived(groups.find((group) => group.id === selected));
const label = $derived(runtime ? i18n.t(GROUP_TYPE_LABELS[runtime]) : '');
$effect(() => {
  const type = runtime;
  if (type)
    untrack(() => {
      query = '';
      selected = config.groups.find((group) => group.type === type && group.id === config.selectedGroupId)?.id ?? '';
      failed = false;
    });
});
async function confirm() {
  if (isBusy() || !selectedGroup?.isEnabled || selectedGroup.id === config.selectedGroupId) return;
  const target = selectedGroup;
  submitting = true;
  failed = false;
  try {
    await config.switchGroup(target.id);
    runtime = null;
    if (target.type !== 'native') switchedGroup = target.name;
  } catch {
    failed = true;
  } finally {
    submitting = false;
  }
}
async function reloadNow() {
  if (busy) return;
  submitting = true;
  try {
    await config.reloadOpencode();
    switchedGroup = null;
  } catch {
    /* Global feedback retains the reload error. */
  } finally {
    submitting = false;
  }
}
</script>

<Dialog.Root open={runtime !== null} onOpenChange={(open) => { if (!open && !isBusy()) runtime = null; }}>
  <Dialog.Content
    class="max-w-2xl gap-0 overflow-hidden border-t-2 border-t-[var(--accent-primary)] p-0 shadow-2xl"
    aria-busy={busy}
    onOpenAutoFocus={(event) => { event.preventDefault(); search?.focus(); }}
    onCloseAutoFocus={(event) => { event.preventDefault(); if (!switchedGroup) trigger?.focus(); }}
    onEscapeKeydown={(event) => { if (isBusy()) event.preventDefault(); }}
    onInteractOutside={(event) => { if (isBusy()) event.preventDefault(); }}
  >
    <div
      class="flex items-start justify-between gap-4 border-b border-[var(--border-default)]/50 bg-[var(--surface-input)]/40 p-5"
    >
      <div class="flex items-center gap-3">
        <span
          class="flex size-9 shrink-0 items-center justify-center rounded-[2px] border border-[var(--border-default)]/50 bg-[var(--surface-hover)] text-[var(--accent-primary)]"
          ><ArrowLeftRight size={18} /></span
        >
        <Dialog.Header class="gap-1">
          <Dialog.Title class="font-[var(--font-heading)] text-base font-bold uppercase tracking-wider"
            >{i18n.t('dashboard.switchGroup')}</Dialog.Title
          >
          <Dialog.Description class="text-xs">{i18n.t('dashboard.selectGroup', { type: label })}</Dialog.Description>
        </Dialog.Header>
      </div>
      <Button
        variant="secondary"
        size="icon"
        disabled={busy}
        aria-label={i18n.t('common.close')}
        onclick={() => (runtime = null)}
        ><X size={14} /></Button
      >
    </div>
    <div class="border-b border-[var(--border-default)]/30 bg-[var(--surface-input)]/20 p-5">
      <div
        class="flex items-center gap-2 rounded-[2px] border border-[var(--border-default)] bg-[var(--surface-input)] px-3 focus-within:border-[var(--accent-primary)]"
      >
        <Search size={14} class="shrink-0 text-[var(--text-secondary)]" aria-hidden="true" />
        <input
          bind:this={search}
          bind:value={query}
          disabled={busy}
          class="h-9 min-w-0 flex-1 border-0 bg-transparent text-[11px] text-[var(--text-primary)] outline-none placeholder:text-[var(--text-secondary)] focus-visible:shadow-none"
          aria-label={i18n.t('dashboard.searchGroups', { type: label })}
          placeholder={i18n.t('dashboard.searchGroups', { type: label })}
        >
      </div>
    </div>
    <fieldset class="m-0 max-h-[min(360px,50dvh)] min-h-32 space-y-2.5 overflow-y-auto border-0 p-5" disabled={busy}>
      <legend class="sr-only">{i18n.t('dashboard.selectGroup', { type: label })}</legend>
      {#each filtered as group (group.id)}
        {@const bindings = groupBindings(group)}
        <label
          class="group-option flex cursor-pointer items-start gap-3 rounded-[3px] border border-[var(--border-default)]/60 bg-[var(--surface-input)]/40 p-3.5 transition-colors hover:border-[var(--accent-primary)]/60"
          class:selected={selected === group.id}
          class:unavailable={!group.isEnabled}
        >
          <input
            type="radio"
            name="dashboard-group"
            value={group.id}
            bind:group={selected}
            disabled={!group.isEnabled}
            aria-label={group.name}
            class="mt-0.5 size-4 shrink-0 accent-[var(--accent-primary)]"
            onchange={() => (failed = false)}
          >
          <span class="min-w-0 flex-1">
            <span class="flex flex-wrap items-center gap-2"
              ><span class="break-all font-[var(--font-heading)] text-xs font-semibold uppercase tracking-wide"
                >{group.name}</span
              >
              {#if group.id === config.selectedGroupId}
                <span
                  class="rounded-[2px] border border-[var(--accent-solid)] bg-[var(--accent-solid)] px-1.5 font-[var(--font-heading)] text-[9px] font-bold uppercase tracking-wide text-[var(--text-on-accent)]"
                  >{i18n.t('dashboard.current')}</span
                >
              {/if}
              {#if !group.isEnabled}
                <span class="border border-[var(--border-default)] px-1.5 text-[9px] text-[var(--text-secondary)]"
                  >{i18n.t('groups.disabled')}</span
                >
              {/if}</span
            >
            <span class="mt-1.5 flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1 text-[10px]">
              <span class="text-[var(--text-secondary)]"
                >{i18n.t('dashboard.groupSummary', { bindings: bindings.length, models: new Set(bindings.map((binding) => binding.modelRef)).size })}</span
              >
              {#if bindings.length}
                <span aria-hidden="true" class="text-[var(--text-muted)]">•</span>
                <span
                  class="min-w-0 flex-1 truncate text-[var(--accent-primary)]"
                  title={bindings.map((binding) => binding.modelRef).join(', ')}
                  >{[...new Set(bindings.map((binding) => binding.modelRef))].join(', ')}</span
                >
              {/if}
            </span>
            {#if group.description}
              <span class="mt-1 block text-[10px] text-[var(--text-secondary)]">{group.description}</span>
            {/if}
          </span>
          <span
            class="selection-label mt-0.5 shrink-0 rounded-[2px] border border-[var(--border-default)] px-2 py-1 font-[var(--font-heading)] text-[10px] font-semibold uppercase text-[var(--text-secondary)]"
            >{i18n.t(!group.isEnabled ? 'groups.disabled' : selected === group.id ? 'dashboard.selected' : 'dashboard.select')}</span
          >
        </label>
      {:else}
        <p class="py-6 text-center text-[11px] text-[var(--text-secondary)]">
          {i18n.t(groups.length ? 'dashboard.noSearchResults' : 'dashboard.noGroups')}
        </p>
      {/each}
    </fieldset>
    {#if failed}
      <p role="alert" class="px-5 pb-4 text-[11px] text-[var(--status-error)]">{i18n.t('dashboard.switchFailed')}</p>
    {/if}
    <div
      class="flex flex-wrap items-center justify-between gap-3 border-t border-[var(--border-default)]/40 bg-[var(--surface-input)]/50 p-4"
    >
      <Button href="/groups/new" variant="ghost" size="sm" disabled={busy} class="text-[var(--accent-primary)]"
        ><Plus size={13} />{i18n.t('dashboard.createGroup')}</Button
      >
      <div class="flex gap-2">
        <Button variant="secondary" disabled={busy} onclick={() => (runtime = null)}>{i18n.t('common.cancel')}</Button>
        <Button
          disabled={busy || !selectedGroup?.isEnabled || selectedGroup.id === config.selectedGroupId}
          onclick={confirm}
          ><Check size={13} />
          {i18n.t(submitting || config.switching ? 'dashboard.switching' : 'dashboard.confirmSwitch')}</Button
        >
      </div>
    </div>
  </Dialog.Content>
</Dialog.Root>

<Dialog.Root open={switchedGroup !== null} onOpenChange={(open) => { if (!open && !isBusy()) switchedGroup = null; }}>
  <Dialog.Content
    aria-busy={busy}
    onCloseAutoFocus={(event) => { event.preventDefault(); trigger?.focus(); }}
    onEscapeKeydown={(event) => { if (isBusy()) event.preventDefault(); }}
    onInteractOutside={(event) => { if (isBusy()) event.preventDefault(); }}
  >
    <Dialog.Header>
      <Dialog.Title>{i18n.t('groups.reloadTitle')}</Dialog.Title>
      <Dialog.Description>
        <span class="block">{i18n.t('groups.reloadDescription', { name: switchedGroup ?? '' })}</span>
        <span class="mt-2 block">{i18n.t('groups.reloadWarning')}</span>
      </Dialog.Description>
      {#if busy}
        <span class="sr-only" role="status">{i18n.t('groups.reloading')}</span>
      {/if}
    </Dialog.Header>
    <Dialog.Footer>
      <Button variant="ghost" disabled={busy} onclick={() => (switchedGroup = null)}>{i18n.t('common.cancel')}</Button>
      <Button variant="outline" disabled={busy} onclick={() => (switchedGroup = null)}
        >{i18n.t('groups.reloadLater')}</Button
      >
      <Button disabled={busy} onclick={reloadNow}>{i18n.t(busy ? 'groups.reloading' : 'groups.reloadNow')}</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>

<style>
.group-option.selected {
  border-color: var(--accent-primary);
  background: color-mix(in srgb, var(--surface-input) 80%, var(--surface-panel));
  box-shadow:
    inset 0 0 0 1px var(--accent-primary),
    0 0 12px color-mix(in srgb, var(--accent-primary) 15%, transparent);
}
.group-option.selected .selection-label {
  border-color: color-mix(in srgb, var(--accent-primary) 30%, transparent);
  background: var(--surface-active);
  color: var(--accent-primary);
}
.group-option:focus-within {
  outline: 1px solid var(--focus-ring);
  outline-offset: 2px;
}
.group-option.unavailable {
  cursor: not-allowed;
  opacity: var(--disabled-opacity);
}
fieldset:disabled .group-option {
  cursor: wait;
  opacity: var(--disabled-opacity);
}
</style>
