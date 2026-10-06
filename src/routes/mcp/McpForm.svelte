<script lang="ts">
import { onDestroy, untrack } from 'svelte';
import { goto } from '$app/navigation';
import { Button } from '$src/components/button/index.js';
import { readError } from '$src/components/configuration/read-error.js';
import FormActions from '$src/components/FormActions.svelte';
import Select from '$src/components/Select.svelte';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import type { McpServer } from '$src/types/mcp.js';

let {
  source,
  onSaved,
  onCancel,
}: { source?: McpServer; onSaved?: (server: McpServer) => void; onCancel?: () => void } = $props();
const config = getConfig();
const i18n = getI18n();
const initial = untrack(() =>
  source ? { ...source, config: JSON.parse(JSON.stringify(source.config)) as Record<string, unknown> } : undefined,
);
let name = $state(initial?.name ?? '');
let text = $state(
  JSON.stringify(initial?.config ?? { type: 'local', command: [''], environment: {}, disabled: false }, null, 2),
);
let pending = $state(false);
let error = $state('');
let syntaxError = $state('');
let active = true;
const kind = $derived.by(() => {
  try {
    return JSON.parse(text)?.type ?? '';
  } catch {
    return '';
  }
});
onDestroy(() => {
  active = false;
});

function parse(): Record<string, unknown> | null {
  try {
    const value = JSON.parse(text);
    if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error();
    syntaxError = '';
    return value;
  } catch {
    syntaxError = i18n.t('mcp.jsonError');
    return null;
  }
}
function selectType(type: string) {
  const value = parse();
  if (!value) return;
  value.type = type;
  if (type === 'local' && !('command' in value)) {
    value.command = [''];
    value.environment ??= {};
  }
  if (type === 'remote' && !('url' in value)) {
    value.url = '';
    value.headers ??= {};
  }
  text = JSON.stringify(value, null, 2);
}
async function save() {
  if (pending) return;
  error = '';
  const value = parse();
  if (!value) return;
  if (!name.trim()) {
    error = i18n.t('mcp.nameRequired');
    return;
  }
  if (value.type !== 'local' && value.type !== 'remote') {
    syntaxError = i18n.t('mcp.typeRequired');
    return;
  }
  if (
    value.type === 'local' &&
    (!Array.isArray(value.command) ||
      !value.command.length ||
      value.command.some((arg) => typeof arg !== 'string') ||
      !value.command[0].trim())
  ) {
    syntaxError = i18n.t('mcp.commandRequired');
    return;
  }
  if (value.type === 'remote' && (typeof value.url !== 'string' || !value.url.trim())) {
    syntaxError = i18n.t('mcp.urlRequired');
    return;
  }
  pending = true;
  try {
    const draft = { name: initial?.name ?? name.trim(), config: value };
    const result = initial
      ? await config.updateMcp({ ...draft, expectedConfig: initial.config, expectedSourcePath: initial.sourcePath })
      : await config.createMcp(draft);
    if (!active) return;
    if (onSaved) onSaved(result);
    else await goto(`/mcp/${encodeURIComponent(result.name)}?saved=1`);
  } catch (cause) {
    if (active) error = readError(cause);
  } finally {
    if (active) pending = false;
  }
}
</script>

<form
  class="panel form-panel min-w-0"
  aria-label={i18n.t(initial ? 'mcp.editTitle' : 'mcp.newTitle')}
  aria-busy={pending}
  onsubmit={(event) => { event.preventDefault(); void save(); }}
>
  {#if syntaxError || error}
    <p id="mcp-config-error" class="field-error mb-4 shrink-0 break-words [overflow-wrap:anywhere]" role="alert">
      {syntaxError || `${i18n.t('configuration.draftKept')} ${error}`}
    </p>
  {/if}
  <div class="form-grid">
    <div class="field">
      <label for="mcp-name">{i18n.t('common.name')}</label>
      <input
        id="mcp-name"
        bind:value={name}
        disabled={!!initial || pending}
        required
        autocomplete="off"
        aria-describedby="mcp-name-hint"
      >
      <small id="mcp-name-hint" class="muted">{i18n.t('mcp.nameHint')}</small>
    </div>
    <div class="field">
      <label for="mcp-type">{i18n.t('common.type')}</label>
      <Select
        id="mcp-type"
        value={kind}
        disabled={pending}
        onchange={selectType}
        options={[{ value: 'local', label: i18n.t('mcp.local') }, { value: 'remote', label: i18n.t('mcp.remote') }]}
      />
      <small class="muted">{i18n.t('mcp.typeHint')}</small>
    </div>
    {#if initial}
      <div class="type-note !border-l-[var(--status-warning)] break-words [overflow-wrap:anywhere]">
        <p class="m-0 text-[var(--status-warning)]">{i18n.t('mcp.editCredentials')}</p>
        <p class="mt-2 mb-0">{i18n.t('configuration.source')}: {initial.sourcePath}</p>
      </div>
    {/if}
    <div class="field full min-w-0">
      <label for="mcp-config">{i18n.t('mcp.jsonLabel')}</label>
      <textarea
        id="mcp-config"
        class="!min-h-72 resize-y font-[var(--font-primary)] text-xs leading-6"
        bind:value={text}
        disabled={pending}
        spellcheck="false"
        autocomplete="off"
        aria-invalid={!!syntaxError}
        aria-describedby={syntaxError ? 'mcp-config-hint mcp-config-error' : 'mcp-config-hint'}
      ></textarea>
      <small id="mcp-config-hint" class="muted leading-5">{i18n.t('mcp.jsonHint')}</small>
      <small class="muted leading-5">{i18n.t('mcp.v2ConfigHint')}</small>
    </div>
    <p class="type-note m-0">{i18n.t('configuration.manualReload')}</p>
  </div>
  <FormActions>
    {#if onCancel}
      <Button variant="outline" disabled={pending} onclick={onCancel}>{i18n.t('common.cancel')}</Button>
    {:else}
      <Button href="/mcp" variant="outline" disabled={pending}>{i18n.t('common.cancel')}</Button>
    {/if}
    <Button type="submit" disabled={pending}>{i18n.t(pending ? 'configuration.saving' : 'configuration.save')}</Button>
  </FormActions>
</form>
