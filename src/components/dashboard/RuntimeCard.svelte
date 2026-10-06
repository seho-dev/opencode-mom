<script lang="ts">
import { ArrowRight, Bolt, Network, Plus, Settings2, Terminal } from '@lucide/svelte';
import { Button } from '$src/components/button/index.js';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import { GROUP_TYPE_LABELS, type GroupType } from '$src/utils/constants.js';
import { groupBindings } from '$src/utils/dashboard.js';

let {
  type,
  busy,
  onswitch,
}: { type: GroupType; busy: boolean; onswitch: (type: GroupType, trigger: HTMLButtonElement) => void } = $props();
const config = getConfig();
const i18n = getI18n();
const groups = $derived(config.groups.filter((group) => group.type === type));
const current = $derived(groups.find((group) => group.id === config.selectedGroupId));
const bindings = $derived(current ? groupBindings(current) : []);
const label = $derived(i18n.t(GROUP_TYPE_LABELS[type]));
const unavailable = $derived(!config.loading && !config.groups.length && !!config.error);
const Icon = $derived(type === 'native' ? Terminal : type === 'slim' ? Bolt : Network);
</script>

<article
  class="runtime-card flex min-w-0 flex-col rounded-[3px] border border-[var(--border-default)] bg-[var(--surface-panel)] p-5 transition-colors hover:border-[var(--accent-primary)]/50"
  aria-labelledby={`runtime-${type}`}
