<script lang="ts">
import { Checkbox } from '../index.js';

let {
  initialChecked = false,
  initialIndeterminate = false,
  disabled = false,
  onCheckedChange,
  onIndeterminateChange,
  onSubmitFormData,
}: {
  initialChecked?: boolean;
  initialIndeterminate?: boolean;
  disabled?: boolean;
  onCheckedChange?: (checked: boolean) => void;
  onIndeterminateChange?: (indeterminate: boolean) => void;
  onSubmitFormData?: (entries: Array<[string, FormDataEntryValue]>) => void;
} = $props();

let checked = $state(false);
let indeterminate = $state(false);
let controlRef = $state<HTMLButtonElement | null>(null);
let initialized = false;

$effect.pre(() => {
  if (initialized) return;
  initialized = true;
  checked = initialChecked;
  indeterminate = initialIndeterminate;
});

export function getControlRef() {
  return controlRef;
}

export function getCheckedState() {
  return checked;
}

export function getIndeterminateState() {
  return indeterminate;
}
</script>

<form
  onsubmit={(event) => {
    event.preventDefault();
    onSubmitFormData?.(Array.from(new FormData(event.currentTarget).entries()));
  }}
>
  <div class="check-row">
    <Checkbox
      id="harness-checkbox"
      bind:ref={controlRef}
      bind:checked
      bind:indeterminate
      name="featureFlag"
      value="enabled"
      {disabled}
      {onCheckedChange}
      {onIndeterminateChange}
    />
    <label for="harness-checkbox">Enable feature</label>
  </div>
  <button type="submit">Submit form</button>
</form>
