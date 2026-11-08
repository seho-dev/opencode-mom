# Configuration state and mutation lifecycle

Source-relative paths below are relative to `src/config/`, not this docs directory.

## Mutations and lossless source snapshots

- **Mutation lifecycle**: Use `run(operation, payload, action)` for tracked writes; successful provider/model/agent/group writes then refresh, while MCP/skill writes return saved entries without a global refresh.

- **Agent snapshots**: Forward `mutation.expectedPath`/`expectedContent` unchanged from frozen backend `sources[].resolvedPath`/`content`; never rebuild the content token from JavaScript raw JSON. Updates require both fields; create is unchanged.

## Request and reload ownership

- **Request ownership**: Preserve separate state/catalog sequence counters; only the newest response updates values or errors. State loading uses an outstanding-request count; catalog loading belongs to its latest request.

- **Refresh failures**: Automatic refresh tolerates catalog failure after a successful mutation. Explicit `refreshAll` or reload rejects state or catalog failure and reports it only through `config.error`; automatic and local catalog reads report through `catalogError`. Failed catalog loads retain the previous catalog.

- **Reload lifecycle**: `reloadOpencode` invokes the selected service configuration reload before refreshing, prevents overlapping reloads, and holds the splash until completion. Initialization also waits for its catalog attempt.

## Drafts and committed preferences

- **Draft recovery**: Snapshot non-secret failed payloads with `structuredClone`; `reloadKeepingDraft` preserves recovery and form versions. A failed refresh preserves recovery, and a successful full refresh clears it and resets forms only if no new recovery appeared during the read. Explicit discard clears recovery before refreshing.

- **Secret drafts**: MCP/skill create/update failures retain no payload or error detail and use a generic message; leave sensitive content in its original form.

- **Selection**: Apply `selectedGroupId` from app state, defaulting missing values to `null`; do not infer selection from `Group.isEnabled` or update it before refreshed state arrives.

- **Preferences**: Apply theme/locale optimistically to state and the document; restore both when preference persistence fails.

## Explicit project resource scope

- **Project browsing**: Default `projects` to `[]` and `selectedProjectId` to `null`; normalize stale IDs to Global. Picker calls stay lazy; add/remove/select use independent `projectSaving` guards and commit only after persistence, without service reload/catalog refresh/draft recovery. Project versions protect local commits from older global snapshots.

- **Resource scope**: Agent/MCP/skill facades take optional explicit project IDs; absent/null means Global with unchanged IPC payloads. Never infer scope from shared selection or replace global `agents` with project reads. Writes may forward a project ID for backend rejection; Global writes keep their lifecycle.

## Regression harness

- **Regression harness**: Tests use `node:test`, injected adapters, and Vite `ssrLoadModule`; close each Vite server in `finally`. Pass `false` to `createConfigStore` when catalog work is intentionally excluded. Shared `__tests__/vite-harness.mjs` exports `createServer(options)`, `withServer(fn, options?)`, and `deferred()`: `withServer` runs the callback with a server and closes it in `finally`, so one callback may load several modules and existing file-level shared callbacks with subtests keep their lifetime.

## Implementation references

| Responsibility | File | Description |
| --- | --- | --- |
| Shared state | `store.svelte.ts` | `createConfigStore(adapter, catalogOnRefresh, initialize)` exposes reactive getters, actions, island facade methods, and operation-specific errors. |
| Context access | `context.ts` | `setConfig` and `getConfig` share the store through a private `Symbol('config-store')`. |
