<script lang="ts">
  import { goto } from '$app/navigation';
  import GroupForm from '$lib/components/app/GroupForm.svelte';
  import PageHead from '$lib/components/app/PageHead.svelte';
  import { getConfig } from '$lib/features/config/context.js';
  import { getI18n } from '$lib/features/i18n/context.js';

  const catalog = getConfig();
  const i18n = getI18n();
  let { data } = $props();
  const group = $derived(catalog.groups.find((entry) => entry.id === data.id));
  $effect(() => {
    if (!group) goto('/groups');
  });
</script>

<svelte:head><title>{i18n.t('groups.editMetaTitle')}</title></svelte:head>
{#if group}<PageHead eyebrow={i18n.t('groups.eyebrow')} title={i18n.t('groups.editTitle')} /><GroupForm {group} />{/if}
