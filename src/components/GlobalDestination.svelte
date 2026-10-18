<script lang="ts">
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';

const config = getConfig();
const i18n = getI18n();
</script>
{#if config.selectedProjectId}
  <div class="state-banner mb-4" role="status">{i18n.t('scope.globalDestination')}</div>
{/if}
