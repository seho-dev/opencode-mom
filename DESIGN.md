# omo-switch Compact Desktop Design System

## 1. Atmosphere & Identity

omo-switch is a compact desktop control surface for developers and operators who switch model groups and edit a small amount of configuration repeatedly. It must feel immediate, utilitarian, and native to a small Tauri window: dense enough to keep the current task visible, quiet enough that status and destructive actions remain obvious, and stable enough that loading or long content never rearranges the interface unexpectedly.

The signature is **bounded utility**: pale tonal changes identify hover, selection, active rows, and degraded regions inside a precise one-pixel frame. The product is not a web dashboard and not a marketing surface. It uses short labels, compact controls, restrained color, small radii, and predictable pane boundaries to keep attention on configuration state.

This contract preserves the information architecture and surface behavior evidenced by:

- `src/routes/+layout.svelte`
- `src/lib/components/QuickSwitch.svelte`
- `src/lib/components/settings/SettingsShell.svelte`
- `src/lib/components/settings/groups/GroupSettingsPane.svelte`
- `src-tauri/tauri.conf.json`

The contract documents the existing compact UI. It does not authorize changes to copy, workflow order, stores, native commands, window selection, window dimensions, theme ownership, or validation behavior.

## 2. Color

### Canonical Token Registry

Every existing layout token appears once in this table. Light and dark values are authoritative. The semantic alias column defines the one-way shadcn/Tailwind mapping for the later foundation work; `None` means the token remains project-specific. The two rows marked `foundation addition` are already prescribed by the migration plan and do not replace an existing layout token.

