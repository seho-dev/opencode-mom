<script lang="ts">
  import { goto } from '$app/navigation';
  import { Button } from '$lib/components/ui/button/index.js';
  import FormActions from '$lib/components/app/FormActions.svelte';
  import PageHead from '$lib/components/app/PageHead.svelte';
  import { getConfig } from '$lib/features/config/context.js';
  import type { AgentDefinition, AgentSource, AgentStorage, ModelRef } from '$lib/features/config/types.js';
  import { toast } from '$lib/components/app/toast.svelte.js';
  type OptionRow = { key: string; value: string };
  type AgentMutation = { source: AgentStorage; fields: Record<string, unknown>; clearFields: string[] };
  type AgentWrite = AgentDefinition & { mutation?: AgentMutation };
  const config = getConfig();
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
      throw new Error(`${label} must be a JSON object`);
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
      ? 'File or environment references are used; they are resolved at runtime after saving.'
      : 'Supports {file:path/to/prompt.md} and {env:VARIABLE_NAME}.';
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
        description: error instanceof Error ? error.message : 'Save failed. Check the configuration state.',
      });
    }
  }
</script>

<svelte:head><title>New agent · opencode-mom</title></svelte:head><PageHead
  eyebrow="CONFIG / AGENTS"
  title="New agent"
/>
<form
  class="panel form-panel"
  onsubmit={(event) => {
    event.preventDefault();
    submit();
  }}
>
  <div class="form-grid">
    <div class="field"><label for="agent-id">Agent ID</label><input id="agent-id" bind:value={id} required /></div>
    <fieldset class="group-fieldset">
      <legend>Storage source</legend>
      <div class="check-row">
        <label><input type="radio" name="source" value="markdown" bind:group={source} /> Markdown</label><label
          ><input type="radio" name="source" value="inline" bind:group={source} /> Inline</label
        >
      </div>
    </fieldset>
    {#if source === 'markdown'}<div class="field full">
        <label for="storage">Markdown location</label><select id="storage" bind:value={storage}
          ><option value="global_markdown">Global Markdown</option><option value="project_markdown"
            >Project Markdown</option
          ></select
        >
      </div>{/if}
    <div class="field"><label for="mode">Mode</label><input id="mode" bind:value={mode} /></div>
    <div class="field">
      <label for="model">Model</label><select id="model" bind:value={modelRef}
        ><option value="">Inherit</option>{#each config.models() as model}<option value={model.ref}>{model.ref}</option
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
      <label for="top-p">Top P</label><input id="top-p" type="number" min="0" max="1" step="0.01" bind:value={topP} />
    </div>
    <div class="field">
      <label for="steps">Steps</label><input id="steps" type="number" min="1" bind:value={steps} />
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
            placeholder="Key"
          /><input
            aria-label={`Option value ${index + 1}`}
            value={row.value}
            oninput={(event) => updateOption(index, 'value', event.currentTarget.value)}
            placeholder="Value (JSON supported)"
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
        >Key/value rows are the only editing entry point; the JSON below is a synchronized preview only and never
        becomes a second save source.</small
      >
    </fieldset>
    <details class="diagnostics full">
      <summary>Danger zone</summary>
      <p class="muted">
        Cleared fields are sent explicitly through clearFields. A new agent has no existing source fields to clear yet.
      </p>
      <div class="state-banner error">To clear fields after creation, use the danger zone on the edit page.</div>
    </details>
  </div>
  <FormActions
    ><Button href="/agents" variant="outline">Cancel</Button><Button type="submit" disabled={config.saving}
      >Create agent</Button
    ></FormActions
  >
</form>
