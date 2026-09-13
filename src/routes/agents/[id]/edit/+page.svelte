<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { Button } from '$lib/components/ui/button/index.js';
  import FormActions from '$lib/components/app/FormActions.svelte';
  import PageHead from '$lib/components/app/PageHead.svelte';
  import { getConfig } from '$lib/features/config/context.js';
  import { getI18n } from '$lib/features/i18n/context.js';
  import type { AgentDefinition, AgentStorage, ModelRef } from '$lib/features/config/types.js';
  import { toast } from '$lib/components/app/toast.svelte.js';
  type OptionRow = { key: string; value: string };
  type SourceSnapshot = {
    storage: AgentStorage;
    path?: string;
    raw?: string;
    prompt?: string;
    fields?: Record<string, unknown>;
  };
  type AgentMutation = { source: AgentStorage; fields: Record<string, unknown>; clearFields: string[] };
  type AgentWrite = AgentDefinition & { mutation?: AgentMutation };
  const config = getConfig();
  const i18n = getI18n();
  const agent = $derived(config.agents.find((entry) => entry.id === decodeURIComponent(page.params.id ?? '')));
  let initialized = $state('');
  let seenReset = $state(-1);
  let storage = $state<AgentStorage>('inline');
  let model = $state('');
  let mode = $state('');
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
  let clearFields = $state<string[]>([]);
  const editableFields = [
    'model',
    'mode',
    'description',
    'disable',
    'hidden',
    'color',
    'variant',
    'temperature',
    'top_p',
    'steps',
    'prompt',
    'permission',
    'options',
  ];
  const object = (value: string, label: string) => {
    const parsed = JSON.parse(value);
    if (!parsed || Array.isArray(parsed) || typeof parsed !== 'object')
      throw new Error(i18n.t('validation.mustBeJsonObject', { label }));
    return parsed as Record<string, unknown>;
  };
  const pretty = (value: unknown) => JSON.stringify(value ?? {}, null, 2);
  const optionRows = (value: unknown) =>
    Object.entries(value && typeof value === 'object' && !Array.isArray(value) ? value : {}).map(([key, item]) => ({
      key,
      value: typeof item === 'string' ? item : JSON.stringify(item),
    }));
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
  const promptHint = (value: string) =>
    /\{(?:file|env):[^}]+\}/.test(value) ? i18n.t('agents.promptHintResolved') : i18n.t('agents.promptHintSupport');
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
    model = typeof fields['model'] === 'string' ? fields['model'] : '';
    mode = typeof fields['mode'] === 'string' ? fields['mode'] : '';
    description = typeof fields['description'] === 'string' ? fields['description'] : '';
    disable = fields['disable'] === true;
    hidden = fields['hidden'] === true;
    color = typeof fields['color'] === 'string' ? fields['color'] : '';
    variant = typeof fields['variant'] === 'string' ? fields['variant'] : '';
    temperature = typeof fields['temperature'] === 'number' ? String(fields['temperature']) : '';
    topP = typeof fields['top_p'] === 'number' ? String(fields['top_p']) : '';
    steps = typeof fields['steps'] === 'number' ? String(fields['steps']) : '';
    prompt = selectedSource?.prompt ?? (typeof fields['prompt'] === 'string' ? fields['prompt'] : '');
    permission = pretty(fields['permission']);
    options = optionRows(fields['options']);
    clearFields = [];
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
  function addOption() {
    options = [...options, { key: '', value: '' }];
  }
  function removeOption(index: number) {
    options = options.filter((_, i) => i !== index);
  }
  function updateOption(index: number, key: 'key' | 'value', value: string) {
    options = options.map((row, i) => (i === index ? { ...row, [key]: value } : row));
  }
  function fieldValues() {
    return {
      model: model ? (model as ModelRef) : undefined,
      mode: mode || undefined,
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
      for (const field of clearFields) delete fields[field];
      const payload = {
        id: agent.id,
        source: agent.source,
        storage,
        mutation: { source: storage, fields, clearFields },
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
      <div class="field full">
        <label for="storage">{i18n.t('agents.editSource')}</label><select
          id="storage"
          bind:value={storage}
          onchange={loadSource}
          >{#each snapshots(agent) as source}<option value={source.storage}
              >{source.storage === 'inline'
                ? i18n.t('agents.sourceInline')
                : source.storage === 'global_markdown'
                  ? i18n.t('agents.sourceGlobalMarkdown')
                  : i18n.t('agents.sourceProjectMarkdown')}</option
            >{/each}</select
        >{#if selectedSource}<small class="muted"
            >{i18n.t('agents.pathLabel', { path: selectedSource.path ?? i18n.t('agents.noPath') })}</small
          >{:else}<small class="muted">{i18n.t('agents.sourceMissing')}</small>{/if}{#if agent.source === 'both'}<small
            class="muted">{i18n.t('agents.effectiveOverrides')}</small
          >{/if}
      </div>
      <div class="field"><label for="mode">{i18n.t('common.mode')}</label><input id="mode" bind:value={mode} /></div>
      <div class="field">
        <label for="model">{i18n.t('common.model')}</label><select id="model" bind:value={model}
          ><option value="">{i18n.t('agents.clearInherit')}</option>{#each config.models() as entry}<option
              value={entry.ref}>{entry.ref}</option
            >{/each}</select
        >
      </div>
      <div class="field full">
        <label for="description">{i18n.t('common.description')}</label><input
          id="description"
          bind:value={description}
        />
      </div>
      <div class="field">
        <label for="variant">{i18n.t('agents.variant')}</label><input id="variant" bind:value={variant} />
      </div>
      <div class="field">
        <label for="color">{i18n.t('agents.color')}</label><input id="color" bind:value={color} />
      </div>
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
          step="0.01"
          bind:value={topP}
        />
      </div>
      <div class="field">
        <label for="steps">{i18n.t('agents.steps')}</label><input
          id="steps"
          type="number"
          min="1"
          step="1"
          bind:value={steps}
        />
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
        <label for="permission">{i18n.t('agents.permissionJson')}</label><textarea
          id="permission"
          bind:value={permission}></textarea>
      </div>
      <fieldset class="field full">
        <legend>{i18n.t('agents.optionsKv')}</legend>{#each options as row, index}<div class="key-value-row">
            <input
              aria-label={i18n.t('agents.optionKey', { index: index + 1 })}
              value={row.key}
              oninput={(event) => updateOption(index, 'key', event.currentTarget.value)}
            /><input
              aria-label={i18n.t('agents.optionValue', { index: index + 1 })}
              value={row.value}
              oninput={(event) => updateOption(index, 'value', event.currentTarget.value)}
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
        <small class="muted">{i18n.t('agents.kvHintEdit')}</small>
      </fieldset>
      <details class="diagnostics full">
        <summary>{i18n.t('agents.dangerClearTitle')}</summary>
        <p class="muted">{i18n.t('agents.dangerClearHint')}</p>
        {#each editableFields as field}<label class="check-row"
            ><input
              type="checkbox"
              checked={clearFields.includes(field)}
              onchange={(event) => {
                clearFields = event.currentTarget.checked
                  ? [...clearFields, field]
                  : clearFields.filter((item) => item !== field);
              }}
            />
            {i18n.t('agents.clearField', { field })}</label
          >{/each}
      </details>
      <details class="diagnostics full">
        <summary>{i18n.t('agents.sourcePreview')}</summary>
        <h3>{i18n.t('agents.effectiveConfig')}</h3>
        <pre>{pretty(agent.effective)}</pre>
        {#each snapshots(agent) as source}<h3>{source.storage}</h3>
          <p class="muted">
            {i18n.t('agents.pathLabel', { path: source.path ?? i18n.t('agents.pathNotProvided') })}
            {source.raw ? i18n.t('agents.rawProvided') : i18n.t('agents.rawNotProvided')}
          </p>
          <pre>{source.raw ?? pretty(source.fields)}</pre>{/each}
      </details>
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
