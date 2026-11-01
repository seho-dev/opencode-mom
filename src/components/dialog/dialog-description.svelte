<script lang="ts">
import { Dialog as DialogPrimitive } from 'bits-ui';
import { cn } from '$src/utils/index.js';

let { ref = $bindable(null), class: className, ...restProps }: DialogPrimitive.DescriptionProps = $props();
</script>

<DialogPrimitive.Description
  bind:ref
  data-slot="dialog-description"
  class={cn('text-[12px] leading-5 text-[var(--text-secondary)]', className)}
  {...restProps}
/>