| Category | Existing or prescribed token | Light | Dark | Semantic shadcn alias | Contract use |
| --- | --- | --- | --- | --- | --- |
| Surface | `--surface-app` | `#f6f7f8` | `#111315` | `--background` | App and backdrop reference |
| Surface | `--surface-panel` | `#ffffff` | `#181b1f` | `--card`, `--popover` | Primary window, pane, control, dialog surface |
| Surface | `--surface-muted` | `#f1f3f5` | `#20242a` | `--secondary`, `--muted` | Sidebars, toolbars, alerts, degraded sections |
| Surface | `--surface-input` | `#ffffff` | `#15181c` | None | Editable and read-only field surface |
| Surface state | `--surface-hover` | `#eceff3` | `#262b32` | `--accent` | Hovered controls and rows |
| Surface state | `--surface-active` | `#e4eaf2` | `#2b3340` | None | Selected navigation, selected rows, compact badges |
| Text | `--text-primary` | `#171a1f` | `#f2f4f7` | `--foreground`, `--card-foreground`, `--popover-foreground`, `--secondary-foreground`, `--accent-foreground` | Primary labels, values, headings |
| Text | `--text-secondary` | `#596170` | `#a7b0bf` | `--muted-foreground` | Help, descriptions, metadata |
| Text | `--text-muted` | `#858e9c` | `#707a88` | None | Inactive indicators and least-prominent text |
| Border | `--border-default` | `#d8dde5` | `#343a44` | `--border`, `--input` | Pane boundaries, controls, dialogs |
| Border | `--border-subtle` | `#e8ebf0` | `#282d35` | None | Section dividers and quiet status rows |
| Focus | `--focus-ring` | `#2f6feb` | `#6aa4ff` | `--ring` | Keyboard focus outline |
| Accent | `--accent-primary` | `#2563eb` | `#6aa4ff` | `--primary` | Primary action and active semantic emphasis |
| Accent | `--accent-hover` | `#1d4ed8` | `#8cb8ff` | None | Primary action hover |
| Status | `--status-success` | `#16843a` | `#36c26b` | None | Enabled state |
| Status | `--status-warning` | `#b76b00` | `#f0a33a` | None | Advisory, degraded, pending approval |
| Status | `--status-error` | `#c9362b` | `#ff6b5f` | `--destructive` | Errors, invalid data, destructive intent |
| Status | `--status-info` | `#2f6feb` | `#6aa4ff` | None | Informational status |
| Spacing | `--space-1` | `4px` | `4px` | None | Tight icon and label gap |
| Spacing | `--space-2` | `8px` | `8px` | None | Compact row and control gap |
| Spacing | `--space-3` | `12px` | `12px` | None | Toolbar and field-section rhythm |
| Spacing | `--space-4` | `16px` | `16px` | None | Window and section padding |
| Spacing | `--space-5` | `20px` | `20px` | None | Settings detail padding |
| Spacing | `--space-6` | `24px` | `24px` | None | Reserved larger compact interval |
| Spacing | `--space-8` | `32px` | `32px` | None | Empty-state padding |
| Radius | `--radius-sm` | `4px` | `4px` | None | Inputs and checkbox |
| Radius | `--radius-md` | `6px` | `6px` | Buttons, rows, alerts |
| Radius | `--radius-lg` | `8px` | `8px` | Dialog and degraded section |
| Radius | `--radius-pill` | `999px` | `999px` | Status dot and badge only |
| Shadow | `--shadow-popover` | `0 8px 24px rgb(15 23 42 / 14%)` | `0 8px 24px rgb(15 23 42 / 14%)` | None | Dialog, tooltip, and popover elevation only |
| Shadow | `--shadow-window` | `0 1px 2px rgb(15 23 42 / 8%)` | `0 1px 2px rgb(15 23 42 / 8%)` | None | Outer floating window treatment only |
| Line | `--border-width` | `1px` | `1px` | None | All declared borders and separators |
| Focus | `--focus-width` | `2px` | `2px` | None | Focus outline width |
| Focus | `--focus-offset` | `2px` | `2px` | None | Focus outline offset |
| State | `--disabled-opacity` | `0.55` | `0.55` | None | Disabled interactive controls only |
| Icon | `--icon-sm` | `12px` | `12px` | None | Dense inline status/action icon |
| Icon | `--icon-md` | `16px` | `16px` | None | Standard control and navigation icon |
| Icon | `--icon-lg` | `24px` | `24px` | None | Loading and empty-state icon |
| Font | `--font-primary` | `system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif` | Same as light | None | All product UI text |
| Font | `--font-mono` | `ui-monospace, "SFMono-Regular", "Cascadia Mono", "Segoe UI Mono", monospace` | Same as light | None | Model refs, paths, code values |
| Type | `--font-window-size` | `20px` | `20px` | None | Window title |
| Type | `--font-window-weight` | `600` | `600` | None | Window title weight |
| Type | `--font-window-line` | `28px` | `28px` | None | Window title line height |
| Type | `--font-section-size` | `15px` | `15px` | None | Section and pane heading |
| Type | `--font-section-weight` | `600` | `600` | None | Section heading weight |
| Type | `--font-section-line` | `22px` | `22px` | None | Section heading line height |
| Type | `--font-row-size` | `13px` | `13px` | None | Row and field label |
| Type | `--font-row-weight` | `500` | `500` | None | Row and field label weight |
| Type | `--font-row-line` | `18px` | `18px` | None | Row and field label line height |
| Type | `--font-body-size` | `13px` | `13px` | None | Default product text |
| Type | `--font-body-weight` | `400` | `400` | None | Default product weight |
| Type | `--font-body-line` | `20px` | `20px` | None | Default product line height |
| Type | `--font-small-size` | `12px` | `12px` | None | Help, status, descriptions |
| Type | `--font-small-line` | `17px` | `17px` | None | Small text line height |
| Type | `--font-caption-size` | `11px` | `11px` | None | Section labels and compact actions |
| Type | `--font-caption-weight` | `500` | `500` | None | Caption and compact action weight |
| Type | `--font-caption-line` | `15px` | `15px` | None | Caption line height |
| Type | `--font-code-size` | `11px` | `11px` | None | Model refs and code metadata |
| Type | `--font-code-line` | `16px` | `16px` | None | Code line height |
| Quick Switch geometry | `--quick-switch-width` | `420px` | `420px` | None | Fixed native window width |
| Quick Switch geometry | `--quick-switch-height` | `360px` | `360px` | None | Fixed native window height |
| Quick Switch geometry | `--quick-switch-min-width` | `300px` | `300px` | None | Browser fixture lower bound |
| Quick Switch geometry | `--quick-switch-min-height` | `0` | `0` | None | Flex/grid overflow safety |
| Quick Switch geometry | `--quick-switch-state-min-height` | `0` | `0` | None | Loading state overflow safety |
| Row geometry | `--target-row-min-height` | `36px` | `36px` | None | Quick Switch target row minimum |
| Row geometry | `--value-row-min-height` | `20px` | `20px` | None | Current-value row minimum |
| Control geometry | `--control-height-compact` | `28px` | `28px` | None | Default control height |
| Control geometry | `--control-height-small` | `24px` | `24px` | None | Small and icon control height |
| Content geometry | `--model-ref-max-width` | `152px` | `152px` | None | Quick Switch model-ref truncation width |
| Accessibility | `--screen-reader-box` | `1px` | `1px` | None | Visually hidden focus/label box |
| Foundation addition | `--text-on-accent` | `#ffffff` | `#111315` | `--primary-foreground` | Text and icons on the primary action fill |
| Foundation addition | `--radius` | `8px` | `8px` | shadcn radius base | Alias basis; owned primitives still use the exact 4/6/8px radii below |

