# Shared UI Components

## Overview

Reusable controls, overlays, feedback, and app-level table, form, dashboard, and configuration components. Bits UI powers selected headless primitives; other families wrap native HTML or Lucide icons. Domain-aware dashboard components sit alongside otherwise prop-driven UI building blocks.

## Directory Structure

```text
components/                       # Shared UI library; no top-level barrel
├── AGENTS.md                      # Directory-specific documentation
├── alert/                         # Native alert container, title, description, action; variant exports
├── badge/                         # Native span with status variants
├── button/                        # Native button or anchor; variant and size exports
│   ├── button.svelte              # Control implementation and exported prop types
│   ├── index.ts                   # Root/Button aliases, types, and buttonVariants
│   └── __tests__/button.test.ts    # DOM click smoke test
├── checkbox/                      # Bits UI Checkbox with checked/indeterminate indicators
│   ├── checkbox.svelte            # Control implementation and exported prop types
│   ├── index.ts                   # Root/Checkbox aliases and CheckboxProps types
│   └── __tests__/                 # checkbox.test.ts and checkbox-harness.svelte
├── combobox/                      # Free-text input with a Bits UI Popover option list
│   └── __tests__/                 # dropdown.test.ts and dropdown-harness.svelte; also exercises Select
├── configuration/                 # Configuration feedback and error inspection
│   ├── Diagnostics.svelte         # Status banner listing diagnostic messages
│   └── read-error.ts              # readError and isNotFound helpers
├── context-menu/                  # Native div/menu/button wrappers, not Bits UI ContextMenu
├── dashboard/                     # Runtime groups and activity UI
│   ├── ActivityHeatmap.svelte     # Prop-driven activity calendar with keyboard cell navigation
│   ├── RuntimeCard.svelte         # Config-backed group summary for one runtime type
│   ├── TokenUsageChart.svelte     # Compact prop-driven token trend chart
│   ├── SwitchGroupDialog.svelte   # Config-backed switch/reload dialogs and trigger focus restoration
│   └── __tests__/                 # Token chart aggregation, ratios, and unavailable-state behavior
├── dialog/                        # Bits UI Dialog wrappers plus native header/footer
├── input/                         # Native input with separate file-input binding branch
├── separator/                     # Bits UI Separator
├── slider/                        # Native range input
├── spinner/                       # Lucide Loader2 SVG with size variants
├── switch/                        # Native checkbox styled as a switch
├── textarea/                      # Native textarea with bindable value/ref
├── toast/                         # Native alert/status presentation with action and close callbacks
├── tooltip/                       # Bits UI Tooltip root/provider/trigger/content/portal wrappers
├── DataTable.svelte               # Table snippet container and conditional Pagination
├── EmptyTableRow.svelte           # Empty-state tr/td with colspan and message
├── FormActions.svelte             # Shared form-actions snippet container
├── FormPage.svelte                # Opt-in available-height page frame; inactive details scroll normally
├── PageHead.svelte                # Eyebrow/title with actions or children snippet
├── ResourceScope.svelte           # Local shared project/Global picker and forget-folder dialog
├── ResourceScopeNotice.svelte     # Read-only detail destination and selected root path
├── GlobalDestination.svelte       # Global write destination notice on direct forms
├── __tests__/                     # Scope picker, form layout contract and native submit tests
├── Pagination.svelte              # Bindable page and previous/next controls
└── Select.svelte                  # Fixed-choice Bits UI Popover picker; search above eight options
```

## Key Files

| Responsibility | File | Description |
| --- | --- | --- |
| Public family API | `button/index.ts`, `checkbox/index.ts`, `dialog/index.ts`, `tooltip/index.ts` | Component aliases; button and checkbox also expose prop types. |
| Variants | `button/button.svelte`, `alert/alert.svelte`, `badge/badge.svelte`, `toast/toast.svelte` | Module-script `tv()` definitions and `VariantProps` types. |
| Modal composition | `dialog/dialog-content.svelte` | Portal, overlay, children snippet, optional close control, destructive variant. |
| Free-text selection | `combobox/combobox.svelte` | Bindable string, label/value filtering, keyboard highlight, portaled options. |
| Fixed-choice selection | `Select.svelte` | Exports `SelectOption`; skips disabled choices and optionally focuses search. |
| Resource browsing | `ResourceScope.svelte`, `ResourceScopeNotice.svelte`, `GlobalDestination.svelte` | Saved root selection shared only by MCP/Agent/Skill lists; read-only details and explicit Global forms. |
| Table composition | `DataTable.svelte`, `Pagination.svelte`, `EmptyTableRow.svelte` | Table contents come from callers; pagination controls do not slice rows. |
| Page/form composition | `PageHead.svelte`, `FormPage.svelte`, `FormActions.svelte` | Snippet-based heading, available-height page frame and in-flow footer. |
| Notifications | `toast/toast.svelte` | Presentation only; actions and dismissal are supplied as callbacks. |
| Dashboard | `dashboard/RuntimeCard.svelte`, `dashboard/SwitchGroupDialog.svelte`, `dashboard/ActivityHeatmap.svelte`, `dashboard/TokenUsageChart.svelte` | Runtime switching, activity-calendar display, and token trend visualization. |
| Configuration feedback | `configuration/Diagnostics.svelte`, `configuration/read-error.ts` | Diagnostic messages and inspection of unknown errors. |
| Interaction coverage | `checkbox/__tests__/checkbox.test.ts`, `combobox/__tests__/dropdown.test.ts` | Checkbox states/form inputs and Combobox/Select portals, focus, selection, dismissal, and disabled options. |

