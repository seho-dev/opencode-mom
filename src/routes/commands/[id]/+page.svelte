<script lang="ts">
import { ArrowLeft, Pencil, RefreshCw, Trash2 } from '@lucide/svelte';
import { onDestroy, untrack } from 'svelte';
import { goto } from '$app/navigation';
import { page } from '$app/state';
import { Button } from '$src/components/button/index.js';
import { isNotFound, readError } from '$src/components/configuration/read-error.js';
import * as Dialog from '$src/components/dialog/index.js';
import FormPage from '$src/components/FormPage.svelte';
import PageHead from '$src/components/PageHead.svelte';
import ResourceScopeNotice from '$src/components/ResourceScopeNotice.svelte';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import type { SlashCommandEntry } from '$src/types/commands.js';
import CommandForm from '../CommandForm.svelte';

const config = getConfig();
const i18n = getI18n();
const projectId = $derived(page.url.searchParams.get('project'));
const readonly = $derived(page.url.searchParams.get('readonly') === '1');
const identity = $derived(JSON.stringify([projectId, page.params.id ?? '', readonly]));
let data = $state<SlashCommandEntry | null>(null);
let loadedIdentity = $state('');
const command = $derived(loadedIdentity === identity ? data : null);
let loading = $state(true);
let error = $state('');
let missing = $state(false);
let editing = $state(false);
let saved = $state(false);
let deleting = $state(false);
let deleteTarget = $state<{ entry: SlashCommandEntry; scope: string | null; identity: string } | null>(null);
let deleteError = $state('');
let request = 0;
let deleteRequest = 0;
let mounted = true;

async function load(edit = false) {
  const seq = ++request;
  const id = page.params.id ?? '';
  const scope = projectId;
  const key = identity;
  loading = true;
  error = '';
  missing = false;
  editing = false;
  saved = false;
  data = null;
  try {
    const result = await config.getSlashCommand(id, scope);
    if (!mounted || seq !== request || key !== identity) return;
    data = result;
    loadedIdentity = key;
    editing = edit && !readonly;
  } catch (cause) {
    if (mounted && seq === request) {
      error = readError(cause);
      missing = isNotFound(cause);
    }
  } finally {
    if (mounted && seq === request) loading = false;
  }
}
$effect(() => {
  void identity;
  const edit = page.url.searchParams.get('edit') === '1';
  deleteTarget = null;
  deleteError = '';
  deleting = false;
  untrack(() => {
    void load(edit);
  });
  return () => {
    request += 1;
    deleteRequest += 1;
  };
});
onDestroy(() => {
  mounted = false;
  request += 1;
  deleteRequest += 1;
});
async function remove() {
  if (!deleteTarget || readonly || deleting) return;
  const target = deleteTarget;
  const seq = ++deleteRequest;
  deleting = true;
  deleteError = '';
  try {
    await config.deleteSlashCommand(
      {
        id: target.entry.id,
        expectedPath: target.entry.resolvedPath,
        expectedContent: target.entry.content,
      },
      target.scope,
    );
    if (!mounted || seq !== deleteRequest || target.identity !== identity) return;
    deleteTarget = null;
    await goto('/commands');
  } catch (cause) {
    if (mounted && seq === deleteRequest && target.identity === identity) deleteError = readError(cause);
  } finally {
    if (mounted && seq === deleteRequest) deleting = false;
  }
}
</script>