### Color Rules

- The operating-system color scheme is authoritative. There is no theme toggle, persisted theme state, or class-driven dark mode.
- Accent color is functional: primary action, focus, active semantic emphasis, or informational status. It is never decorative.
- Status tones identify success, warning, error, and information through icon, text, and when needed a border. Color must not be the only communicated signal.
- Error status may derive a quieter border by mixing the error tone with the subtle border. No unrelated derived palette is permitted.
- Read-only fields keep normal text contrast and the input surface so preserved values remain legible and selectable. They do not use disabled opacity.
- Disabled controls use 0.55 opacity, a not-allowed cursor, and native disabled semantics. Disabled content must remain readable in both themes.
- Degraded sections retain data, explain why editing is unavailable, use the warning tone, and use a muted surface with an 8px radius and one-pixel warning border. Degraded is not destructive and must not discard or hide preserved values.

## 3. Typography

### Scale

| Level | Size | Weight | Line height | Letter spacing | Usage |
| --- | --- | --- | --- | --- | --- |
| Window title | `20px` | `600` | `28px` | `0` | Settings title and full-pane title |
| Section heading | `15px` | `600` | `22px` | `0` | Group names, pane titles, field sections |
| Row label | `13px` | `500` | `18px` | `0` | Group rows and form labels |
| Body | `13px` | `400` | `20px` | `0` | Default product copy and values |
| Small | `12px` | `400` | `17px` | `0` | Help, descriptions, status details |
| Caption | `11px` | `500` | `15px` | `0` | Section labels, badges, compact actions |
| Code | `11px` | `400` | `16px` | `0` | Model refs, paths, timestamps, code values |

### Font Rules

- Use the system UI stack for all interface text and the system monospace stack only for machine-facing values.
- Do not add a display font, web font, serif, or third font family.
- Letter spacing is always zero. Compactness comes from the scale and layout, not compressed tracking.
- Sentence case is the default. Existing English labels and capitalization remain unchanged.
- The 11px and 12px sizes are deliberate desktop utility sizes. They are restricted to secondary information and compact controls, never long-form copy.
- Headings do not scale with viewport width. Responsive layout changes topology before changing type size.

### Truncation and Wrapping

- Quick Switch names and model refs use single-line ellipsis with the complete value available through a title or accessible name.
- The current-group description is limited to two lines. Omitted descriptions do not reserve blank space.
- Settings values that are intrinsically long, including paths, timestamps, and model refs, may wrap anywhere rather than widen a pane.
- Buttons normally remain one line. Below 520px, Settings toolbar buttons may wrap to multiple lines and grow in height; text must not clip, overlap, or force horizontal document overflow.
- Navigation labels may wrap only where the current responsive topology permits it. They must remain fully readable and retain their accessible name.

