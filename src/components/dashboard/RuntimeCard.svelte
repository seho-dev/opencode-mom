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
  variant = 'full',
  onswitch,
}: {
  type: GroupType;
  busy: boolean;
  variant?: 'tile' | 'featured' | 'full';
  onswitch: (type: GroupType, trigger: HTMLButtonElement) => void;
} = $props();
const config = getConfig();
const i18n = getI18n();
const groups = $derived(config.groups.filter((group) => group.type === type));
const enabledCount = $derived(groups.filter((group) => group.isEnabled).length);
const current = $derived(groups.find((group) => group.id === config.selectedGroupId));
const bindings = $derived(current ? groupBindings(current) : []);
const label = $derived(i18n.t(GROUP_TYPE_LABELS[type]));
const unavailable = $derived(!config.loading && !config.groups.length && !!config.error);
const Icon = $derived(type === 'native' ? Terminal : type === 'slim' ? Bolt : Network);
const headingId = $derived(variant === 'featured' ? `runtime-featured-${type}` : `runtime-${type}`);
</script>

<article
  class="runtime-card flex min-w-0 flex-col rounded-[3px] border border-[var(--border-default)] bg-[var(--surface-panel)] transition-colors hover:border-[var(--accent-primary)]/50"
  class:p-3.5={variant === 'tile'}
  class:p-5={variant !== 'tile'}
  class:h-full={variant === 'featured'}
  data-variant={variant}
  data-active={Boolean(current)}
  aria-labelledby={headingId}
