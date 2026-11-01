<script lang="ts">
import FormActions from '../FormActions.svelte';
import FormPage from '../FormPage.svelte';
import PageHead from '../PageHead.svelte';

let { active = true, onsubmit }: { active?: boolean; onsubmit: () => void } = $props();
</script>

<FormPage {active}>
  <PageHead eyebrow="Provider" title="Edit provider" />
  <form
    class="panel form-panel"
    onsubmit={(event) => {
      event.preventDefault();
      onsubmit();
    }}
  >
    <div class="form-body form-grid">
      <label>Name<input name="name" required value="example"></label>
    </div>
    <FormActions>
      <button type="button">Cancel</button>
      <button type="submit">Save</button>
    </FormActions>
  </form>
</FormPage>
