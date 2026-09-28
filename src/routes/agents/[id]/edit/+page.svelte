<script lang="ts">
import { page } from '$app/state';
import PageHead from '$src/components/PageHead.svelte';
import { getI18n } from '$src/i18n/context.js';
import AgentForm from '../../AgentForm.svelte';

const i18n = getI18n();
const id = $derived(decodeURIComponent(page.params.id ?? ''));
</script>

<svelte:head><title>{i18n.t('agents.editMetaTitle')}</title></svelte:head>
<PageHead eyebrow={i18n.t('agents.eyebrow')} title={i18n.t('agents.editTitle')} />
<AgentForm {id} />
