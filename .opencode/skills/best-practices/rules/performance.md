# Performance Optimization Rules

## Core Principles

- **Code Splitting & Lazy Loading**: Split routes or defer heavy non-critical components when current workload, loading budgets, profiling, or established platform constraints justify the boundary; do not split speculatively.
- **Selective Memoization**: Memoize computations only when workload, budgets, or profiling justify the cost. Stabilize callback identity only when a consumer actually depends on it for correctness or a justified optimization; dependency lists must remain accurate.
- **DOM Virtualization**: Virtualize when representative workload, rendering budgets, or profiling show that DOM cost warrants it; item count alone is not a universal threshold.

## MUST

| Rule | Description |
| ---- | ----------- |
| Justified code-splitting | Base loading boundaries on current workload, profiling, budgets, or established constraints, not a requirement to load every route asynchronously |
| No heavy sync work in render | Avoid running heavy synchronous loops, deep cloning, or large JSON parsing inside component render passes |
| Unique and stable keys | Collection items must use unique, stable entity IDs for keys, not array indices when items can reorder/filter |

## SHOULD

| Rule | Description |
| ---- | ----------- |
| Memoize expensive transforms | Cache transformations with framework tools (`useMemo`, `computed`) when workload, budgets, or profiling justify it, not merely because they sort or group |
| Stable callback references | Stabilize identity only when the consumer depends on it; reuse an existing callback when possible, and always keep effect/callback dependencies accurate |
| Virtualize costly datasets | Use virtualization when representative workload, rendering budgets, or profiling establish a need; preserve stable keys and interaction correctness |

---

## Examples

Consult [../examples/performance.md](../examples/performance.md) for concrete patterns and examples.