## 4. Spacing & Layout

### Base Scale

All spacing derives from a 4px unit. Approved steps are 4, 8, 12, 16, 20, 24, and 32px. New component CSS must consume the matching token from Section 2 rather than introduce an intermediate value.

| Step | Typical use |
| --- | --- |
| 4px | Icon-to-label, tightly related row content, field label-to-control |
| 8px | Control groups, compact rows, alert content, dialog actions |
| 12px | Toolbars, field sections, standard horizontal button padding |
| 16px | Quick Switch window padding, section boundaries, Settings footer |
| 20px | Settings editor and dialog content padding |
| 24px | Reserved larger interval when the current layout requires it |
| 32px | Empty-state breathing room only |

### Radii and Controls

- 4px: inputs, textareas, and checkbox boxes.
- 6px: buttons, icon buttons, list rows, target rows, alerts, and compact state blocks.
- 8px: dialogs and explicitly degraded sections.
- Pill radius: badges and status dots only. It is not a general control shape.
- 24px: small buttons and small icon buttons.
- 28px: default buttons, icon buttons, inputs, and single-line text controls.
- 32px: Settings navigation buttons. This keeps the navigation target stable without making all controls roomy.
- Composite rows retain their current minimums: 20px current-value rows, 34px group rows, 36px Quick Switch target rows, and 44px toolbars.
- Icons are 12px for dense inline actions, 16px for normal controls/navigation, and 24px for loading or empty-state emphasis.

### Surface Geometry

- Quick Switch is a fixed, non-resizable `420x360` native window. Its content is one vertical flow with 16px outer padding, 12px section gaps, internal scrolling when required, and no side panel.
- Settings opens at `980x620` and remains resizable. The detail region owns overflow; the whole document must not scroll horizontally.
- At widths `>=720`, Settings keeps the outer sidebar at 180-200px and the detail surface beside it.
- At the default `980x620`, the outer sidebar remains visible, while the Groups list stacks above the editor because the remaining Groups surface is below the side-by-side threshold.
- At `1280x800`, the outer sidebar remains visible and the Groups list/editor may render side by side, with the list at 220-260px and the editor taking the remaining width.
- At `680x720`, Settings uses top navigation, a stacked Groups list, then the toolbar and editor in document order.
- Below 520px, Settings navigation may place icon above label and toolbar buttons may use a two-column wrapping grid. A fifth action spans both columns. Labels wrap without clipping.
- Group list/editor topology changes below 1040px of available Groups-pane width. This is a local pane breakpoint, not a replacement for the 720px outer-shell breakpoint.
- Forms use the available width up to 680px. Port remains compact at a maximum of 160px. Multi-column mapping and replace rows collapse to one column with the stacked Groups editor.

### Stable Geometry Rules

- Loading indicators replace icons or occupy an already reserved state block; they do not change control height or reorder controls.
- Buttons keep stable width where an action label changes to a pending label. Icon-only controls remain square.
- Rows set `min-width: 0` on flexible text children so ellipsis and wrapping happen inside the row.
- Selection, active, focus, disabled, read-only, and degraded states must not shift neighboring content.
- Toolbars may wrap at documented breakpoints, but controls must retain DOM order and accessible focus order.
- Dialog content is at most 420px wide and must fit within viewport padding without changing the underlying pane geometry.

## 5. Components

Only the ten primitives below are approved for the owned shadcn-svelte layer. Product components compose them without changing store ownership, command behavior, copy, focus order, or responsive topology.

### Button

