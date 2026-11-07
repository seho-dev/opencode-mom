<script lang="ts">
import { page } from '$app/state';
import { Button } from '$src/components/button/index.js';
import FormPage from '$src/components/FormPage.svelte';
import PageHead from '$src/components/PageHead.svelte';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import type { Group } from '$src/types/groups.js';
import GroupForm from '../GroupForm.svelte';

const config = getConfig();
const i18n = getI18n();
const copyFrom = $derived(page.url.searchParams.get('copyFrom'));
const source = $derived(config.groups.find((entry) => entry.id === copyFrom));
let copy = $state.raw<{ sourceId: string; draft: Omit<Group, 'id' | 'updatedAt'> } | null>(null);

$effect(() => {
  if (!copyFrom) {
    copy = null;
    return;
  }
  if (copy?.sourceId === copyFrom) return;
  if (!source) {
    copy = null;
    return;
  }
  const group = structuredClone($state.snapshot(source));
  const names = new Set(config.groups.map((entry) => entry.name.trim().toLowerCase()));
  const baseName = i18n.t('groups.copyName', { name: group.name });
  let name = baseName;
  for (let suffix = 2; names.has(name.trim().toLowerCase()); suffix += 1) name = `${baseName} (${suffix})`;
  copy = {
    sourceId: copyFrom,
    draft: {
      name,
      description: group.description,
      type: group.type,
      openCodeAgentOverrides: group.openCodeAgentOverrides,
      slimAgentOverrides: group.slimAgentOverrides,
      omoAgentOverrides: group.omoAgentOverrides,
      omoCategoryMappings: group.omoCategoryMappings,
      isEnabled: group.isEnabled,
    },
  };
});
</script>

<svelte:head><title>{i18n.t(copyFrom ? 'groups.copyMetaTitle' : 'groups.newMetaTitle')}</title></svelte:head>
<FormPage>
  <PageHead eyebrow={i18n.t('groups.eyebrow')} title={i18n.t(copyFrom ? 'groups.copyTitle' : 'groups.new')} />
  {#if !copyFrom}
    <GroupForm />
  {:else if copy?.sourceId === copyFrom}
    {#key copyFrom}
      <GroupForm initialGroup={copy.draft} />
    {/key}
  {:else if config.loading || source}
    <p class="muted" role="status">{i18n.t('groups.loading')}</p>
  {:else}
    <div class="state-banner error" role="alert">{i18n.t('groups.copyNotFound')}</div>
    <Button href="/groups" variant="outline">{i18n.t('groups.directoryButton')}</Button>
  {/if}
</FormPage>
