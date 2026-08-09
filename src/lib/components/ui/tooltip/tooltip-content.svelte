<script lang="ts">
	import { Tooltip as TooltipPrimitive } from "bits-ui";
	import { cn } from "$lib/utils.js";
	import TooltipPortal from "./tooltip-portal.svelte";
	import type { ComponentProps } from "svelte";
	import type { WithoutChildrenOrChild } from "$lib/utils.js";

	let {
		ref = $bindable(null),
		class: className,
		role = "tooltip",
		sideOffset = 4,
		side = "top",
		children,
		portalProps,
		...restProps
	}: TooltipPrimitive.ContentProps & {
		portalProps?: WithoutChildrenOrChild<ComponentProps<typeof TooltipPortal>>;
	} = $props();
</script>

<TooltipPortal {...portalProps}>
	<TooltipPrimitive.Content
		bind:ref
		data-slot="tooltip-content"
		{role}
		{sideOffset}
		{side}
		class={cn(
			"z-50 inline-flex w-fit max-w-xs origin-(--bits-tooltip-content-transform-origin) items-center rounded-[var(--radius-md)] border border-[var(--border-default)] bg-[var(--surface-panel)] px-[var(--space-2)] py-[var(--space-1)] text-[length:var(--font-small-size)] leading-[var(--font-small-line)] text-[var(--text-primary)] shadow-[var(--shadow-popover)] data-open:animate-in data-open:fade-in-0 data-open:zoom-in-95 data-closed:animate-out data-closed:fade-out-0 data-closed:zoom-out-95 duration-100 motion-reduce:animate-none motion-reduce:transition-none",
			className
		)}
		{...restProps}
	>
		{@render children?.()}
	</TooltipPrimitive.Content>
</TooltipPortal>
