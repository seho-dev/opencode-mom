# Utils

## Overview

Centralizes domain values/options, class composition, builtin-agent/model selection, and dashboard
token/calendar calculations. Helpers consume supplied data or the shared agent catalog, never loading
config or performing IPC.

## Directory Structure

```text
utils/
├── AGENTS.md
├── constants.ts  # Domain tuples, unions, labels, options
├── dashboard.ts  # Runtime bindings and token/calendar statistics
├── index.ts      # Classes, prop types, builtin agents, model helpers
└── __tests__/    # Dashboard, model-limit, and model-option coverage
```

## Key Files

| Responsibility | File | Description |
| :--- | :--- | :--- |
| Domain vocabulary | `constants.ts` | Defines readonly value tuples, derived unions, MessageKey labels, options, and OMO categories. |
| Shared helpers | `index.ts` | Class composition, builtin catalog, model selection, token validation, and JSON object conversion. |
| Dashboard calculations | `dashboard.ts` | Projects runtime bindings, aggregates timezone-aware daily usage, and builds period summaries. |

## Conventions

- **Domain vocabulary**: Extend tuples in `constants.ts` and derive unions/options from them; keep label
  maps typed as `MessageKey` and translate component option labels through `getI18n().t()`.
- **Class composition**: Use `cn(...inputs)` for conditional classes and Tailwind conflict merging;
  it applies `clsx` before `twMerge`.
- **Builtin catalog**: Derive `BUILTIN_AGENTS` and reserved IDs from `shared/agents.catalog.json`;
  use `isBuiltinAgentId` for membership checks.
- **Model choices**: Preserve catalog priority and unavailable current selections when extending model
  options; retain the linked ordering, empty-option, and variant-selection contracts.
- **Token safety**: Validate token inputs before aggregation; reject malformed, unsafe, or overflowing
  counts rather than silently coercing them. Keep exact parsing and error behavior in the linked contracts.
- **Calendar semantics**: Respect the supplied timezone and UTC period arithmetic; keep future/padding
  usage null rather than zero, with week and grid alignment defined in the linked contracts.
- **Existing coverage**: Extend matching helper tests; model `.test.mjs` tests load `index.ts` through
  Vite's `ssrLoadModule` and must close the server in `finally`.

## Details

| Section | File |
| --- | --- |
| Selection and usage calculations | [docs/model-and-usage-contracts.md](docs/model-and-usage-contracts.md) |

## Sync

Keep this guide and linked docs in sync with changes to this directory.
