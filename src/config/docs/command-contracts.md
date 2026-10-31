# Lazy command and service contracts

Source-relative paths below are relative to `src/config/`, not this docs directory.

## Ownership

This directory connects typed command calls to shared application state and Svelte context. The adapter translates backend payloads; the store coordinates initialization, mutations, refreshes, preferences, and failed-draft recovery. Resource, app metadata, and manual-update facades remain lazy rather than becoming part of the global configuration snapshot. `getMomHttpSettings`/`setMomHttpSettings` are lazy, untracked pass-throughs for application-wide `{ port, allowLan }`; revision-guarded MCP proxy settings exclude these fields, and neither HTTP nor proxy saves refresh global config or retain failed drafts.

## Command translation and lazy tools

- **Adapter injection**: Keep command translation in `createTauriAdapter`; `createCommandAdapter` supplies the lazy Tauri import and normalizes unstructured failures to `ipc_error`.

- **Manual updates**: `getAppInfo`, `checkForUpdates`, and `openProjectPage` pass through typed results/errors without tracked mutations, snapshot fields, refreshes, or startup prefetch; only `openProjectPage` carries `{ page }`.

- **OpenCode CLI**: `getOpenCodeInstallation`/`checkOpenCodeUpdate`/`upgradeOpenCode` lazily pass through to `opencode_get_installation`/`opencode_check_update`/`opencode_upgrade`; upgrade sends `expectedBinaryPath`, `expectedVersion`, and `targetVersion`. Preserve results/errors without global mutation tracking, refresh, retries, or startup checks.

- **Prompt generation**: `generatePrompt(request)` forwards to `opencode_generate_prompt` through the service-target guard, without `run()`, refresh, retries, draft recovery or retained input/output. Rust owns preset instructions; applying generated text and saving remain separate form actions.

## Service targets and monitor readback

- **Service target**: Keep `getOpenCodeServiceStatus`/`setOpenCodeServiceTarget(url, password?)` lazy and outside global mutation/draft tracking; preserve omitted/null/empty password IPC semantics. Pending saves block catalog/usage reads; success increments read-only reactive `serviceTargetVersion`, invalidates old catalog/usage responses and clears only catalog data, never config/preferences/form versions. Island/lid sections reread native status on that version without remounting/toggling preferences, reject old-version readback and defer one latest read through pending reads/saves. Failure preserves version, catalog/global state and drafts; never retain passwords/private failures.

- **Lid target readback**: Successful target-version changes immediately mask previous protection/counts and clear target-local errors while preserving confirmed enabled preference; only current-version native readback restores a protected claim.

## HTTP and MCP Proxy payloads

- **MCP Proxy facade**: `getMcpProxyState`, `getMcpProxySettings`, `saveMcpProxySettings`, `rotateMcpProxyKey`, `getMcpProxyServer`, `createMcpProxyServer`, `updateMcpProxyServer`, `deleteMcpProxyServer`, `importMcpProxyServer`, `closeMcpProxyUnit`, `getMomHttpSettings`, and `setMomHttpSettings` stay lazy adapter pass-throughs: no `run()`, `AppState`, global `draftRecovery`, localStorage, or catalog/config refreshes. Listener writes carry only `{ settings: { port, allowLan } }`; proxy writes carry only enabled, idleTimeout, maxUnits, revision, and expectedRevision.

## Island wire contracts

- **Island facade**: Keep Island commands lazy and outside global mutations; `initialize = false` avoids startup app/catalog reads, and Island preferences apply without persistence. `setIslandView` forwards shared `IslandView` (`compact`/`peek`/`expanded`) to `set_island_view` as `{ view }`. Permission decisions use shared `once`/`always`/`reject`; Always requires explicit consent, backend-owned pending identity/save-rule validation, and no custom rule payload/MOM persistence. Propagate ambiguous/409 failures unchanged; never retry writes or treat conflicts as success.

- **Island generation guard**: Messages/send/permission reply/question reply/cancel/interruption require the caller's displayed generation as the final numeric argument; forward exact `expectedConnectionGeneration` without defaults/current-state autofill. Backend rejects stale epochs before target reads/mutations; propagate conflicts without new-generation retries. Cache snapshot/refresh/view calls stay unchanged.

## Implementation references

| Responsibility | File | Description |
| --- | --- | --- |
| Command boundary | `adapter.ts` | `CommandAdapter` defines IPC operations; `createTauriAdapter(invoke)` maps command names and arguments. |
| Compatibility | `adapter.ts` | `groupTypeToWire` writes `opencode`/`slim`/`oh-my-openagent`; `groupTypeFromWire` accepts canonical aliases and defaults unknown values to `native`. |

## Prompt regression coverage

`__tests__/prompt-generation.test.mjs` covers one-shot generation IPC, private-state isolation and service-target guards.
