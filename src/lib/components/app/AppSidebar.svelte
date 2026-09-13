<script lang="ts">
  import { page } from '$app/state';
  import { getI18n } from '$lib/features/i18n/context.js';
  import { isNavigationItemActive, navigationItems } from './navigation.js';

  const i18n = getI18n();
  const brand = i18n.t('header.brand');
</script>

<aside class="sidebar" aria-label={i18n.t('header.mainNav')}>
  <a class="brand" href="/"><span class="brand-mark">{brand.slice(0, 1)}</span>{brand.slice(1)}</a>
  <nav class="nav">
    {#each navigationItems as item}<a
        href={item.href}
        aria-current={isNavigationItemActive(page.url.pathname, item.href) ? 'page' : undefined}
        ><item.icon size={16} />{i18n.t(item.labelKey)}</a
      >{/each}
  </nav>
  <div class="system-status">{i18n.t('header.version')}</div>
</aside>
