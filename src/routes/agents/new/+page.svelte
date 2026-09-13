<script lang="ts">
  import { goto } from '$app/navigation';
  import { Button } from '$lib/components/ui/button/index.js';
  import FormActions from '$lib/components/app/FormActions.svelte';
  import PageHead from '$lib/components/app/PageHead.svelte';
  import { getConfig } from '$lib/features/config/context.js';
  import { getI18n } from '$lib/features/i18n/context.js';
  import type { AgentDefinition, AgentSource, AgentStorage, ModelRef } from '$lib/features/config/types.js';
  import { toast } from '$lib/components/app/toast.svelte.js';
  type OptionRow = { key: string; value: string };
  type AgentMutation = { source: AgentStorage; fields: Record<string, unknown>; clearFields: string[] };
  type AgentWrite = AgentDefinition & { mutation?: AgentMutation };
  const config = getConfig();
  const i18n = getI18n();
  let id = $state('');
  let source = $state<Exclude<AgentSource, 'both'>>('markdown');
  let storage = $state<AgentStorage>('global_markdown');
  let modelRef = $state('');
  let mode = $state('primary');
  let description = $state('');
  let disable = $state(false);
  let hidden = $state(false);
  let color = $state('');
  let variant = $state('');
  let temperature = $state('');
  let topP = $state('');
  let steps = $state('');
  let prompt = $state('');
  let permission = $state('{}');
  let options = $state<OptionRow[]>([]);
  const object = (value: string, label: string) => {
    const parsed = JSON.parse(value);
    if (!parsed || Array.isArray(parsed) || typeof parsed !== 'object')
      throw new Error(i18n.t('validation.mustBeJsonObject', { label }));
    return parsed as Record<string, unknown>;
  };
  const pretty = (value: unknown) => JSON.stringify(value ?? {}, null, 2);
  const optionObject = () =>
    Object.fromEntries(
      options
        .filter((row) => row.key.trim())
        .map((row) => {
          try {
            return [row.key.trim(), JSON.parse(row.value)];
          } catch {
            return [row.key.trim(), row.value];
          }
        }),
    );
  function addOption() {
    options = [...options, { key: '', value: '' }];
  }
  function removeOption(index: number) {
    options = options.filter((_, i) => i !== index);
  }
  function updateOption(index: number, key: 'key' | 'value', value: string) {
    options = options.map((row, i) => (i === index ? { ...row, [key]: value } : row));
  }
  $effect(() => {
    if (source === 'inline') storage = 'inline';
    else if (storage === 'inline') storage = 'global_markdown';
  });
  function promptHint(value: string) {
    return /\{(?:file|env):[^}]+\}/.test(value)
      ? i18n.t('agents.promptHintResolvedNew')
      : i18n.t('agents.promptHintSupport');
  }
  async function submit() {
    try {
      const fields = {
        model: modelRef ? (modelRef as ModelRef) : undefined,
        mode,
        description: description || undefined,
        disable,
        hidden,
        color: color || undefined,
        variant: variant || undefined,
        temperature: temperature ? Number(temperature) : undefined,
        top_p: topP ? Number(topP) : undefined,
        steps: steps ? Number(steps) : undefined,
        prompt: prompt || undefined,
        permission: object(permission, 'permission'),
        options: optionObject(),
      };
      const payload = {
        id: id.trim(),
        source,
        storage,
        mutation: { source: storage, fields, clearFields: [] },
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
    <div class="field">
      <label for="agent-id">{i18n.t('agents.agentId')}</label><input id="agent-id" bind:value={id} required />
    </div>
    <fieldset class="group-fieldset">
      <legend>{i18n.t('agents.storageSource')}</legend>
      <div class="check-row">
        <label
          ><input type="radio" name="source" value="markdown" bind:group={source} /> {i18n.t('agents.markdown')}</label
        ><label
          ><input type="radio" name="source" value="inline" bind:group={source} />
          {i18n.t('agents.sourceInline')}</label
        >
      </div>
    </fieldset>
    {#if source === 'markdown'}<div class="field full">
        <label for="storage">{i18n.t('agents.markdownLocation')}</label><select id="storage" bind:value={storage}
          ><option value="global_markdown">{i18n.t('agents.sourceGlobalMarkdown')}</option><option
            value="project_markdown">{i18n.t('agents.sourceProjectMarkdown')}</option
          ></select
        >
      </div>{/if}
    <div class="field"><label for="mode">{i18n.t('common.mode')}</label><input id="mode" bind:value={mode} /></div>
    <div class="field">
      <label for="model">{i18n.t('common.model')}</label><select id="model" bind:value={modelRef}
        ><option value="">{i18n.t('agents.inherit')}</option>{#each config.models() as model}<option value={model.ref}
            >{model.ref}</option
          >{/each}</select
      >
    </div>
    <div class="field full">
      <label for="description">{i18n.t('common.description')}</label><input id="description" bind:value={description} />
    </div>
    <div class="field">
      <label for="variant">{i18n.t('agents.variant')}</label><input id="variant" bind:value={variant} />
    </div>
    <div class="field"><label for="color">{i18n.t('agents.color')}</label><input id="color" bind:value={color} /></div>
    <div class="field">
      <label for="temperature">{i18n.t('agents.temperature')}</label><input
        id="temperature"
        type="number"
        step="0.01"
        bind:value={temperature}
      />
    </div>
    <div class="field">
      <label for="top-p">{i18n.t('agents.topP')}</label><input
        id="top-p"
        type="number"
        min="0"
        max="1"
        step="0.01"
        bind:value={topP}
      />
    </div>
    <div class="field">
      <label for="steps">{i18n.t('agents.steps')}</label><input id="steps" type="number" min="1" bind:value={steps} />
    </div>
    <label class="check-row"><input type="checkbox" bind:checked={disable} /> {i18n.t('agents.disable')}</label><label
      class="check-row"><input type="checkbox" bind:checked={hidden} /> {i18n.t('agents.hidden')}</label
    >
    <div class="field full">
      <label for="prompt">{i18n.t('agents.prompt')}</label><textarea id="prompt" bind:value={prompt}></textarea><small
        class="muted">{promptHint(prompt)}</small
      >
    </div>
    <div class="field full">
      <label for="permission">{i18n.t('agents.permissionJson')}</label><textarea id="permission" bind:value={permission}
      ></textarea>
    </div>
    <fieldset class="field full">
      <legend>{i18n.t('agents.optionsKv')}</legend>{#each options as row, index}<div class="key-value-row">
          <input
            aria-label={i18n.t('agents.optionKey', { index: index + 1 })}
            value={row.key}
            oninput={(event) => updateOption(index, 'key', event.currentTarget.value)}
            placeholder={i18n.t('agents.keyPlaceholder')}
          /><input
            aria-label={i18n.t('agents.optionValue', { index: index + 1 })}
            value={row.value}
            oninput={(event) => updateOption(index, 'value', event.currentTarget.value)}
            placeholder={i18n.t('agents.valuePlaceholder')}
          /><Button
            type="button"
            size="icon-sm"
            variant="ghost"
            aria-label={i18n.t('agents.removeOption')}
            onclick={() => removeOption(index)}>{i18n.t('common.remove')}</Button
          >
        </div>{/each}<Button type="button" size="sm" variant="outline" onclick={addOption}
        >{i18n.t('agents.addOption')}</Button
      >
      <pre>{pretty(optionObject())}</pre>
      <small class="muted">{i18n.t('agents.kvHintNew')}</small>
    </fieldset>
    <details class="diagnostics full">
      <summary>{i18n.t('agents.dangerZone')}</summary>
      <p class="muted">{i18n.t('agents.dangerHintNew')}</p>
      <div class="state-banner error">{i18n.t('agents.dangerClearNew')}</div>
    </details>
  </div>
  <FormActions
    ><Button href="/agents" variant="outline">{i18n.t('common.cancel')}</Button><Button
      type="submit"
      disabled={config.saving}>{i18n.t('agents.create')}</Button
    ></FormActions
  >
</form>
