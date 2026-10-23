# Performance Optimization Rules

## Core Principles

- **Code Splitting & Lazy Loading**: Split bundles by route and defer heavy non-critical components (e.g. rich text editors, charts, modal drawers) until needed.
- **Selective Memoization**: Memoize expensive computations and callback references when passing to memoized children to prevent cascade re-renders. Avoid speculative memoization on trivial computations.
- **DOM Virtualization**: When rendering collections with hundreds or thousands of items, virtualize the list to keep active DOM nodes constant.

## MUST

| Rule | Description |
| ---- | ----------- |
| Route code-splitting | Application routes and heavy conditional modules must be loaded asynchronously |
| No heavy sync work in render | Avoid running heavy synchronous loops, deep cloning, or large JSON parsing inside component render passes |
| Unique and stable keys | Collection items must use unique, stable entity IDs for keys, not array indices when items can reorder/filter |

## SHOULD

| Rule | Description |
| ---- | ----------- |
| Memoize expensive transforms | Cache sorting, grouping, or complex transformations with framework memoization tools (`useMemo`, `computed`) |
| Stable callback references | Keep event handler references stable when passed to memoized children or effect dependency arrays |
| Virtualize large datasets | Use virtualized lists for tables and feeds exceeding viewport capacity (~100+ items) |

---

## Examples

See [../examples/performance.md](../examples/performance.md) for concrete patterns and code examples.
