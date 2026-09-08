<script lang="ts">
  let { visible = true }: { visible?: boolean } = $props();

  let root: HTMLElement | null = null;
  let removalTimer: ReturnType<typeof setTimeout> | undefined;
  let exitStarted = false;

  $effect(() => {
    if (visible) {
      // Show again: cancel any pending removal and drop the exit state.
      if (removalTimer !== undefined) {
        clearTimeout(removalTimer);
        removalTimer = undefined;
      }
      exitStarted = false;
      root?.classList.remove('splash-exit');
      return;
    }

    // Fade out, pause the internal animations, then drop the overlay from the
    // DOM once the fade has finished so it never intercepts pointer events.
    if (exitStarted) return;
    exitStarted = true;
    root?.classList.add('splash-exit');
    removalTimer = setTimeout(() => {
      root?.remove();
      removalTimer = undefined;
    }, 520);

    return () => {
      if (removalTimer !== undefined) {
        clearTimeout(removalTimer);
        removalTimer = undefined;
      }
    };
  });
</script>

<div class="splash-root" bind:this={root} aria-hidden="true">
  <div class="splash-stage">
    <h1 class="splash-title"><span class="splash-title-accent">O</span>PENCODE-MOM</h1>
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
    animation: splash-enter 0.9s ease 0.2s both;
  }
  @keyframes splash-enter {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }

  @keyframes splash-breathe {
    from {
      transform: scale(1);
    }
    to {
      transform: scale(1.05);
    }
  }
  @keyframes splash-glow {
    from {
      opacity: 0.45;
    }
    to {
      opacity: 0.85;
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
  :global(.splash-root.splash-exit *) {
    animation-play-state: paused;
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
