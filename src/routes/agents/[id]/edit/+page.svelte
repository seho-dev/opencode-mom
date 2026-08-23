<script lang="ts">
  import { page } from '$app/state';
  import { goto } from '$app/navigation';
  import { Button } from '$lib/components/ui/button/index.js';
  import FormActions from '$lib/components/app/FormActions.svelte';
  import PageHead from '$lib/components/app/PageHead.svelte';
  import { getConfig } from '$lib/features/config/context.js';
  import type { AgentDefinition, AgentStorage, ModelRef } from '$lib/features/config/types.js';
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
  let message = $state('');
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
      throw new Error(`${label} must be a JSON object`);
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
    /\{(?:file|env):[^}]+\}/.test(value)
      ? 'File or environment references are used; they are resolved at runtime.'
      : 'Supports {file:path/to/prompt.md} and {env:VARIABLE_NAME}.';
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
    message = '';
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
      message = 'The selected source was not provided in the DTO sources; it cannot be edited safely.';
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
      message = error instanceof Error ? error.message : 'Save failed. Check the configuration state.';
    }
  }
</script>

<svelte:head><title>Edit agent · opencode-mom</title></svelte:head><PageHead
  eyebrow="CONFIG / AGENTS"
  title="Edit agent"
/>{#if agent}<p class="muted">Editing writes only to the selected source; the effective view is never written back.</p>
  <form
    class="panel form-panel"
    onsubmit={(event) => {
      event.preventDefault();
      submit();
    }}
  >
    <div class="form-grid">
      <div class="field full">
        <label for="storage">Edit source</label><select id="storage" bind:value={storage} onchange={loadSource}
          >{#each snapshots(agent) as source}<option value={source.storage}
              >{source.storage === 'inline'
                ? 'Inline'
                : source.storage === 'global_markdown'
                  ? 'Global Markdown'
                  : 'Project Markdown'}</option
            >{/each}</select
        >{#if selectedSource}<small class="muted">Path: {selectedSource.path ?? 'no path provided by the DTO'}.</small
          >{:else}<small class="muted"
            >The selected source is not provided by the DTO; it cannot be written safely.</small
          >{/if}{#if agent.source === 'both'}<small class="muted"
            >Effective overrides: the effective view is read-only and never saved.</small
          >{/if}
      </div>
      <div class="field"><label for="mode">Mode</label><input id="mode" bind:value={mode} /></div>
      <div class="field">
        <label for="model">Model</label><select id="model" bind:value={model}
          ><option value="">Clear / inherit</option>{#each config.models() as entry}<option value={entry.ref}
              >{entry.ref}</option
            >{/each}</select
        >
      </div>
      <div class="field full">
        <label for="description">Description</label><input id="description" bind:value={description} />
      </div>
      <div class="field"><label for="variant">Variant</label><input id="variant" bind:value={variant} /></div>
      <div class="field"><label for="color">Color</label><input id="color" bind:value={color} /></div>
      <div class="field">
        <label for="temperature">Temperature</label><input
          id="temperature"
          type="number"
          step="0.01"
          bind:value={temperature}
        />
      </div>
      <div class="field">
        <label for="top-p">Top P</label><input id="top-p" type="number" min="0" step="0.01" bind:value={topP} />
      </div>
      <div class="field">
        <label for="steps">Steps</label><input id="steps" type="number" min="1" step="1" bind:value={steps} />
      </div>
      <label class="check-row"><input type="checkbox" bind:checked={disable} /> Disable</label><label class="check-row"
        ><input type="checkbox" bind:checked={hidden} /> Hidden</label
      >
      <div class="field full">
        <label for="prompt">Prompt</label><textarea id="prompt" bind:value={prompt}></textarea><small class="muted"
          >{promptHint(prompt)}</small
        >
      </div>
      <div class="field full">
        <label for="permission">Permission JSON</label><textarea id="permission" bind:value={permission}></textarea>
      </div>
      <fieldset class="field full">
        <legend>Options key/value pairs</legend>{#each options as row, index}<div class="key-value-row">
            <input
              aria-label={`Option key ${index + 1}`}
              value={row.key}
              oninput={(event) => updateOption(index, 'key', event.currentTarget.value)}
            /><input
              aria-label={`Option value ${index + 1}`}
              value={row.value}
              oninput={(event) => updateOption(index, 'value', event.currentTarget.value)}
            /><Button
              type="button"
              size="icon-sm"
              variant="ghost"
              aria-label="Remove option"
              onclick={() => removeOption(index)}>Remove</Button
            >
          </div>{/each}<Button type="button" size="sm" variant="outline" onclick={addOption}>Add option</Button>
        <pre>{pretty(optionObject())}</pre>
        <small class="muted"
          >Key/value rows are the only editing entry point; the JSON is a synchronized preview only.</small
        >
      </fieldset>
      <details class="diagnostics full">
        <summary>Danger zone: clear fields of the selected source</summary>
        <p class="muted">
          Checked fields are embedded into mutation.clearFields; effective values are never copied back into the source.
        </p>
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
            Clear {field}</label
          >{/each}
      </details>
      <details class="diagnostics full">
        <summary>Source preview</summary>
        <h3>Effective configuration (read-only)</h3>
        <pre>{pretty(agent.effective)}</pre>
        {#each snapshots(agent) as source}<h3>{source.storage}</h3>
          <p class="muted">
            Path: {source.path ?? 'not provided by the DTO'}; {source.raw
              ? 'The following is the real raw content provided by the DTO.'
              : 'The DTO provides no raw content; the following is the field snapshot of this source only.'}
          </p>
          <pre>{source.raw ?? pretty(source.fields)}</pre>{/each}
      </details>
      {#if message}<div class="state-banner error full" role="alert">{message}</div>{/if}
    </div>
    <FormActions
      ><Button href="/agents" variant="outline">Cancel</Button><Button
        type="submit"
        disabled={config.saving || !selectedSource}>Save changes</Button
      ></FormActions
    >
  </form>{:else if !config.loading}<div class="state-banner error" role="alert">
    The agent does not exist or has not been loaded yet.
  </div>{/if}
