# Engineering Conventions & Project Structure

## Core Principles

- **Feature & Domain Boundary**: Organize codebase by business features or architectural layers. Directories express domain ownership; files express specific artifacts.
- **Decoupled API Layer**: Keep HTTP client configuration, endpoint contracts, and request functions outside UI components in a designated API/service layer.
- **Named Domain Constants & Enums**: Name non-obvious domain values near their owner using `SCREAMING_SNAKE_CASE`. Use a shared constants module only for actual shared ownership; small clear literals and local expressions can stay inline.
- **Path Aliases over Deep Traversal**: Use configured module path aliases (e.g. `@/`) for cross-domain imports. Never write fragile relative paths (`../../../../`).

## MUST

| Rule | Description |
| ---- | ----------- |
| Path aliases | Cross-domain and deep imports must use the project's configured alias (e.g. `@/components/`, `@/utils/`) |
| No deep relative imports | Prohibit relative traversal deeper than parent directory (`../` is okay for siblings; `../../..` is banned) |
| Decoupled API calls | UI components must not call raw HTTP libraries (`axios.get`, `fetch`) inline; import from an API/service module |
| Named domain values | Give non-obvious business statuses and fixed domain values meaningful names near their owner; do not extract every literal into a standalone constants module |
| File naming consistency | Components use `PascalCase`; logic/hooks and utilities use `camelCase`; styles and configs use `kebab-case` |

## SHOULD

| Rule | Description |
| ---- | ----------- |
| Directory naming | Directories use `kebab-case` (e.g. `order-detail/`, `user-profile/`) |
| Structured import order | 3rd-party dependencies → internal alias modules → local relative modules → style files |
| File size diagnostic | Use size as a prompt to inspect cohesion, not a quota; split by coherent responsibility only when it improves understanding or ownership |

---

## Examples

See [../examples/conventions.md](../examples/conventions.md) for concrete patterns and code examples.
