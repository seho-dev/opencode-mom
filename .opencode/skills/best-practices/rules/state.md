# State Management Rules

## Core Principles

- **State Minimization**: Store only essential, raw state. If a value can be computed from existing state or props, calculate it dynamically on the fly.
- **Single Source of Truth**: Every piece of state has a designated owner. Shared state can stay in a common ancestor; use context or a store when ownership and access needs warrant it.
- **Avoid Prop Drilling**: Use composition, context, or an existing store when pass-through props obscure ownership or create real propagation/maintenance friction. Component depth alone does not justify a store.
- **Immutable Updates**: State transitions must be pure and immutable to ensure predictable change detection and render consistency.

## MUST

| Rule | Description |
| ---- | ----------- |
| No duplicated state | Never copy shared state or props into local state unless intentionally creating an isolated editing buffer |
| Predictable mutations | Shared state updates go through its owner's callbacks, actions, dispatchers, or store methods |
| Shallow state scope | Place state as close to where it is consumed as possible; do not lift to global scope prematurely |

## SHOULD

| Rule | Description |
| ---- | ----------- |
| Derive over sync | Calculate filtered lists, counts, and active items directly in render or memoized selectors |
| Component composition | Prefer slot/children composition when it removes real pass-through or ownership friction; clear prop chains can remain |

---

## Examples

Consult [../examples/state.md](../examples/state.md) for concrete patterns and code examples.
