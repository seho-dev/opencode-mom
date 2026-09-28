<script lang="ts">
import PageHead from '$src/components/PageHead.svelte';
import { getI18n } from '$src/i18n/context.js';
import ProviderForm from '$src/routes/providers/ProviderForm.svelte';

const i18n = getI18n();
</script>

<svelte:head><title>{i18n.t('providers.newMetaTitle')}</title></svelte:head>
<PageHead eyebrow={i18n.t('providers.eyebrow')} title={i18n.t('providers.newTitle')} />
<ProviderForm />
