<script lang="ts">
import { onDestroy, untrack } from 'svelte';
import { beforeNavigate } from '$app/navigation';
import { Button } from '$src/components/button/index.js';
import { readError } from '$src/components/configuration/read-error.js';
import FormActions from '$src/components/FormActions.svelte';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import type { BehaviorDocument } from '$src/types/behavior.js';

let { active = true }: { active?: boolean } = $props();
const config = getConfig();
const i18n = getI18n();
let snapshot = $state<BehaviorDocument | null>(null);
let content = $state('');
let loading = $state(false);
let saving = $state(false);
let error = $state('');
let failedSave = $state(false);
let saved = $state(false);
let attempted = $state(false);
let mounted = true;
let request = 0;
const dirty = $derived(snapshot !== null && content !== snapshot.content);
const busy = $derived(loading || saving);

beforeNavigate(({ to, cancel, willUnload }) => {
  if (!dirty || to?.url.pathname === '/settings') return;
  if (willUnload || !window.confirm(i18n.t('settings.behaviorLeaveConfirm'))) cancel();
});
onDestroy(() => {
  mounted = false;
  request += 1;
});
$effect(() => {
  if (active)
    untrack(() => {
      if (!attempted) void load();
    });
});
async function load() {
  if (!active || busy) return;
  const seq = ++request;
  loading = true;
  attempted = true;
  error = '';
  failedSave = false;
  saved = false;
  try {
    const result = await config.getGlobalBehavior();
    if (!mounted || seq !== request) return;
    snapshot = result;
    content = result.content;
  } catch (cause) {
    if (mounted && seq === request) error = readError(cause);
  } finally {
    if (mounted && seq === request) loading = false;
  }
}
export async function refresh() {
  if (!active || busy) return;
  if (dirty && !window.confirm(i18n.t('settings.behaviorDiscardConfirm'))) return;
  await load();
}
async function save() {
  if (!active || busy || !snapshot) return;
  const initial = snapshot;
  const seq = ++request;
  saving = true;
  error = '';
  failedSave = true;
  saved = false;
  try {
    const result = await config.saveGlobalBehavior({
      content,
      expectedPath: initial.exists ? initial.resolvedPath : null,
      expectedContent: initial.exists ? initial.content : null,
    });
    if (!mounted || seq !== request) return;
    snapshot = result;
    content = result.content;
    saved = true;
  } catch (cause) {
    if (mounted && seq === request) error = readError(cause);
  } finally {
    if (mounted && seq === request) saving = false;
  }
}
</script>

<section class="max-w-4xl min-w-0" aria-labelledby="settings-behavior-title" aria-busy={busy}>
  <h2 id="settings-behavior-title" tabindex="-1" class="mt-0 mb-2 font-[var(--font-heading)] text-sm font-semibold">
    {i18n.t('settings.behaviorTitle')}
  </h2>
  <p class="field-hint mb-4">{i18n.t('settings.behaviorHint')}</p>
  {#if error}
    <div class="state-banner error" role="alert">
      <span class="min-w-0 break-words [overflow-wrap:anywhere]"
        >{i18n.t(failedSave ? 'settings.behaviorSaveFailed' : 'settings.behaviorLoadFailed')} {error}</span
      >
      {#if !snapshot}
        <Button variant="outline" size="sm" disabled={!active || busy} onclick={refresh}
          >{i18n.t('configuration.retry')}</Button
        >
      {/if}
    </div>
  {/if}
  {#if saved}
    <div class="state-banner success" role="status">{i18n.t('settings.behaviorSaved')}</div>
  {/if}
  {#if loading && !snapshot}
    <div class="state-banner" role="status">{i18n.t('settings.behaviorLoading')}</div>
  {/if}
  <form
    class="panel form-panel min-w-0"
    aria-label={i18n.t('settings.behaviorTitle')}
    onsubmit={(event) => { event.preventDefault(); void save(); }}
  >
    <div class="form-body form-grid">
      <div class="type-note min-w-0">
        <p class="m-0">{i18n.t('settings.behaviorScope')}</p>
        {#if snapshot}
          <p class="mt-2 mb-0 break-words [overflow-wrap:anywhere]">{snapshot.resolvedPath ?? snapshot.path}</p>
        {/if}
      </div>
      {#if snapshot && !snapshot.exists}
        <p class="form-hint" role="status">{i18n.t('settings.behaviorMissing')}</p>
      {/if}
      <div class="field full min-w-0">
        <label for="settings-behavior-content">{i18n.t('settings.behaviorContent')}</label>
        <textarea
          id="settings-behavior-content"
          name="global-behavior"
          class="!min-h-96 resize-y font-[var(--font-primary)] text-xs leading-6"
          bind:value={content}
          disabled={!active || busy || !snapshot}
          spellcheck="false"
          autocomplete="off"
          aria-describedby="settings-behavior-apply-hint"
        ></textarea>
        <small id="settings-behavior-apply-hint" class="field-hint">{i18n.t('settings.behaviorNextRequest')}</small>
      </div>
    </div>
    <FormActions>
      <Button variant="outline" disabled={!active || busy} onclick={refresh}
        >{i18n.t('settings.behaviorReload')}</Button
      >
      <Button variant="outline" disabled={!active || busy || !dirty} onclick={refresh}
        >{i18n.t('settings.behaviorDiscard')}</Button
      >
      <Button type="submit" disabled={!active || busy || !snapshot || snapshot.exists && !dirty}
        >{i18n.t(saving ? 'settings.behaviorSaving' : 'settings.behaviorSave')}</Button
      >
    </FormActions>
  </form>
</section>
