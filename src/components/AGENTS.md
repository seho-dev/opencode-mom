# Shared UI Components

## Overview

Shared native and Bits UI controls support forms, tables, overlays, and configuration feedback. Domain-aware dashboard components sit beside prop-driven building blocks; Lucide supplies icon-based controls.

## Directory Structure

```text
components/                       # Shared UI library; no top-level barrel
├── button/                        # Native button or anchor; variant and size exports
├── checkbox/                      # Bits UI Checkbox with checked/indeterminate indicators
├── combobox/                      # Free-text input with a Bits UI Popover option list
├── configuration/                 # Configuration feedback and error inspection
├── dashboard/                     # Runtime groups and activity UI
├── dialog/                        # Bits UI Dialog wrappers plus native header/footer
├── input/                         # Native input with separate file-input binding branch
├── switch/                        # Native checkbox styled as a switch
├── tooltip/                       # Bits UI Tooltip root/provider/trigger/content/portal wrappers
├── DataTable.svelte               # Table snippet container and conditional Pagination
├── FormActions.svelte             # Shared form-actions snippet container
├── FormPage.svelte                # Opt-in available-height page frame; inactive details scroll normally
├── PageHead.svelte                # Eyebrow/title with actions or children snippet
├── ResourceScope.svelte           # Local shared project/Global picker and forget-folder dialog
├── ResourceScopeNotice.svelte     # Read-only detail destination and selected root path
├── Select.svelte                  # Fixed-choice Bits UI Popover picker; search above eight options
└── __tests__/                     # Scope picker, form layout contract and native submit tests
```

## Key Files

| Responsibility | File | Description |
| --- | --- | --- |
| Control APIs | `button/index.ts`, `checkbox/index.ts`, `dialog/index.ts` | Public aliases and typed primitive boundaries. |
| Selection | `combobox/combobox.svelte`, `Select.svelte` | Free-text versus fixed-choice selection. |
| Form layout | `FormPage.svelte`, `FormActions.svelte`, `PageHead.svelte` | Reusable editor frame and action composition. |
| Resource scope | `ResourceScope.svelte` | Saved-folder browsing and Global destinations. |
| Runtime actions | `dashboard/SwitchGroupDialog.svelte` | Config-backed switching, reloads, and focus restoration. |

## Conventions

- **Public APIs**: Reuse existing family aliases and native or Bits prop types; preserve bindings and styling
  hooks. See [control contracts](docs/control-contracts.md#primitive-apis-and-bindings) before changing
  wrappers.

- **Accessible controls**: Preserve labels, keyboard behavior, focus restoration, and disabled semantics.
  Follow the [interaction contracts](docs/control-contracts.md#keyboard-and-accessibility), especially for
  Checkbox and portaled pickers.

- **Form composition**: Keep form bodies and actions inside native forms; read-only frames must not remount
  children. Use the shared [composition contracts](docs/composition-contracts.md#forms-and-tables).

- **Table ownership**: Callers own row selection and pagination data; keep action cells as table cells. Review
  [table composition](docs/composition-contracts.md#forms-and-tables) and [cell
  layout](docs/control-contracts.md#table-cell-layout).

- **Resource scope**: Keep project selection local to resource lists and persist through config; forgetting a
  folder never deletes files. Follow [resource
  workflows](docs/composition-contracts.md#resource-and-prompt-workflows).

- **Prompt previews**: Apply generated text only after confirmation, replacing the local draft without saving.
  Preserve the shared [preview workflow](docs/composition-contracts.md#resource-and-prompt-workflows) and lazy
  model reads.

- **Dashboard hierarchy**: Keep runtime identity readable and telemetry controls independent; preserve Heatmap
  scrolling and keyboard access. Consult [dashboard
  composition](docs/composition-contracts.md#dashboard-hierarchy) for layout boundaries.

- **Interaction tests**: Use role-based queries and the existing harnesses; verify portaled focus and keyboard
  behavior. Keep layout stubs and mock cleanup within [local
  tests](docs/control-contracts.md#keyboard-and-accessibility).

## Sync

Keep this guide and linked docs in sync with changes to this directory.
