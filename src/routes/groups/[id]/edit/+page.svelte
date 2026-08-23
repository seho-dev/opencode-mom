<script lang="ts">
  import { goto } from '$app/navigation';
  import { Button } from '$lib/components/ui/button/index.js';
  import GroupForm from '$lib/components/app/GroupForm.svelte';
  import PageHead from '$lib/components/app/PageHead.svelte';
  import { getConfig } from '$lib/features/config/context.js';

  const catalog = getConfig();
  let { data } = $props();
  const group = $derived(catalog.groups.find((entry) => entry.id === data.id));
  $effect(() => {
    if (!group) goto('/groups');
  });
</script>

<svelte:head><title>Edit group · opencode-mom</title></svelte:head>
{#if group}<PageHead eyebrow="CONTROL PLANE / GROUPS" title="Edit group"
    ><Button href="/groups" variant="ghost" size="sm">Group Directory</Button></PageHead
  ><GroupForm {group} />{/if}
