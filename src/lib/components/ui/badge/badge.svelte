<script lang="ts" module>
	import { type VariantProps, tv } from "tailwind-variants";

	export const badgeVariants = tv({
		base: "inline-flex w-fit shrink-0 items-center justify-center gap-[var(--space-1)] overflow-hidden whitespace-nowrap rounded-[var(--radius-pill)] border px-[var(--space-2)] py-0 text-[length:var(--font-caption-size)] font-[var(--font-caption-weight)] leading-[var(--font-caption-line)] [&>svg]:size-[var(--icon-sm)] [&>svg]:shrink-0",
		variants: {
			variant: {
				secondary: "border-transparent bg-[var(--surface-active)] text-[var(--text-primary)]",
				success: "border-transparent bg-[var(--surface-muted)] text-[var(--status-success)]",
				warning: "border-transparent bg-[var(--surface-muted)] text-[var(--status-warning)]",
				destructive: "border-transparent bg-[var(--surface-muted)] text-[var(--status-error)]",
				outline: "border-[var(--border-default)] bg-transparent text-[var(--text-primary)]",
			},
		},
		defaultVariants: {
			variant: "secondary",
		},
	});

	export type BadgeVariant = VariantProps<typeof badgeVariants>["variant"];
</script>

<script lang="ts">
	import type { HTMLAttributes } from "svelte/elements";
	import { cn, type WithElementRef } from "$lib/utils.js";

	let {
		ref = $bindable(null),
		class: className,
		variant = "secondary",
		children,
		...restProps
	}: WithElementRef<HTMLAttributes<HTMLSpanElement>> & {
		variant?: BadgeVariant;
	} = $props();
</script>

<span
	bind:this={ref}
	data-slot="badge"
	class={cn(badgeVariants({ variant }), className)}
	{...restProps}
>
	{@render children?.()}
</span>
