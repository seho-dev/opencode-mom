<script lang="ts">
import { page } from '$app/state';
import { Button } from '$src/components/button/index.js';
import { getConfig } from '$src/config/context.js';
import { getI18n } from '$src/i18n/context.js';
import { isNavigationItemActive, navigationItems } from './navigation.js';

const config = getConfig();
const i18n = getI18n();
const brand = i18n.t('header.brand');
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
  <div class="system-status flex items-center justify-between gap-2">
    <span>{i18n.t('header.version')}</span>
    <Button
      variant="outline"
      size="sm"
      class="shrink-0 px-2 text-[var(--text-secondary)] hover:text-[var(--accent-primary)]"
      disabled={config.reloading || config.switching}
      onclick={() => void config.reloadOpencode().catch(() => {})}
      >{i18n.t(config.reloading ? 'header.reloading' : 'header.reloadOpencode')}</Button
    >
  </div>
</aside>
