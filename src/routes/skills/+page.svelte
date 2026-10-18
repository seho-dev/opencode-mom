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
import type { SkillEntry } from '$src/types/skills.js';

const config = getConfig();
const i18n = getI18n();
let skills = $state<SkillEntry[]>([]);
let globalSkills = $state<SkillEntry[]>([]);
let globalDiagnostics = $state<string[]>([]);
let globalError = $state('');
let globalLoading = $state(false);
let showGlobal = $state(false);
const projectId = $derived(config.selectedProjectId ?? null);
let diagnostics = $state<string[]>([]);
let loading = $state(true);
let error = $state('');
let query = $state('');
let page = $state(1);
const pageSize = 10;
let request = 0;
let globalRequest = 0;
const entries = $derived([
  ...skills.map((entry) => ({ entry, projectId })),
  ...(projectId && showGlobal ? globalSkills.map((entry) => ({ entry, projectId: null })) : []),
]);
const filtered = $derived(
  entries.filter(({ entry: skill }) =>
    `${skill.id} ${skill.name} ${skill.description ?? ''} ${skill.source}`
      .toLowerCase()
      .includes(query.trim().toLowerCase()),
  ),
);
const currentPage = $derived(Math.min(page, Math.max(1, Math.ceil(filtered.length / pageSize))));
const rows = $derived(filtered.slice((currentPage - 1) * pageSize, currentPage * pageSize));
$effect(() => {
  void query;
  void skills.length;
  page = 1;
});

async function load() {
  const seq = ++request;
  const scope = projectId;
  loading = true;
  error = '';
  diagnostics = [];
  try {
    const result = await (scope === null ? config.listSkills() : config.listSkills(scope));
    if (seq !== request) return;
    skills = result.data;
    diagnostics = result.diagnostics;
  } catch (cause) {
    if (seq === request) {
      error = readError(cause);
      skills = [];
    }
  } finally {
    if (seq === request) loading = false;
  }
}
$effect(() => {
  void projectId;
  skills = [];
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
  const compare = projectId !== null && showGlobal;
  globalSkills = [];
  globalDiagnostics = [];
  globalError = '';
  if (compare)
    untrack(() => {
      void loadGlobal();
    });
  else globalLoading = false;
  return () => {
    globalRequest += 1;
  };
});
async function loadGlobal() {
  const id = ++globalRequest;
  globalLoading = true;
  globalError = '';
  globalSkills = [];
  globalDiagnostics = [];
  try {
    const result = await config.listSkills();
    if (id !== globalRequest) return;
    globalSkills = result.data;
    globalDiagnostics = result.diagnostics;
  } catch (cause) {
    if (id === globalRequest) globalError = readError(cause);
  } finally {
    if (id === globalRequest) globalLoading = false;
  }
}
function refresh() {
  void load();
  if (projectId && showGlobal) void loadGlobal();
}
function href(id: string, scope: string | null) {
  return `/skills/${encodeURIComponent(id)}${scope !== null ? `?project=${encodeURIComponent(scope)}` : projectId ? '?readonly=1' : ''}`;
}
</script>

