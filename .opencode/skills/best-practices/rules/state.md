# State Management Rules

## Core Principles

- **State Minimization**: Store only essential, raw state. If a value can be computed from existing state or props, calculate it dynamically on the fly.
- **Single Source of Truth**: Every piece of state has a designated owner. Shared state belongs in a well-defined store, context, or service layer with explicit accessors.
- **Avoid Prop Drilling**: When values or callbacks pass through more than 3 component layers without intermediate consumption, use context, composition, or the project's state store.
- **Immutable Updates**: State transitions must be pure and immutable to ensure predictable change detection and render consistency.

## MUST

| Rule | Description |
| ---- | ----------- |
| No duplicated state | Never copy shared state or props into local state unless intentionally creating an isolated editing buffer |
| Predictable mutations | Shared state mutations must go through explicit actions, dispatchers, or store methods |
| Shallow state scope | Place state as close to where it is consumed as possible; do not lift to global scope prematurely |

## SHOULD

| Rule | Description |
| ---- | ----------- |
| Derive over sync | Calculate filtered lists, counts, and active items directly in render or memoized selectors |
| Component composition | Prefer slot/children composition over passing state down multiple intermediate container components |

---

## Examples

See [../examples/state.md](../examples/state.md) for concrete patterns and code examples.