>
  {#if variant === 'tile'}
    <button
      type="button"
      class="tile-trigger group flex w-full flex-1 min-w-0 flex-col justify-between text-left disabled:cursor-not-allowed"
      disabled={busy || !groups.length || unavailable}
      aria-haspopup="dialog"
      aria-label={i18n.t('dashboard.switchRuntime', { type: label })}
      onclick={(event) => onswitch(type, event.currentTarget)}
    >
      <div class="flex w-full items-start justify-between gap-2.5">
        <div class="flex min-w-0 items-center gap-3">
          <span
            class="runtime-icon flex size-9 shrink-0 items-center justify-center rounded-[2px] border border-[var(--border-default)]/70 bg-[var(--surface-input)] text-[var(--accent-primary)] transition-colors group-hover:border-[var(--accent-primary)]/60"
            ><Icon size={18} aria-hidden="true" /></span
          >
          <div class="min-w-0">
            <span
              id={headingId}
              class="block font-[var(--font-heading)] text-[13px] font-bold uppercase tracking-wider text-[var(--text-primary)]"
            >
              {i18n.t('dashboard.runtimeTitle', { type: label })}
            </span>
            <p class="mt-0.5 truncate text-[11px] text-[var(--text-secondary)]">
              {i18n.t(
                type === 'native'
                  ? 'dashboard.nativeDescription'
                  : type === 'slim'
                    ? 'dashboard.slimDescription'
                    : 'dashboard.omoDescription',
              )}
            </p>
          </div>
        </div>
        <span
          class="tile-radio mt-0.5 flex size-4 shrink-0 items-center justify-center rounded-full border transition-colors"
          class:border-[var(--accent-primary)]={Boolean(current)}
          class:bg-[var(--surface-active)]={Boolean(current)}
          class:border-[var(--border-default)]={!current}
          class:bg-[var(--surface-input)]={!current}
          aria-hidden="true"
        >
          {#if current}
            <span class="size-2 rounded-full bg-[var(--accent-primary)]"></span>
          {/if}
        </span>
      </div>

      {#if config.loading || unavailable || groups.length > 0}
        <div
          class="tile-status mt-3 flex w-full items-center justify-between gap-2 border-t border-[var(--border-default)]/50 pt-2.5 text-[11px]"
        >
          {#if config.loading}
            <span class="truncate text-[var(--text-secondary)]">{i18n.t('common.loadingConfiguration')}</span>
          {:else if unavailable}
            <span class="truncate text-[var(--status-error)]">{i18n.t('dashboard.configFailed')}</span>
          {:else if current}
            <span
              class="inline-flex min-w-0 items-center gap-1.5 font-semibold text-[var(--text-primary)]"
              title={current.name}
            >
              <span class="size-1.5 shrink-0 rounded-full bg-[var(--accent-primary)]" aria-hidden="true"></span>
              <span class="truncate">{current.name}</span>
            </span>
            <span
              class="shrink-0 rounded-[2px] border border-[var(--accent-primary)]/50 bg-[var(--surface-active)] px-1.5 py-0.5 font-[var(--font-heading)] text-[9px] font-bold uppercase tracking-wider text-[var(--accent-primary)]"
              >{i18n.t('dashboard.current')}</span
            >
          {:else}
            <span class="truncate text-[var(--text-secondary)]">{i18n.t('dashboard.noActiveGroup')}</span>
            <span
              class="shrink-0 rounded-[2px] border border-[var(--border-default)]/40 bg-[var(--surface-input)] px-1.5 py-0.5 font-mono text-[10px] tabular-nums text-[var(--text-secondary)]"
            >
              {i18n.t('dashboard.groupsConfigured', { enabled: enabledCount, total: groups.length })}
            </span>
          {/if}
        </div>
      {/if}
    </button>

    {#if !config.loading && !unavailable && !groups.length}
      <div
        class="tile-status mt-3 flex w-full items-center justify-between gap-2 border-t border-[var(--border-default)]/50 pt-2.5 text-[11px]"
      >
        <span class="truncate text-[var(--text-secondary)]">{i18n.t('dashboard.noActiveGroup')}</span>
        <Button
          href="/groups/new"
          variant="ghost"
          size="sm"
          disabled={busy}
          class="h-auto px-1.5 py-0.5 text-[11px] text-[var(--accent-primary)]"
          ><Plus size={11} />{i18n.t('dashboard.createGroup')}</Button
        >
      </div>
    {/if}
  {:else}
    <header class="mb-4 flex items-start justify-between gap-3 border-b border-[var(--border-default)]/40 pb-3.5">
      <div class="flex min-w-0 items-center gap-3">
        <span
          class="runtime-icon flex size-10 shrink-0 items-center justify-center rounded-[2px] border border-[var(--border-default)]/70 bg-[var(--surface-input)] text-[var(--accent-primary)]"
          ><Icon size={20} aria-hidden="true" /></span
        >
        <div class="min-w-0">
          {#if variant === 'featured'}
            <p
              class="mb-0.5 font-[var(--font-heading)] text-[10px] font-bold uppercase tracking-[0.12em] text-[var(--accent-primary)]"
            >
              {i18n.t('dashboard.featuredRuntime')}
            </p>
          {/if}
          <h2
            id={headingId}
            class="font-[var(--font-heading)] text-sm font-bold uppercase tracking-wider text-[var(--text-primary)]"
          >
            {i18n.t('dashboard.runtimeTitle', { type: label })}
          </h2>
          <p class="mt-0.5 truncate text-xs text-[var(--text-secondary)]">
            {i18n.t(
              type === 'native'
                ? 'dashboard.nativeDescription'
                : type === 'slim'
                  ? 'dashboard.slimDescription'
                  : 'dashboard.omoDescription',
            )}
          </p>
        </div>
      </div>
      {#if current && variant !== 'featured'}
        <span
          class="shrink-0 rounded-[2px] border border-[var(--accent-primary)]/40 bg-[var(--surface-active)] px-1.5 py-0.5 font-[var(--font-heading)] text-[9px] font-bold uppercase tracking-wider text-[var(--accent-primary)]"
          >{i18n.t('dashboard.current')}</span
        >
      {:else if variant === 'featured' && !config.loading && !unavailable}
        <span
          class="shrink-0 rounded-[2px] border border-[var(--border-default)]/60 bg-[var(--surface-input)] px-2 py-1 font-mono text-[10px] tabular-nums text-[var(--text-secondary)]"
        >
          {i18n.t('dashboard.groupsConfigured', { enabled: enabledCount, total: groups.length })}
        </span>
      {/if}
    </header>
    <div
      class="current-group-slot mb-4 rounded-[2px] border border-[var(--border-default)]/60 bg-[var(--surface-input)] p-3.5"
      data-has-current={Boolean(current)}
    >
      <div class="mb-2 flex flex-wrap items-center justify-between gap-2">
        <span
          class="font-[var(--font-heading)] text-[10px] font-bold uppercase tracking-[0.1em] text-[var(--text-secondary)]"
          >{i18n.t('dashboard.currentGroup')}</span
        >
      </div>
      {#if config.loading}
        <p class="text-xs text-[var(--text-secondary)]">{i18n.t('common.loadingConfiguration')}</p>
      {:else if unavailable}
        <p class="text-xs text-[var(--status-error)]">{i18n.t('dashboard.configFailed')}</p>
      {:else if current}
        <a
          class="group-name flex min-w-0 items-center justify-between gap-3 rounded-[2px] border border-[var(--border-default)]/70 bg-[var(--surface-panel)] px-3 py-2 text-[13px] font-semibold text-[var(--text-primary)] transition-colors hover:border-[var(--accent-primary)] hover:text-[var(--accent-primary)]"
          href={`/groups/${current.id}/edit`}
          aria-label={i18n.t('groups.editAria', { name: current.name })}
          title={current.name}
        >
          <span class="truncate">{current.name}</span>
          <ArrowRight size={14} class="shrink-0 text-[var(--accent-primary)]" aria-hidden="true" />
        </a>
      {:else}
        <p class="py-1 text-xs text-[var(--text-secondary)]">
          {i18n.t(variant === 'featured' ? (groups.length ? 'dashboard.chooseGroup' : 'dashboard.noGroups') : 'dashboard.noActiveGroup')}
        </p>
      {/if}
    </div>
    <div class="mb-4 flex flex-1 flex-col">
      <div class="mb-2.5 flex flex-wrap items-center justify-between gap-2 text-[10px] text-[var(--text-secondary)]">
        <h3
          class="font-[var(--font-heading)] text-[11px] font-bold uppercase tracking-[0.08em] text-[var(--text-secondary)]"
        >
          {i18n.t('dashboard.mappings')}
        </h3>
        {#if !config.loading && !unavailable}
          <span
            class="rounded-[2px] border border-[var(--border-default)]/50 bg-[var(--surface-input)] px-2 py-0.5 font-mono text-[10px] tabular-nums text-[var(--text-secondary)]"
          >
            {i18n.t('dashboard.mappingCount', { count: bindings.length })}
          </span>
        {/if}
      </div>
      {#if bindings.length}
        <ul class="binding-roster space-y-1.5">
          {#each bindings as binding, index (`${binding.kind}-${binding.name}-${index}`)}
            <li
              class="flex min-w-0 items-center justify-between gap-3 rounded-[2px] border border-[var(--border-default)]/45 bg-[var(--surface-input)]/80 px-3 py-2 text-xs transition-colors hover:border-[var(--border-default)]"
            >
              <span class="min-w-0 truncate font-medium text-[var(--text-primary)]" title={binding.name}
                >{binding.name}
                {#if binding.kind === 'category'}
                  <span
                    class="ml-1.5 rounded-[2px] border border-[var(--border-default)]/50 bg-[var(--surface-panel)] px-1.5 py-0.5 font-[var(--font-heading)] text-[9px] font-semibold uppercase tracking-wider text-[var(--text-secondary)]"
                    >{i18n.t('dashboard.category')}</span
                  >
                {/if}</span
              >
              <ArrowRight size={12} class="shrink-0 text-[var(--text-muted)]" aria-hidden="true" />
              <span
                class="min-w-0 flex-1 truncate text-right font-mono text-[11px] font-medium text-[var(--accent-primary)]"
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
          class="flex min-h-32 flex-1 flex-col items-center justify-center gap-3 rounded-[2px] border border-dashed border-[var(--border-default)]/60 bg-[var(--surface-input)]/40 px-4 py-7 text-center"
        >
          <p class="text-[11px] leading-5 text-[var(--text-secondary)]">
            {i18n.t(
              config.loading
                ? 'common.loadingConfiguration'
                : unavailable
                  ? 'dashboard.configFailed'
                  : current
                    ? 'dashboard.noMappings'
                    : groups.length
                      ? 'dashboard.chooseGroup'
                      : 'dashboard.noGroups',
            )}
          </p>
          {#if !config.loading && !unavailable && (variant !== 'featured' || current)}
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
    <footer class="mt-auto flex gap-2.5 border-t border-[var(--border-default)]/50 pt-3.5">
      <Button
        variant="secondary"
        class="flex-1 bg-[var(--surface-active)] text-[var(--accent-primary)] uppercase tracking-wide"
        disabled={busy || !groups.length || unavailable}
        aria-haspopup="dialog"
        aria-label={variant === 'featured' ? undefined : i18n.t('dashboard.switchRuntime', { type: label })}
        onclick={(event) => onswitch(type, event.currentTarget)}
        >{i18n.t('dashboard.switchGroup')}</Button
      >
      <Button
        href={current ? `/groups/${current.id}/edit` : '/groups'}
        variant={variant === 'featured' ? 'outline' : 'secondary'}
        size={variant === 'featured' ? 'sm' : 'icon'}
        disabled={busy}
        class={variant === 'featured' ? 'text-[var(--accent-primary)] uppercase tracking-wide' : ''}
        aria-label={current ? i18n.t('groups.editAria', { name: current.name }) : i18n.t('groups.directoryButton')}
        title={i18n.t('groups.editTitle')}
      >
        <Settings2 size={14} />
      </Button>
    </footer>
  {/if}
</article>

<style>
.runtime-card {
  position: relative;
}

.runtime-card[data-variant="full"],
.runtime-card[data-variant="featured"] {
  min-height: 332px;
}

.runtime-card::before {
  content: "";
  position: absolute;
  inset: -1px -1px auto;
  height: 2px;
  border-top: 2px solid var(--border-default);
  border-top-color: color-mix(in srgb, var(--accent-primary) 35%, var(--border-default));
}

.runtime-card[data-active="true"] {
  border-color: var(--accent-primary);
  background:
    linear-gradient(160deg, color-mix(in srgb, var(--surface-active) 50%, transparent) 0%, var(--surface-panel) 48%),
    var(--surface-panel);
  box-shadow: var(--focus-glow);
}

.runtime-card[data-active="true"]::before {
  height: 3px;
  border-top-width: 3px;
  border-top-color: var(--accent-primary);
}

.runtime-card[data-active="true"] .runtime-icon {
  border-color: color-mix(in srgb, var(--accent-primary) 55%, var(--border-default));
  background: var(--surface-active);
}

.current-group-slot[data-has-current="true"] {
  border-left: 2px solid var(--accent-primary);
}

.tile-trigger:disabled .tile-radio {
  opacity: 0.45;
}

.tile-trigger:focus-visible {
  outline: var(--focus-width) solid var(--focus-ring);
  outline-offset: var(--focus-offset);
  border-radius: var(--radius-sm);
}

.binding-roster {
  max-height: 220px;
  overflow-y: auto;
}
</style>