<svelte:head><title>{i18n.t('skills.metaTitle')}</title></svelte:head>
<PageHead eyebrow={i18n.t('skills.eyebrow')} title={i18n.t('skills.title')}>
  <div class="flex flex-wrap justify-end gap-2">
    <Button variant="outline" disabled={loading || globalLoading || config.saving} onclick={refresh}
      ><RefreshCw size={14} />{i18n.t('configuration.refresh')}</Button
    >
    {#if projectId === null}
      <Button href="/skills/new"><Plus size={14} />{i18n.t('skills.newTitle')}</Button>
    {/if}
  </div>
</PageHead>
<ResourceScope disabled={config.saving} bind:showGlobal />
<Diagnostics messages={diagnostics} />
{#if projectId && showGlobal}
  <Diagnostics messages={globalDiagnostics} />
  {#if globalLoading}
    <div class="state-banner" role="status">{i18n.t('scope.global')} · {i18n.t('common.loadingConfiguration')}</div>
  {/if}
  {#if globalError}
    <div class="state-banner error" role="alert">
      <span class="min-w-0 break-words [overflow-wrap:anywhere]"
        >{i18n.t('scope.globalComparisonFailed')} {globalError}</span
      ><Button variant="outline" size="sm" onclick={loadGlobal}>{i18n.t('configuration.retry')}</Button>
    </div>
  {/if}
{/if}
{#if error}
  <div class="state-banner error" role="alert">
    <span class="min-w-0 break-words [overflow-wrap:anywhere]">{i18n.t('skills.loadFailed')} {error}</span>
    <Button variant="outline" size="sm" onclick={load}>{i18n.t('configuration.retry')}</Button>
  </div>
{/if}
<div class="search-toolbar">
  <input aria-label={i18n.t('skills.search')} placeholder={i18n.t('skills.searchPlaceholder')} bind:value={query}>
</div>
<DataTable label={i18n.t('skills.title')} total={loading || error ? 0 : filtered.length} bind:page {pageSize}>
  <thead>
    <tr>
      <th>{i18n.t('common.name')}</th>
      <th>{i18n.t('configuration.id')}</th>
      <th>{i18n.t('common.type')}</th>
      <th>{i18n.t('common.description')}</th>
      <th>{i18n.t('configuration.source')}</th>
      <th class="th-actions">{i18n.t('common.actions')}</th>
    </tr>
  </thead>
  <tbody>
    {#if loading}
      <EmptyTableRow colspan={6} message={i18n.t('common.loadingConfiguration')} />
    {:else if error}
      <EmptyTableRow colspan={6} message={i18n.t('skills.loadFailed')} />
    {:else if !filtered.length}
      <EmptyTableRow
        colspan={6}
        message={i18n.t(query.trim() ? 'common.noMatches' : projectId ? 'scope.empty' : 'skills.empty')}
      />
    {:else}
      {#each rows as { entry: skill, projectId: rowProjectId } (`${rowProjectId ?? 'global'}:${skill.id}`)}
        <tr>
          <td class="model-name">
            <a
              class="inline-block max-w-56 break-words [overflow-wrap:anywhere] hover:text-[var(--accent-primary)] hover:underline"
              href={href(skill.id, rowProjectId)}
              >{skill.name}</a
            >
          </td>
          <td><span class="block max-w-64 break-words [overflow-wrap:anywhere]">{skill.id}</span></td>
          <td>
            <span
              class="inline-flex border border-[var(--border-default)] px-2 py-0.5 text-[10px] text-[var(--text-secondary)]"
              >{i18n.t(skill.kind === 'local' ? 'skills.local' : 'skills.remote')}</span
            >
          </td>
          <td>
            <span
              class="block max-w-md whitespace-pre-wrap break-words text-[var(--text-secondary)] [overflow-wrap:anywhere]"
              >{skill.description || '—'}</span
            >
          </td>
          <td>
            {#if projectId}
              <span
                class="mb-1 inline-block border border-[var(--border-default)] px-1.5 py-0.5 text-[10px] text-[var(--text-muted)]"
                >{i18n.t(rowProjectId ? 'scope.project' : 'scope.global')}</span
              >
            {/if}
            <span class="block max-w-64 break-words text-[var(--text-secondary)] [overflow-wrap:anywhere]"
              >{skill.source}</span
            >
          </td>
          <td class="row-actions">
            <Button
              href={href(skill.id, rowProjectId)}
              variant="ghost"
              size="icon-sm"
              aria-label={i18n.t('configuration.viewAria', { name: skill.name })}
              ><Eye size={14} /></Button
            >
            {#if projectId === null && skill.kind === 'local'}
              <Button
                href={`/skills/${encodeURIComponent(skill.id)}?edit=1`}
                variant="ghost"
                size="icon-sm"
                aria-label={i18n.t('configuration.editAria', { name: skill.name })}
                ><Pencil size={14} /></Button
              >
            {/if}
          </td>
        </tr>
      {/each}
    {/if}
  </tbody>
</DataTable>
