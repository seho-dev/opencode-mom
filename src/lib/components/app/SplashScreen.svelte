<script lang="ts">
  import { getI18n } from '$lib/features/i18n/context.js';

  const i18n = getI18n();
  const brand = i18n.t('splash.brand');
  let { visible = true }: { visible?: boolean } = $props();

  let root: HTMLElement | null = null;
  let hideTimer: ReturnType<typeof setTimeout> | undefined;
  let shownAt = 0;
  const MIN_VISIBLE_MS = 3000;

  // Toggle the exit state via classList instead of unmounting: the overlay must
  // stay mounted so it can show again on later `loading` cycles (refresh, group
  // switch). `.splash-exit` sets `pointer-events: none`, so it never intercepts
  // clicks while hidden. Keep the overlay up for at least MIN_VISIBLE_MS so a
  // fast config load still shows the brand instead of a single-frame flash.
  $effect(() => {
    const clearHideTimer = () => {
      if (hideTimer !== undefined) {
        clearTimeout(hideTimer);
        hideTimer = undefined;
      }
    };

    if (visible) {
      clearHideTimer();
      shownAt = Date.now();
      root?.classList.remove('splash-exit');
      return clearHideTimer;
    }

    const remaining = MIN_VISIBLE_MS - (Date.now() - shownAt);
    if (remaining <= 0) {
      root?.classList.add('splash-exit');
      return;
    }
    hideTimer = setTimeout(() => {
      hideTimer = undefined;
      root?.classList.add('splash-exit');
    }, remaining);
    return clearHideTimer;
  });
</script>

<div class="splash-root" bind:this={root} aria-hidden="true">
  <div class="splash-stage">
    <h1 class="splash-title"><span class="splash-title-accent">{brand.slice(0, 1)}</span>{brand.slice(1)}</h1>
  </div>
</div>

<style>
  /* Full-viewport overlay that sits above the AppShell while the config store loads. */
  .splash-root {
    position: fixed;
    inset: 0;
    z-index: 100;
    overflow: hidden;
    contain: layout style paint;
    background: var(--surface-app);
    color: var(--text-primary);
    font-family: var(--font-primary);
    box-sizing: border-box;
  }
  .splash-root *,
  .splash-root *::before,
  .splash-root *::after {
    box-sizing: border-box;
  }

  /* centered stage: single soft reveal for the whole composition */
  .splash-stage {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 32px;
    animation: splash-enter 0.6s ease both;
  }
  @keyframes splash-enter {
    from {
      opacity: 0.35;
      transform: translateY(10px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }

  /* brand title, matching the sidebar "O"PENCODE-MOM lockup */
  .splash-title {
    margin: 28px 0 0;
    font-family: var(--font-heading);
    font-size: 30px;
    font-weight: 600;
    letter-spacing: 0.12em;
    color: var(--text-primary);
  }
  .splash-title-accent {
    color: var(--accent-primary);
    text-shadow: 0 0 24px rgba(0, 229, 255, 0.35);
  }

  /* fade-out state, toggled at runtime via classList, so keep these global to
     prevent the compiler from dropping them as "unused" */
  :global(.splash-root.splash-exit) {
    opacity: 0;
    transition: opacity 460ms ease;
    pointer-events: none;
  }

  @media (prefers-reduced-motion: reduce) {
    .splash-root *,
    .splash-root *::before,
    .splash-root *::after {
      animation: none !important;
      animation-delay: 0s !important;
      transition: none !important;
    }
    .splash-stage {
      opacity: 1;
      transform: none;
    }
  }
</style>
