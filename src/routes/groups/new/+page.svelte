<script lang="ts">
  import { Button } from '$lib/components/ui/button/index.js';
  import GroupForm from '$lib/components/app/GroupForm.svelte';
  import PageHead from '$lib/components/app/PageHead.svelte';
</script>

<svelte:head><title>New group · opencode-mom</title></svelte:head>
<PageHead eyebrow="CONTROL PLANE / GROUPS" title="New group">
  {#snippet actions()}<Button href="/groups" variant="ghost" size="sm">Group Directory</Button>{/snippet}
</PageHead>
<GroupForm />
