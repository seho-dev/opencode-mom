<script lang="ts">
import { onDestroy, untrack } from 'svelte';
import { goto } from '$app/navigation';
import { Button } from '$src/components/button/index.js';
import { readError } from '$src/components/configuration/read-error.js';
import FormActions from '$src/components/FormActions.svelte';
import PromptGenerator from '$src/components/PromptGenerator.svelte';
import ResourceScopeNotice from '$src/components/ResourceScopeNotice.svelte';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import type { SlashCommandEntry } from '$src/types/commands.js';

let {
  source,
  projectId = null,
  readonly = false,
  onSaved,
  onCancel,
}: {
  source?: SlashCommandEntry;
  projectId?: string | null;
  readonly?: boolean;
  onSaved?: (command: SlashCommandEntry) => void;
  onCancel?: () => void;
} = $props();
const config = getConfig();
const i18n = getI18n();
const initial = untrack(() => (source ? { ...source } : undefined));
const scope = untrack(() => projectId);
let name = $state(initial?.name ?? '');
let content = $state(initial?.content ?? '');
let pending = $state(false);
let error = $state('');
let active = true;
onDestroy(() => {
  active = false;
});

async function save() {
  if (pending || readonly) return;
  error = '';
  if (!name.trim()) {
    error = i18n.t('commands.nameRequired');
    return;
  }
  pending = true;
  try {
    const result = initial
      ? await config.updateSlashCommand(
          {
            id: initial.id,
            content,
            expectedPath: initial.resolvedPath,
            expectedContent: initial.content,
          },
          scope,
        )
      : await config.createSlashCommand({ name: name.trim(), content }, scope);
    if (!active) return;
    if (onSaved) onSaved(result);
    else {
      const params = new URLSearchParams({ saved: '1' });
      if (scope !== null) params.set('project', scope);
      await goto(`/commands/${encodeURIComponent(result.id)}?${params}`);
    }
  } catch (cause) {
    if (active) error = readError(cause);
  } finally {
    if (active) pending = false;
  }
}
</script>

<form
  class="panel form-panel min-w-0"
  aria-label={i18n.t(initial ? 'commands.editTitle' : 'commands.newTitle')}
  aria-busy={pending}
  onsubmit={(event) => { event.preventDefault(); void save(); }}
>
  <div class="form-body form-grid">
    <div class="col-span-full"><ResourceScopeNotice projectId={scope} {readonly} writable /></div>
    {#if error}
      <p class="field-error col-span-full break-words [overflow-wrap:anywhere]" role="alert">
        {i18n.t('configuration.draftKept')} {error}
      </p>
    {/if}
    <div class="field full">
      <label for="command-name">{i18n.t('commands.name')}</label>
      <input
        id="command-name"
        name="command-name"
        bind:value={name}
        disabled={!!initial || pending || readonly}
        required
        autocomplete="off"
        aria-describedby="command-name-hint"
      >
      <small id="command-name-hint" class="field-hint"
        >{i18n.t(initial ? 'commands.nameImmutable' : 'commands.nameHint')}</small
      >
    </div>
    {#if initial}
      <p class="type-note m-0 break-words [overflow-wrap:anywhere]">{initial.sourcePath}</p>
    {/if}
    <div class="field full min-w-0">
      <label for="command-document">{i18n.t('commands.content')}</label>
      <textarea
        id="command-document"
        name="command-document"
        class="!min-h-96 resize-y font-[var(--font-primary)] text-xs leading-6"
        bind:value={content}
        disabled={pending || readonly}
        spellcheck="false"
        autocomplete="off"
        aria-describedby="command-document-hint"
      ></textarea>
      <small id="command-document-hint" class="field-hint"
        >{i18n.t('commands.contentHint')} <code>$ARGUMENTS</code> · <code>$1</code> · <code>$2</code></small
      >
      <PromptGenerator
        kind="command"
        value={content}
        disabled={pending || readonly}
        onapply={(text) => (content = text)}
      />
    </div>
  </div>
  <FormActions>
    {#if onCancel}
      <Button variant="outline" disabled={pending} onclick={onCancel}>{i18n.t('common.cancel')}</Button>
    {:else}
      <Button href="/commands" variant="outline" disabled={pending}>{i18n.t('common.cancel')}</Button>
    {/if}
    <Button type="submit" disabled={pending || readonly}
      >{i18n.t(pending ? 'configuration.saving' : 'configuration.save')}</Button
    >
  </FormActions>
</form>
