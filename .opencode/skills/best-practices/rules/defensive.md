# Boundary Safety & Simplicity Rules

Language-independent baseline for designing, writing, refactoring, and reviewing code. Use the language's idiomatic validation, error propagation, and resource-lifetime constructs.

## MUST

- **Justify complexity with evidence.** Before adding guards, retries, locks, compatibility branches, or similar machinery, identify the triggering condition, evidence from a requirement/contract/reachable execution path, failure consequence, and existing protection. Do not invent probabilities; omit mechanisms justified only by speculation.
- **Preserve necessary safety.** Security checks, permissions, data integrity, resource safety, and protection against reachable races are not optional complexity. Ask the user when speculative scope adds meaningful complexity or the contract needs a decision, not for ordinary safety work.
- **Validate each relevant trust boundary once.** Parse external inputs into reliable internal contracts with runtime checks for required fields, types, ranges, and allowed values. Types, annotations, and casts alone, in any language, are not runtime validation. Internal functions rely on guarantees already established; a new trust boundary, changed data, or new invariant may require another check.
- **Normalize only agreed representations.** Apply documented transforms, not guessed field aliases, coercions, casing rules, or unknown-value mappings. Reject malformed mandatory data; never map an unknown status to a legitimate status.
- **Default only permitted absence.** A default is valid only when the contract allows absence and defines the default's meaning. Distinguish absence from invalid values and preserve valid zero, false, and empty values. Even a single fallback must not turn invalid required data or failed operations into success, empty collections, or placeholders.
- **Handle errors at a capable owner.** Recover, translate with useful context while preserving the cause, or report at an application/protocol boundary. Otherwise propagate through idiomatic exceptions, results, or error returns. Do not swallow failures, add blanket fallbacks, or catch-log-rethrow at every layer. Report a failure once at the responsible boundary.
- **Always release resources.** Use structured lifecycle constructs or guaranteed cleanup for files, transactions, subscriptions, locks, and other acquired resources, including error and cancellation paths. Cleanup does not require a redundant error handler.

## SHOULD

- **Start direct and local.** Reuse codebase facilities, standard libraries, native/platform features, then existing dependencies before adding abstractions or dependencies.
- **Extract for a real purpose.** Domain meaning, reduced cognitive burden, actual reuse, and responsibility/side-effect boundaries justify extraction. Caller count, line count, or testability alone do not; a useful domain policy may have one caller, while a pass-through helper may have many.
- **Keep one authoritative owner.** Derive dependent data instead of synchronizing copies. Intentional drafts or caches need an explicit lifecycle, not a second competing source of truth.

---

## Examples

Consult [../examples/defensive.md](../examples/defensive.md) for concrete patterns, illustrative pseudocode, and language-independent examples.
