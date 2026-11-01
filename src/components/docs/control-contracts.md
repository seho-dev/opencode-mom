# Control APIs and accessible interaction

Source-relative paths below are relative to `src/components/`, not this docs directory.

## Primitive APIs and bindings

- **Barrels**: Primitive-family folders expose `index.ts`; preserve short/prefixed aliases. Badge/spinner export named components without `Root`; dashboard/configuration and top-level components have no barrel.

- **Class merging**: Class-based wrappers destructure `class` as `className` and use `cn(baseOrVariants, className)` from `../utils/index.ts`; it combines `clsx` and `tailwind-merge`.

- **Prop contracts**: Native wrappers use `svelte/elements` attributes, often `WithElementRef`; Bits wrappers use primitive `*Props`. Use existing `WithoutChildren`/`WithoutChildrenOrChild` helpers when wrappers own child rendering.

- **Binding and children**: Preserve exposed bindable refs, values, checked/indeterminate state, and open state. Native elements use `bind:this`; Bits primitives use `bind:ref`; snippet containers render their typed children.

- **Bits composition**: Dialog, Tooltip, Checkbox, and Separator forward remaining props to Bits primitives. Dialog/Tooltip content wraps its portal internally; Popover pickers spread both `wrapperProps` and `props` in child snippets.

- **Styling hooks**: Combine Tailwind classes with `../styles/tokens.css` tokens and preserve `data-slot`. Table/heading/form/feedback classes are shared with `../styles/table.css`, `head.css`, `forms.css`, and `feedback.css`.

## Keyboard and accessibility

- **Checkbox semantics**: Use explicit `id`/`for`; Space toggles, Enter neither toggles nor submits by default. Preserve Bits' real button ref without native-input properties; tests assert `aria-checked`/`data-state` and harness state, never `button.checked`. Named checkboxes retain Bits' hidden form input; Switch retains its native input. Cover QuestionForm option/custom keyboard behavior and field-key/option-value submission.

- **Picker accessibility**: Preserve combobox/listbox IDs, expanded/selected state, arrow/Enter selection, Escape/Tab dismissal, and outside-focus handling. Combobox keeps input focus; Select restores trigger focus after selection and skips disabled options.

- **Accessible names**: Switch consumers provide an external label or `aria-label`; Select accepts `ariaLabel` or a label targeting its `id`. Spinner is decorative; Toast uses alert for warning/error and status otherwise.

- **Disabled navigation**: Button anchors use `aria-disabled`, remove navigation and Tab entry, and share native disabled styling.

- **Local tests**: Follow the button click test and dropdown harness: role/name queries, `fireEvent`, and `waitFor` for portaled state. The dropdown harness installs i18n; tests stub missing JSDOM layout/scroll APIs and restore mocks.

## Implementation references

| Responsibility | File | Description |
| --- | --- | --- |
| Public family API | `button/index.ts`, `checkbox/index.ts`, `dialog/index.ts`, `tooltip/index.ts` | Component aliases; button and checkbox also expose prop types. |
| Variants | `button/button.svelte`, `alert/alert.svelte`, `badge/badge.svelte`, `toast/toast.svelte` | Module-script `tv()` definitions and `VariantProps` types. |
| Modal composition | `dialog/dialog-content.svelte` | Portal, overlay, children snippet, optional close control, destructive variant. |
| Free-text selection | `combobox/combobox.svelte` | Bindable string, label/value filtering, keyboard highlight, portaled options. |
| Fixed-choice selection | `Select.svelte` | Exports `SelectOption`; skips disabled choices and optionally focuses search. |
| Notifications | `toast/toast.svelte` | Presentation only; actions and dismissal are supplied as callbacks. |
| Interaction coverage | `checkbox/__tests__/checkbox.test.ts`, `combobox/__tests__/dropdown.test.ts` | Checkbox states/form inputs and Combobox/Select portals, focus, selection, dismissal, and disabled options. |

## Table cell layout

- **Table cells**: Do not turn `td.row-actions` into a flex container; `../styles/table.css` keeps it a table cell to preserve column sizing.
