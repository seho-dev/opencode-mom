<script lang="ts">
  import { cn } from '$lib/utils.js';
  import Loader2Icon from '@lucide/svelte/icons/loader-2';
  import type { SVGAttributes } from 'svelte/elements';

  type SpinnerSize = 'sm' | 'default' | 'lg';

  const spinnerSizes: Readonly<Record<SpinnerSize, string>> = {
    sm: 'size-[var(--icon-sm)]',
    default: 'size-[var(--icon-md)]',
    lg: 'size-6',
  };

  let {
    class: className,
    size = 'default',
    ...restProps
  }: SVGAttributes<SVGSVGElement> & { size?: SpinnerSize } = $props();
</script>

<Loader2Icon
  data-slot="spinner"
  aria-hidden="true"
  focusable="false"
  class={cn('animate-spin motion-reduce:animate-none', spinnerSizes[size], className)}
  {...restProps}
/>
