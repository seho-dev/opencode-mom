<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { Button } from '$lib/components/ui/button/index.js';
  import AgentFields from '$lib/components/app/AgentFields.svelte';
  import FormActions from '$lib/components/app/FormActions.svelte';
  import PageHead from '$lib/components/app/PageHead.svelte';
  import Select from '$lib/components/app/Select.svelte';
  import { getConfig } from '$lib/features/config/context.js';
  import { getI18n } from '$lib/features/i18n/context.js';
  import { AGENT_MODES, STORAGE_LABELS } from '$lib/features/config/constants.js';
  import type { AgentDefinition, AgentStorage, AgentWrite, ModelRef, OptionRow } from '$lib/features/config/types.js';
  import {
    buildModelOptions,
    buildModelVariants,
    optionObject,
    optionRows,
    permissionObject,
    pretty,
    type AgentFieldValues,
  } from '$lib/features/config/agentForm.js';
  import { toast } from '$lib/components/app/toast.svelte.js';
  type SourceSnapshot = {
    storage: AgentStorage;
    path?: string;
    raw?: string;
    prompt?: string;
    fields?: Record<string, unknown>;
  };
  const config = getConfig();
  const i18n = getI18n();
  const agent = $derived(config.agents.find((entry) => entry.id === decodeURIComponent(page.params.id ?? '')));
  let initialized = $state('');
  let seenReset = $state(-1);
  let storage = $state<AgentStorage>('inline');
  let values = $state<AgentFieldValues>({
    model: '',
    mode: '',
    description: '',
    disable: false,
    hidden: false,
    color: '',
    variant: '',
    temperature: '',
    topP: '',
    steps: '',
    prompt: '',
  });
  let permission = $state('{}');
  let options = $state<OptionRow[]>([]);

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
    values.model = typeof fields['model'] === 'string' ? fields['model'] : '';
    values.mode = typeof fields['mode'] === 'string' ? fields['mode'] : '';
    values.description = typeof fields['description'] === 'string' ? fields['description'] : '';
    values.disable = fields['disable'] === true;
    values.hidden = fields['hidden'] === true;
    values.color = typeof fields['color'] === 'string' ? fields['color'] : '';
    values.variant = typeof fields['variant'] === 'string' ? fields['variant'] : '';
    values.temperature = typeof fields['temperature'] === 'number' ? String(fields['temperature']) : '';
    values.topP = typeof fields['top_p'] === 'number' ? String(fields['top_p']) : '';
    values.steps = typeof fields['steps'] === 'number' ? String(fields['steps']) : '';
    values.prompt = selectedSource?.prompt ?? (typeof fields['prompt'] === 'string' ? fields['prompt'] : '');
    permission = pretty(fields['permission']);
    options = optionRows(fields['options']);
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
  function fieldValues() {
    return {
      model: values.model ? (values.model as ModelRef) : undefined,
      mode: values.mode || undefined,
      description: values.description || undefined,
      disable: values.disable,
      hidden: values.hidden,
      color: values.color || undefined,
      variant: values.variant || undefined,
      temperature: values.temperature ? Number(values.temperature) : undefined,
      top_p: values.topP ? Number(values.topP) : undefined,
      steps: values.steps ? Number(values.steps) : undefined,
      prompt: values.prompt || undefined,
      permission: permissionObject(permission),
      options: optionObject(options),
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
      const fields: Record<string, unknown> = fieldValues();
      const original = selectedSource.fields ?? {};
      const clearFields: string[] = ['model', 'mode', 'color', 'variant', 'temperature', 'top_p', 'steps'].filter(
        (key) => fields[key] === undefined && key in original,
      );
      if ((selectedSource.prompt ?? '') && !values.prompt) clearFields.push('prompt');
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

<svelte:head><title>{i18n.t('agents.editMetaTitle')}</title></svelte:head><PageHead
  eyebrow={i18n.t('agents.eyebrow')}
  title={i18n.t('agents.editTitle')}
/>{#if agent}<p class="muted">{i18n.t('agents.editNotice')}</p>
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
            <label for="storage">{i18n.t('agents.editSource')}</label><Select
              id="storage"
              bind:value={storage}
              onchange={() => loadSource()}
              options={snapshots(agent).map((source) => ({
                value: source.storage,
                label: i18n.t(STORAGE_LABELS[source.storage]),
              }))}
            />{#if selectedSource}<small class="muted"
                >{i18n.t('agents.pathLabel', { path: selectedSource.path ?? i18n.t('agents.noPath') })}</small
              >{:else}<small class="muted">{i18n.t('agents.sourceMissing')}</small
              >{/if}{#if agent.source === 'both'}<small class="muted">{i18n.t('agents.effectiveOverrides')}</small>{/if}
          </div>
        </div>
      </fieldset>
      <AgentFields
        bind:values
        bind:options
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
      ><Button href="/agents" variant="outline">{i18n.t('common.cancel')}</Button><Button
        type="submit"
        disabled={config.saving || !selectedSource}>{i18n.t('agents.saveChanges')}</Button
      ></FormActions
    >
  </form>{:else if !config.loading}<div class="state-banner error" role="alert">
    {i18n.t('agents.notFound')}
  </div>{/if}