- **Structure**: Native `button` root with optional leading icon, text label, optional trailing icon, and an optional Spinner replacing an icon during pending work. Icon-only buttons contain one icon and require an accessible name.
- **Variants**: `default`, `secondary`, `outline`, `destructive`, and `ghost`. Default is the filled primary action. Secondary is a bordered panel action. Outline is a transparent bordered action. Destructive communicates deletion without creating a filled red dashboard button. Ghost is reserved for Settings navigation and quiet row actions.
- **Sizes and spacing**: `default` is 28px high with 12px horizontal padding; `sm` is 24px high with 8px horizontal padding; `icon` is 28px square; `icon-sm` is 24px square. Icon-label gap is 4px. Radius is 6px.
- **States**: Default uses its variant surface; hover uses the documented hover surface or primary hover tone; active uses the active surface and may expose `aria-pressed` or current-page semantics when the product owns selection; focus uses the 2px ring with 2px offset; disabled uses native `disabled`, 0.55 opacity, and no callback; loading uses `aria-busy`, disables repeat activation, preserves geometry, and shows Spinner; empty is inapplicable because an unlabeled button is invalid; error is inapplicable as an internal state, while destructive variant expresses intent and an adjacent Alert expresses failure.
- **Accessibility**: Preserve native keyboard activation, visible focus, label text or `aria-label`, and the existing DOM/focus order. Do not nest buttons. Tooltips never replace the accessible name.
- **Motion**: Color and tonal feedback may transition in at most 150ms. Loading icon rotation is delegated to Spinner. No translation, bounce, or size animation.

### Input

- **Structure**: Native single-line input with a visible label supplied by the product component, optional help text, and separate validation/status text.
- **Variants**: One visual variant. Semantic differences are default, invalid, read-only, and disabled attributes rather than cosmetic variants.
- **Sizes and spacing**: Minimum height 28px, 4px radius, one-pixel border, 4px vertical and 8px horizontal padding. Field label gap is 4px.
- **States**: Default uses the input surface; hover has no required visual change; active follows native text-editing behavior; focus uses the documented ring; disabled uses native disabled semantics and 0.55 opacity; loading is inapplicable to the field itself and belongs to the owning action; empty is a valid value and placeholder text remains secondary; error uses `aria-invalid`, the error tone, and associated error copy; read-only remains legible/selectable and does not use disabled opacity.
- **Accessibility**: Every input has a visible label or exact `aria-label`, preserves input mode and native editing, associates help/error text when present, and exposes read-only versus disabled accurately.
- **Motion**: Border and ring feedback may transition in at most 150ms. No width, height, or label animation.

### Textarea

- **Structure**: Native multiline textarea with visible label, optional help text, and separate validation/status text.
- **Variants**: One visual variant with default, invalid, read-only, and disabled semantics.
- **Sizes and spacing**: Minimum control height 28px; product rows determine any larger initial height. Radius is 4px with 4px vertical and 8px horizontal padding. Vertical resize is allowed.
- **States**: Default uses the input surface; hover has no required visual change; active is native text editing; focus uses the documented ring; disabled uses native disabled semantics and 0.55 opacity; loading is inapplicable; empty is a valid value; error uses `aria-invalid` and associated error copy; read-only remains selectable, legible, and visually distinct from disabled through semantics rather than opacity.
- **Accessibility**: Visible label, native keyboard editing, preserved rows/value, and programmatic help/error association are required. Resize must not cause horizontal overflow.
- **Motion**: Same ring/border timing as Input. Resizing is user-driven and is not animated.

### Checkbox

- **Structure**: A 16px square checkbox control plus a visible text label in one row. The indicator renders checked or indeterminate state inside the box.
- **Variants**: Unchecked, checked, and indeterminate. There is no switch-style variant.
- **Sizes and spacing**: 16px box, 4px radius, one-pixel border, and 8px control-to-label gap. The label row may be taller than the box but must align centrally.
- **States**: Default may be unchecked or checked; hover uses the hover surface without changing geometry; active is the pressed native/bits state; focus uses the documented ring; disabled reports disabled, blocks pointer/Space callbacks, and uses 0.55 opacity; loading is inapplicable, with pending state owned by the surrounding setting or Alert; empty is inapplicable because checked state is explicit; error is inapplicable to the primitive, with validation communicated by adjacent error text or Alert; indeterminate is supported for the primitive showcase even when no current product checkbox consumes it.
- **Accessibility**: Expose checkbox role, accessible name from the visible label, accurate checked or mixed state, Space keyboard toggling, and disabled semantics. Controlled callbacks must preserve current store behavior.
- **Motion**: Indicator opacity/scale may transition in at most 100ms. Reduced motion removes scale and leaves an immediate state change.

