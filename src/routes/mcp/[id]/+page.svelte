<script lang="ts">
import { ArrowLeft, Pencil, RefreshCw, Trash2 } from '@lucide/svelte';
import { goto } from '$app/navigation';
import { page } from '$app/state';
import { Button } from '$src/components/button/index.js';
import { isNotFound, readError } from '$src/components/configuration/read-error.js';
import PageHead from '$src/components/PageHead.svelte';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import type { McpServer } from '$src/types/mcp.js';
import { maskConfig, maskTarget } from '../config-view.js';
import DeleteMcpDialog from '../DeleteMcpDialog.svelte';
import McpForm from '../McpForm.svelte';

const config = getConfig();
const i18n = getI18n();
let data = $state<McpServer | null>(null);
let loadedId = $state('');
let loading = $state(true);
let error = $state('');
let missing = $state(false);
let deleting = $state<McpServer | null>(null);
let editing = $state(false);
let saved = $state(false);
let request = 0;
const server = $derived(loadedId === page.params.id ? data : null);
const fields = $derived(Object.entries((maskConfig(server?.config ?? {}) ?? {}) as Record<string, unknown>));

async function load(id: string, edit = false) {
  const seq = ++request;
  loading = true;
  data = null;
  deleting = null;
  editing = false;
  saved = false;
  error = '';
  missing = false;
  try {
    const result = await config.getMcp(id);
    if (seq !== request) return;
    data = result;
    loadedId = id;
    editing = edit;
  } catch (cause) {
    if (seq === request) {
      error = readError(cause);
      missing = isNotFound(cause);
    }
  } finally {
    if (seq === request) loading = false;
  }
}
$effect(() => {
  void load(page.params.id ?? '', page.url.searchParams.get('edit') === '1');
  return () => {
    request += 1;
  };
});
</script>

<svelte:head><title>{i18n.t('mcp.detailMetaTitle')}</title></svelte:head>
<div class="[&_.page-head]:flex-wrap [&_.page-head>div]:min-w-0 [&_h1]:[overflow-wrap:anywhere]">
  <PageHead eyebrow={i18n.t('mcp.eyebrow')} title={server?.name ?? i18n.t('mcp.detailTitle')}>
    {#if !editing}
      <div class="flex flex-wrap justify-end gap-2">
        <Button href="/mcp" variant="outline" size="sm"><ArrowLeft size={14} />{i18n.t('mcp.title')}</Button>
        <Button
          variant="outline"
          size="sm"
          disabled={loading}
          onclick={() => load(page.params.id ?? '')}
          aria-label={i18n.t('configuration.refresh')}
          ><RefreshCw size={14} /></Button
        >
        {#if server}
          <Button variant="outline" size="sm" onclick={() => { editing = true; saved = false; }}
            ><Pencil size={14} />{i18n.t('configuration.edit')}</Button
          >
          <Button variant="destructive" size="sm" onclick={() => (deleting = server)}
            ><Trash2 size={14} />{i18n.t('common.delete')}</Button
          >
        {/if}
      </div>
    {/if}
  </PageHead>
</div>
<p class="page-description">{i18n.t('mcp.description')}</p>
{#if saved || page.url.searchParams.get('saved') === '1'}
  <div class="state-banner success" role="status">{i18n.t('configuration.saved')}</div>
{/if}
{#if loading}
  <div class="state-banner" role="status">{i18n.t('common.loadingConfiguration')}</div>
{:else if error}
  <div class="state-banner error" role="alert">
    <span class="min-w-0 break-words [overflow-wrap:anywhere]"
      >{i18n.t(missing ? 'mcp.notFound' : 'mcp.loadFailed')} {error}</span
    >
    <Button variant="outline" size="sm" onclick={() => load(page.params.id ?? '')}
      >{i18n.t('configuration.retry')}</Button
    >
  </div>
{:else if server}
  {#if editing}
    {#key server.name}
      <McpForm
        source={server}
        onCancel={() => { editing = false; }}
        onSaved={(result) => { data = result; editing = false; saved = true; }}
      />
    {/key}
  {:else}
    <section class="panel min-w-0 p-4 sm:p-5" aria-label={i18n.t('mcp.detailTitle')}>
      <dl class="m-0 grid gap-x-8 gap-y-5 sm:grid-cols-2">
        <div class="min-w-0">
          <dt class="text-xs text-[var(--text-muted)]">{i18n.t('common.type')}</dt>
          <dd class="mt-2 ml-0">{i18n.t(server.type === 'local' ? 'mcp.local' : 'mcp.remote')}</dd>
        </div>
        <div>
          <dt class="text-xs text-[var(--text-muted)]">{i18n.t('mcp.configuration')}</dt>
          <dd class="mt-2 ml-0">{i18n.t(server.disabled ? 'common.disabled' : 'common.enabled')}</dd>
        </div>
        <div class="min-w-0 sm:col-span-2">
          <dt class="text-xs text-[var(--text-muted)]">{i18n.t('mcp.target')}</dt>
          <dd class="mt-2 ml-0 whitespace-pre-wrap break-words [overflow-wrap:anywhere]">
            {maskTarget(server) || '—'}
          </dd>
        </div>
        <div class="min-w-0 sm:col-span-2">
          <dt class="text-xs text-[var(--text-muted)]">{i18n.t('configuration.globalSources')}</dt>
          <dd class="mt-2 ml-0">
            <ul class="m-0 pl-4 space-y-1 break-words [overflow-wrap:anywhere]">
              {#each server.sourcePaths.length ? server.sourcePaths : [server.sourcePath] as path}
                <li>{path}</li>
              {/each}
            </ul>
          </dd>
        </div>
      </dl>
    </section>
    <section class="panel mt-4 min-w-0 p-4 sm:p-5" aria-labelledby="native-config-title">
      <h2 id="native-config-title" class="mt-0 mb-2 font-[var(--font-heading)] text-base">
        {i18n.t('mcp.nativeConfig')}
      </h2>
      <p class="mt-0 mb-5 text-xs text-[var(--text-secondary)]">{i18n.t('mcp.credentialHint')}</p>
      <dl class="m-0 divide-y divide-[var(--border-subtle)]">
        {#each fields as [key, value]}
          <div class="grid min-w-0 gap-2 py-3 first:pt-0 sm:grid-cols-[160px_minmax(0,1fr)]">
            <dt class="break-words text-xs text-[var(--text-muted)] [overflow-wrap:anywhere]">{key}</dt>
            <dd class="m-0 min-w-0">
              <pre
                class="m-0 whitespace-pre-wrap break-words font-[var(--font-primary)] text-xs leading-6 [overflow-wrap:anywhere]"
              >{typeof value === 'string' ? value : JSON.stringify(value, null, 2)}</pre>
            </dd>
          </div>
        {:else}
          <p class="muted">{i18n.t('mcp.noConfig')}</p>
        {/each}
      </dl>
    </section>
  {/if}
{/if}
<DeleteMcpDialog bind:server={deleting} onDeleted={() => goto('/mcp?removed=1')} />
