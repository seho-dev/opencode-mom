<script lang="ts" module>
	import { type VariantProps, tv } from "tailwind-variants";

	export const alertVariants = tv({
		base: "group/alert grid w-full grid-cols-[minmax(0,1fr)_auto] items-start gap-x-[var(--space-2)] gap-y-[var(--space-1)] rounded-[var(--radius-md)] border border-[var(--border-subtle)] bg-[var(--surface-muted)] p-[var(--space-2)] text-left text-[length:var(--font-small-size)] leading-[var(--font-small-line)] text-[var(--text-primary)] has-[>svg]:grid-cols-[auto_minmax(0,1fr)_auto] [&>svg]:size-[var(--icon-md)] [&>svg]:text-current",
		variants: {
			variant: {
				default: "text-[var(--text-primary)]",
				success: "border-[var(--status-success)] text-[var(--status-success)]",
				warning: "border-[var(--status-warning)] text-[var(--status-warning)]",
				destructive: "border-[var(--status-error)] text-[var(--status-error)]",
			},
		},
		defaultVariants: {
			variant: "default",
		},
	});

	export type AlertVariant = VariantProps<typeof alertVariants>["variant"];
</script>

<script lang="ts">
	import type { HTMLAttributes } from "svelte/elements";
	import { cn, type WithElementRef } from "$lib/utils.js";

	let {
		ref = $bindable(null),
		class: className,
		variant = "default",
		children,
		...restProps
	}: WithElementRef<HTMLAttributes<HTMLDivElement>> & {
		variant?: AlertVariant;
	} = $props();
</script>

<div
	bind:this={ref}
	data-slot="alert"
	role="alert"
	class={cn(alertVariants({ variant }), className)}
	{...restProps}
>
	{@render children?.()}
</div>
