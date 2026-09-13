<script lang="ts">
  import Select from './Select.svelte';
  import { PERMISSION_ACTIONS, PERMISSION_KEYS } from '$lib/features/config/constants.js';
  import { permissionView, setPermissionAction } from '$lib/features/config/agentForm.js';

  let { permission = $bindable('{}') }: { permission: string } = $props();

  const view = $derived(permissionView(permission));
</script>

<fieldset class="form-section">
  <legend>Permissions</legend>
  {#if view.mode === 'rows'}<div class="permission-rows">
      {#each PERMISSION_KEYS as key (key)}<div class="permission-row">
          <label for={`permission-${key}`}>{key}</label><Select
            id={`permission-${key}`}
            value={view.values[key] ?? ''}
            onchange={(value) => (permission = setPermissionAction(permission, key, value))}
            options={[{ value: '', label: 'Inherit' }, ...PERMISSION_ACTIONS.map((action) => ({ value: action }))]}
          />
        </div>{/each}
    </div>{:else if view.mode === 'json'}<p class="muted">
      Some permission values are glob rules. Edit them with the JSON below so those rules are preserved.
    </p>{/if}
  <div class="field full">
    <label for="permission-json">Permissions JSON</label><textarea id="permission-json" bind:value={permission}
    ></textarea>
    {#if view.mode === 'invalid'}<small class="field-error" role="alert">Permissions must be a valid JSON object.</small
      >{/if}<small class="muted"
      >Values are allow, ask or deny. A key may map to a glob-to-action object; the last matching rule wins.</small
    >
  </div>
</fieldset>
