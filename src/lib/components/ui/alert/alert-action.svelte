<script lang="ts">
	import type { HTMLAttributes } from "svelte/elements";
	import { cn, type WithElementRef } from "$lib/utils.js";

	let {
		ref = $bindable(null),
		class: className,
		children,
		...restProps
	}: WithElementRef<HTMLAttributes<HTMLDivElement>> = $props();
</script>

<div
	bind:this={ref}
	data-slot="alert-action"
	class={cn("col-start-2 row-span-2 row-start-1 self-center has-[button]:shrink-0 group-has-[>svg]/alert:col-start-3", className)}
	{...restProps}
>
	{@render children?.()}
</div>