### Badge

- **Structure**: Inline non-interactive text marker. It may include a small status icon only when the icon adds meaning.
- **Variants**: `secondary`, `success`, `warning`, `destructive`, and `outline`.
- **Sizes and spacing**: Caption typography, pill radius, 8px horizontal padding, and no independent minimum height beyond the caption line.
- **States**: Default renders the assigned semantic variant; hover, active, focus, disabled, and loading are inapplicable because Badge is not interactive; empty is inapplicable because a blank badge is forbidden; error is represented by the destructive variant, but error announcements belong to Alert.
- **Accessibility**: Text must carry the state meaning; color alone is insufficient. Decorative icons are hidden from assistive technology. Do not use a badge as a button or tooltip trigger.
- **Motion**: None. Badge content changes immediately without animation or layout flourish.

### Alert

- **Structure**: Compact alert root with optional semantic icon, text container, optional title, optional description, and optional single AlertAction aligned without nesting controls.
- **Variants**: `default`, `success`, `warning`, and `destructive`.
- **Sizes and spacing**: One-pixel subtle border, 6px radius, muted surface, 8px padding and 8px internal gap. Text uses the small scale. An action uses the 24px Button size when space is constrained.
- **States**: Default is a neutral status row; hover, active, and focus are inapplicable to the alert root, while AlertAction owns its own interactive states; disabled is inapplicable to the root; loading is supported only when the owning region supplies `aria-busy` and a Spinner/action state; empty is inapplicable because an alert requires meaningful copy; error is the destructive variant with error role where the existing channel requires it. Success and warning remain separate tones.
- **Accessibility**: The primitive defaults to `role="alert"`, but consumers must be able to override the role. Existing warning channels use `role="status"`; non-live summaries pass no live-region role; errors remain alerts. Icons are hidden from assistive technology and actions retain accessible names.
- **Motion**: No decorative entry animation is required for inline status rows. Content appears without moving surrounding geometry. Spinner or action motion is delegated to those primitives.

### Dialog

- **Structure**: Root, Trigger, Portal, Overlay, Content, Header, Title, Description, Footer, and Close composition. Delete confirmation uses one destructive trigger Button, title, explanatory description, secondary cancel Button, and destructive confirm Button. No extra close icon.
- **Variants**: Default modal shell and destructive-confirmation composition. AlertDialog is not an approved substitute.
- **Sizes and spacing**: Content width is `min(100%, 420px)` inside 20px viewport padding, with 20px content padding, 12px content gap, 8px action gap, 8px radius, one-pixel default border, and overlay-only elevation.
- **States**: Default is closed; hover and active belong to Trigger/Close/confirm Buttons; focus is trapped inside open content with visible rings on controls; disabled applies to a disabled trigger or action, not the dialog root; loading may occur after confirmation but the current delete contract closes before awaiting the command; empty is inapplicable because title and description are required; error is not shown by reopening the dialog and instead appears through the store-owned status Alert; open state is modal and blocks background interaction.
- **Accessibility**: `role="dialog"`, `aria-modal="true"`, labelled title, descriptive copy, default Escape handling, focus trap, and automatic trigger-focus restoration are required. Deletion sets outside interaction to ignore. Confirm/cancel order and existing copy remain unchanged.
- **Motion**: Overlay opacity and content opacity/scale may animate for 150ms ease-out. Reduced motion removes scale and uses an immediate or opacity-only transition. No slide across the window and no background layout animation.

### Spinner

- **Structure**: One decorative loading icon inside a control or state block. The owning region supplies readable loading text and live/busy semantics.
- **Variants**: 12px inline, 16px standard, and 24px state-block sizes.
- **Sizes and spacing**: Uses the existing icon scale and occupies a fixed square so pending state cannot resize its owner.
- **States**: Default means actively loading; hover, active, focus, and disabled are inapplicable because Spinner is not interactive; loading is its only semantic state; empty and error are inapplicable, and error replaces loading through Alert/status content.
- **Accessibility**: The icon is hidden from assistive technology. The owning Button or region uses `aria-busy`, and a nearby live region provides text such as loading, switching, or updating.
- **Motion**: One-second linear rotation. Under reduced motion, rotation stops; the static icon and readable loading text remain.

