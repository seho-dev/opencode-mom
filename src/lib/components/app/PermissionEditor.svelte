<script lang="ts">
  import Select from './Select.svelte';
  import { getI18n } from '$lib/features/i18n/context.js';
  import { PERMISSION_ACTIONS, PERMISSION_KEYS } from '$lib/features/config/constants.js';
  import { permissionView, setPermissionAction } from '$lib/features/config/agentForm.js';

  let { permission = $bindable('{}') }: { permission: string } = $props();

  const i18n = getI18n();
  const view = $derived(permissionView(permission));
</script>

<fieldset class="form-section">
  <legend>{i18n.t('agents.permissions')}</legend>
  {#if view.mode === 'rows'}<div class="permission-rows">
      {#each PERMISSION_KEYS as key (key)}<div class="permission-row">
          <label for={`permission-${key}`}>{key}</label><Select
            id={`permission-${key}`}
            value={view.values[key] ?? ''}
            onchange={(value) => (permission = setPermissionAction(permission, key, value))}
            options={[
              { value: '', label: i18n.t('agents.inherit') },
              ...PERMISSION_ACTIONS.map((action) => ({ value: action })),
            ]}
          />
        </div>{/each}
    </div>{:else if view.mode === 'json'}<p class="muted">
      {i18n.t('agents.permissionsGlobHint')}
    </p>{/if}
  <div class="field full">
    <label for="permission-json">{i18n.t('agents.permissionsJson')}</label><textarea
      id="permission-json"
      bind:value={permission}></textarea>
    {#if view.mode === 'invalid'}<small class="field-error" role="alert">{i18n.t('agents.permissionsInvalid')}</small
      >{/if}<small class="muted">{i18n.t('agents.permissionsHint')}</small>
  </div>
</fieldset>
