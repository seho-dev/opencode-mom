<script lang="ts">
import PageHead from '$src/components/PageHead.svelte';
import { getI18n } from '$src/i18n/context.js';
import AgentForm from '../AgentForm.svelte';

const i18n = getI18n();
</script>

<svelte:head><title>{i18n.t('agents.newMetaTitle')}</title></svelte:head>
<PageHead eyebrow={i18n.t('agents.eyebrow')} title={i18n.t('agents.newTitle')} />
<AgentForm />
