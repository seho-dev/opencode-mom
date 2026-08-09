<script lang="ts">
	import type { HTMLInputAttributes, HTMLInputTypeAttribute } from "svelte/elements";
	import { cn, type WithElementRef } from "$lib/utils.js";

	type InputType = Exclude<HTMLInputTypeAttribute, "file">;

	type Props = WithElementRef<
		Omit<HTMLInputAttributes, "type"> &
			({ type: "file"; files?: FileList } | { type?: InputType; files?: undefined })
	>;

	let {
		ref = $bindable(null),
		value = $bindable(),
		type,
		files = $bindable(),
		class: className,
		"data-slot": dataSlot = "input",
		...restProps
	}: Props = $props();
</script>

{#if type === "file"}
	<input
		bind:this={ref}
		data-slot={dataSlot}
		class={cn(
			"h-[var(--control-height-compact)] w-full min-w-0 rounded-[var(--radius-sm)] border border-[var(--border-default)] bg-[var(--surface-input)] px-[var(--space-2)] py-[var(--space-1)] text-[length:var(--font-body-size)] leading-[var(--font-body-line)] text-[var(--text-primary)] outline-none transition-colors duration-150 placeholder:text-[var(--text-secondary)] focus-visible:outline-[var(--focus-width)] focus-visible:outline-[var(--focus-ring)] focus-visible:outline-offset-[var(--focus-offset)] aria-invalid:border-[var(--status-error)] disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-[var(--disabled-opacity)] file:inline-flex file:h-[var(--control-height-small)] file:border-0 file:bg-transparent file:text-[length:var(--font-small-size)] file:font-[var(--font-row-weight)]",
			className
		)}
		type="file"
		bind:files
		bind:value
		{...restProps}
	/>
{:else}
	<input
		bind:this={ref}
		data-slot={dataSlot}
		class={cn(
			"h-[var(--control-height-compact)] w-full min-w-0 rounded-[var(--radius-sm)] border border-[var(--border-default)] bg-[var(--surface-input)] px-[var(--space-2)] py-[var(--space-1)] text-[length:var(--font-body-size)] leading-[var(--font-body-line)] text-[var(--text-primary)] outline-none transition-colors duration-150 placeholder:text-[var(--text-secondary)] focus-visible:outline-[var(--focus-width)] focus-visible:outline-[var(--focus-ring)] focus-visible:outline-offset-[var(--focus-offset)] aria-invalid:border-[var(--status-error)] disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-[var(--disabled-opacity)]",
			className
		)}
		{type}
		bind:value
		{...restProps}
	/>
{/if}
