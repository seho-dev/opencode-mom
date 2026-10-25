<script lang="ts">
import { ArrowLeft, RefreshCw } from '@lucide/svelte';
import { page } from '$app/state';
import { Button } from '$src/components/button/index.js';
import { isNotFound, readError } from '$src/components/configuration/read-error.js';
import FormPage from '$src/components/FormPage.svelte';
import PageHead from '$src/components/PageHead.svelte';
import ResourceScopeNotice from '$src/components/ResourceScopeNotice.svelte';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import type { AgentDefinition } from '$src/types/agents.js';
import { prettyJson } from '$src/utils/index.js';

const config = getConfig();
const i18n = getI18n();
let data = $state<AgentDefinition | null>(null);
let loadedId = $state('');
let loading = $state(true);
let error = $state('');
let missing = $state(false);
let request = 0;
const projectId = $derived(page.url.searchParams.get('project'));
const identity = $derived(`${projectId ?? 'global'}:${page.params.id ?? ''}`);
const agent = $derived(loadedId === identity ? data : null);

async function load() {
  const seq = ++request;
  const id = page.params.id ?? '';
  const scope = projectId;
  const key = `${scope ?? 'global'}:${id}`;
  loading = true;
  data = null;
  error = '';
  missing = false;
  try {
    const result = await (scope === null ? config.getAgent(id) : config.getAgent(id, scope));
    if (seq !== request) return;
    data = result;
    loadedId = key;
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
  void load();
  return () => {
    request += 1;
  };
});
function text(value: unknown) {
  return typeof value === 'string' ? value : value == null ? '—' : prettyJson(value);
}
</script>

