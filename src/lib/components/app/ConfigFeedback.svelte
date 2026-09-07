<script lang="ts">
  import { toast } from './toast.svelte.js';
  import { getConfig } from '$lib/features/config/context.js';
  const config = getConfig();

  // Watch for command errors and surface them as error toasts, carrying the
  // branch-specific recovery action(s). Mounted globally in AppShell.
  $effect(() => {
    const err = config.error;
    if (!err) return;
    const base = { variant: 'error', title: err.message } as const;
    if (err.code === 'conflict') {
      toast({
        ...base,
        description: 'The local draft is kept in the current form and was not overwritten.',
        actions: [
          { label: 'Keep draft and reload', onclick: () => config.reloadKeepingDraft() },
          { label: 'Review and merge manually', onclick: () => config.continueEditing() },
          { label: 'Discard draft and refresh', onclick: () => config.discardDraftAndRefresh() },
        ],
      });
    } else if (err.code === 'busy') {
      toast({
        ...base,
        description:
          'Another configuration operation is still running. The local draft is kept; retry in the original form later.',
        action: { label: 'Continue editing', onclick: () => config.continueEditing() },
      });
    } else if (err.code === 'references_blocked') {
      toast({
        ...base,
        description: 'Remove the references first.',
        action: { label: 'Back to draft', onclick: () => config.continueEditing() },
      });
    } else {
      toast({
        ...base,
        description: config.draftRecovery
          ? 'The request content is preserved; no automatic merge was performed. Review it in the original form and retry.'
          : 'Review the current page and retry.',
        action: { label: 'Continue editing', onclick: () => config.continueEditing() },
      });
    }
  });

  // Watch for success notices and surface them as success toasts.
  $effect(() => {
    const notice = config.notice;
    if (!notice) return;
    toast({ variant: 'success', description: notice });
    config.clearNotice();
  });
</script>