### Separator

- **Structure**: One horizontal or vertical line between owned regions. It may be semantic when it separates named groups or decorative when the surrounding structure already communicates grouping.
- **Variants**: Horizontal and vertical orientations.
- **Sizes and spacing**: One-pixel line using the default or subtle border tone. Spacing is owned by the surrounding layout, not embedded as arbitrary separator margins.
- **States**: Default is visible; hover, active, focus, disabled, loading, empty, and error are all inapplicable because Separator is non-interactive and carries no state.
- **Accessibility**: Use semantic separator role and orientation only when the boundary adds structure for assistive technology; otherwise mark it decorative.
- **Motion**: None. Separators never animate, grow, or draw in.

### Tooltip

- **Structure**: One Provider, Root, Trigger, Portal, and Content. Trigger props are applied to the single existing interactive control, usually an icon Button; no wrapper button is introduced.
- **Variants**: One compact informational style. It names icon-only New and Copy actions and similarly unfamiliar compact controls.
- **Sizes and spacing**: Small typography, 4px vertical and 8px horizontal padding, 6px radius, one-pixel border, and overlay elevation. Provider delay is 300ms.
- **States**: Default is closed/rest; hover opens after the delay; active is inapplicable to content; focus opens from keyboard focus on the trigger; disabled must not be the only way to explain why a control is disabled and the accessible name remains present; loading, empty, and error are inapplicable. Open content must not change trigger geometry.
- **Accessibility**: Tooltip text supplements, never replaces, the trigger accessible name. Keyboard focus must reveal it. Content is concise, non-interactive, and dismisses on Escape or loss of trigger context according to the underlying primitive.
- **Motion**: Opacity plus a minimal transform may animate for at most 100ms. Reduced motion removes the transform. No bouncing, cursor-following, or delayed layout movement.

### Primitive Consumption Matrix

| Product surface | Button | Input | Textarea | Checkbox | Badge | Alert | Dialog | Spinner | Separator | Tooltip |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Quick Switch root | Settings | - | - | - | - | - | - | Initial loading | Footer boundary | - |
| Current group summary and values | - | - | - | - | - | - | - | - | - | - |
| Quick Switch targets | Switch action | - | - | - | Active marker only when semantics remain equivalent | Empty/status composition when required | - | Pending switch | - | - |
| Settings shell/navigation | Ghost navigation | - | - | - | - | Shell status messages | - | Initial loading | Sidebar/header boundaries | - |
| Groups list | New, select, copy | - | - | - | Active and Draft | - | - | - | List/editor boundaries | New and Copy icon actions |
| Groups editor | New, Save, Cancel, Delete, Switch, Replace, Add | Metadata and mapping fields | Description | Enabled | Counts/status where already concise | Validation and degraded feedback | Delete confirmation | Pending action only where currently represented | Section/footer boundaries | Icon-only actions only |
| Primitive QA fixture only | Every approved variant/state | Every approved state | Every approved state | All three values/states | Every tone | Every tone/action role | Closed/open/focus/Escape | Normal/reduced motion | Both orientations | Rest/hover/focus/open |

## 6. Motion & Interaction

### Timing

| Interaction | Duration | Easing | Contract |
| --- | --- | --- | --- |
| Hover, pressed, focus color feedback | `100-150ms` | `ease-out` | Color, border, outline, and opacity only |
| Checkbox indicator | `100ms` | `ease-out` | Opacity and minimal scale only |
| Tooltip open/close | `100ms` | `ease-out` | Opacity and minimal transform |
| Dialog open/close | `150ms` | `ease-out` | Overlay opacity and content opacity/minimal scale |
| Spinner | `1000ms` | `linear` | Continuous rotation while busy |

### Interaction Rules

