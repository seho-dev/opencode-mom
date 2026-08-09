<script lang="ts" module>
	import { cn, type WithElementRef } from "$lib/utils.js";
	import type { HTMLAnchorAttributes, HTMLButtonAttributes } from "svelte/elements";
	import { type VariantProps, tv } from "tailwind-variants";

		export const buttonVariants = tv({
			base: "group/button inline-flex shrink-0 items-center justify-center whitespace-nowrap rounded-[var(--radius-md)] border font-[var(--font-caption-weight)] text-[length:var(--font-caption-size)] leading-[var(--font-caption-line)] transition-colors duration-150 outline-none select-none focus-visible:outline-[var(--focus-width)] focus-visible:outline-[var(--focus-ring)] focus-visible:outline-offset-[var(--focus-offset)] disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-[var(--disabled-opacity)] [&_svg]:pointer-events-none [&_svg]:size-[var(--icon-md)] [&_svg]:shrink-0",
			variants: {
				variant: {
					default: "border-[var(--accent-primary)] bg-[var(--accent-primary)] text-[var(--text-on-accent)] hover:border-[var(--accent-hover)] hover:bg-[var(--accent-hover)] active:border-[var(--accent-hover)] active:bg-[var(--accent-hover)]",
					secondary: "border-[var(--border-default)] bg-[var(--surface-panel)] text-[var(--text-primary)] hover:bg-[var(--surface-hover)] active:bg-[var(--surface-active)]",
					outline: "border-[var(--border-default)] bg-transparent text-[var(--text-primary)] hover:bg-[var(--surface-hover)] active:bg-[var(--surface-active)]",
					destructive: "border-[var(--status-error)] bg-transparent text-[var(--status-error)] hover:bg-[var(--surface-hover)] active:bg-[var(--surface-active)]",
					ghost: "border-transparent bg-transparent text-[var(--text-primary)] hover:bg-[var(--surface-hover)] active:bg-[var(--surface-active)] aria-current:bg-[var(--surface-active)]",
				},
				size: {
					default: "h-[var(--control-height-compact)] gap-[var(--space-1)] px-[var(--space-3)]",
					sm: "h-[var(--control-height-small)] gap-[var(--space-1)] px-[var(--space-2)]",
					icon: "size-[var(--control-height-compact)]",
					"icon-sm": "size-[var(--control-height-small)] [&_svg]:size-[var(--icon-sm)]",
			},
		},
		defaultVariants: {
			variant: "default",
			size: "default",
		},
	});

	export type ButtonVariant = VariantProps<typeof buttonVariants>["variant"];
	export type ButtonSize = VariantProps<typeof buttonVariants>["size"];

	export type ButtonProps = WithElementRef<HTMLButtonAttributes> &
		WithElementRef<HTMLAnchorAttributes> & {
			variant?: ButtonVariant;
			size?: ButtonSize;
		};
</script>

<script lang="ts">
	let {
		class: className,
		variant = "default",
		size = "default",
		ref = $bindable(null),
		href = undefined,
		type = "button",
		disabled,
		children,
		...restProps
	}: ButtonProps = $props();
</script>

{#if href}
	<a
		bind:this={ref}
		data-slot="button"
		class={cn(buttonVariants({ variant, size }), className)}
		href={disabled ? undefined : href}
		aria-disabled={disabled}
		role={disabled ? "link" : undefined}
		tabindex={disabled ? -1 : undefined}
		{...restProps}
	>
		{@render children?.()}
	</a>
{:else}
	<button
		bind:this={ref}
		data-slot="button"
		class={cn(buttonVariants({ variant, size }), className)}
		{type}
		{disabled}
		{...restProps}
	>
		{@render children?.()}
	</button>
{/if}
