# Configuration Boundary and State

## Overview

This directory connects typed command calls to shared application state and Svelte context. The adapter translates backend payloads; the store coordinates initialization, mutations, refreshes, preferences, and failed-draft recovery. Resource facades remain lazy rather than becoming part of the global configuration snapshot.

## Directory Structure

```text
src/config/                         # Command translation and shared state
├── adapter.ts                      # CommandAdapter implementations and group-type conversion
├── context.ts                      # Symbol-keyed ConfigStore context access
├── store.svelte.ts                 # Reactive state, refresh sequencing, and recovery
└── __tests__/                      # Node regression tests with Vite module loading
    ├── adapter.test.mjs            # Command names, payloads, and wire conversions
    ├── config-refresh.test.mjs     # Initialization, reload, races, selection, and drafts
    └── resources-bridge.test.mjs   # Lazy resource calls and secret-draft protection
```

## Key Files

| Responsibility | File | Description |
| :--- | :--- | :--- |
| Command boundary | `adapter.ts` | `CommandAdapter` defines IPC operations; `createTauriAdapter(invoke)` maps command names and arguments. |
| Compatibility | `adapter.ts` | `groupTypeToWire` writes `opencode`/`slim`/`oh-my-openagent`; `groupTypeFromWire` accepts canonical aliases and defaults unknown values to `native`. |
| Shared state | `store.svelte.ts` | `createConfigStore(adapter, catalogOnRefresh)` exposes reactive getters, actions, and operation-specific errors. |
| Context access | `context.ts` | `setConfig` and `getConfig` share the store through a private `Symbol('config-store')`. |

## Conventions

- **Adapter injection**: Keep command translation in `createTauriAdapter`; `createCommandAdapter` supplies the lazy Tauri import and normalizes unstructured failures to `ipc_error`.
- **Mutation lifecycle**: Use `run(operation, payload, action)` for tracked writes; successful provider/model/agent/group writes then refresh, while MCP/skill writes return saved entries without a global refresh.
- **Request ownership**: Preserve separate state/catalog sequence counters; only the newest response updates values or errors. State loading uses an outstanding-request count; catalog loading belongs to its latest request.
- **Refresh failures**: Automatic refresh tolerates catalog failure after a successful mutation. `refreshAll` rejects catalog failure through `refreshAllError`; failed catalog loads retain the previous catalog.
- **Reload lifecycle**: `reloadOpencode` invokes the CLI before refreshing, prevents overlapping reloads, and holds the splash until completion. Initialization also waits for its catalog attempt.
- **Draft recovery**: Snapshot non-secret failed payloads with `structuredClone`; `reloadKeepingDraft` preserves recovery and form versions, while discard refresh clears recovery and increments `formResetVersion`.
- **Secret drafts**: MCP/skill create/update failures retain no payload or error detail and use a generic message; leave sensitive content in its original form.
- **Selection**: Apply `selectedGroupId` from app state, defaulting missing values to `null`; do not infer selection from `Group.isEnabled` or update it before refreshed state arrives.
- **Preferences**: Apply theme/locale optimistically to state and the document; restore both when preference persistence fails.
- **Regression harness**: Tests use `node:test`, injected adapters, and Vite `ssrLoadModule`; close each Vite server in `finally`. Pass `false` to `createConfigStore` when catalog work is intentionally excluded.

## Sync

This file is maintained as the directory scope changes. When files in this directory are added, modified, or deleted (components, logic units, utilities, constants, etc.), update this file's directory structure, key files, and conventions to keep the documentation in sync with the code.
