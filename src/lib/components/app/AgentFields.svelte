<script lang="ts">
import type { AgentFieldValues } from '$lib/features/config/agentForm.js';
import { AGENT_MODES, COLOR_THEMES } from '$lib/features/config/constants.js';
import { getConfig } from '$lib/features/config/context.js';
import { getI18n } from '$lib/features/i18n/context.js';
import PermissionEditor from './PermissionEditor.svelte';
import Select from './Select.svelte';

let {
  values = $bindable(),
  modelOptions = [],
  modelVariants = [],
  variantUnavailable = false,
  variantHint = '',
  promptHint = '',
  knownMode = false,
  permission = $bindable('[]'),
}: {
  values: AgentFieldValues;
  modelOptions: { value: string; label: string }[];
  modelVariants: string[];
  variantUnavailable: boolean;
  variantHint: string;
  promptHint: string;
  knownMode: boolean;
  permission: string;
} = $props();

const config = getConfig();
const i18n = getI18n();
</script>

<fieldset class="form-section">
  <legend>{i18n.t('agents.legendBasics')}</legend>
  <div class="form-grid">
    <div class="field full">
      <label for="description"
        >{i18n.t('common.description')}<span class="field-required" aria-hidden="true">*</span></label
      ><input id="description" bind:value={values.description} required>
      <small class="muted">{i18n.t('agents.descriptionRequired')}</small>
    </div>
    <div class="field">
      <label for="mode">{i18n.t('common.mode')}</label>
      <Select
        id="mode"
        bind:value={values.mode}
        options={[
          { value: '', label: i18n.t('agents.inheritAll') },
          ...AGENT_MODES.map((option) => ({ value: option })),
          ...(values.mode && !knownMode
            ? [{ value: values.mode, label: i18n.t('agents.optionUnknown', { name: values.mode }) }]
            : []),
        ]}
      />
    </div>
    <div class="field">
      <label for="model">{i18n.t('common.model')}</label>
      <Select id="model" bind:value={values.model} options={modelOptions} />
    </div>
    <div class="field full">
      <label for="variant">{i18n.t('agents.variant')}</label>
      <Select
        id="variant"
        bind:value={values.variant}
        disabled={!values.model || config.catalogLoading}
        options={[
          { value: '', label: i18n.t('agents.optionNone') },
          ...modelVariants.map((name) => ({ value: name })),
          ...(variantUnavailable
            ? [{ value: values.variant, label: i18n.t('agents.optionUnavailable', { name: values.variant }) }]
            : []),
        ]}
      /><small class="muted">{variantHint}</small>
    </div>
  </div>
</fieldset>
<fieldset class="form-section">
  <legend>{i18n.t('agents.legendLimits')}</legend>
  <div class="limits-grid">
    <div class="field">
      <label for="steps">{i18n.t('agents.steps')}</label>
      <input id="steps" type="number" min="1" step="1" bind:value={values.steps}>
    </div>
    <div class="field">
      <label for="color">{i18n.t('agents.color')}</label>
      <input
        id="color"
        list="agent-color-options"
        bind:value={values.color}
        placeholder={i18n.t('agents.colorPlaceholder')}
      >
      <datalist id="agent-color-options">
        {#each COLOR_THEMES as theme}
          <option value={theme}></option>
        {/each}
      </datalist>
    </div>
    <div class="field">
      <label class="check-row"><input type="checkbox" bind:checked={values.hidden}> {i18n.t('agents.hidden')}</label
      ><small class="muted">{i18n.t('agents.hiddenHint')}</small>
    </div>
  </div>
</fieldset>
<fieldset class="form-section">
  <legend>{i18n.t('agents.legendStatus')}</legend>
  <label class="check-row"><input type="checkbox" bind:checked={values.disabled}> {i18n.t('agents.disable')}</label
  ><small class="muted">{i18n.t('agents.disableHint')}</small>
</fieldset>
<fieldset class="form-section">
  <legend>{i18n.t('agents.legendPrompt')}</legend>
  <div class="field">
    <label for="prompt">{i18n.t('agents.prompt')}</label><textarea id="prompt" bind:value={values.prompt}></textarea
    ><small class="muted">{promptHint}</small>
  </div>
</fieldset>
<PermissionEditor bind:permission />
