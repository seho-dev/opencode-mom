# Logic & Asynchronous Flow Rules

## Core Principles

- **Separation of Presentation & Domain Logic**: Separate complex domain policy and async orchestration from rendering when they form a coherent responsibility. Simple calculations, state updates, and local handlers can remain in components; a dedicated hook or service is not required for every operation.
- **Explicit Lifecycle & Cleanup**: Side effects that establish listeners, timers, or ongoing network requests must be safely cleaned up on unmount or dependency change. Cancel obsolete work or ignore stale completions where these changes can race.
- **Predictable Async States**: Workflow owners represent relevant pending, fulfilled, and rejected outcomes explicitly. Lower-level helpers may return values or promises without their own state wrappers.

## MUST

| Rule | Description |
| ---- | ----------- |
| Separation of concerns | Give complex domain policy, transformations, and orchestration a clear responsibility boundary; keep simple local handlers in the component |
| Effect cleanup | Clean up subscriptions, DOM event listeners, intervals, and pending network requests on teardown |
| Async error ownership | The designated owner recovers, translates, or reports failures so they remain observable; other layers propagate errors, without mandatory catches at every async layer or unhandled rejections |
| No stale closures | Effect and callback dependencies must remain accurate and synchronized |

## SHOULD

| Rule | Description |
| ---- | ----------- |
| Justified logic extraction | Extract for domain meaning, cognitive clarity, actual reuse, or a responsibility/effect boundary; use hooks/composables when framework lifecycle or reactive state calls for them |
| Abortable requests | Pass `AbortSignal` to async requests when ongoing work should be canceled on re-trigger or unmount |
| Return explicit states | Workflow owners expose state (`data`, `isLoading`, `error`) and actions when consumers need them; lower-level helpers can return values or promises |

---

## Examples

Consult [../examples/logic.md](../examples/logic.md) for concrete patterns and examples.
