<script lang="ts">
import PageHead from '$src/components/PageHead.svelte';
import { getI18n } from '$src/i18n/context.js';
import SkillForm from '../SkillForm.svelte';

const i18n = getI18n();
</script>

<svelte:head><title>{i18n.t('skills.newMetaTitle')}</title></svelte:head>
<PageHead eyebrow={i18n.t('skills.eyebrow')} title={i18n.t('skills.newTitle')} />
<p class="page-description">{i18n.t('skills.newDescription')}</p>
<SkillForm />
