# Lazy command and service contracts

Source-relative paths below are relative to `src/config/`, not this docs directory.

## Ownership

This directory connects typed command calls to shared application state and Svelte context. The adapter translates backend payloads; the store coordinates initialization, mutations, refreshes, preferences, and failed-draft recovery. Resource, app metadata, and manual-update facades remain lazy rather than becoming part of the global configuration snapshot. `getMomHttpSettings`/`setMomHttpSettings` are lazy, untracked pass-throughs for application-wide `{ port, allowLan }`; revision-guarded MCP proxy settings exclude these fields, and neither HTTP nor proxy saves refresh global config or retain failed drafts.

## Command translation and lazy tools

- **Adapter injection**: Keep command translation in `createTauriAdapter`; `createCommandAdapter` supplies the lazy Tauri import and normalizes unstructured failures to `ipc_error`.

- **Manual updates**: `getAppInfo`, `checkForUpdates`, and `openProjectPage` pass through typed results/errors without tracked mutations, snapshot fields, refreshes, or startup prefetch; only `openProjectPage` carries `{ page }`.

- **OpenCode CLI**: `getOpenCodeInstallation`/`checkOpenCodeUpdate`/`upgradeOpenCode` lazily pass through to `opencode_get_installation`/`opencode_check_update`/`opencode_upgrade`; upgrade sends `expectedBinaryPath`, `expectedVersion`, and `targetVersion`. Preserve results/errors without global mutation tracking, refresh, retries, or startup checks.

- **Prompt generation**: `generatePrompt(request)` forwards to `opencode_generate_prompt` through the service-target guard, without `run()`, refresh, retries, draft recovery or retained input/output. Rust owns preset instructions; applying generated text and saving remain separate form actions.

- **Plugin package updates**: `updatePluginPackage(target)` forwards to backend `update_plugin_package` with Global-only scope. Updating validates Global eligibility and health before dispatch; HTTP 204 from the runtime confirms success and resolves void. The subsequent UI runtime list query is an independent read where status-unavailable warnings do not alter the confirmed update. On network timeout or bad HTTP status, the outcome is unknown without automatic retries. Safe diagnostic categories surface as `detail` strings on the existing `configuration_failed` code.

- **Native OS notifications**: `getNotificationSettings()` and `setNotificationSettings(settings)` lazily forward to backend `get_notification_settings` / `set_notification_settings` with typed `NotificationSettings` (`{ enabled, agentCompleted, approvalRequired, agentError }`). They are untracked pass-throughs without `AppState`, snapshots, recovery, prefetch, or `localStorage`; `enabled` defaults `false` and the three categories default `true` but have no effect when off. Failure surfaces as `CommandError`, matching the frontend type. OS notification settings and DND / Focus modes govern delivery; Windows requires the installed app.

- **Notification permission**: No-argument `getNotificationPermission`, `requestNotificationPermission`, and `openNotificationSettings` map to `get_notification_permission`, `request_notification_permission`, and `open_notification_settings`. Get/request accept only `prompt | granted | denied | unavailable | unsupported`; malformed results fail with a fixed `configuration_failed` message. Permission stays outside saved preferences and global state. Reads never request authorization or open settings. Only explicit user actions request permission; a master off-to-on change saves successfully before requesting, while categories, disabling, failed saves and stale completions never request. Authorization failure cannot roll back a confirmed preference save.

## Local diagnostic logs

`getLoggingState`, `setLoggingEnabled`, `queryLogs`, `openLogsDirectory` and `clearLogs` are lazy, untracked adapter pass-throughs. They neither prefetch at startup nor enter `AppState`, `AppPreferences`, draft recovery or localStorage. Enablement is persisted by Rust as `state.loggingEnabled`, independently of theme/locale preferences.

`setLoggingEnabled` forwards only `{ enabled }`; `queryLogs` forwards `{ query }` with the type/status/time/operation filters and opaque cursor unchanged. Other logging commands have no payload. Never wrap all IPC calls in a payload/error logger: request bodies and raw errors can contain credentials or private content. The Settings section owns confirmed switch readback; `/logs` owns its bounded pages and operation detail requests.

## Service targets and monitor readback

- **Service target**: Keep `getOpenCodeServiceStatus`/`setOpenCodeServiceTarget(url, password?)` lazy and outside global mutation/draft tracking; preserve omitted/null/empty password IPC semantics. Pending saves block catalog/usage reads; success increments read-only reactive `serviceTargetVersion`, invalidates old catalog/usage responses and clears only catalog data, never config/preferences/form versions. Island/lid sections reread native status on that version without remounting/toggling preferences, reject old-version readback and defer one latest read through pending reads/saves. Failure preserves version, catalog/global state and drafts; never retain passwords/private failures.

- **Lid target readback**: Successful target-version changes clear target-local errors while preserving the confirmed enabled preference and queue one latest native read through pending reads or saves. The switch reflects only that preference; the frontend does not display protection phases or session counts, or claim that protection is active.

## HTTP and MCP Proxy payloads

- **HTTP and Proxy facades**: `getMomHttpSettings`, `setMomHttpSettings`, `rotateMomHttpKey`, `getMcpProxyState`, `getMcpProxySettings`, `saveMcpProxySettings`, `getMcpProxyServer`, `createMcpProxyServer`, `updateMcpProxyServer`, `deleteMcpProxyServer`, `importMcpProxyServer`, and `closeMcpProxyUnit` stay lazy adapter pass-throughs: no `run()`, `AppState`, global `draftRecovery`, localStorage, or catalog/config refreshes. Listener writes carry only `{ settings: { port, allowLan } }`; proxy writes carry only enabled, idleTimeout, maxUnits, revision, and expectedRevision.

- **Shared HTTP token**: `rotateMomHttpKey()` invokes `rotate_mom_http_key` without arguments and returns the generated string directly. Rotation is independent of Proxy enablement and reads, and neither results nor failures enter shared state or recovery. HTTP settings remain `{ port, allowLan }`, without a token or revision. The General dialog alone owns one-time display and cleanup.

## Island wire contracts

- **Island facade**: Keep Island commands lazy and outside global mutations; `initialize = false` avoids startup app/catalog reads, and Island preferences apply without persistence. `setIslandView` forwards shared `IslandView` (`compact`/`peek`/`expanded`) to `set_island_view` as `{ view }`. Permission decisions use shared `once`/`always`/`reject`; Always requires explicit consent, backend-owned pending identity/save-rule validation, and no custom rule payload/MOM persistence. Propagate ambiguous/409 failures unchanged; never retry writes or treat conflicts as success.

- **Island generation guard**: Messages/send/permission reply/question reply/cancel/interruption require the caller's displayed generation as the final numeric argument; forward exact `expectedConnectionGeneration` without defaults/current-state autofill. Backend rejects stale epochs before target reads/mutations; propagate conflicts without new-generation retries. Cache snapshot/refresh/view calls stay unchanged.

## Implementation references

| Responsibility | File | Description |
| --- | --- | --- |
| Command boundary | `adapter.ts` | `CommandAdapter` defines IPC operations; `createTauriAdapter(invoke)` maps command names and arguments. |
| Compatibility | `adapter.ts` | `groupTypeToWire` writes `opencode`/`slim`/`oh-my-openagent`; `groupTypeFromWire` accepts only frozen wire types `opencode`/`slim`/`oh-my-openagent` and rejects unknown/legacy values (`native`/`omo`). |

## Prompt regression coverage

`__tests__/prompt-generation.test.mjs` covers one-shot generation IPC, private-state isolation and service-target guards.
