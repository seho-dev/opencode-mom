<script lang="ts">
	import CircleAlert from "@lucide/svelte/icons/circle-alert";
	import Info from "@lucide/svelte/icons/info";
	import Plus from "@lucide/svelte/icons/plus";
	import TriangleAlert from "@lucide/svelte/icons/triangle-alert";

	import * as Alert from "./alert/index.js";
	import { Badge } from "./badge/index.js";
	import { Button } from "./button/index.js";
	import { Checkbox } from "./checkbox/index.js";
	import * as Dialog from "./dialog/index.js";
	import { Input } from "./input/index.js";
	import { Separator } from "./separator/index.js";
	import { Spinner } from "./spinner/index.js";
	import { Textarea } from "./textarea/index.js";
	import * as Tooltip from "./tooltip/index.js";

	const buttonVariants = ["default", "secondary", "outline", "destructive", "ghost"] as const;
	const buttonSizes = ["default", "sm", "icon", "icon-sm"] as const;
	const badgeVariants = ["secondary", "success", "warning", "destructive", "outline"] as const;
	const alertVariants = ["default", "success", "warning", "destructive"] as const;

	let keyboardChecked = $state(false);
</script>

<main class="showcase" data-surface="ui-primitives" aria-labelledby="showcase-title">
	<header class="showcase-header">
		<div>
			<p class="eyebrow">Owned UI system</p>
			<h1 id="showcase-title">Primitive state matrix</h1>
		</div>
		<Badge variant="outline">Vega compact</Badge>
	</header>

	<section aria-labelledby="buttons-title">
		<h2 id="buttons-title">Button</h2>
		<div class="state-grid">
			{#each buttonVariants as variant}
				<Button {variant}>{variant}</Button>
			{/each}
			{#each buttonSizes as size}
				<Button variant="secondary" {size} aria-label={size.startsWith("icon") ? `${size} button` : undefined}>
					{#if size.startsWith("icon")}<Plus aria-hidden="true" />{:else}{size}{/if}
				</Button>
			{/each}
		</div>
		<div class="state-grid interaction-row">
			<Button variant="secondary" data-qa="button-hover">Hover</Button>
			<Button variant="secondary" data-qa="button-focus">Focus</Button>
			<Button variant="secondary" aria-pressed="true" data-qa="button-pressed">Pressed</Button>
			<Button variant="secondary" disabled>Disabled</Button>
			<Button aria-busy="true" disabled><Spinner size="sm" />Loading</Button>
		</div>
	</section>

	<section aria-labelledby="fields-title">
		<h2 id="fields-title">Input and Textarea</h2>
		<div class="field-grid">
			<label>Default input<Input placeholder="Model reference" /></label>
			<label>Focus input<Input data-qa="input-focus" value="Focused value" /></label>
			<label>Invalid input<Input aria-invalid="true" value="Invalid value" /></label>
			<label>Read-only input<Input readonly value="Inspectable value" /></label>
			<label>Disabled input<Input disabled value="Unavailable value" /></label>
			<label>Default textarea<Textarea rows={3} placeholder="One origin per line" /></label>
			<label>Focus textarea<Textarea data-qa="textarea-focus" rows={3} value="Focused configuration" /></label>
			<label>Invalid textarea<Textarea rows={3} aria-invalid="true" value="Invalid configuration" /></label>
			<label>Read-only textarea<Textarea rows={3} readonly value="Inspectable configuration" /></label>
			<label>Disabled textarea<Textarea rows={3} disabled value="Unavailable configuration" /></label>
		</div>
	</section>

	<section aria-labelledby="checkbox-title">
		<h2 id="checkbox-title">Checkbox</h2>
		<div class="state-grid checkbox-grid">
			<label><Checkbox aria-label="Unchecked checkbox" />Unchecked</label>
			<label><Checkbox aria-label="Checked checkbox" checked />Checked</label>
			<label><Checkbox aria-label="Indeterminate checkbox" indeterminate />Indeterminate</label>
			<label><Checkbox aria-label="Keyboard checkbox" bind:checked={keyboardChecked} />Keyboard focus</label>
			<label><Checkbox aria-label="Disabled checkbox" checked disabled />Disabled</label>
		</div>
	</section>

	<section aria-labelledby="status-title">
		<h2 id="status-title">Badge and Alert</h2>
		<div class="state-grid">
			{#each badgeVariants as variant}<Badge {variant}>{variant}</Badge>{/each}
		</div>
		<div class="alert-grid">
			{#each alertVariants as variant}
				<Alert.Root {variant} role={variant === "warning" ? "status" : variant === "destructive" ? "alert" : "note"}>
					{#if variant === "default"}<Info aria-hidden="true" />{:else if variant === "warning"}<TriangleAlert aria-hidden="true" />{:else}<CircleAlert aria-hidden="true" />{/if}
					<Alert.Title>{variant === "destructive" ? "Destructive alert" : `${variant[0]?.toUpperCase()}${variant.slice(1)} alert`}</Alert.Title>
					<Alert.Description>Compact status copy remains visible beside its action.</Alert.Description>
					{#if variant === "warning"}<Alert.Action><Button size="sm" variant="secondary" data-showcase-action="warning">Resolve warning</Button></Alert.Action>{/if}
				</Alert.Root>
			{/each}
		</div>
	</section>

	<section aria-labelledby="overlay-title">
		<h2 id="overlay-title">Dialog and Tooltip</h2>
		<div class="state-grid">
			<Dialog.Root>
				<Dialog.Trigger>
					{#snippet child({ props })}<Button variant="destructive" {...props}>Open dialog</Button>{/snippet}
				</Dialog.Trigger>
				<Dialog.Content interactOutsideBehavior="ignore">
					<Dialog.Header><Dialog.Title>Primitive dialog</Dialog.Title><Dialog.Description>Escape closes this modal and restores focus to the trigger.</Dialog.Description></Dialog.Header>
					<Input aria-label="Dialog focus target" value="Focus is trapped here" />
					<Dialog.Footer>
						<Dialog.Close>{#snippet child({ props })}<Button variant="secondary" {...props}>Cancel dialog</Button>{/snippet}</Dialog.Close>
						<Button variant="destructive">Confirm action</Button>
					</Dialog.Footer>
				</Dialog.Content>
			</Dialog.Root>
			<Tooltip.Provider delayDuration={300}>
				<Tooltip.Root>
					<Tooltip.Trigger>
						{#snippet child({ props })}<Button variant="secondary" size="icon" aria-label="Tooltip action" {...props}><Plus aria-hidden="true" /></Button>{/snippet}
					</Tooltip.Trigger>
					<Tooltip.Content side="right" sideOffset={8}>Keyboard tooltip</Tooltip.Content>
				</Tooltip.Root>
			</Tooltip.Provider>
			<span class="state-label">Closed at rest, open on focus</span>
		</div>
	</section>

	<section aria-labelledby="utility-title">
		<h2 id="utility-title">Spinner and Separator</h2>
		<div class="utility-row">
			<span><Spinner size="sm" />Inline</span><span><Spinner />Standard</span><span><Spinner size="lg" />State block</span><span class="reduced-motion"><Spinner />Reduced motion</span>
		</div>
		<Separator />
		<div class="vertical-separator"><span>Left region</span><Separator orientation="vertical" /><span>Right region</span></div>
	</section>
</main>

<style>
	.showcase { display: grid; width: 100%; min-width: 0; min-height: 100dvh; align-content: start; gap: var(--space-4); overflow: auto; background: var(--surface-app); color: var(--text-primary); padding: var(--space-4); }
	.showcase-header { display: flex; align-items: center; justify-content: space-between; gap: var(--space-3); border-bottom: var(--border-width) solid var(--border-default); padding-bottom: var(--space-3); }
	h1, h2, p { margin: 0; }
	h1 { font-size: var(--font-window-size); font-weight: var(--font-window-weight); line-height: var(--font-window-line); }
	h2 { font-size: var(--font-section-size); font-weight: var(--font-section-weight); line-height: var(--font-section-line); }
	.eyebrow, .state-label { color: var(--text-secondary); font-size: var(--font-caption-size); font-weight: var(--font-caption-weight); line-height: var(--font-caption-line); }
	section { display: grid; min-width: 0; gap: var(--space-2); border-bottom: var(--border-width) solid var(--border-subtle); padding-bottom: var(--space-4); }
	.state-grid, .utility-row, .vertical-separator { display: flex; min-width: 0; align-items: center; flex-wrap: wrap; gap: var(--space-2); }
	.field-grid, .alert-grid { display: grid; min-width: 0; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: var(--space-2); }
	.field-grid label, .checkbox-grid label { display: grid; min-width: 0; gap: var(--space-1); color: var(--text-secondary); font-size: var(--font-small-size); line-height: var(--font-small-line); }
	.checkbox-grid label { grid-template-columns: auto 1fr; align-items: center; gap: var(--space-2); color: var(--text-primary); }
	.utility-row span { display: inline-flex; align-items: center; gap: var(--space-1); color: var(--text-secondary); font-size: var(--font-small-size); }
	.vertical-separator { height: var(--space-6); }
	:global([data-qa="button-hover"]) { background: var(--surface-hover); }
	:global([data-qa="button-focus"]), :global([data-qa="input-focus"]) { outline: var(--focus-width) solid var(--focus-ring); outline-offset: var(--focus-offset); }
	:global([data-qa="textarea-focus"]) { outline: var(--focus-width) solid var(--focus-ring); outline-offset: var(--focus-offset); }
	:global([data-qa="button-pressed"]) { background: var(--surface-active); }
	.reduced-motion :global([data-slot="spinner"]) { animation: none; }
	@media (width < 600px) { .field-grid, .alert-grid { grid-template-columns: minmax(0, 1fr); } .showcase-header { align-items: flex-start; } }
</style>
