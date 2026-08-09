<script lang="ts">
	import { cn, type WithElementRef, type WithoutChildren } from "$lib/utils.js";
	import type { HTMLTextareaAttributes } from "svelte/elements";

	let {
		ref = $bindable(null),
		value = $bindable(),
		class: className,
		"data-slot": dataSlot = "textarea",
		...restProps
	}: WithoutChildren<WithElementRef<HTMLTextareaAttributes>> = $props();
</script>

<textarea
	bind:this={ref}
	data-slot={dataSlot}
	class={cn(
		"flex min-h-[var(--control-height-compact)] w-full resize-y rounded-[var(--radius-sm)] border border-[var(--border-default)] bg-[var(--surface-input)] px-[var(--space-2)] py-[var(--space-1)] text-[length:var(--font-body-size)] leading-[var(--font-body-line)] text-[var(--text-primary)] outline-none transition-colors duration-150 placeholder:text-[var(--text-secondary)] focus-visible:outline-[var(--focus-width)] focus-visible:outline-[var(--focus-ring)] focus-visible:outline-offset-[var(--focus-offset)] aria-invalid:border-[var(--status-error)] disabled:cursor-not-allowed disabled:opacity-[var(--disabled-opacity)]",
		className
	)}
	bind:value
	{...restProps}
></textarea>
