<script lang="ts">
import { goto } from '$app/navigation';
import { page } from '$app/state';
import { Button } from '$src/components/button/index.js';
import FormPage from '$src/components/FormPage.svelte';
import PageHead from '$src/components/PageHead.svelte';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import type { ModelDef } from '$src/types/models.js';
import ModelForm from '../ModelForm.svelte';

const config = getConfig();
const i18n = getI18n();
const copyFrom = $derived(page.url.searchParams.get('copyFrom'));
const models = $derived(config.models());
const source = $derived(models.find((model) => model.ref === copyFrom));
let copy = $state.raw<{ sourceRef: string; providerId: string; draft: ModelDef } | null>(null);

$effect(() => {
  if (copyFrom === null) {
    copy = null;
    return;
  }
  if (copy?.sourceRef === copyFrom) return;
  if (!source) {
    copy = null;
    return;
  }
  const { ref, providerId, ...draft } = structuredClone($state.snapshot(source));
  const ids = new Set(models.filter((model) => model.providerId === providerId).map((model) => model.id));
  const baseId = `${draft.id}-copy`;
  let id = baseId;
  for (let suffix = 2; ids.has(id); suffix += 1) id = `${baseId}-${suffix}`;
  draft.id = id;
  copy = { sourceRef: ref, providerId, draft };
});

async function create(providerId: string, value: ModelDef) {
  await config.createModel(providerId, value);
  await goto('/models');
}
</script>

<svelte:head><title>{i18n.t(copyFrom === null ? 'models.newMetaTitle' : 'models.copyMetaTitle')}</title></svelte:head>
<FormPage>
  <PageHead
    eyebrow={i18n.t('models.eyebrow')}
    title={i18n.t(copyFrom === null ? 'models.newTitle' : 'models.copyTitle')}
  />
  {#if copyFrom !== null}
    {#if copy?.sourceRef === copyFrom}
      {#key copyFrom}
        <ModelForm
          mode="new"
          initial={copy.draft}
          defaultProviderId={copy.providerId}
          providers={config.providers}
          saving={config.saving}
          onSave={create}
        />
      {/key}
    {:else if config.loading || source}
      <p class="muted" role="status">{i18n.t('models.loadingConfiguration')}</p>
    {:else}
      <div class="state-banner error" role="alert">{i18n.t('models.copyNotFound')}</div>
      <Button href="/models" variant="outline">{i18n.t('models.directoryButton')}</Button>
    {/if}
  {:else if !config.loading && config.providers.length === 0}
    <div class="state-banner error" role="alert">
      {i18n.t('models.noProvider')}
    </div>
  {:else}
    <ModelForm mode="new" providers={config.providers} saving={config.saving} onSave={create} />
  {/if}
</FormPage>
