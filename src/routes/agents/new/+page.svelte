<script lang="ts">
  import { goto } from '$app/navigation';
  import { Button } from '$lib/components/ui/button/index.js';
  import AgentFields from '$lib/components/app/AgentFields.svelte';
  import FormActions from '$lib/components/app/FormActions.svelte';
  import PageHead from '$lib/components/app/PageHead.svelte';
  import Select from '$lib/components/app/Select.svelte';
  import { getConfig } from '$lib/features/config/context.js';
  import { getI18n } from '$lib/features/i18n/context.js';
  import { AGENT_MODES, MARKDOWN_STORAGE_OPTIONS } from '$lib/features/config/constants.js';
  import type { AgentSource, AgentStorage, AgentWrite, ModelRef, OptionRow } from '$lib/features/config/types.js';
  import {
    buildModelOptions,
    buildModelVariants,
    optionObject,
    permissionObject,
    type AgentFieldValues,
  } from '$lib/features/config/agentForm.js';
  import { toast } from '$lib/components/app/toast.svelte.js';

  const config = getConfig();
  const i18n = getI18n();
  let id = $state('');
  let source = $state<Exclude<AgentSource, 'both'>>('markdown');
  let markdownStorage = $state<AgentStorage>('global_markdown');
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

  const knownMode = $derived((AGENT_MODES as readonly string[]).includes(values.mode));
  const storeModels = $derived(config.models());
  const modelVariants = $derived(buildModelVariants(config.catalogModels(), storeModels, values.model));
  const modelOptions = $derived(
    buildModelOptions(config.catalogModels(), storeModels, values.model, i18n.t('agents.inherit')),
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
    source === 'markdown' ? i18n.t('agents.promptHintMarkdown') : i18n.t('agents.promptHintInline'),
  );

  async function submit() {
    try {
      const fields = {
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
      const storage: AgentStorage = source === 'inline' ? 'inline' : markdownStorage;
      const payload = {
        id: id.trim(),
        source,
        storage,
        mutation: { fields },
      } as AgentWrite;
      await config.createAgent(payload);
      goto('/agents');
    } catch (error) {
      toast({
        variant: 'error',
        description: error instanceof Error ? error.message : i18n.t('toast.saveFailedAgent'),
      });
    }
  }
</script>

<svelte:head><title>{i18n.t('agents.newMetaTitle')}</title></svelte:head><PageHead
  eyebrow={i18n.t('agents.eyebrow')}
  title={i18n.t('agents.newTitle')}
/>
<form
  class="panel form-panel"
  onsubmit={(event) => {
    event.preventDefault();
    submit();
  }}
>
  <div class="form-grid">
    <fieldset class="form-section">
      <legend>{i18n.t('agents.identity')}</legend>
      <div class="form-grid">
        <div class="field">
          <label for="agent-id">{i18n.t('agents.agentId')}</label><input id="agent-id" bind:value={id} required />
        </div>
        <fieldset class="group-fieldset">
          <legend>{i18n.t('agents.storageSource')}</legend>
          <div class="check-row">
            <label
              ><input type="radio" name="source" value="markdown" bind:group={source} />
              {i18n.t('agents.markdown')}</label
            ><label
              ><input type="radio" name="source" value="inline" bind:group={source} />
              {i18n.t('agents.sourceInline')}</label
            >
          </div>
        </fieldset>
        {#if source === 'markdown'}
          <div class="field">
            <label for="agent-storage">{i18n.t('agents.storage')}</label><Select
              id="agent-storage"
              bind:value={markdownStorage}
              options={MARKDOWN_STORAGE_OPTIONS.map((option) => ({ value: option.value, label: i18n.t(option.label) }))}
            />
          </div>
        {/if}
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
      disabled={config.saving}>{i18n.t('agents.create')}</Button
    ></FormActions
  >
</form>
