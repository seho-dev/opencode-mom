<script lang="ts">
  import { Button } from '$lib/components/ui/button/index.js';
  import GroupForm from '$lib/components/app/GroupForm.svelte';
  import PageHead from '$lib/components/app/PageHead.svelte';
  import { getI18n } from '$lib/features/i18n/context.js';

  const i18n = getI18n();
</script>

<svelte:head><title>{i18n.t('groups.newMetaTitle')}</title></svelte:head>
<PageHead eyebrow={i18n.t('groups.eyebrow')} title={i18n.t('groups.new')}>
  {#snippet actions()}<Button href="/groups" variant="ghost" size="sm">{i18n.t('groups.directoryButton')}</Button
    >{/snippet}
</PageHead>
<GroupForm />
