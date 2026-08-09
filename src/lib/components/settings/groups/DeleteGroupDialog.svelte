<script lang="ts">
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import { Button } from '$lib/components/ui/button';
  import * as Dialog from '$lib/components/ui/dialog';

  type Props = Readonly<{ groupName: string; disabled: boolean; onDelete: () => Promise<void> }>;
  let { groupName, disabled, onDelete }: Props = $props();
  let open = $state(false);

  const confirmDelete = async (): Promise<void> => {
    open = false;
    await onDelete();
  };
</script>

<Dialog.Root bind:open>
  <Dialog.Trigger>
    {#snippet child({ props })}
      <Button variant="destructive" disabled={disabled} {...props}><Trash2 size={16} aria-hidden="true" />Delete</Button>
    {/snippet}
  </Dialog.Trigger>
  <Dialog.Content interactOutsideBehavior="ignore" showCloseButton={false} aria-labelledby="delete-title">
    <Dialog.Header>
      <Dialog.Title id="delete-title">Delete {groupName}?</Dialog.Title>
      <Dialog.Description>This removes the group permanently. Target configuration files are not changed.</Dialog.Description>
    </Dialog.Header>
    <Dialog.Footer>
      <Dialog.Close>
        {#snippet child({ props })}<Button variant="secondary" {...props}>Cancel</Button>{/snippet}
      </Dialog.Close>
      <Button variant="destructive" onclick={() => void confirmDelete()}>Delete Group</Button>
    </Dialog.Footer>
  </Dialog.Content>
</Dialog.Root>
