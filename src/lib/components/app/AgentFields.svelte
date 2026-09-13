<script lang="ts">
  import { Button } from '$lib/components/ui/button/index.js';
  import Select from './Select.svelte';
  import PermissionEditor from './PermissionEditor.svelte';
  import { COLOR_THEMES, AGENT_MODES } from '$lib/features/config/constants.js';
  import { getConfig } from '$lib/features/config/context.js';
  import { pretty, optionObject, type AgentFieldValues } from '$lib/features/config/agentForm.js';
  import type { OptionRow } from '$lib/features/config/types.js';

  let {
    values = $bindable(),
    modelOptions = [],
    modelVariants = [],
    variantUnavailable = false,
    variantHint = '',
    promptHint = '',
    knownMode = false,
    options = $bindable([]),
    permission = $bindable('{}'),
  }: {
    values: AgentFieldValues;
    modelOptions: { value: string; label: string }[];
    modelVariants: string[];
    variantUnavailable: boolean;
    variantHint: string;
    promptHint: string;
    knownMode: boolean;
    options: OptionRow[];
    permission: string;
  } = $props();

  const config = getConfig();

  function addOption() {
    options = [...options, { key: '', value: '' }];
  }
  function removeOption(index: number) {
    options = options.filter((_, i) => i !== index);
  }
  function updateOption(index: number, key: 'key' | 'value', value: string) {
    options = options.map((row, i) => (i === index ? { ...row, [key]: value } : row));
  }
</script>

<fieldset class="form-section">
  <legend>Basics</legend>
  <div class="form-grid">
    <div class="field full">
      <label for="description">Description<span class="field-required" aria-hidden="true">*</span></label><input
        id="description"
        bind:value={values.description}
        required
      /><small class="muted">Required by opencode.</small>
    </div>
    <div class="field">
      <label for="mode">Mode</label><Select
        id="mode"
        bind:value={values.mode}
        options={[
          { value: '', label: 'Inherit (all)' },
          ...AGENT_MODES.map((option) => ({ value: option })),
          ...(values.mode && !knownMode ? [{ value: values.mode, label: `${values.mode} (unknown)` }] : []),
        ]}
      />
    </div>
    <div class="field">
      <label for="model">Model</label><Select id="model" bind:value={values.model} options={modelOptions} />
    </div>
  </div>
</fieldset>
<fieldset class="form-section">
  <legend>Limits</legend>
  <div class="limits-grid">
    <div class="field">
      <label for="temperature">Temperature</label><input
        id="temperature"
        type="number"
        min="0"
        max="1"
        step="0.01"
        bind:value={values.temperature}
      />
    </div>
    <div class="field">
      <label for="top-p">Top P</label><input
        id="top-p"
        type="number"
        min="0"
        max="1"
        step="0.01"
        bind:value={values.topP}
      />
    </div>
    <div class="field">
      <label for="steps">Steps</label><input id="steps" type="number" min="1" step="1" bind:value={values.steps} />
    </div>
    <div class="field">
      <label for="color">Color</label><input
        id="color"
        list="agent-color-options"
        bind:value={values.color}
        placeholder="#RRGGBB or theme name"
      /><datalist id="agent-color-options"
        >{#each COLOR_THEMES as theme}<option value={theme}></option>{/each}</datalist
      ><small class="muted">Hex #RRGGBB or a theme: primary, secondary, accent, success, warning, error, info.</small>
    </div>
    <div class="field">
      <label class="check-row"><input type="checkbox" bind:checked={values.hidden} /> Hidden</label><small class="muted"
        >Only applies when Mode is subagent.</small
      >
    </div>
  </div>
</fieldset>
<fieldset class="form-section">
  <legend>Status</legend>
  <label class="check-row"><input type="checkbox" bind:checked={values.disable} /> Disable</label><small class="muted"
    >Disabled agents are kept in config but not loaded by opencode.</small
  >
</fieldset>
<fieldset class="form-section">
  <legend>Prompt</legend>
  <div class="field">
    <label for="prompt">Prompt</label><textarea id="prompt" bind:value={values.prompt}></textarea><small class="muted"
      >{promptHint}</small
    >
  </div>
</fieldset>
<PermissionEditor bind:permission />
<details class="form-section advanced-section">
  <summary>Advanced</summary>
  <div class="form-grid">
    <div class="field full">
      <label for="variant">Variant</label><Select
        id="variant"
        bind:value={values.variant}
        disabled={!values.model || config.catalogLoading}
        options={[
          { value: '', label: 'None' },
          ...modelVariants.map((name) => ({ value: name })),
          ...(variantUnavailable ? [{ value: values.variant, label: `${values.variant} (unavailable)` }] : []),
        ]}
      /><small class="muted">{variantHint}</small>
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
      <pre>{pretty(optionObject(options))}</pre>
      <small class="muted"
        >Key/value rows are the only editing entry point; the JSON below is a synchronized preview only and never
        becomes a second save source.</small
      >
    </fieldset>
  </div>
</details>
