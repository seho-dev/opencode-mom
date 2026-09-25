<script lang="ts">
import { goto } from '$app/navigation';
import { page } from '$app/state';
import AgentFields from '$lib/components/app/AgentFields.svelte';
import FormActions from '$lib/components/app/FormActions.svelte';
import PageHead from '$lib/components/app/PageHead.svelte';
import Select from '$lib/components/app/Select.svelte';
import { toast } from '$lib/components/app/toast.svelte.js';
import { Button } from '$lib/components/ui/button/index.js';
import {
  type AgentFieldValues,
  buildModelOptions,
  buildModelVariants,
  joinModelSelector,
  permissionRules,
  splitModelSelector,
} from '$lib/features/config/agentForm.js';
import { isBuiltinAgentId } from '$lib/features/config/builtinAgents.js';
import { AGENT_MODES, STORAGE_LABELS } from '$lib/features/config/constants.js';
import { getConfig } from '$lib/features/config/context.js';
import type { AgentDefinition, AgentStorage, AgentWrite } from '$lib/features/config/types.js';
import { getI18n } from '$lib/features/i18n/context.js';

type SourceSnapshot = {
  storage: AgentStorage;
  path?: string;
  raw?: string;
  prompt?: string;
  fields?: Record<string, unknown>;
};
const config = getConfig();
const i18n = getI18n();
const routeId = $derived(decodeURIComponent(page.params.id ?? ''));
// Built-in catalog agents (native/slim/omo) are read-only; never expose their edit form.
const builtin = $derived(isBuiltinAgentId(routeId));
const agent = $derived(builtin ? undefined : config.agents.find((entry) => entry.id === routeId));
let initialized = $state('');
let seenReset = $state(-1);
let storage = $state<AgentStorage>('inline');
let values = $state<AgentFieldValues>({
  model: '',
  mode: '',
  description: '',
  disabled: false,
  hidden: false,
  color: '',
  variant: '',
  steps: '',
  prompt: '',
});
let permission = $state('[]');

const snapshots = (value: AgentDefinition | undefined) => {
  const raw = (value as (AgentDefinition & Record<string, unknown>) | undefined)?.['sources'];
  if (!Array.isArray(raw)) return [] as SourceSnapshot[];
  return raw
    .filter(
      (item): item is SourceSnapshot =>
        !!item && typeof item === 'object' && typeof (item as Record<string, unknown>)['storage'] === 'string',
    )
    .map((item) => {
      const record = item as Record<string, unknown>;
      return {
        storage: record['storage'] as AgentStorage,
        path: typeof record['path'] === 'string' ? record['path'] : undefined,
        raw: typeof record['raw'] === 'string' ? record['raw'] : undefined,
        prompt: typeof record['prompt'] === 'string' ? record['prompt'] : undefined,
        fields:
          record['fields'] && typeof record['fields'] === 'object' && !Array.isArray(record['fields'])
            ? (record['fields'] as Record<string, unknown>)
            : undefined,
      };
    });
};
const selectedSource = $derived(snapshots(agent).find((entry) => entry.storage === storage));
const knownMode = $derived((AGENT_MODES as readonly string[]).includes(values.mode));
const storeModels = $derived(config.models());
const modelVariants = $derived(buildModelVariants(config.catalogModels(), storeModels, values.model));
const modelOptions = $derived(
  buildModelOptions(config.catalogModels(), storeModels, values.model, i18n.t('agents.clearInherit')),
);
const variantUnavailable = $derived(!!values.variant && !modelVariants.includes(values.variant));
const variantHint = $derived(
  !values.model
    ? i18n.t('agents.variantSelectModel')
    : !config.catalogLoading && modelVariants.length === 0
      ? i18n.t('agents.variantNoVariants')
      : i18n.t('agents.variantFromModel'),
);
const promptHint = $derived(
  storage === 'inline' ? i18n.t('agents.promptHintInline') : i18n.t('agents.promptHintMarkdown'),
);
function loadSource() {
  const fields = selectedSource?.fields ?? {};
  const selector =
    typeof fields['model'] === 'string' ? splitModelSelector(fields['model']) : { model: '', variant: '' };
  values.model = selector.model;
  values.variant = selector.variant;
  values.mode = typeof fields['mode'] === 'string' ? fields['mode'] : '';
  values.description = typeof fields['description'] === 'string' ? fields['description'] : '';
  values.disabled = fields['disabled'] === true;
  values.hidden = fields['hidden'] === true;
  values.color = typeof fields['color'] === 'string' ? fields['color'] : '';
  values.steps = typeof fields['steps'] === 'number' ? String(fields['steps']) : '';
  values.prompt = selectedSource?.prompt ?? (typeof fields['system'] === 'string' ? fields['system'] : '');
  const rules = Array.isArray(fields['permissions']) ? fields['permissions'] : [];
  permission = JSON.stringify(rules, null, 2);
}
$effect(() => {
  if (!agent) return;
  if (initialized !== agent.id) {
    initialized = agent.id;
    storage = snapshots(agent)[0]?.storage ?? 'inline';
    seenReset = config.formResetVersion;
    loadSource();
  } else if (seenReset !== config.formResetVersion) {
    seenReset = config.formResetVersion;
    loadSource();
  }
});
function fieldValues(): Record<string, unknown> {
  const rules = permissionRules(permission);
  return {
    model: values.model ? joinModelSelector(values.model, values.variant) : undefined,
    mode: values.mode || undefined,
    description: values.description || undefined,
    disabled: values.disabled,
    hidden: values.hidden,
    color: values.color || undefined,
    steps: values.steps ? Number(values.steps) : undefined,
    prompt: values.prompt || undefined,
    ...(rules.length ? { permissions: rules } : {}),
  };
}
async function submit() {
  if (!agent || !selectedSource) {
    toast({
      variant: 'error',
      description: i18n.t('agents.sourceCannotEdit'),
    });
    return;
  }
  try {
    const fields = fieldValues();
    const original = selectedSource.fields ?? {};
    const clearFields: string[] = ['model', 'mode', 'color', 'steps'].filter(
      (key) => fields[key] === undefined && key in original,
    );
    if (fields['permissions'] === undefined && 'permissions' in original) clearFields.push('permissions');
    const hadPrompt = (selectedSource.prompt ?? '') !== '' || 'prompt' in original || 'system' in original;
    if (hadPrompt && !values.prompt) clearFields.push('prompt');
    const payload = {
      id: agent.id,
      source: agent.source,
      storage,
      mutation: { fields, clearFields },
    } as AgentWrite;
    await config.updateAgent(payload);
    goto('/agents');
  } catch (error) {
    toast({
      variant: 'error',
      description: error instanceof Error ? error.message : i18n.t('toast.saveFailedAgent'),
    });
  }
}
</script>

