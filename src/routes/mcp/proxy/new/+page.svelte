<script lang="ts">
import { goto } from '$app/navigation';
import FormPage from '$src/components/FormPage.svelte';
import PageHead from '$src/components/PageHead.svelte';
import { getI18n } from '$src/i18n/context.js';
import { toast } from '$src/shell/toast.svelte.js';
import ProxyServerForm from '../../ProxyServerForm.svelte';

const i18n = getI18n();

async function handleSaved() {
  toast({ variant: 'success', description: i18n.t('mcp.proxy.form.save') });
  await goto('/mcp?tab=proxy');
}

async function handleCancel() {
  await goto('/mcp?tab=proxy');
}
</script>

<svelte:head><title>{i18n.t('mcp.proxy.form.createTitle')}</title></svelte:head>
<FormPage>
  <PageHead eyebrow={i18n.t('mcp.proxy.eyebrow')} title={i18n.t('mcp.proxy.form.createTitle')} />
  <ProxyServerForm dialog={false} onSaved={handleSaved} onCancel={handleCancel} />
</FormPage>