- Motion communicates hover, pressed, focus, opening, closing, or pending work. There is no decorative idle animation.
- Animate only transform, opacity, filter, color, border color, background color, and outline color. Never animate layout dimensions, grid tracks, padding, margin, or position.
- Hover never carries information that is unavailable through text, selection state, focus, or accessible semantics.
- Active navigation uses current-page semantics. Active groups and switch targets retain explicit text or accessible labels in addition to tonal selection.
- Pending operations expose `aria-busy`, block duplicate activation where required, retain control geometry, and preserve other visible status channels.
- Focus is never removed. Every interactive primitive uses the same 2px ring and 2px offset. Focus restoration after switching and dialog closure is part of product behavior, not optional polish.
- Disabled means unavailable and non-interactive. Read-only means inspectable/selectable but not editable. Degraded means a subsection is constrained while saved values and an explanation remain visible.
- Error, warning, success, and informational messages preserve their current live-region roles. Do not convert every status row into an assertive alert.
- Under `prefers-reduced-motion: reduce`, stop Spinner rotation, remove dialog/tooltip scale or translation, and make non-essential state transitions immediate. State, text, focus, and busy semantics remain unchanged.

## 7. Depth & Surface

### Strategy: Border and Tonal Shift with Overlay-Only Elevation

The single depth strategy is a compact combination of one-pixel boundaries and adjacent tonal shifts. The app does not simulate floating web cards. Macro panes are separated by default borders; sections use subtle borders; selected, hovered, active, toolbar, sidebar, status, and degraded regions use the documented surface tones. Shadows are exceptional elevation for the outer floating window and portal-based overlays only.

| Depth level | Treatment | Approved use |
| --- | --- | --- |
| Base | App surface behind panel surface | Native/browser fixture background |
| Panel | Panel surface with no shadow | Quick Switch and Settings detail panes |
| Muted structural surface | Muted surface plus default or subtle boundary | Sidebar, toolbar, alerts, empty state |
| Interactive tonal state | Hover or active surface, no new shadow | Rows, navigation, ghost/secondary controls |
| Degraded boundary | Muted surface, one-pixel warning border, 8px radius | Read-only OpenCode agent-mapping and unavailable operating-system sections |
| Window elevation | Existing window shadow plus one-pixel outer frame | Floating Quick Switch/settings fixture only |
| Overlay elevation | Existing popover shadow plus one-pixel default border | Dialog and Tooltip only |

Status does not create a separate elevation system. Success, warning, error, and information use semantic icon/text/border treatment inside the same muted or panel surfaces. Dialog backdrops use a token-derived translucent app surface; they are not gradients or decorative atmosphere.

No component may introduce an undeclared shadow, border width, border color, blur, inset highlight, glass treatment, or elevation level. New depth requires an explicit update to this section before implementation.

### Prohibited Patterns

- No stock shadcn dashboard composition, generic admin template, marketing hero, feature-card grid, explanatory promo copy, or decorative visual asset.
- No Card, Sidebar, Tabs, Switch, AlertDialog, Label, Formsnap, Superforms, schema-driven form layer, or registry primitive outside the ten approved Section 5 primitives.
- No cards around page sections, nested cards, floating section containers, or card-like wrappers around the existing panes.
- No gradients, decorative blobs, orbs, bokeh, noise overlays, glassmorphism, backdrop blur, or ornamental shadows.
- No radius larger than 8px except the pill radius used only for badges and status dots.
- No roomy web spacing, oversized headings, viewport-scaled type, negative letter spacing, or mobile-first landing-page proportions.
- No new color outside Section 2, no hard-coded visual spacing outside the 4px scale, and no ad hoc control height.
- No theme toggle, persisted theme preference, class-based dark mode, or product-specific override of the operating-system theme.
- No animation that ignores reduced motion, changes layout geometry, delays a command, or serves only decoration.
- No text clipping, hidden action labels, icon-only control without an accessible name, or tooltip used as the sole accessible label.
- No undeclared borders or shadows. If a surface cannot be explained by the depth table above, it is outside this contract.
