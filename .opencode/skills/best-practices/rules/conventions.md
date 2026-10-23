# Engineering Conventions & Project Structure

## Core Principles

- **Feature & Domain Boundary**: Organize codebase by business features or architectural layers. Directories express domain ownership; files express specific artifacts.
- **Decoupled API Layer**: Keep HTTP client configuration, endpoint contracts, and request functions outside UI components in a designated API/service layer.
- **Centralized Constants & Enums**: Extract magic numbers, business status codes, and configuration keys into dedicated constants modules with `SCREAMING_SNAKE_CASE`.
- **Path Aliases over Deep Traversal**: Use configured module path aliases (e.g. `@/`) for cross-domain imports. Never write fragile relative paths (`../../../../`).

## MUST

| Rule | Description |
| ---- | ----------- |
| Path aliases | Cross-domain and deep imports must use the project's configured alias (e.g. `@/components/`, `@/utils/`) |
| No deep relative imports | Prohibit relative traversal deeper than parent directory (`../` is okay for siblings; `../../..` is banned) |
| Decoupled API calls | UI components must not call raw HTTP libraries (`axios.get`, `fetch`) inline; import from an API/service module |
| No magic strings/numbers | Business statuses and fixed domain values must be imported from constants |
| File naming consistency | Components use `PascalCase`; logic/hooks and utilities use `camelCase`; styles and configs use `kebab-case` |

## SHOULD

| Rule | Description |
| ---- | ----------- |
| Directory naming | Directories use `kebab-case` (e.g. `order-detail/`, `user-profile/`) |
| Structured import order | 3rd-party dependencies → internal alias modules → local relative modules → style files |
| File size budget | Keep files focused (< 300 lines normal; > 500 lines strongly indicates a need to refactor) |

---

## Examples

See [../examples/conventions.md](../examples/conventions.md) for concrete patterns and code examples.
