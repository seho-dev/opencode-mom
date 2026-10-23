# Business Logic & Async Rules

## Core Principles

- **Separation of Presentation & Domain Logic**: Components focus on rendering and user interaction; business calculation, async orchestration, and state transitions belong in dedicated hooks, composables, or domain service functions.
- **Explicit Lifecycle & Cleanup**: Side effects that establish listeners, timers, or ongoing network requests must be safely cleaned up on unmount or dependency change.
- **Predictable Async States**: Asynchronous workflows must handle pending, fulfilled, and rejected outcomes explicitly.

## MUST

| Rule | Description |
| ---- | ----------- |
| Separation of concerns | Complex business calculations and multi-step data transformations live outside UI components |
| Effect cleanup | Clean up subscriptions, DOM event listeners, intervals, and pending network requests on teardown |
| Error boundaries | Async functions must handle errors gracefully without unhandled promise rejections |
| No stale closures | Effect and callback dependencies must remain accurate and synchronized |

## SHOULD

| Rule | Description |
| ---- | ----------- |
| Reusable logic abstraction | Abstract cross-cutting or shared logic into standard composables/hooks matching framework idioms |
| Abortable requests | Pass `AbortSignal` to async requests when ongoing work should be canceled on re-trigger or unmount |
| Return explicit states | Logic units handling async operations should expose state (`data`, `isLoading`, `error`) and actions |

---

## Examples

See [../examples/logic.md](../examples/logic.md) for concrete patterns and code examples.
