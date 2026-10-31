<script lang="ts">
import { Eye, Pencil, Plus, RefreshCw } from '@lucide/svelte';
import { untrack } from 'svelte';
import { Button } from '$src/components/button/index.js';
import Diagnostics from '$src/components/configuration/Diagnostics.svelte';
import { readError } from '$src/components/configuration/read-error.js';
import DataTable from '$src/components/DataTable.svelte';
import EmptyTableRow from '$src/components/EmptyTableRow.svelte';
import PageHead from '$src/components/PageHead.svelte';
import ResourceScope from '$src/components/ResourceScope.svelte';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import type { SlashCommandEntry } from '$src/types/commands.js';

const config = getConfig();
const i18n = getI18n();
const projectId = $derived(config.selectedProjectId ?? null);
let commands = $state<SlashCommandEntry[]>([]);
let globalCommands = $state<SlashCommandEntry[]>([]);
let diagnostics = $state<string[]>([]);
let globalDiagnostics = $state<string[]>([]);
let loading = $state(true);
let globalLoading = $state(false);
let error = $state('');
let globalError = $state('');
let showGlobal = $state(false);
let query = $state('');
let page = $state(1);
let globalPage = $state(1);
let request = 0;
let globalRequest = 0;
const pageSize = 10;
const filtered = $derived(filter(commands));
const globalFiltered = $derived(filter(globalCommands));
function filter(entries: SlashCommandEntry[]) {
  const text = query.trim().toLowerCase();
  return entries.filter((entry) =>
    `${entry.name} ${entry.description ?? ''} ${entry.sourcePath}`.toLowerCase().includes(text),
  );
}
$effect(() => {
  void query;
  page = 1;
  globalPage = 1;
});

async function load() {
  const seq = ++request;
  const scope = projectId;
  loading = true;
  error = '';
  diagnostics = [];
  try {
    const result = await config.listSlashCommands(scope);
    if (seq !== request) return;
    commands = result.data;
    diagnostics = result.diagnostics;
  } catch (cause) {
    if (seq === request) {
      error = readError(cause);
      commands = [];
    }
  } finally {
    if (seq === request) loading = false;
  }
}
async function loadGlobal() {
  const seq = ++globalRequest;
  globalLoading = true;
  globalError = '';
  globalCommands = [];
  globalDiagnostics = [];
  try {
    const result = await config.listSlashCommands(null);
    if (seq !== globalRequest) return;
    globalCommands = result.data;
    globalDiagnostics = result.diagnostics;
  } catch (cause) {
    if (seq === globalRequest) globalError = readError(cause);
  } finally {
    if (seq === globalRequest) globalLoading = false;
  }
}
$effect(() => {
  void projectId;
  commands = [];
  query = '';
  page = 1;
  untrack(() => {
    void load();
  });
  return () => {
    request += 1;
  };
});
$effect(() => {
  const scope = projectId;
  const compare = showGlobal;
  globalCommands = [];
  globalDiagnostics = [];
  globalError = '';
  globalPage = 1;
  if (scope !== null && compare)
    untrack(() => {
      void loadGlobal();
    });
  else globalLoading = false;
  return () => {
    globalRequest += 1;
  };
});
function href(id: string, scope: string | null, compare = false, edit = false) {
  const params = new URLSearchParams();
  if (scope !== null) params.set('project', scope);
  if (compare) params.set('readonly', '1');
  if (edit) params.set('edit', '1');
  return `/commands/${encodeURIComponent(id)}${params.size ? `?${params}` : ''}`;
}
</script>

<svelte:head><title>{i18n.t('commands.metaTitle')}</title></svelte:head>
<PageHead eyebrow={i18n.t('commands.eyebrow')} title={i18n.t('commands.title')}>
  <div class="flex flex-wrap justify-end gap-2">
    <Button
      variant="outline"
      size="sm"
      disabled={loading || globalLoading}
      aria-busy={loading || globalLoading}
      onclick={() => { void load(); if (projectId !== null && showGlobal) void loadGlobal(); }}
    >
      <RefreshCw
        size={14}
        class={loading || globalLoading ? 'animate-spin motion-reduce:animate-none' : ''}
        aria-hidden="true"
      />{i18n.t('configuration.refresh')}
    </Button>
    <Button size="sm" href={`/commands/new${projectId !== null ? `?project=${encodeURIComponent(projectId)}` : ''}`}
      ><Plus size={14} />{i18n.t('commands.newTitle')}</Button
    >
  </div>
