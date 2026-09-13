<script lang="ts">
  import { getI18n } from '$lib/features/i18n/context.js';
  import { toast } from './toast.svelte.js';
  import { getConfig } from '$lib/features/config/context.js';

  const i18n = getI18n();
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
        description: i18n.t('feedback.conflictDescription'),
        actions: [
          { label: i18n.t('feedback.keepDraftReload'), onclick: () => config.reloadKeepingDraft() },
          { label: i18n.t('feedback.reviewMerge'), onclick: () => config.continueEditing() },
          { label: i18n.t('feedback.discardRefresh'), onclick: () => config.discardDraftAndRefresh() },
        ],
      });
    } else if (err.code === 'busy') {
      toast({
        ...base,
        description: i18n.t('feedback.busyDescription'),
        action: { label: i18n.t('feedback.continueEditing'), onclick: () => config.continueEditing() },
      });
    } else if (err.code === 'references_blocked') {
      toast({
        ...base,
        description: i18n.t('feedback.referencesDescription'),
        action: { label: i18n.t('feedback.backToDraft'), onclick: () => config.continueEditing() },
      });
    } else {
      toast({
        ...base,
        description: config.draftRecovery
          ? i18n.t('feedback.preservedDescription')
          : i18n.t('feedback.retryDescription'),
        action: { label: i18n.t('feedback.continueEditing'), onclick: () => config.continueEditing() },
      });
    }
  });

  // Watch for success notices and surface them as success toasts.
  $effect(() => {
    const notice = config.notice;
    if (!notice) return;
    // ponytail: config.notice is currently only set by the group-switch flow; revisit if other notices appear.
    toast({ variant: 'success', description: i18n.t('toast.groupSwitched') });
    config.clearNotice();
  });
</script>
