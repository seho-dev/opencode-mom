<script lang="ts">
  import PageHead from '$lib/components/app/PageHead.svelte';
  import { getI18n } from '$lib/features/i18n/context.js';

  const i18n = getI18n();
</script>

<svelte:head><title>{i18n.t('dashboard.metaTitle')}</title></svelte:head>
<PageHead eyebrow={i18n.t('dashboard.eyebrow')} title={i18n.t('dashboard.title')} />
<section class="terminal" aria-label={i18n.t('dashboard.statusLabel')}>
  <strong>$ {i18n.t('dashboard.status')}</strong><br />
  <span>{i18n.t('dashboard.underDevelopment')}</span>
</section>