<svelte:head><title>{i18n.t('agents.detailMetaTitle')}</title></svelte:head>
<FormPage active={false}>
  <div class="[&_.page-head]:flex-wrap [&_.page-head>div]:min-w-0 [&_h1]:[overflow-wrap:anywhere]">
    <PageHead eyebrow={i18n.t('agents.eyebrow')} title={agent?.id ?? i18n.t('agents.detailTitle')}>
      <div class="flex flex-wrap justify-end gap-2">
        <Button href="/agents" variant="outline" size="sm"><ArrowLeft size={14} />{i18n.t('agents.title')}</Button>
        <Button
          variant="outline"
          size="sm"
          disabled={loading}
          aria-busy={loading}
          onclick={load}
          aria-label={i18n.t('configuration.refresh')}
          ><RefreshCw
            size={14}
            class={loading ? 'animate-spin motion-reduce:animate-none' : ''}
            aria-hidden="true"
          /></Button
        >
      </div>
    </PageHead>
  </div>
  <ResourceScopeNotice {projectId} readonly />
  {#if loading}
    <div class="state-banner" role="status">{i18n.t('common.loadingConfiguration')}</div>
  {:else if error}
    <div class="state-banner error" role="alert">
      <span class="min-w-0 break-words [overflow-wrap:anywhere]"
        >{i18n.t(missing ? projectId !== null ? 'scope.notFound' : 'agents.notFound' : 'agents.loadFailed')}
        {error}</span
      ><Button variant="outline" size="sm" onclick={load}>{i18n.t('configuration.retry')}</Button>
    </div>
  {:else if agent}
    <section class="panel min-w-0 p-4 sm:p-5" aria-label={i18n.t('agents.detailTitle')}>
      <dl class="m-0 grid gap-x-8 gap-y-5 sm:grid-cols-2">
        <div class="min-w-0">
          <dt class="text-xs text-[var(--text-muted)]">{i18n.t('common.mode')}</dt>
          <dd class="ml-0 mt-2 break-words">{agent.mode ?? i18n.t('common.unset')}</dd>
        </div>
        <div class="min-w-0">
          <dt class="text-xs text-[var(--text-muted)]">{i18n.t('common.model')}</dt>
          <dd class="ml-0 mt-2 break-words [overflow-wrap:anywhere]">{agent.modelRef ?? i18n.t('common.inherited')}</dd>
        </div>
        <div class="min-w-0 sm:col-span-2">
          <dt class="text-xs text-[var(--text-muted)]">{i18n.t('common.description')}</dt>
          <dd class="ml-0 mt-2 whitespace-pre-wrap break-words [overflow-wrap:anywhere]">{agent.description || '—'}</dd>
        </div>
      </dl>
    </section>
    {#each agent.sources ?? [] as source, index (`${source.path ?? source.storage}:${index}`)}
      <section class="panel mt-4 min-w-0 p-4 sm:p-5" aria-label={i18n.t('agents.sourcePreview')}>
        <h2 class="m-0 font-[var(--font-heading)] text-base">
          {i18n.t('agents.sourcePreview')}
          · {i18n.t(source.storage === 'inline' ? 'agents.sourceInline' : 'agents.markdown')}
        </h2>
        <p class="mb-4 mt-2 break-words text-xs text-[var(--text-muted)] [overflow-wrap:anywhere]">
          {source.path ?? i18n.t('agents.pathNotProvided')}
        </p>
        <!-- biome-ignore-start lint/a11y/noNoninteractiveTabindex lint/a11y/useSemanticElements: Named scrollable previews retain preformatted content and keyboard access. -->
        <!-- svelte-ignore a11y_no_noninteractive_tabindex (Scrollable previews need keyboard access.) -->
        <pre
          tabindex="0"
          role="region"
          aria-label={`${i18n.t('agents.sourcePreview')}: ${source.path ?? agent.id}`}
          class="m-0 max-h-96 overflow-y-auto whitespace-pre-wrap break-words font-[var(--font-primary)] text-xs leading-6 [overflow-wrap:anywhere]"
        >{text(source.raw ?? source.fields)}</pre>
        <!-- biome-ignore-end lint/a11y/noNoninteractiveTabindex lint/a11y/useSemanticElements: Named scrollable previews retain preformatted content and keyboard access. -->
        {#if source.prompt && typeof source.raw !== 'string'}
          <h3 class="mb-2 mt-5 text-sm">{i18n.t('agents.prompt')}</h3>
          <!-- biome-ignore-start lint/a11y/noNoninteractiveTabindex lint/a11y/useSemanticElements: Named scrollable previews retain preformatted content and keyboard access. -->
          <!-- svelte-ignore a11y_no_noninteractive_tabindex (Scrollable previews need keyboard access.) -->
          <pre
            tabindex="0"
            role="region"
            aria-label={`${i18n.t('agents.prompt')}: ${source.path ?? agent.id}`}
            class="m-0 max-h-96 overflow-y-auto whitespace-pre-wrap break-words font-[var(--font-primary)] text-xs leading-6 [overflow-wrap:anywhere]"
          >{source.prompt}</pre>
        <!-- biome-ignore-end lint/a11y/noNoninteractiveTabindex lint/a11y/useSemanticElements: Named scrollable previews retain preformatted content and keyboard access. -->
        {/if}
      </section>
    {:else}
      <section class="panel mt-4 min-w-0 p-4 sm:p-5" aria-label={i18n.t('agents.sourcePreview')}>
        <h2 class="mb-3 mt-0 font-[var(--font-heading)] text-base">{i18n.t('agents.sourcePreview')}</h2>
        <!-- biome-ignore-start lint/a11y/noNoninteractiveTabindex lint/a11y/useSemanticElements: Named scrollable previews retain preformatted content and keyboard access. -->
        <!-- svelte-ignore a11y_no_noninteractive_tabindex (Scrollable previews need keyboard access.) -->
        <pre
          tabindex="0"
          role="region"
          aria-label={`${i18n.t('agents.sourcePreview')}: ${agent.id}`}
          class="m-0 max-h-96 overflow-y-auto whitespace-pre-wrap break-words font-[var(--font-primary)] text-xs leading-6 [overflow-wrap:anywhere]"
        >{text(agent.inline ?? agent.markdown ?? agent)}</pre>
        <!-- biome-ignore-end lint/a11y/noNoninteractiveTabindex lint/a11y/useSemanticElements: Named scrollable previews retain preformatted content and keyboard access. -->
      </section>
    {/each}
  {/if}
</FormPage>
