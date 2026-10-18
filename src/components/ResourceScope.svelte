<script lang="ts">
import { FolderPlus, Globe, Settings2 } from '@lucide/svelte';
import { Button } from '$src/components/button/index.js';
import { readError } from '$src/components/configuration/read-error.js';
import * as Dialog from '$src/components/dialog/index.js';
import Select from '$src/components/Select.svelte';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';

let { disabled = false, showGlobal = $bindable(false) }: { disabled?: boolean; showGlobal?: boolean } = $props();
const config = getConfig();
const i18n = getI18n();
let managing = $state(false);
let pending = $state(false);
let error = $state('');
let picker = $state('global');
const projects = $derived(config.projects ?? []);
const selectedId = $derived(config.selectedProjectId ?? null);
const selected = $derived(projects.find((project) => project.id === selectedId));
const busy = $derived(disabled || pending || config.saving || config.projectSaving);
const options = $derived([
  { value: 'global', label: i18n.t('scope.global'), globe: true },
  ...projects.map((project) => ({ value: project.id, label: basename(project.path), hint: project.path })),
]);
$effect(() => {
  picker = selectedId ?? 'global';
});

function basename(path: string) {
  return (
    path
      .replace(/[\\/]+$/, '')
      .split(/[\\/]/)
      .pop() || path
  );
}
async function select(value: string) {
  if (busy) return;
  picker = selectedId ?? 'global';
  pending = true;
  error = '';
  try {
    await config.selectProject(value === 'global' ? null : value);
  } catch (cause) {
    error = readError(cause);
  } finally {
    picker = config.selectedProjectId ?? 'global';
    pending = false;
  }
}
async function add() {
  if (busy) return;
  pending = true;
  error = '';
  try {
    const path = await config.pickProjectFolder();
    if (path !== null) await config.addProject(path);
  } catch (cause) {
    error = readError(cause);
  } finally {
    pending = false;
  }
}
async function remove(id: string) {
  if (busy) return;
  pending = true;
  error = '';
  try {
    await config.removeProject(id);
  } catch (cause) {
    error = readError(cause);
  } finally {
    pending = false;
  }
}
</script>

<section
  class="mb-4 min-w-0 border border-[var(--border-default)] bg-[var(--surface-panel)] p-3"
  aria-label={i18n.t('scope.label')}
>
  <div class="flex flex-wrap items-center gap-2">
    <span class="mr-1 text-xs text-[var(--text-muted)]">{i18n.t('scope.label')}</span>
    <div class="min-w-0 basis-52 grow sm:max-w-80">
      <Select
        bind:value={picker}
        {options}
        alwaysSearch
        stacked
        pinnedValue="global"
        ariaLabel={i18n.t('scope.select')}
        disabled={busy}
        onchange={select}
      />
    </div>
    <Button variant="outline" size="sm" disabled={busy} onclick={add}
      ><FolderPlus size={14} />{i18n.t('scope.add')}</Button
    >
    <Button variant="ghost" size="sm" disabled={busy} onclick={() => { error = ''; managing = true; }}
      ><Settings2 size={14} />{i18n.t('scope.manage')}</Button
    >
    {#if selectedId}
      <Button variant="outline" size="sm" disabled={busy} onclick={() => select('global')}
        ><Globe size={14} />{i18n.t('scope.returnGlobal')}</Button
      >
    {/if}
  </div>
  {#if selectedId}
    <div
      class="mt-3 flex min-w-0 flex-wrap items-start justify-between gap-3 border-t border-[var(--border-subtle)] pt-3 text-xs"
    >
      <div class="min-w-0 flex-1">
        <p class="m-0 break-words text-[var(--text-secondary)] [overflow-wrap:anywhere]">
          {selected?.path ?? selectedId}
        </p>
        <p class="mb-0 mt-1 text-[var(--text-muted)]">{i18n.t('scope.projectReadOnly')}</p>
      </div>
      <label class="flex cursor-pointer items-center gap-2 text-[var(--text-secondary)]">
        <input type="checkbox" bind:checked={showGlobal} disabled={busy} class="accent-[var(--accent-primary)]">
        {i18n.t('scope.showGlobal')}
      </label>
    </div>
  {/if}
  {#if error && !managing}
    <p class="mb-0 mt-3 break-words text-xs text-[var(--status-error)] [overflow-wrap:anywhere]" role="alert">
      {i18n.t('scope.operationFailed')} {error}
    </p>
  {/if}
</section>
<Dialog.Root open={managing} onOpenChange={(open) => { if (!pending) managing = open; }}>
  <Dialog.Content
    onEscapeKeydown={(event) => { if (pending) event.preventDefault(); }}
    onInteractOutside={(event) => { if (pending) event.preventDefault(); }}
  >
    <Dialog.Header>
      <Dialog.Title>{i18n.t('scope.manage')}</Dialog.Title>
      <Dialog.Description>{i18n.t('scope.removeHint')}</Dialog.Description>
    </Dialog.Header>
    <ul class="m-0 max-h-80 list-none space-y-3 overflow-y-auto p-0">
      {#each projects as project (project.id)}
        <li class="flex min-w-0 items-start justify-between gap-3 border-b border-[var(--border-subtle)] pb-3">
          <div class="min-w-0">
            <p class="m-0 text-sm">{basename(project.path)}</p>
            <p class="mb-0 mt-1 break-words text-xs text-[var(--text-muted)] [overflow-wrap:anywhere]">
              {project.path}
            </p>
          </div>
          <Button
            variant="outline"
            size="sm"
            disabled={busy}
            onclick={() => remove(project.id)}
            aria-label={i18n.t('scope.removeAria', { path: project.path })}
            >{i18n.t('common.remove')}</Button
          >
        </li>
      {:else}
        <li class="text-sm text-[var(--text-muted)]">{i18n.t('scope.noProjects')}</li>
      {/each}
    </ul>
    {#if error}
      <p class="m-0 break-words text-xs text-[var(--status-error)] [overflow-wrap:anywhere]" role="alert">
        {i18n.t('scope.operationFailed')} {error}
      </p>
    {/if}
    <Dialog.Footer
      ><Button variant="outline" disabled={pending} onclick={() => { managing = false; error = ''; }}
        >{i18n.t('common.close')}</Button
      ></Dialog.Footer
    >
  </Dialog.Content>
</Dialog.Root>
