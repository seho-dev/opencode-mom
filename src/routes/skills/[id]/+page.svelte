<script lang="ts">
import { ArrowLeft, Pencil, RefreshCw } from '@lucide/svelte';
import { page } from '$app/state';
import { Button } from '$src/components/button/index.js';
import { isNotFound, readError } from '$src/components/configuration/read-error.js';
import PageHead from '$src/components/PageHead.svelte';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import type { SkillEntry } from '$src/types/skills.js';
import SkillForm from '../SkillForm.svelte';

const config = getConfig();
const i18n = getI18n();
let data = $state<SkillEntry | null>(null);
let loadedId = $state('');
let loading = $state(true);
let error = $state('');
let missing = $state(false);
let editing = $state(false);
let saved = $state(false);
let request = 0;
const skill = $derived(loadedId === page.params.id ? data : null);

async function load(id: string, edit = false) {
  const seq = ++request;
  loading = true;
  data = null;
  editing = false;
  saved = false;
  error = '';
  missing = false;
  try {
    const result = await config.getSkill(id);
    if (seq !== request) return;
    data = result;
    loadedId = id;
    editing = edit && result.kind === 'local';
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

<svelte:head><title>{i18n.t('skills.detailMetaTitle')}</title></svelte:head>
<div class="[&_.page-head]:flex-wrap [&_.page-head>div]:min-w-0 [&_h1]:[overflow-wrap:anywhere]">
  <PageHead eyebrow={i18n.t('skills.eyebrow')} title={skill?.name ?? i18n.t('skills.detailTitle')}>
    {#if !editing}
      <div class="flex flex-wrap justify-end gap-2">
        <Button href="/skills" variant="outline" size="sm"><ArrowLeft size={14} />{i18n.t('skills.title')}</Button>
        <Button
          variant="outline"
          size="sm"
          disabled={loading}
          aria-label={i18n.t('configuration.refresh')}
          onclick={() => load(page.params.id ?? '')}
          ><RefreshCw size={14} /></Button
        >
        {#if skill?.kind === 'local'}
          <Button variant="outline" size="sm" onclick={() => { editing = true; saved = false; }}
            ><Pencil size={14} />{i18n.t('configuration.edit')}</Button
          >
        {/if}
      </div>
    {/if}
  </PageHead>
</div>
{#if saved || page.url.searchParams.get('saved') === '1'}
  <div class="state-banner success" role="status">{i18n.t('configuration.saved')}</div>
{/if}
{#if loading}
  <div class="state-banner" role="status">{i18n.t('common.loadingConfiguration')}</div>
{:else if error}
  <div class="state-banner error" role="alert">
    <span class="min-w-0 break-words [overflow-wrap:anywhere]"
      >{i18n.t(missing ? 'skills.notFound' : 'skills.loadFailed')} {error}</span
    >
    <Button variant="outline" size="sm" onclick={() => load(page.params.id ?? '')}
      >{i18n.t('configuration.retry')}</Button
    >
  </div>
{:else if skill}
  {#if editing && skill.kind === 'local'}
    {#key skill.id}
      <SkillForm
        source={skill}
        onCancel={() => { editing = false; }}
        onSaved={(result) => { data = result; editing = false; saved = true; }}
      />
    {/key}
  {:else}
    {#if skill.kind === 'remote'}
      <div class="state-banner" role="status">{i18n.t('skills.remoteHint')}</div>
    {/if}
    <section class="panel min-w-0 p-4 sm:p-5" aria-label={i18n.t('skills.detailTitle')}>
      <dl class="m-0 grid gap-x-8 gap-y-5 sm:grid-cols-2">
        <div class="min-w-0">
          <dt class="text-xs text-[var(--text-muted)]">{i18n.t('configuration.id')}</dt>
          <dd class="mt-2 ml-0 break-words [overflow-wrap:anywhere]">{skill.id}</dd>
        </div>
        <div>
          <dt class="text-xs text-[var(--text-muted)]">{i18n.t('common.type')}</dt>
          <dd class="mt-2 ml-0">
            <span class="inline-flex border border-[var(--border-default)] px-2 py-0.5 text-[10px]"
              >{i18n.t(skill.kind === 'local' ? 'skills.local' : 'skills.remote')}</span
            >
          </dd>
        </div>
        <div>
          <dt class="text-xs text-[var(--text-muted)]">{i18n.t('skills.autoinvoke')}</dt>
          <dd class="mt-2 ml-0">{i18n.t(skill.autoinvoke ? 'common.enabled' : 'common.disabled')}</dd>
        </div>
        <div class="min-w-0 sm:col-span-2">
          <dt class="text-xs text-[var(--text-muted)]">{i18n.t('common.description')}</dt>
          <dd class="mt-2 ml-0 whitespace-pre-wrap break-words [overflow-wrap:anywhere]">{skill.description || '—'}</dd>
        </div>
        <div class="min-w-0">
          <dt class="text-xs text-[var(--text-muted)]">{i18n.t('configuration.source')}</dt>
          <dd class="mt-2 ml-0 break-words [overflow-wrap:anywhere]">{skill.source}</dd>
        </div>
        <div class="min-w-0">
          <dt class="text-xs text-[var(--text-muted)]">{i18n.t('configuration.path')}</dt>
          <dd class="mt-2 ml-0 break-words [overflow-wrap:anywhere]">{skill.path}</dd>
        </div>
      </dl>
    </section>
    <section class="panel mt-4 min-w-0 p-4 sm:p-5" aria-labelledby="skill-body-title">
      <div class="mb-5 flex flex-wrap items-center justify-between gap-2">
        <h2 id="skill-body-title" class="m-0 font-[var(--font-heading)] text-base">{i18n.t('skills.content')}</h2>
        <span class="text-xs text-[var(--text-muted)]">{i18n.t('skills.contentHint')}</span>
      </div>
      {#if skill.content.trim()}
        <pre
          class="m-0 whitespace-pre-wrap break-words font-[var(--font-primary)] text-xs leading-6 [overflow-wrap:anywhere]"
        >{skill.content}</pre>
      {:else}
        <p class="muted m-0">{i18n.t('skills.emptyContent')}</p>
      {/if}
    </section>
  {/if}
{/if}
