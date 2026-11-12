<script lang="ts">
import { page } from '$app/state';
import { Button } from '$src/components/button/index.js';
import FormPage from '$src/components/FormPage.svelte';
import PageHead from '$src/components/PageHead.svelte';
import ResourceScopeNotice from '$src/components/ResourceScopeNotice.svelte';
import { getI18n } from '$src/i18n/context.js';
import McpForm from '../McpForm.svelte';

const i18n = getI18n();
const projectId = $derived(page.url.searchParams.get('project'));
const readonly = $derived(page.url.searchParams.get('readonly') === '1');
</script>

<svelte:head><title>{i18n.t('mcp.newMetaTitle')}</title></svelte:head>
<FormPage active={projectId === null}>
  <PageHead eyebrow={i18n.t('mcp.eyebrow')} title={i18n.t('mcp.newTitle')} />
  {#if projectId !== null}
    <ResourceScopeNotice {projectId} writable />
    <div class="state-banner" role="status">{i18n.t('mcp.projectEditOnly')}</div>
    <Button href="/mcp" variant="outline">{i18n.t('mcp.title')}</Button>
  {:else}
    {#key readonly}
      <McpForm {readonly} />
    {/key}
  {/if}
</FormPage>
