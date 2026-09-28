<script lang="ts">
import { goto } from '$app/navigation';
import { page } from '$app/state';
import PageHead from '$src/components/PageHead.svelte';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import GroupForm from '../../GroupForm.svelte';

const catalog = getConfig();
const i18n = getI18n();
const group = $derived(catalog.groups.find((entry) => entry.id === page.params.id));
$effect(() => {
  if (!group) goto('/groups');
});
</script>

<svelte:head><title>{i18n.t('groups.editMetaTitle')}</title></svelte:head>
{#if group}
  <PageHead eyebrow={i18n.t('groups.eyebrow')} title={i18n.t('groups.editTitle')} /><GroupForm {group} />
{/if}
