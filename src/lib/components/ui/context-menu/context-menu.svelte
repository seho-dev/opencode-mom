<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  let {
    children,
    class: className,
    ...restProps
  }: HTMLAttributes<HTMLDivElement> & { children?: Snippet; class?: string } = $props();
</script>

<div data-slot="context-menu" class={className} {...restProps}>
  {@render children?.()}
</div>
