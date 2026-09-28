<script lang="ts">
import { goto } from '$app/navigation';
import { Button } from '$src/components/button/index.js';
import FormActions from '$src/components/FormActions.svelte';
import Select from '$src/components/Select.svelte';
import { Switch } from '$src/components/switch/index.js';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import { toast } from '$src/shell/toast.svelte.js';
import type { AgentDefinition, AgentSource, AgentStorage, AgentWrite } from '$src/types/agents.js';
import { AGENT_MODES, COLOR_THEMES, STORAGE_LABELS } from '$src/utils/constants.js';
import { buildModelOptions, buildModelVariants, isBuiltinAgentId } from '$src/utils/index.js';
import {
  type AgentFormValues,
  agentMutationFields,
  createAgentForm,
  fieldError,
  splitModelSelector,
} from './agentForm.js';
import PermissionEditor from './PermissionEditor.svelte';

let { id }: { id?: string } = $props();
type SourceSnapshot = {
  storage: AgentStorage;
  path?: string;
  raw?: string;
  prompt?: string;
  fields?: Record<string, unknown>;
};
const config = getConfig();
const i18n = getI18n();
const edit = $derived(id !== undefined);
const builtin = $derived(edit && isBuiltinAgentId(id ?? ''));
const agent = $derived(edit && !builtin ? config.agents.find((entry) => entry.id === id) : undefined);
let source = $state<Exclude<AgentSource, 'both'>>('markdown');
let storage = $state<AgentStorage>('inline');
let initialized = $state('');
let seenReset = $state(-1);
// svelte-ignore state_referenced_locally
const form = createAgentForm(
  {
    idRequired: i18n.t('agents.idRequired'),
    idReserved: i18n.t('agents.idReserved'),
    descriptionRequired: i18n.t('agents.descriptionRequired'),
    stepsInvalid: i18n.t('agents.stepsInvalid'),
    permissionsInvalid: i18n.t('agents.permissionsInvalid'),
  },
  submit,
  !edit,
);
const formState = form.useSelector((state) => state.values);
const values = $derived(formState.current);

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
function loadSource() {
  const fields = selectedSource?.fields ?? {};
  const selector =
    typeof fields['model'] === 'string' ? splitModelSelector(fields['model']) : { model: '', variant: '' };
  const rules = Array.isArray(fields['permissions']) ? fields['permissions'] : [];
  form.reset({
    id: agent?.id ?? '',
    model: selector.model,
    variant: selector.variant,
    mode: typeof fields['mode'] === 'string' ? fields['mode'] : '',
    description: typeof fields['description'] === 'string' ? fields['description'] : '',
    disabled: fields['disabled'] === true,
    hidden: fields['hidden'] === true,
    color: typeof fields['color'] === 'string' ? fields['color'] : '',
    steps: typeof fields['steps'] === 'number' ? String(fields['steps']) : '',
    prompt: selectedSource?.prompt ?? (typeof fields['system'] === 'string' ? fields['system'] : ''),
    permission: JSON.stringify(rules, null, 2),
  });
}
$effect(() => {
  if (!edit || !agent) return;
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
async function submit(values: AgentFormValues) {
  if (edit && (!agent || !selectedSource)) {
    toast({ variant: 'error', description: i18n.t('agents.sourceCannotEdit') });
    return;
  }
  try {
    const fields = agentMutationFields(values);
    if (edit && agent && selectedSource) {
      const original = selectedSource.fields ?? {};
      const clearFields: string[] = ['model', 'mode', 'color', 'steps'].filter(
        (key) => fields[key] === undefined && key in original,
      );
      if (fields['permissions'] === undefined && 'permissions' in original) clearFields.push('permissions');
      const hadPrompt = (selectedSource.prompt ?? '') !== '' || 'prompt' in original || 'system' in original;
      if (hadPrompt && !values.prompt) clearFields.push('prompt');
      await config.updateAgent({
        id: agent.id,
        source: agent.source,
        storage,
        mutation: { fields, clearFields },
      } as AgentWrite);
    } else {
      await config.createAgent({
        id: values.id.trim(),
        source,
        storage: source === 'inline' ? 'inline' : 'global_markdown',
        mutation: { fields },
      } as AgentWrite);
    }
    await goto('/agents');
  } catch (error) {
    toast({
      variant: 'error',
      description: error instanceof Error ? error.message : i18n.t('toast.saveFailedAgent'),
    });
  }
}

const knownMode = $derived((AGENT_MODES as readonly string[]).includes(values.mode));
const storeModels = $derived(config.models());
const modelVariants = $derived(buildModelVariants(config.catalogModels(), storeModels, values.model));
const modelOptions = $derived(
  buildModelOptions(
    config.catalogModels(),
    storeModels,
    values.model,
    i18n.t(edit ? 'agents.clearInherit' : 'agents.inherit'),
  ),
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
  i18n.t((edit ? storage : source) === 'inline' ? 'agents.promptHintInline' : 'agents.promptHintMarkdown'),
);
function changeModel(model: string) {
  if (values.variant && !buildModelVariants(config.catalogModels(), config.models(), model).includes(values.variant))
    form.setFieldValue('variant', '');
  form.setFieldValue('model', model);
}
</script>

{#if builtin}
  <div class="state-banner" role="status">{i18n.t('agents.builtinReadOnly')}</div>
{:else if !edit || agent}
  {#if edit}
    <p class="muted">{i18n.t('agents.editNotice')}</p>
  {/if}
  <form
    class="panel form-panel"
    novalidate
    onsubmit={(event) => {
      event.preventDefault();
      form.handleSubmit();
    }}
  >
    <div class="form-grid">
      {#if edit}
        <fieldset class="form-section">
          <legend>{i18n.t('agents.colSource')}</legend>
          <div class="form-grid">
            <div class="field full">
              <label for="storage">{i18n.t('agents.editSource')}</label>
              <Select
                id="storage"
                bind:value={storage}
                onchange={() => loadSource()}
                options={snapshots(agent).map((entry) => ({
                  value: entry.storage,
                  label: i18n.t(STORAGE_LABELS[entry.storage]),
                }))}
              />
              {#if selectedSource}
                <small class="muted"
                  >{i18n.t('agents.pathLabel', { path: selectedSource.path ?? i18n.t('agents.noPath') })}</small
                >
              {:else}
                <small class="muted">{i18n.t('agents.sourceMissing')}</small>
              {/if}
              {#if agent?.source === 'both'}
                <small class="muted">{i18n.t('agents.effectiveOverrides')}</small>
              {/if}
            </div>
          </div>
        </fieldset>
      {:else}
        <fieldset class="form-section">
          <legend>{i18n.t('agents.identity')}</legend>
          <div class="form-grid">
            <div class="field">
              <label for="agent-id"
                >{i18n.t('agents.agentId')}<span class="field-required" aria-hidden="true">*</span></label
              >
              <form.Field name="id">
                {#snippet children(field)}
                  {@const error = fieldError(field.state.meta.errors)}
                  <input
                    id="agent-id"
                    value={field.state.value}
                    required
                    oninput={(event) => field.handleChange(event.currentTarget.value)}
                    onblur={field.handleBlur}
                    aria-invalid={error ? 'true' : undefined}
                    aria-describedby={error ? 'agent-id-error' : undefined}
                  >
                  {#if error}
                    <p id="agent-id-error" class="field-error" role="alert">{error}</p>
                  {/if}
                {/snippet}
              </form.Field>
            </div>
            <fieldset class="group-fieldset">
              <legend>{i18n.t('agents.storageSource')}</legend>
              <div class="check-row source-options">
                <label
                  ><input type="radio" name="source" value="markdown" bind:group={source}>
                  {i18n.t('agents.markdown')}</label
                ><label
                  ><input type="radio" name="source" value="inline" bind:group={source}>
                  {i18n.t('agents.sourceInline')}</label
                >
              </div>
            </fieldset>
          </div>
        </fieldset>
      {/if}
      <fieldset class="form-section">
        <legend>{i18n.t('agents.legendBasics')}</legend>
        <div class="form-grid">
          <div class="field full">
            <label for="description"
              >{i18n.t('common.description')}<span class="field-required" aria-hidden="true">*</span></label
            >
            <form.Field name="description">
              {#snippet children(field)}
                {@const error = fieldError(field.state.meta.errors)}
                <input
                  id="description"
                  value={field.state.value}
                  oninput={(event) => field.handleChange(event.currentTarget.value)}
                  onblur={field.handleBlur}
                  required
                  aria-invalid={error ? 'true' : undefined}
                  aria-describedby={error ? 'description-error' : undefined}
                >
                {#if error}
                  <small id="description-error" class="field-error" role="alert">{error}</small>
                {/if}
              {/snippet}
            </form.Field>
          </div>
          <div class="field">
            <label for="mode">{i18n.t('common.mode')}</label>
            <form.Field name="mode">
              {#snippet children(field)}
                <Select
                  id="mode"
                  value={field.state.value}
                  onchange={(value) => field.handleChange(value)}
                  options={[
                    { value: '', label: i18n.t('agents.inheritAll') },
                    ...AGENT_MODES.map((option) => ({ value: option })),
                    ...(values.mode && !knownMode
                      ? [{ value: values.mode, label: i18n.t('agents.optionUnknown', { name: values.mode }) }]
                      : []),
                  ]}
                />
              {/snippet}
            </form.Field>
          </div>
          <div class="field">
            <label for="model">{i18n.t('common.model')}</label>
            <form.Field name="model">
              {#snippet children(field)}
                <Select id="model" value={field.state.value} onchange={changeModel} options={modelOptions} />
              {/snippet}
            </form.Field>
          </div>
          <div class="field full">
            <label for="variant">{i18n.t('agents.variant')}</label>
            <form.Field name="variant">
              {#snippet children(field)}
                <Select
                  id="variant"
                  value={field.state.value}
                  onchange={(value) => field.handleChange(value)}
                  disabled={!values.model || config.catalogLoading}
                  options={[
                    { value: '', label: i18n.t('agents.optionNone') },
                    ...modelVariants.map((name) => ({ value: name })),
                    ...(variantUnavailable
                      ? [{ value: values.variant, label: i18n.t('agents.optionUnavailable', { name: values.variant }) }]
                      : []),
                  ]}
                />
              {/snippet}
            </form.Field><small class="muted">{variantHint}</small>
          </div>
        </div>
      </fieldset>
      <fieldset class="form-section">
        <legend>{i18n.t('agents.legendLimits')}</legend>
        <div class="limits-grid">
          <div class="field">
            <label for="steps">{i18n.t('agents.steps')}</label>
            <form.Field name="steps">
              {#snippet children(field)}
                {@const error = fieldError(field.state.meta.errors)}
                <input
                  id="steps"
                  type="number"
                  min="1"
                  step="1"
                  value={field.state.value}
                  oninput={(event) => field.handleChange(event.currentTarget.value)}
                  onblur={field.handleBlur}
                  aria-invalid={error ? 'true' : undefined}
                  aria-describedby={error ? 'steps-error' : undefined}
                >
                {#if error}
                  <small id="steps-error" class="field-error" role="alert">{error}</small>
                {/if}
              {/snippet}
            </form.Field>
          </div>
          <div class="field">
            <label for="color">{i18n.t('agents.color')}</label>
            <form.Field name="color">
              {#snippet children(field)}
                <input
                  id="color"
                  list="agent-color-options"
                  value={field.state.value}
                  oninput={(event) => field.handleChange(event.currentTarget.value)}
                  onblur={field.handleBlur}
                  placeholder={i18n.t('agents.colorPlaceholder')}
                >
              {/snippet}
            </form.Field>
            <datalist id="agent-color-options">
              {#each COLOR_THEMES as theme}
                <option value={theme}></option>
              {/each}
            </datalist>
          </div>
          <div class="field">
            <div class="check-row">
              <form.Field name="hidden">
                {#snippet children(field)}
                  <Switch
                    id="agent-hidden"
                    checked={field.state.value}
                    onchange={(event) => field.handleChange(event.currentTarget.checked)}
                    onblur={field.handleBlur}
                  />
                {/snippet}
              </form.Field>
              <label for="agent-hidden">{i18n.t('agents.hidden')}</label>
            </div>
            <small class="muted">{i18n.t('agents.hiddenHint')}</small>
          </div>
        </div>
      </fieldset>
      <fieldset class="form-section">
        <legend>{i18n.t('agents.legendStatus')}</legend>
        <div class="check-row">
          <form.Field name="disabled">
            {#snippet children(field)}
              <Switch
                id="agent-disabled"
                checked={field.state.value}
                onchange={(event) => field.handleChange(event.currentTarget.checked)}
                onblur={field.handleBlur}
              />
            {/snippet}
          </form.Field>
          <label for="agent-disabled">{i18n.t('agents.disable')}</label>
        </div>
        <small class="muted">{i18n.t('agents.disableHint')}</small>
      </fieldset>
      <fieldset class="form-section">
        <legend>{i18n.t('agents.legendPrompt')}</legend>
        <div class="field">
          <label for="prompt">{i18n.t('agents.prompt')}</label>
          <form.Field name="prompt">
            {#snippet children(field)}
              <textarea
                id="prompt"
                value={field.state.value}
                oninput={(event) => field.handleChange(event.currentTarget.value)}
                onblur={field.handleBlur}
              ></textarea>
            {/snippet}
          </form.Field><small class="muted">{promptHint}</small>
        </div>
      </fieldset>
      <form.Field name="permission">
        {#snippet children(field)}
          <PermissionEditor
            permission={field.state.value}
            onchange={(value) => field.handleChange(value)}
            error={fieldError(field.state.meta.errors)}
          />
        {/snippet}
      </form.Field>
    </div>
    <FormActions
      ><Button href="/agents" variant="outline">{i18n.t('common.cancel')}</Button
      ><Button type="submit" disabled={config.saving || (edit && !selectedSource)}
        >{i18n.t(edit ? 'agents.saveChanges' : 'agents.create')}</Button
      ></FormActions
    >
  </form>
{:else if !config.loading}
  <div class="state-banner error" role="alert">{i18n.t('agents.notFound')}</div>
{/if}
