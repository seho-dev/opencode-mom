<script lang="ts">
import { onDestroy } from 'svelte';
import { Button } from '$src/components/button/index.js';
import { readError } from '$src/components/configuration/read-error.js';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import AppearanceControls from './AppearanceControls.svelte';
import MobileNavigation from './MobileNavigation.svelte';
import { toast } from './toast.svelte.js';

const config = getConfig();
const i18n = getI18n();
let opening = $state(false);
let request = 0;

async function openRepository() {
  if (opening) return;
  const seq = ++request;
  opening = true;
  try {
    await config.openProjectPage('repository');
  } catch (cause) {
    if (seq === request) {
      toast({ variant: 'error', description: i18n.t('header.projectOpenFailed', { message: readError(cause) }) });
    }
  } finally {
    if (seq === request) opening = false;
  }
}
onDestroy(() => {
  request += 1;
});
</script>

<header class="header">
  <div class="header-main">
    <MobileNavigation />
  </div>
  <div class="header-meta">
    <div class="flex items-center gap-2">
      <AppearanceControls />
      <Button
        variant="outline"
        size="icon-sm"
        class="relative size-[var(--control-height-small)] p-0 text-[var(--text-secondary)] active:translate-y-0 [&_svg]:size-[14px] max-[760px]:after:absolute max-[760px]:after:inset-x-0 max-[760px]:after:-inset-y-[9px]"
        aria-label={i18n.t('header.openProject')}
        title={i18n.t('header.openProject')}
        aria-busy={opening}
        disabled={opening}
        onclick={openRepository}
      >
        <svg viewBox="0 0 24 24" width="14" height="14" fill="currentColor" aria-hidden="true">
          <path
            d="M12 .75a11.25 11.25 0 0 0-3.558 21.922c.563.105.77-.244.77-.542 0-.267-.01-.974-.015-1.912-3.13.68-3.79-1.508-3.79-1.508-.512-1.302-1.25-1.649-1.25-1.649-1.023-.7.078-.686.078-.686 1.13.08 1.725 1.16 1.725 1.16 1.006 1.724 2.64 1.226 3.283.938.102-.729.393-1.226.715-1.508-2.499-.284-5.126-1.25-5.126-5.564 0-1.229.439-2.233 1.16-3.02-.117-.285-.503-1.43.11-2.98 0 0 .945-.303 3.094 1.154A10.783 10.783 0 0 1 12 6.176c.956.005 1.918.13 2.816.379 2.148-1.457 3.09-1.154 3.09-1.154.615 1.55.229 2.695.112 2.98.722.787 1.158 1.791 1.158 3.02 0 4.325-2.632 5.277-5.14 5.555.404.35.765 1.042.765 2.1 0 1.515-.014 2.737-.014 3.109 0 .3.202.652.774.541A11.25 11.25 0 0 0 12 .75Z"
          />
        </svg>
      </Button>
    </div>
  </div>
</header>
