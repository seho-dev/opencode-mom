<script lang="ts">
import { page } from '$app/state';
import FormPage from '$src/components/FormPage.svelte';
import PageHead from '$src/components/PageHead.svelte';
import { getI18n } from '$src/i18n/context.js';
import CommandForm from '../CommandForm.svelte';

const i18n = getI18n();
const projectId = $derived(page.url.searchParams.get('project'));
const readonly = $derived(page.url.searchParams.get('readonly') === '1');
</script>

<svelte:head><title>{i18n.t('commands.newMetaTitle')}</title></svelte:head>
<FormPage>
  <PageHead eyebrow={i18n.t('commands.eyebrow')} title={i18n.t('commands.newTitle')} />
  {#key JSON.stringify([projectId, readonly])}
    <CommandForm {projectId} {readonly} />
  {/key}
</FormPage>
