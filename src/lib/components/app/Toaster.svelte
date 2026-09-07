<script lang="ts">
  import { Toast } from '$lib/components/ui/toast/index.js';
  import { dismissToast, getToasts } from './toast.svelte.js';

  const toasts = getToasts();
</script>

<div class="toaster-viewport">
  {#each toasts as item (item.id)}
    <div class="toaster-item" class:toaster-leave={item.leaving}>
      <Toast
        variant={item.variant}
        title={item.title}
        description={item.description}
        actions={item.actions}
        onClose={() => dismissToast(item.id)}
      />
    </div>
  {/each}
</div>

<style>
  .toaster-viewport {
    position: fixed;
    top: 16px;
    right: 16px;
    z-index: 100;
    display: flex;
    flex-direction: column;
    gap: 12px;
    width: min(100% - 32px, 22rem);
    pointer-events: none;
  }
  .toaster-item {
    pointer-events: auto;
    animation: toaster-in 180ms ease-out;
  }
  .toaster-item.toaster-leave {
    opacity: 0;
    transform: translateX(8px);
    transition:
      opacity 200ms ease,
      transform 200ms ease;
  }
  @keyframes toaster-in {
    from {
      opacity: 0;
      transform: translateY(-8px) scale(0.98);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
</style>
