<script lang="ts">
import { onDestroy, untrack } from 'svelte';
import { goto } from '$app/navigation';
import { Button } from '$src/components/button/index.js';
import { readError } from '$src/components/configuration/read-error.js';
import FormActions from '$src/components/FormActions.svelte';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import type { SkillEntry } from '$src/types/skills.js';

let {
  source,
  onSaved,
  onCancel,
}: { source?: SkillEntry; onSaved?: (skill: SkillEntry) => void; onCancel?: () => void } = $props();
const config = getConfig();
const i18n = getI18n();
const initial = untrack(() => (source ? { ...source } : undefined));
let id = $state(initial?.id ?? '');
let content = $state(initial?.content ?? '---\nname: ""\ndescription: ""\n---\n\n');
let pending = $state(false);
let error = $state('');
let active = true;
onDestroy(() => {
  active = false;
});

async function save() {
  if (pending || (initial && initial.kind !== 'local')) return;
  error = '';
  if (!id.trim()) {
    error = i18n.t('skills.idRequired');
    return;
  }
  pending = true;
  try {
    const draft = { id: initial?.id ?? id.trim(), content };
    const result = initial
      ? await config.updateSkill({ ...draft, expectedContent: initial.content, expectedPath: initial.path })
      : await config.createSkill(draft);
    if (!active) return;
    if (onSaved) onSaved(result);
    else await goto(`/skills/${encodeURIComponent(result.id)}?saved=1`);
  } catch (cause) {
    if (active) error = readError(cause);
  } finally {
    if (active) pending = false;
  }
}
</script>

{#if initial && initial.kind !== 'local'}
  <p class="state-banner" role="status">{i18n.t('skills.remoteHint')}</p>
{:else}
  <form
    class="panel form-panel min-w-0"
    aria-label={i18n.t(initial ? 'skills.editTitle' : 'skills.newTitle')}
    aria-busy={pending}
    onsubmit={(event) => { event.preventDefault(); void save(); }}
  >
    {#if error}
      <p class="field-error mb-4 shrink-0 break-words [overflow-wrap:anywhere]" role="alert">
        {i18n.t('configuration.draftKept')} {error}
      </p>
    {/if}
    <div class="form-grid">
      <div class="field full">
        <label for="skill-id">{i18n.t('configuration.id')}</label>
        <input
          id="skill-id"
          bind:value={id}
          disabled={!!initial || pending}
          required
          autocomplete="off"
          aria-describedby="skill-id-hint"
        >
        <small id="skill-id-hint" class="muted leading-5"
          >{i18n.t(initial ? 'skills.idImmutable' : 'skills.idHint')}</small
        >
      </div>
      {#if initial}
        <div class="type-note break-words [overflow-wrap:anywhere]">
          <p class="m-0">{i18n.t('configuration.source')}: {initial.source}</p>
          <p class="mt-2 mb-0">{i18n.t('configuration.path')}: {initial.path}</p>
        </div>
      {/if}
      <div class="field full min-w-0">
        <label for="skill-document">{i18n.t('skills.documentLabel')}</label>
        <textarea
          id="skill-document"
          class="!min-h-80 resize-y font-[var(--font-primary)] text-xs leading-6"
          bind:value={content}
          disabled={pending}
          spellcheck="false"
          autocomplete="off"
          aria-describedby="skill-document-hint"
        ></textarea>
        <small id="skill-document-hint" class="muted leading-5">{i18n.t('skills.documentManagementHint')}</small>
      </div>
      <p class="type-note m-0">{i18n.t('configuration.manualReload')}</p>
    </div>
    <FormActions>
      {#if onCancel}
        <Button variant="outline" disabled={pending} onclick={onCancel}>{i18n.t('common.cancel')}</Button>
      {:else}
        <Button href="/skills" variant="outline" disabled={pending}>{i18n.t('common.cancel')}</Button>
      {/if}
      <Button type="submit" disabled={pending}
        >{i18n.t(pending ? 'configuration.saving' : 'configuration.save')}</Button
      >
    </FormActions>
  </form>
{/if}