>
  <header class="mb-3 flex items-center gap-2.5">
    <span
      class="flex size-8 shrink-0 items-center justify-center rounded-[2px] border border-[var(--border-default)]/50 bg-[var(--surface-hover)] text-[var(--accent-primary)]"
      ><Icon size={18} aria-hidden="true" /></span
    >
    <div class="min-w-0">
      <h2 id={`runtime-${type}`} class="font-[var(--font-heading)] text-xs font-bold uppercase tracking-wider">
        {i18n.t('dashboard.runtimeTitle', { type: label })}
      </h2>
      <p class="text-[11px] text-[var(--text-secondary)]">
        {i18n.t(type === 'native' ? 'dashboard.nativeDescription' : type === 'slim' ? 'dashboard.slimDescription' : 'dashboard.omoDescription')}
      </p>
    </div>
  </header>
  <div class="mb-3.5 rounded-[2px] border border-[var(--border-default)]/40 bg-[var(--surface-input)] p-3">
    <div class="mb-1.5 flex flex-wrap items-center justify-between gap-2">
      <span
        class="font-[var(--font-heading)] text-[10px] font-semibold uppercase tracking-wider text-[var(--text-secondary)]"
        >{i18n.t('dashboard.currentGroup')}</span
      >
      {#if current}
        <span
          class="border border-[var(--accent-primary)]/30 bg-[var(--surface-active)] px-1.5 py-0.5 text-[9px] text-[var(--accent-primary)]"
          >{i18n.t('dashboard.current')}</span
        >
      {/if}
    </div>
    {#if config.loading}
      <p class="text-xs text-[var(--text-secondary)]">{i18n.t('common.loadingConfiguration')}</p>
    {:else if unavailable}
      <p class="text-xs text-[var(--status-error)]">{i18n.t('dashboard.configFailed')}</p>
    {:else if current}
      <a
        class="group-name flex min-w-0 items-center justify-between gap-2 rounded-[2px] border border-[var(--border-default)]/50 bg-[var(--surface-panel)] px-2.5 py-1.5 text-xs transition-colors hover:border-[var(--accent-primary)] hover:text-[var(--accent-primary)]"
        href={`/groups/${current.id}/edit`}
        aria-label={i18n.t('groups.editAria', { name: current.name })}
        title={current.name}
      >
        <span class="truncate">{current.name}</span>
        <ArrowRight size={13} class="shrink-0 text-[var(--text-secondary)]" aria-hidden="true" />
      </a>
    {:else}
      <p class="py-1.5 text-xs text-[var(--text-secondary)]">{i18n.t('dashboard.noActiveGroup')}</p>
    {/if}
  </div>
  <div class="mb-4 flex-1">
    <div class="mb-2 flex flex-wrap items-center justify-between gap-2 text-[10px] text-[var(--text-secondary)]">
      <h3 class="font-[var(--font-heading)] font-semibold uppercase tracking-wider">{i18n.t('dashboard.mappings')}</h3>
      {#if !config.loading && !unavailable}
        <span>{i18n.t('dashboard.mappingCount', { count: bindings.length })}</span>
      {/if}
    </div>
    {#if bindings.length}
      <ul class="binding-roster space-y-1">
        {#each bindings as binding, index (`${binding.kind}-${binding.name}-${index}`)}
          <li
            class="flex min-w-0 items-center justify-between gap-2 rounded-[2px] border border-[var(--border-default)]/30 bg-[var(--surface-input)]/60 px-2.5 py-1.5 text-[11px]"
          >
            <span class="min-w-0 truncate" title={binding.name}
              >{binding.name}
              {#if binding.kind === 'category'}
                <span class="ml-1.5 text-[9px] text-[var(--text-secondary)]">{i18n.t('dashboard.category')}</span>
              {/if}</span
            >
            <ArrowRight size={11} class="shrink-0 text-[var(--text-muted)]" aria-hidden="true" />
            <span
              class="min-w-0 flex-1 truncate text-right text-[var(--accent-primary)]"
              title={`${binding.modelRef}${binding.variant ? ` · ${binding.variant}` : ''}`}
              >{binding.modelRef}
              {#if binding.variant}
                <span class="text-[var(--text-secondary)]"> · {binding.variant}</span>
              {/if}</span
            >
          </li>
        {/each}
      </ul>
    {:else}
      <div
        class="flex min-h-32 flex-col items-center justify-center gap-3 rounded-[2px] border border-dashed border-[var(--border-default)]/60 bg-[var(--surface-input)]/40 px-4 py-7 text-center"
      >
        <p class="text-[11px] leading-5 text-[var(--text-secondary)]">
          {i18n.t(config.loading ? 'common.loadingConfiguration' : unavailable ? 'dashboard.configFailed' : current ? 'dashboard.noMappings' : groups.length ? 'dashboard.chooseGroup' : 'dashboard.noGroups')}
        </p>
        {#if !config.loading && !unavailable}
          <Button
            href={current ? `/groups/${current.id}/edit` : '/groups/new'}
            variant="outline"
            size="sm"
            disabled={busy}
            class="text-[var(--accent-primary)]"
            ><Plus size={12} />{i18n.t(current ? 'dashboard.assignMappings' : 'dashboard.createGroup')}</Button
          >
        {/if}
      </div>
    {/if}
  </div>
  <footer class="flex gap-2 border-t border-[var(--border-default)]/40 pt-2">
    <Button
      variant="secondary"
      class="flex-1 bg-[var(--surface-active)] text-[var(--accent-primary)] uppercase tracking-wide"
      disabled={busy || !groups.length || unavailable}
      aria-haspopup="dialog"
      aria-label={i18n.t('dashboard.switchRuntime', { type: label })}
      onclick={(event) => onswitch(type, event.currentTarget)}
      >{i18n.t('dashboard.switchGroup')}</Button
    >
    <Button
      href={current ? `/groups/${current.id}/edit` : '/groups'}
      variant="secondary"
      size="icon"
      disabled={busy}
      aria-label={current ? i18n.t('groups.editAria', { name: current.name }) : i18n.t('groups.directoryButton')}
      title={i18n.t('groups.editTitle')}
      ><Settings2 size={14} /></Button
    >
  </footer>
</article>

<style>
.runtime-card {
  position: relative;
  min-height: 352px;
}
.runtime-card::before {
  content: "";
  position: absolute;
  inset: -1px -1px auto;
  height: 2px;
  border-top: 2px solid;
  border-top-color: inherit;
}
.binding-roster {
  max-height: 222px;
  overflow-y: auto;
  padding: 1px;
}
</style>
