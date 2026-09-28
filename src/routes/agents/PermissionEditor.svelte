<script lang="ts">
import { ChevronDown, ChevronUp, Plus, X } from '@lucide/svelte';
import { Button } from '$src/components/button/index.js';
import Select from '$src/components/Select.svelte';
import { getI18n } from '$src/i18n/context.js';
import type { PermissionRule } from '$src/types/agents.js';
import { PERMISSION_ACTIONS, PERMISSION_EFFECTS } from '$src/utils/constants.js';
import { permissionView, serializePermissionRules } from './agentForm.js';

let {
  permission,
  onchange,
  error = '',
}: { permission: string; onchange: (value: string) => void; error?: string } = $props();

const i18n = getI18n();
const view = $derived(permissionView(permission));

function commit(rules: PermissionRule[]) {
  onchange(serializePermissionRules(rules));
}
function update(index: number, field: keyof PermissionRule, value: string) {
  if (view.mode !== 'rules') return;
  commit(view.rules.map((rule, i) => (i === index ? { ...rule, [field]: value } : rule)));
}
function move(index: number, delta: number) {
  if (view.mode !== 'rules') return;
  const rules = [...view.rules];
  const target = index + delta;
  if (target < 0 || target >= rules.length) return;
  const [rule] = rules.splice(index, 1);
  rules.splice(target, 0, rule as PermissionRule);
  commit(rules);
}
function remove(index: number) {
  if (view.mode !== 'rules') return;
  commit(view.rules.filter((_, i) => i !== index));
}
function add() {
  const rules = view.mode === 'rules' ? view.rules : [];
  commit([...rules, { action: '*', resource: '*', effect: 'ask' }]);
}
</script>

<fieldset class="form-section">
  <legend>{i18n.t('agents.permissions')}</legend>
  {#if view.mode === 'rules'}
    <div class="permission-rule-head" aria-hidden="true">
      <span>{i18n.t('agents.permissionAction')}</span>
      <span>{i18n.t('agents.permissionResource')}</span>
      <span>{i18n.t('agents.permissionEffect')}</span>
      <span></span>
    </div>
    {#each view.rules as rule, index (index)}
      <div class="permission-rule-row">
        <input
          aria-label={`${i18n.t('agents.permissionAction')} ${index + 1}`}
          list="permission-action-options"
          value={rule.action}
          oninput={(event) => update(index, 'action', event.currentTarget.value)}
        >
        <input
          aria-label={`${i18n.t('agents.permissionResource')} ${index + 1}`}
          value={rule.resource}
          oninput={(event) => update(index, 'resource', event.currentTarget.value)}
        >
        <Select
          ariaLabel={`${i18n.t('agents.permissionEffect')} ${index + 1}`}
          value={rule.effect}
          onchange={(value) => update(index, 'effect', value)}
          options={PERMISSION_EFFECTS.map((effect) => ({ value: effect }))}
        />
        <span class="permission-rule-actions">
          <Button
            type="button"
            size="icon-sm"
            variant="ghost"
            aria-label={i18n.t('agents.moveRuleUp')}
            disabled={index === 0}
            onclick={() => move(index, -1)}
            ><ChevronUp size={14} /></Button
          ><Button
            type="button"
            size="icon-sm"
            variant="ghost"
            aria-label={i18n.t('agents.moveRuleDown')}
            disabled={index === view.rules.length - 1}
            onclick={() => move(index, 1)}
            ><ChevronDown size={14} /></Button
          ><Button
            type="button"
            size="icon-sm"
            variant="ghost"
            aria-label={i18n.t('common.remove')}
            onclick={() => remove(index)}
            ><X size={14} /></Button
          >
        </span>
      </div>
    {/each}
    <datalist id="permission-action-options">
      {#each PERMISSION_ACTIONS as action}
        <option value={action}></option>
      {/each}
    </datalist>
    <Button type="button" size="sm" variant="outline" class="permission-add" onclick={add}
      ><Plus size={14} /> {i18n.t('agents.addPermissionRule')}</Button
    >
  {:else if view.mode === 'json'}
    <p class="muted">
      {i18n.t('agents.permissionsGlobHint')}
    </p>
  {/if}
  <div class="field full">
    <label for="permission-json">{i18n.t('agents.permissionsJson')}</label
    ><textarea
      id="permission-json"
      value={permission}
      oninput={(event) => onchange(event.currentTarget.value)}
      aria-invalid={error || view.mode === 'invalid' ? 'true' : undefined}
      aria-describedby={error || view.mode === 'invalid' ? 'permission-json-error' : undefined}
    ></textarea>
    {#if error || view.mode === 'invalid'}
      <small id="permission-json-error" class="field-error" role="alert"
        >{error || i18n.t('agents.permissionsInvalid')}</small
      >
    {/if}
    <small class="muted">{i18n.t('agents.permissionsHint')}</small>
  </div>
</fieldset>
