# Defensive Coding & Simplicity Rules

## Core Principles

- **Zero Speculative Abstraction**: Write direct code first. Never extract a one-line wrapper function or create intermediate helper layers simply to claim code is "more testable" or "more decoupled". Extract only when real business logic is encapsulated, duplication is eliminated, or a public boundary is formed.
- **Normalize at Boundaries**: Parse and normalize untrusted, heterogeneous, or raw external inputs (API responses, URL params, local storage) once at the ingestion boundary. Downstream components and logic must receive predictable, strongly typed shapes.
- **Single-Hop Defensive Fallback**: One fallback hop is healthy (`value ?? defaultValue`). Long guessing chains (`a ?? b ?? c ?? d`) signal an ambiguous data contract that should be normalized at the source rather than patched in UI render logic.
- **Native JS First**: Standard JavaScript methods (`map`, `filter`, `find`, `some`, `every`, `includes`) are clear and self-documenting. Do not invent thin alias functions around them.

## MUST

| Rule | Description |
| ---- | ----------- |
| Direct expressions first | Do not extract trivial single-caller wrapper functions that add no domain meaning |
| Normalize at ingestion | Transform raw API enums, casings, or nullish properties at the parse layer once |
| Single lookup tables | Use lookup dictionaries / maps for status/enum mappings instead of sprawling `if/else` or `switch` chains |
| Single-hop fallbacks | Provide immediate, safe defaults; avoid cascading fallback chains in display code |
| Predictable nullish handling | Prefer `??` and `?.` over loose truthy `||` to prevent falsy bugs with `0`, `""`, and `false` |

## SHOULD

| Rule | Description |
| ---- | ----------- |
| Fail-safe return shapes | Boundary parsers should return guaranteed empty shapes (`[]` for missing lists, `{}` for missing objects) |
| Early exit / guard clauses | Check boundary conditions early to eliminate nested blocks |

---

## Examples

See [../examples/defensive.md](../examples/defensive.md) for concrete patterns and code examples.
