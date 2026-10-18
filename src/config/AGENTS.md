# Configuration Boundary and State

## Overview

This directory connects typed command calls to shared application state and Svelte context. The adapter translates backend payloads; the store coordinates initialization, mutations, refreshes, preferences, and failed-draft recovery. Resource, app metadata, and manual-update facades remain lazy rather than becoming part of the global configuration snapshot.

## Directory Structure

```text
src/config/                         # Command translation and shared state
├── adapter.ts                      # CommandAdapter implementations and group-type conversion
├── context.ts                      # Symbol-keyed ConfigStore context access
├── store.svelte.ts                 # Reactive state, refresh sequencing, recovery, and island facade
└── __tests__/                      # Node regression tests with Vite module loading
    ├── adapter.test.mjs            # Command names, payloads, wire conversions, and update IPC
    ├── config-refresh.test.mjs     # Initialization, reload, races, selection, and drafts
    ├── projects.test.mjs           # Explicit resource scope, project persistence, and snapshot freshness
    ├── resources-bridge.test.mjs   # Lazy resource/update/target calls, secret drafts and target-read freshness
    └── island.test.mjs             # Island IPC arguments and no-preload store behavior
```

## Key Files

| Responsibility | File | Description |
| :--- | :--- | :--- |
| Command boundary | `adapter.ts` | `CommandAdapter` defines IPC operations; `createTauriAdapter(invoke)` maps command names and arguments. |
| Compatibility | `adapter.ts` | `groupTypeToWire` writes `opencode`/`slim`/`oh-my-openagent`; `groupTypeFromWire` accepts canonical aliases and defaults unknown values to `native`. |
| Shared state | `store.svelte.ts` | `createConfigStore(adapter, catalogOnRefresh, initialize)` exposes reactive getters, actions, island facade methods, and operation-specific errors. |
| Context access | `context.ts` | `setConfig` and `getConfig` share the store through a private `Symbol('config-store')`. |

## Conventions

- **Adapter injection**: Keep command translation in `createTauriAdapter`; `createCommandAdapter` supplies the lazy Tauri import and normalizes unstructured failures to `ipc_error`.
- **Manual updates**: `getAppInfo`, `checkForUpdates`, and `openProjectPage` pass through their typed results/errors without tracked mutations, snapshot fields, refreshes, or startup prefetch. Only `openProjectPage` carries a `{ page }` payload.
- **OpenCode CLI**: `getOpenCodeInstallation`, `checkOpenCodeUpdate`, and `upgradeOpenCode` are lazy direct pass-throughs to `opencode_get_installation`, `opencode_check_update`, and `opencode_upgrade`. Upgrade carries `expectedBinaryPath`, `expectedVersion`, and `targetVersion`; preserve results/errors without global mutation tracking, refresh, retries, or startup checks.
- **Service target**: `getOpenCodeServiceStatus` and `setOpenCodeServiceTarget(url, password?)` stay lazy and outside global mutation/draft tracking. Preserve omitted/null/empty password semantics at IPC. Pending saves block catalog/usage reads; successful saves increment the reactive read-only `serviceTargetVersion`, invalidate old catalog/usage responses and clear only catalog data, never config/preferences/form versions. Island/lid settings reread native status on this version without remounting or toggling preferences, reject old-version readback and defer one latest read through pending reads/saves. Failed saves leave the version, catalog/global state and drafts unchanged; never retain passwords or private failures in the store.
- **Mutation lifecycle**: Use `run(operation, payload, action)` for tracked writes; successful provider/model/agent/group writes then refresh, while MCP/skill writes return saved entries without a global refresh.
- **Request ownership**: Preserve separate state/catalog sequence counters; only the newest response updates values or errors. State loading uses an outstanding-request count; catalog loading belongs to its latest request.
- **Refresh failures**: Automatic refresh tolerates catalog failure after a successful mutation. `refreshAll` rejects catalog failure through `refreshAllError`; failed catalog loads retain the previous catalog.
- **Reload lifecycle**: `reloadOpencode` invokes the CLI before refreshing, prevents overlapping reloads, and holds the splash until completion. Initialization also waits for its catalog attempt.
- **Draft recovery**: Snapshot non-secret failed payloads with `structuredClone`; `reloadKeepingDraft` preserves recovery and form versions, while discard refresh clears recovery and increments `formResetVersion`.
- **Secret drafts**: MCP/skill create/update failures retain no payload or error detail and use a generic message; leave sensitive content in its original form.
- **Selection**: Apply `selectedGroupId` from app state, defaulting missing values to `null`; do not infer selection from `Group.isEnabled` or update it before refreshed state arrives.
- **Project browsing**: `projects` defaults to `[]`; `selectedProjectId` defaults to `null` and stale IDs normalize to Global. Picker calls are lazy pass-through; add/remove/select use independent `projectSaving` guarding and commit state only after persistence succeeds, without CLI/catalog refresh or draft recovery. Project versions protect local commits from older global snapshot reads.
- **Resource scope**: Agent/MCP/skill facades take an optional explicit project ID; absent/null means Global and leaves existing IPC payloads unchanged. Never infer scope from shared selection or replace global `agents` with project reads. Resource writes may forward an explicit project ID for backend rejection; global writes retain their existing lifecycle.
- **Preferences**: Apply theme/locale optimistically to state and the document; restore both when preference persistence fails.
- **Regression harness**: Tests use `node:test`, injected adapters, and Vite `ssrLoadModule`; close each Vite server in `finally`. Pass `false` to `createConfigStore` when catalog work is intentionally excluded.
- **Island facade**: Island commands remain lazy and do not use global config mutations; pass `initialize = false` to avoid startup app/catalog reads, and apply island-provided preferences without persistence. `setIslandView` forwards the shared `IslandView` (`compact`/`peek`/`expanded`) to `set_island_view` with `{ view }`; retain `setIslandExpanded` as the boolean compatibility command. Permission decisions use the shared `once`/`always`/`reject` type; Always is explicit user consent, with pending identity/save-rule validation owned by the backend and no custom rule payload or MOM persistence. Propagate ambiguous/409 failures unchanged; never retry writes or convert conflicts to success.
- **Island generation guard**: Messages/send/permission reply/question reply/cancel/interruption require the caller's displayed generation as their final numeric argument; adapter IPC forwards exact `expectedConnectionGeneration`, without defaults or current-state autofill. Backend rejects stale epochs before reading/mutating the selected target; propagate conflict unchanged and never retry against a new generation. Cache snapshot, refresh and view calls remain unchanged.
- **Lid target readback**: A successful target version change immediately masks previous protection/counts and clears target-local errors while preserving the confirmed enabled preference; only current-version native readback can restore a protected claim.

## Sync

This file is maintained as the directory scope changes. When files in this directory are added, modified, or deleted (components, logic units, utilities, constants, etc.), update this file's directory structure, key files, and conventions to keep the documentation in sync with the code.
