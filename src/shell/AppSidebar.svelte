<script lang="ts">
import { ExternalLink } from '@lucide/svelte';
import { onDestroy } from 'svelte';
import { page } from '$app/state';
import { Button } from '$src/components/button/index.js';
import { readError } from '$src/components/configuration/read-error.js';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import { version } from '../../package.json';
import { isNavigationItemActive, navigationItems } from './navigation.js';
import { toast } from './toast.svelte.js';

const config = getConfig();
const i18n = getI18n();
const brand = i18n.t('header.brand');
const latestVersion = $derived(config.updateCheck?.updateAvailable ? config.updateCheck.latestVersion : null);
let opening = $state(false);
let request = 0;

async function openRelease() {
  if (opening || !latestVersion) return;

  const seq = ++request;
  opening = true;

  try {
    await config.openProjectPage('releases');
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

<aside class="sidebar" aria-label={i18n.t('header.mainNav')}>
  <a class="brand" href="/"><span class="brand-mark">{brand.slice(0, 1)}</span>{brand.slice(1)}</a>
  <nav class="nav">
    {#each navigationItems as item}
      <a href={item.href} aria-current={isNavigationItemActive(page.url.pathname, item.href) ? 'page' : undefined}
        ><item.icon size={16} />{i18n.t(item.labelKey)}</a
      >
    {/each}
  </nav>
  <div class="system-status flex flex-wrap items-center justify-between gap-x-2 gap-y-1">
    <span>{i18n.t('header.version', { version })}</span>
    {#if latestVersion}
      <Button
        variant="ghost"
        size="sm"
        class="h-6 min-w-0 max-w-full gap-1 px-1.5 text-[var(--accent-primary)] [&_svg]:size-3"
        aria-label={i18n.t('header.openRelease', { version: latestVersion })}
        title={i18n.t('header.openRelease', { version: latestVersion })}
        aria-busy={opening}
        disabled={opening}
        onclick={openRelease}
      >
        <span class="truncate">{i18n.t('header.updateAvailable', { version: latestVersion })}</span>
        <ExternalLink size={12} aria-hidden="true" />
      </Button>
    {/if}
  </div>
</aside>
