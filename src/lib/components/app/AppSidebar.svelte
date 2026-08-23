<script lang="ts">
  import { page } from '$app/state';
  import { isNavigationItemActive, navigationItems } from './navigation.js';
</script>

<aside class="sidebar" aria-label="Main navigation">
  <a class="brand" href="/"><span class="brand-mark">O</span>PENCODE-MOM</a>
  <nav class="nav">
    {#each navigationItems as item}<a
        href={item.href}
        aria-current={isNavigationItemActive(page.url.pathname, item.href) ? 'page' : undefined}
        ><item.icon size={16} />{item.label}</a
      >{/each}
  </nav>
  <div class="system-status"><span class="online">● SYSTEM ONLINE</span><br />VERSION 0.1.0</div>
</aside>