## Conventions

- **Dashboard hierarchy**: Keep runtime tiles compact and readable even when switching is unavailable; use the featured workbench for current-group identity and bindings. Telemetry panels preserve independent controls and align at the panel boundary; responsive refinements must retain Heatmap scrolling and roving keyboard focus.

- **Barrels**: Primitive-family folders expose `index.ts`; preserve existing short and prefixed aliases. Badge and spinner export named components without `Root`; dashboard/configuration and top-level components have no barrel.
- **Class merging**: Class-based wrappers destructure `class` as `className` and use `cn(baseOrVariants, className)` from `../utils/index.ts`; it combines `clsx` and `tailwind-merge`.
- **Prop contracts**: Native wrappers use `svelte/elements` attributes, often `WithElementRef`; Bits wrappers use primitive `*Props`. Use existing `WithoutChildren`/`WithoutChildrenOrChild` helpers when wrappers own child rendering.
- **Binding and children**: Preserve exposed bindable refs, values, checked/indeterminate state, and open state. Native elements use `bind:this`; Bits primitives use `bind:ref`; snippet containers render their typed children.
- **Bits composition**: Dialog, Tooltip, Checkbox, and Separator forward remaining props to Bits primitives. Dialog/Tooltip content wraps its portal internally; Popover pickers spread both `wrapperProps` and `props` in child snippets.
- **Checkbox semantics**: Associate labels by explicit `id`/`for`. Space toggles; Enter does not toggle or submit by default. Preserve Bits' real button ref without adding native-input properties; assert `aria-checked`/`data-state` and harness state in tests, never `button.checked`. Named checkboxes retain Bits' hidden form input; Switch retains its native input. Checkbox tests also cover QuestionForm option/custom keyboard behavior and field-key/option-value submission.
- **Styling hooks**: Wrappers combine Tailwind classes with tokens from `../styles/tokens.css`; preserve `data-slot` hooks. Table, heading, form, and feedback classes are shared with `../styles/table.css`, `head.css`, `forms.css`, and `feedback.css`.
- **Picker accessibility**: Preserve combobox/listbox IDs, expanded/selected state, arrow/Enter selection, Escape/Tab dismissal, and outside-focus handling. Combobox keeps input focus; Select restores trigger focus after selection and skips disabled options.
- **Scope picker**: Select supports opt-in always-search, pinned choice, stacked path hints, and globe icons. ResourceScope persists selection only through config, restores the confirmed selection on failure, treats native picker cancellation as a no-op, and disables management during writes but not list reads. Forgetting a folder never deletes its files.
- **Accessible names**: Switch consumers provide an external label or `aria-label`; Select accepts `ariaLabel` or a label targeting its `id`. Spinner is decorative; Toast uses alert for warning/error and status otherwise.
- **Disabled navigation**: Button anchors use `aria-disabled`, remove navigation and Tab entry, and share native disabled styling.
- **Table ownership**: `DataTable` receives a table-content snippet and total count, with page size defaulting to five; callers own row selection. Keep PageHead's explicit actions snippet precedence over children.
- **Form ownership**: Use `FormPage` around the heading and editor; `active={false}` leaves read-only details in normal document flow without remounting their children. Editors keep `.form-body` and `FormActions` as siblings inside the native form so only fields/notices scroll, while submit ownership and keyboard behavior remain native. Dialog editors reuse the body/actions boundary within their available height. Ordinary help uses shared `.field-hint` or `.form-hint`; safety feedback keeps its separate semantics.
- **Local tests**: Follow the button click test and dropdown harness: role/name queries, `fireEvent`, and `waitFor` for portaled state. The dropdown harness installs i18n; tests stub missing JSDOM layout/scroll APIs and restore mocks.

## Anti-patterns

- **Table cells**: Do not turn `td.row-actions` into a flex container; `../styles/table.css` keeps it a table cell to preserve column sizing.

## Sync

This file is maintained as the directory scope changes. When files in this directory are added, modified, or deleted (components, logic units, utilities, constants, etc.), update this file's directory structure, key files, and conventions to keep the documentation in sync with the code.
