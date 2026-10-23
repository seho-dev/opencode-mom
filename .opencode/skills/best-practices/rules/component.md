# Component Architecture & Design Rules

## Core Principles

- **Unidirectional Data Flow**: Props flow down, events flow up. Components are view boundaries that project inputs into UI and emit user intent.
- **Single Source of Truth**: State has one owner. Downstream components read values or emit events; they must never mirror parent state into local state just to keep them in sync.
- **Pure Derivation**: Compute derived values synchronously on the fly. Do not sync derived state back to parent via callbacks or cache them in redundant local state.
- **Behavioral Imperative Handles**: Reserve imperative handles (`ref`) for actions (focus, scroll, reset). Never expose raw state setters or duplicate state snapshots.

## MUST

| Rule | Description |
| ---- | ----------- |
| Unidirectional flow | Parent owns state; child receives data and emits event callbacks |
| Explicit prop destructuring | Destructure props with default parameters at the component signature |
| No state mirroring | Never duplicate parent props into local state unless intentionally creating an isolated draft buffer |
| No direct DOM queries | Direct DOM queries (`document.querySelector`, `getElementById`) are prohibited; use framework refs |
| Behavior-only handles | Imperative handles expose actions (`focus()`, `reset()`), never internal state mutators |
| Accessible tab navigation | Tabs must use `role="tablist"`/`tab`/`tabpanel`, roving `tabindex`, and arrow key roving |
| Async action feedback & guards | Buttons must indicate state via `aria-busy`, disabled guard (`disabled={isPending || isSaving}`), and motion-safe indicator |
| Context-aware action dispatch | Global/header actions in multi-tab or multi-scoped views must target the active context |
| Standardized table empty states | Tables must use shared empty/loading row primitives; ad-hoc `<tr><td colspan>` prohibited |
| Bounded raw content containers | Raw text, `<pre>`, logs, and JSON previews must have explicit max-bounds and overflow containment |
| Form vs detail page frames | Constrained forms scroll only `.form-body` above pinned actions; read-only details preserve document flow |

## SHOULD

| Rule | Description |
| ---- | ----------- |
| Semantic prop ordering | Group props: identity/data → state/flags → event callbacks (`onXxx` last) |
| Early returns | Return early for loading, empty, and error states before primary render |
| Pure helper extraction | Stateless rendering helpers without closure dependencies should live outside the component |

---

## Examples

See [../examples/component.md](../examples/component.md) for concrete patterns and code examples.