</PageHead>
<ResourceScope writable bind:showGlobal />
<Diagnostics messages={diagnostics} />
{#if error}
  <div class="state-banner error" role="alert">
    <span class="min-w-0 break-words [overflow-wrap:anywhere]">{i18n.t('commands.loadFailed')} {error}</span
    ><Button variant="outline" size="sm" onclick={load}>{i18n.t('configuration.retry')}</Button>
  </div>
{/if}
<div class="search-toolbar">
  <input
    name="commands-search"
    aria-label={i18n.t('commands.search')}
    placeholder={i18n.t('commands.searchPlaceholder')}
    bind:value={query}
  >
</div>
<DataTable label={i18n.t('commands.title')} total={loading || error ? 0 : filtered.length} bind:page {pageSize}>
  {@render head()}
  {@render body(filtered, projectId, false, loading, error, page)}
</DataTable>
{#if projectId !== null && showGlobal}
  <section class="mt-6 min-w-0" aria-labelledby="commands-global-title">
    <h2 id="commands-global-title" class="mb-2 font-[var(--font-heading)] text-sm">
      {i18n.t('scope.global')}
      · {i18n.t('common.readOnly')}
    </h2>
    <Diagnostics messages={globalDiagnostics} />
    {#if globalError}
      <div class="state-banner error" role="alert">
        <span>{i18n.t('scope.globalComparisonFailed')} {globalError}</span
        ><Button variant="outline" size="sm" onclick={loadGlobal}>{i18n.t('configuration.retry')}</Button>
      </div>
    {/if}
    <DataTable
      label={`${i18n.t('commands.title')} · ${i18n.t('scope.global')}`}
      total={globalLoading || globalError ? 0 : globalFiltered.length}
      bind:page={globalPage}
      {pageSize}
    >
      {@render head()}
      {@render body(globalFiltered, null, true, globalLoading, globalError, globalPage)}
    </DataTable>
  </section>
{/if}

{#snippet head()}
  <thead>
    <tr>
      <th>{i18n.t('commands.name')}</th>
      <th>{i18n.t('common.description')}</th>
      <th>{i18n.t('configuration.source')}</th>
      <th class="th-actions">{i18n.t('common.actions')}</th>
    </tr>
  </thead>
{/snippet}
{#snippet body(entries: SlashCommandEntry[], scope: string | null, compare: boolean, busy: boolean, failure: string, current: number)}
  <tbody>
    {#if busy}
      <EmptyTableRow colspan={4} message={i18n.t('common.loadingConfiguration')} />
    {:else if failure}
      <EmptyTableRow colspan={4} message={i18n.t('commands.loadFailed')} />
    {:else if !entries.length}
      <EmptyTableRow colspan={4} message={i18n.t(query.trim() ? 'common.noMatches' : 'commands.empty')} />
    {:else}
      {#each entries.slice((Math.min(current, Math.ceil(entries.length / pageSize)) - 1) * pageSize, Math.min(current, Math.ceil(entries.length / pageSize)) * pageSize) as entry (JSON.stringify([scope, entry.id]))}
        <tr>
          <td class="model-name">
            <a
              class="inline-block max-w-64 break-words hover:underline [overflow-wrap:anywhere]"
              href={href(entry.id, scope, compare)}
              >/{entry.name}</a
            >
            {#if compare && commands.some((command) => command.name === entry.name)}
              <small class="mt-1 block text-[var(--text-muted)]">{i18n.t('commands.sameName')}</small>
            {/if}
          </td>
          <td>
            <span
              class="block max-w-md whitespace-pre-wrap break-words text-[var(--text-secondary)] [overflow-wrap:anywhere]"
              >{entry.description || '—'}</span
            >
          </td>
          <td>
            <span class="block max-w-72 break-words text-[var(--text-secondary)] [overflow-wrap:anywhere]"
              >{entry.sourcePath}</span
            >
          </td>
          <td class="row-actions">
            <Button
              href={href(entry.id, scope, compare)}
              variant="ghost"
              size="icon-sm"
              aria-label={i18n.t('configuration.viewAria', { name: entry.name })}
              ><Eye size={14} /></Button
            >
            {#if !compare}
              <Button
                href={href(entry.id, scope, false, true)}
                variant="ghost"
                size="icon-sm"
                aria-label={i18n.t('configuration.editAria', { name: entry.name })}
                ><Pencil size={14} /></Button
              >
            {/if}
          </td>
        </tr>
      {/each}
    {/if}
  </tbody>
{/snippet}
