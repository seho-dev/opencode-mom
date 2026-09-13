<script lang="ts">
  import { Menu, X } from '@lucide/svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { tick } from 'svelte';
  import { getI18n } from '$lib/features/i18n/context.js';
  import { isNavigationItemActive, navigationItems } from './navigation.js';

  const i18n = getI18n();
  const brand = i18n.t('header.brand');
  let open = $state(false);
  let menuButton: HTMLButtonElement | null = $state(null);
  let closeButton: HTMLButtonElement | null = $state(null);
  let navigationLinks: HTMLAnchorElement[] = $state([]);

  async function openDrawer() {
    open = true;
    await tick();
    closeButton?.focus();
  }
  async function closeDrawer() {
    open = false;
    await tick();
    menuButton?.focus();
  }
  async function navigateFromDrawer(event: MouseEvent, href: string) {
    event.preventDefault();
    await goto(href);
    open = false;
  }
  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      closeDrawer();
      return;
    }
    if (event.key !== 'Tab') return;
    const focusable = [closeButton, ...navigationLinks];
    const currentIndex = focusable.indexOf(document.activeElement as HTMLButtonElement | HTMLAnchorElement);
    const nextIndex = event.shiftKey
      ? currentIndex <= 0
        ? focusable.length - 1
        : currentIndex - 1
      : currentIndex === focusable.length - 1
        ? 0
        : currentIndex + 1;
    event.preventDefault();
    focusable[nextIndex]?.focus();
  }
</script>

<Button
  bind:ref={menuButton}
  class="mobile-menu"
  variant="ghost"
  size="icon"
  aria-label={i18n.t('header.openMenu')}
  aria-expanded={open}
  onclick={openDrawer}><Menu size={20} /></Button
>
{#if open}<div
    class="drawer open"
    role="dialog"
    aria-modal="true"
    aria-label={i18n.t('header.mobileNav')}
    tabindex="-1"
    onclick={(event) => {
      if (event.currentTarget === event.target) closeDrawer();
    }}
    onkeydown={handleKeydown}
  >
    <aside class="drawer-panel" aria-label={i18n.t('header.mobileNav')}>
      <div class="drawer-top">
        <span class="brand"><span class="brand-mark">{brand.slice(0, 1)}</span>{brand.slice(1)}</span><Button
          bind:ref={closeButton}
          class="mobile-menu"
          variant="ghost"
          size="icon"
          aria-label={i18n.t('header.closeMenu')}
          onclick={closeDrawer}><X size={20} /></Button
        >
      </div>
      <nav class="nav">
        {#each navigationItems as item, index}<a
            bind:this={navigationLinks[index]}
            href={item.href}
            aria-current={isNavigationItemActive(page.url.pathname, item.href) ? 'page' : undefined}
            onclick={(event) => navigateFromDrawer(event, item.href)}><item.icon size={16} />{i18n.t(item.labelKey)}</a
          >{/each}
      </nav>
      <div class="system-status">{i18n.t('header.version')}</div>
    </aside>
  </div>{/if}