<svelte:head><title>{i18n.t('commands.detailMetaTitle')}</title></svelte:head>
<FormPage active={editing && !readonly}>
  <div class="[&_.page-head]:flex-wrap [&_.page-head>div]:min-w-0 [&_h1]:[overflow-wrap:anywhere]">
    <PageHead
      eyebrow={i18n.t('commands.eyebrow')}
      title={command ? `/${command.name}` : i18n.t('commands.detailTitle')}
    >
      {#if !editing}
        <div class="flex flex-wrap justify-end gap-2">
          <Button href="/commands" variant="outline" size="sm"
            ><ArrowLeft size={14} />{i18n.t('commands.title')}</Button
          >
          <Button
            variant="outline"
            size="icon-sm"
            disabled={loading || deleting}
            aria-label={i18n.t('configuration.refresh')}
            onclick={() => load()}
            ><RefreshCw size={14} class={loading ? 'animate-spin motion-reduce:animate-none' : ''} /></Button
          >
          {#if command && !readonly}
            <Button variant="outline" size="sm" disabled={deleting} onclick={() => { editing = true; saved = false; }}
              ><Pencil size={14} />{i18n.t('configuration.edit')}</Button
            >
            <Button
              variant="destructive"
              size="sm"
              disabled={deleting}
              onclick={() => { if (command) deleteTarget = { entry: { ...command }, scope: projectId, identity }; }}
              ><Trash2 size={14} />{i18n.t('common.delete')}</Button
            >
          {/if}
        </div>
      {/if}
    </PageHead>
  </div>
  {#if !editing}
    <ResourceScopeNotice {projectId} {readonly} writable />
  {/if}
  {#if !readonly && (saved || page.url.searchParams.get('saved') === '1')}
    <div class="state-banner success" role="status">{i18n.t('commands.saved')}</div>
  {/if}
  {#if loading}
    <div class="state-banner" role="status">{i18n.t('common.loadingConfiguration')}</div>
  {:else if error}
    <div class="state-banner error" role="alert">
      <span class="min-w-0 break-words [overflow-wrap:anywhere]"
        >{i18n.t(missing ? 'commands.notFound' : 'commands.loadFailed')} {error}</span
      ><Button variant="outline" size="sm" onclick={() => load()}>{i18n.t('configuration.retry')}</Button>
    </div>
  {:else if command}
    {#if editing && !readonly}
      {#key identity}
        <CommandForm
          source={command}
          {projectId}
          onCancel={() => { editing = false; }}
          onSaved={(result) => { data = result; editing = false; saved = true; }}
        />
      {/key}
    {:else}
      <section class="panel min-w-0 p-4 sm:p-5" aria-label={i18n.t('commands.detailTitle')}>
        <dl class="m-0 grid gap-x-8 gap-y-5 sm:grid-cols-2">
          <div class="min-w-0 sm:col-span-2">
            <dt class="text-xs text-[var(--text-muted)]">{i18n.t('common.description')}</dt>
            <dd class="mt-2 ml-0 whitespace-pre-wrap break-words [overflow-wrap:anywhere]">
              {command.description || '—'}
            </dd>
          </div>
          <div>
            <dt class="text-xs text-[var(--text-muted)]">{i18n.t('commands.agent')}</dt>
            <dd class="mt-2 ml-0 break-words [overflow-wrap:anywhere]">{command.agent || '—'}</dd>
          </div>
          <div>
            <dt class="text-xs text-[var(--text-muted)]">{i18n.t('common.model')}</dt>
            <dd class="mt-2 ml-0 break-words [overflow-wrap:anywhere]">{command.model || '—'}</dd>
          </div>
          <div>
            <dt class="text-xs text-[var(--text-muted)]">{i18n.t('commands.subagent')}</dt>
            <dd class="mt-2 ml-0">
              {command.subagent === undefined ? '—' : i18n.t(command.subagent ? 'common.enabled' : 'common.disabled')}
            </dd>
          </div>
          <div class="min-w-0 sm:col-span-2">
            <dt class="text-xs text-[var(--text-muted)]">{i18n.t('configuration.source')}</dt>
            <dd class="mt-2 ml-0 break-words [overflow-wrap:anywhere]">{command.sourcePath}</dd>
          </div>
          <div class="min-w-0 sm:col-span-2">
            <dt class="text-xs text-[var(--text-muted)]">{i18n.t('configuration.path')}</dt>
            <dd class="mt-2 ml-0 break-words [overflow-wrap:anywhere]">{command.resolvedPath}</dd>
          </div>
        </dl>
      </section>
      <section class="panel mt-4 min-w-0 p-4 sm:p-5" aria-labelledby="command-content-title">
        <h2 id="command-content-title" class="mt-0 mb-2 font-[var(--font-heading)] text-sm">
          {i18n.t('commands.content')}
        </h2>
        <p class="field-hint mb-5">{i18n.t('commands.previewHint')}</p>
        <pre
          class="m-0 whitespace-pre-wrap break-words font-[var(--font-primary)] text-xs leading-6 [overflow-wrap:anywhere]"
        >{command.content || i18n.t('commands.emptyContent')}</pre>
      </section>
    {/if}
  {/if}
</FormPage>
<Dialog.Root
  open={deleteTarget !== null}
  onOpenChange={(open) => { if (!open && !deleting) { deleteTarget = null; deleteError = ''; } }}
>
  <Dialog.Content
    variant="destructive"
    aria-busy={deleting}
    onEscapeKeydown={(event) => { if (deleting) event.preventDefault(); }}
    onInteractOutside={(event) => { if (deleting) event.preventDefault(); }}
  >
    <Dialog.Header
      ><Dialog.Title>{i18n.t('commands.deleteTitle')}</Dialog.Title
      ><Dialog.Description
        >{i18n.t('commands.deleteConfirm', { name: deleteTarget?.entry.name ?? '' })}</Dialog.Description
      ></Dialog.Header
    >
    <p class="m-0 break-words text-xs text-[var(--text-secondary)] [overflow-wrap:anywhere]">
      {deleteTarget?.entry.sourcePath}
    </p>
    {#if deleteError}
      <p class="field-error break-words [overflow-wrap:anywhere]" role="alert">{deleteError}</p>
    {/if}
    <Dialog.Footer
      ><Button variant="outline" disabled={deleting} onclick={() => { deleteTarget = null; deleteError = ''; }}
        >{i18n.t('common.cancel')}</Button
      ><Button variant="destructive" disabled={deleting || readonly} onclick={remove}
        >{i18n.t(deleting ? 'configuration.deleting' : 'common.delete')}</Button
      ></Dialog.Footer
    >
  </Dialog.Content>
</Dialog.Root>
