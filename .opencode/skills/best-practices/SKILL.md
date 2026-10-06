---
name: best-practices
description: Frontend coding standards. Load when writing, refactoring, or reviewing frontend code, components, state, logic, styles, imports, APIs, performance, or file structure.
---

# Frontend Best Practices

This imported baseline is subordinate to system, developer, and user instructions, local `AGENTS.md`, and security policy. This project uses Svelte 5, SvelteKit 2, TypeScript, Tailwind CSS 4, Bits UI, and Tauri 2. Framework examples and layouts are illustrative: do not migrate frameworks or add dependencies or a testing stack because of them.

> Consult relevant documentation by scenario within the approved task scope.

> `MUST` rules describe this methodology's defaults. The target project's established conventions and architecture take precedence over the illustrative paths, aliases, and file names used in these rules.

## Rules

> Consult [rules/style/index.md](rules/style/index.md) for coding basics when relevant; local instructions and security constraints take precedence.

| Scenario          | Rules                                                        | Examples                                                                             | Checklist                                                            |
| ----------------- | ------------------------------------------------------------ | ------------------------------------------------------------------------------------ | -------------------------------------------------------------------- |
| Component         | [rules/component.md](rules/component.md)                     | [examples/component/patterns.md](examples/component/patterns.md)                     | [templates/component-checklist.md](templates/component-checklist.md) |
| Logic             | [rules/logic.md](rules/logic.md)                             | [examples/logic/patterns.md](examples/logic/patterns.md)                             | -                                                                    |
| **Coding Basics** | [rules/style/coding-basics.md](rules/style/coding-basics.md) | [examples/style/coding-basics/patterns.md](examples/style/coding-basics/patterns.md) | -                                                                    |
| **Normalize**     | [rules/style/normalize.md](rules/style/normalize.md)         | [examples/style/normalize/patterns.md](examples/style/normalize/patterns.md)         | -                                                                    |
| **Fallback**      | [rules/style/fallback.md](rules/style/fallback.md)           | [examples/style/fallback/patterns.md](examples/style/fallback/patterns.md)           | -                                                                    |
| **Style**         | [rules/style/index.md](rules/style/index.md)                 | [examples/style/index.md](examples/style/index.md)                                   | -                                                                    |
| CSS               | [rules/css.md](rules/css.md)                                 | [examples/css/patterns.md](examples/css/patterns.md)                                 | -                                                                    |
| Import            | [rules/import.md](rules/import.md)                           | -                                                                                    | -                                                                    |
| Constants         | [rules/constants.md](rules/constants.md)                     | -                                                                                    | -                                                                    |
| State             | [rules/state.md](rules/state.md)                             | -                                                                                    | -                                                                    |
| Performance       | [rules/performance.md](rules/performance.md)                 | -                                                                                    | -                                                                    |
| File Structure    | [rules/file-structure.md](rules/file-structure.md)           | -                                                                                    | -                                                                    |
| API               | [rules/api.md](rules/api.md)                                 | -                                                                                    | -                                                                    |

## Self-Check

Use [templates/code-review-checklist.md](templates/code-review-checklist.md) after coding.