<svelte:head><title>{i18n.t('agents.editMetaTitle')}</title></svelte:head>
<PageHead eyebrow={i18n.t('agents.eyebrow')} title={i18n.t('agents.editTitle')} />
{#if builtin}
  <div class="state-banner" role="status">{i18n.t('agents.builtinReadOnly')}</div>
{:else if agent}
  <p class="muted">
    {i18n.t('agents.editNotice')}
  </p>
  <form
    class="panel form-panel"
    onsubmit={(event) => {
      event.preventDefault();
      submit();
    }}
  >
    <div class="form-grid">
      <fieldset class="form-section">
        <legend>{i18n.t('agents.colSource')}</legend>
        <div class="form-grid">
          <div class="field full">
            <label for="storage">{i18n.t('agents.editSource')}</label>
            <Select
              id="storage"
              bind:value={storage}
              onchange={() => loadSource()}
              options={snapshots(agent).map((source) => ({
                value: source.storage,
                label: i18n.t(STORAGE_LABELS[source.storage]),
              }))}
            />
            {#if selectedSource}
              <small class="muted"
                >{i18n.t('agents.pathLabel', { path: selectedSource.path ?? i18n.t('agents.noPath') })}</small
              >
            {:else}
              <small class="muted">{i18n.t('agents.sourceMissing')}</small>
            {/if}
            {#if agent.source === 'both'}
              <small class="muted">{i18n.t('agents.effectiveOverrides')}</small>
            {/if}
          </div>
        </div>
      </fieldset>
      <AgentFields
        bind:values
        bind:permission
        {modelOptions}
        {modelVariants}
        {variantUnavailable}
        {variantHint}
        {promptHint}
        {knownMode}
      />
    </div>
    <FormActions
      ><Button href="/agents" variant="outline">{i18n.t('common.cancel')}</Button
      ><Button type="submit" disabled={config.saving || !selectedSource}
        >{i18n.t('agents.saveChanges')}</Button
      ></FormActions
    >
  </form>
{:else if !config.loading}
  <div class="state-banner error" role="alert">
    {i18n.t('agents.notFound')}
  </div>
{/if}
