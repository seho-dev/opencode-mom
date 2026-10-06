# Utils

## Overview

Centralizes domain values and derived options, class-name composition, shared builtin-agent and model-selection helpers, and dashboard token/calendar calculations. These helpers consume supplied data or the shared agent catalog; they do not load config or perform IPC.

## Directory Structure

```text
utils/                            # Shared domain and presentation helpers
├── AGENTS.md                     # Directory-specific guidance
├── constants.ts                  # Domain tuples, unions, label keys, and options
├── dashboard.ts                  # Runtime bindings and token/calendar statistics
├── index.ts                      # cn, prop types, builtin agents, and model helpers
└── __tests__/                    # Helper behavior coverage
    ├── dashboard.test.ts         # Dates, periods, counts, and runtime bindings
    ├── model-limit.test.mjs      # Raw token-count validation
    └── model-options.test.mjs    # Catalog priority and unavailable selections
```

## Key Files

| Responsibility | File | Description |
| :--- | :--- | :--- |
| Domain vocabulary | `constants.ts` | Defines readonly value tuples, derived unions, MessageKey labels, options, and OMO categories. |
| Shared helpers | `index.ts` | Composes classes, maps the shared agent catalog, validates token strings, and builds model choices. |
| Dashboard calculations | `dashboard.ts` | Projects runtime bindings, aggregates timezone-aware daily usage, and builds period summaries. |

## Conventions

- **Domain values**: Extend the tuples in `constants.ts` and derive unions/options from them; group values here are `native`, `slim`, and `omo`.
- **Label keys**: Keep `STORAGE_LABELS` and `GROUP_TYPE_LABELS` typed as `MessageKey`; components translate option labels with `getI18n().t()`.
- **Class composition**: Use `cn(...inputs)` for conditional classes and Tailwind conflict merging; it applies `clsx` before `twMerge`.
- **Builtin catalog**: Derive `BUILTIN_AGENTS` and reserved IDs from `shared/agents.catalog.json`; use `isBuiltinAgentId` for membership checks.
- **Model choices**: Preserve catalog priority over provider entries, ref sorting, the leading empty option, and unavailable current selections in `buildModelOptions`; `buildModelVariants` also prefers the catalog.
- **Token inputs**: `isValidTokenCount` accepts trimmed digit strings representing safe integers, not unit suffixes or decimals; dashboard aggregation rejects invalid counts and overflow with `RangeError`.
- **Calendar semantics**: Bucket timestamps in the supplied timezone, then use ISO dates and UTC date arithmetic for periods; weeks start Monday, cells are Sunday-aligned, and future/padding usage stays null rather than zero.
- **Runtime bindings**: `groupBindings` includes only the group's active runtime mappings and removes bindings with blank names or model refs; OMO includes categories.
- **Existing coverage**: Extend the matching helper test; the two `.test.mjs` files load `index.ts` through Vite's `ssrLoadModule` and close the server in `finally`.

## Sync

This file is maintained as the directory scope changes. When files in this directory are added, modified, or deleted (components, logic units, utilities, constants, etc.), update this file's directory structure, key files, and conventions to keep the documentation in sync with the code.
