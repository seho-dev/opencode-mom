# Tauri Commands

`app.rs` exposes `get_mom_http_settings` / `set_mom_http_settings` for application-wide `{ port, allowLan }`. MCP settings contain only enabled, timeout, capacity and revision; MCP state reads its bound address/network settings from managed `HttpServer`, and reports running only when HTTP, proxy enablement and authentication are available. Proxy settings never restart the shared HTTP listener.

## Overview

Domain-scoped Tauri IPC handlers translate frontend requests into backend operations for agents, app state, autostart, OpenCode, groups, models, power, providers, MCP, skills, and manual updates. Backend modules own the underlying operations; handlers adapt DTOs, validate command inputs, and send window notifications.

## Directory Structure

```text
commands/              # IPC handler boundary
├── AGENTS.md           # Directory-specific instructions
├── mod.rs              # Domain exports and tray handler re-exports
├── agent.rs            # Agent DTO conversion, CRUD, and model validation
├── app.rs              # App state, preferences, project folders, and native chooser
├── app/                # App command tests
│   └── tests.rs        # AppStateResponse serialization and mock project-add boundary tests
├── autostart.rs        # Read and toggle platform autostart
├── group.rs            # Save, copy, delete, and switch groups
├── island.rs           # Mini Island fixed IPC contract and managed service requests
├── mcp_proxy.rs        # Built-in MCP HTTP proxy IPC handlers and DTO mappings
├── mcp_proxy/          # MCP proxy IPC tests
│   └── tests.rs        # Serialization, secret masking, CRUD and conflict tests
├── model.rs            # Provider-scoped model CRUD and reference checks
├── opencode.rs         # OpenCode service operations and CLI discovery/installation/update/upgrade
├── power.rs            # Lid protection through managed PowerService
├── provider.rs         # Provider CRUD and deletion reference checks
├── resources.rs        # MCP and skill list, get, and mutation handlers
└── updates.rs          # Actual package metadata, manual update check, and fixed project links
```

## Key Files

| Responsibility | File | Description |
| :--- | :--- | :--- |
| Command exports | `mod.rs` | Declares domain modules and re-exports their handlers plus `crate::tray::*`. |
| Agent boundary | `agent.rs` | Converts source/effective data to DTOs; selects storage and validates model references. |
| App state | `app.rs` | Returns `AppStateResponse`; saves preferences and project metadata; runs the native folder chooser in spawn_blocking. |
| App command tests | `app/tests.rs` | Checks snapshot serialization and rejects Global-overlapping project adds before persistence through mock IPC, without opening the chooser. |
| Autostart | `autostart.rs` | Uses the autostart plugin on macOS, Windows, and Linux; rejects unsupported platforms. |
| OpenCode bridge | `opencode.rs` | Delegates catalog, binary, installation, explicit update/upgrade, reload, active-session, and token-usage operations to `opencode`; existing `opencode_*` IPC names stay unchanged. |
| Group mutations | `group.rs` | Delegates persistence/projection to `groups`; tray switches notify the main window. |
| Mini Island | `island.rs` | Cached state, enable/disable, explicit compact/peek/expanded view, refresh, messages and guarded session actions through `IslandService`. |
| MCP Proxy boundary | `mcp_proxy.rs` | Built-in MCP HTTP proxy state, settings, key rotation, server CRUD, import, and unit teardown IPC handlers. |
| Model mutations | `model.rs` | Parses `ModelRef` and blocks deletion when references remain. |
| Power boundary | `power.rs` | Delegates reads and toggles to managed `PowerService`. |
| Provider mutations | `provider.rs` | Lists providers and checks references before deletion. |
| Resource boundary | `resources.rs` | Delegates MCP operations directly and skill operations through blocking tasks. |
| Update boundary | `updates.rs` | Reads `AppHandle.package_info()` for the actual version; delegates manual release checks and fixed-destination browser opens. |

## Conventions

- **Domains**: Keep handlers in their resource module; delegate persistence and system operations to the existing backend modules.
- **Exposure**: Use snake_case `#[tauri::command]` functions, re-export through `mod.rs`, and add `commands::name` to `../lib.rs`'s `invoke_handler` list.
- **Managed state**: Config-backed handlers accept `State<'_, ConfigPaths>`; power handlers accept `State<'_, PowerService>` instead.
- **Resource scope**: Agent/MCP/skill reads accept optional projectId; omitted/null remains Global regardless of persisted selection. Reuse ConfigPaths::for_project and existing domain functions. All resource mutations reject a provided projectId before reading or writing; no project edit API.
- **Project folders**: pick_project_folder uses only the Rust dialog plugin and returns null on cancellation. add_project validates the canonical root against captured Global config/resource boundaries before models.rs persistence; scoped reads recheck fixed sources. Filesystem roots/overlaps return `validation_failed`, missing roots `not_found`, without Global fallback; no home/depth blacklist. add/remove/select share existing AppConfig persistence/locking. projects and selectedProjectId join AppStateResponse; stale selections normalize null on config load. No frontend dialog/fs capability is needed.
- **Reads and mutations**: `load_app_state`, `list_*`, and `get_*` expose reads; `create_*`, `update_*`, `delete_*`, `save_*`, `copy_group`, `switch_group`, and `set_*` perform mutations. `list_models` returns provider-grouped `Vec<ProviderDef>`.
- **Command results**: Return `Result<T, AppError>` directly or via local `CommandResult<T>`. Serialization yields frontend `CommandError` fields `{ code, message, detail? }`; codes are `not_found`, `references_blocked`, `validation_failed`, `configuration_failed`, and `busy` for guarded upgrades or service-target operations.
- **Agent inputs**: Creation/update require explicit storage and consume `mutation`; validate nonempty model references against existing provider models. Updates require `expectedPath`/`expectedContent` copied from `sources[].resolvedPath`/`content`; content is backend-generated inline agent JSON text or full Markdown, not JavaScript-stringified `raw`. Missing snapshots return `validation_failed`; stale snapshots are rejected. Create payloads remain unchanged. Deletion may infer storage only for an unambiguous source.
- **Deletion guards**: Collect agent references before agent deletion; reject referenced models/providers with `AppError::references` and location details before deleting them.
- **Wire DTOs**: `AgentDefinitionDto`, `AgentSourcePreviewDto`, and `AppStateResponse` serialize fields as camelCase; `AgentStorageDto` serializes variants as snake_case.
- **Notifications**: Emit `app:preferences-changed` to `tray` and `island` after saving preferences; tray-origin group switches emit `tray:config-changed` to `main`. Log notification failures without failing a completed mutation.
- **Island boundary**: Never load the aggregate app catalog for the island. State/preferences reads are cache-only; successful preference saves update the cache before notifying. Failed window setup returns an error with disabled readback. Explicit Refresh restarts a missing/error connection while enabled without writes. Messages/send/permission/question/cancel/interrupt require the caller's `expectedConnectionGeneration`; reject mismatches under the same cache lock before selecting a session or cloning the API/action token. Missing arguments never default to the current generation. Keep enabled/API/action-generation guards through mutation-lock waits and completion; cancellation never claims an accepted POST was withdrawn. Never retry writes automatically. Message SSE notifies only the expanded selected session. Session arguments use Tauri's default camelCase mapping.
- **Island view**: Async `set_island_view { view: 'compact' | 'peek' | 'expanded' }` keeps the enum-to-void wire contract; capture enabled generation before `on_main`, return the native operation's oneshot receiver from that dispatch and await it outside the main thread/cache/TLS borrow. Resolve only after still-current final native animation readback; latest pending replacement returns a safe superseded error, never a stale success/focus/rollback. Snapshot notifications are content-only, peek is nonactivating, only explicit expanded focuses, and native compact/blur/close immediately cache compact, clear selection and emit empty `island:collapsed` before shrink without session actions. State includes the cache-only numeric `connectionGeneration` for target-local UI lifetimes.
- **Blocking work**: Async OpenCode catalog/service-status/target-save/installation/update/upgrade/reload/session/usage, skill, update-check, and project-opener handlers use `tauri::async_runtime::spawn_blocking`; map join failures to configuration errors.
- **MCP proxy async ownership**: Import project resolution/discovery/conversion runs through `spawn_blocking_task`. Proxy save/rotation owns publication and changed/deleted service cleanup inside `ProxyService`; IPC handlers must not add cancellable teardown after a completed save. Explicit unit close propagates cleanup errors after all teardown branches finish.
- **Service target IPC**: `opencode_get_service_status` probes locally without bootstrap. `opencode_set_service_target(url?, password?)` prepares pure power/Island guards under the shared transition mutex, persists the target, then invalidates old observations/connections before releasing the guards. Persistence failure leaves monitors unchanged. Wake/reconnect happens outside the transition, before the status probe; post-commit refresh errors never turn a saved target into a save failure. Preserve enabled preferences and current authorization; disabled features stay disabled. Active bootstrap/writes or initial power authorization can return `busy`, without requiring disabled monitors. Null URL returns Shared and clears password; omitted/null password retains only on unchanged normalized URL, empty clears. Save offline succeeds with connected=false status. DTOs always serialize nullable url/pid/version/error, expose only hasPassword, and never include password.
- **OpenCode installation**: `opencode_get_installation` is local-only; `opencode_check_update` is an explicit stable V2 check. `opencode_upgrade` accepts `expectedBinaryPath`, `expectedVersion`, and `targetVersion` through default Tauri camelCase mapping and delegates all upgrade guards to the installation module. No service lifecycle actions or automatic checks/upgrades.
- **Manual updates**: `get_app_info` performs no network work; `check_for_updates` has no caller-controlled endpoint or credentials; `open_project_page` accepts only the domain-validated `repository`/`releases` destination names. No automatic checks or installation.
- **MCP proxy**: `get_mcp_proxy_state` returns actual state, boolean `desiredState`, nullable `boundAddr`, owner-budget `activeUnits`, unit capacity, and sanitized summaries without commands or env; `get_mcp_proxy_settings` and `save_mcp_proxy_settings` manage gateway configuration guarded by `expectedRevision` and never expose tokens or verifiers; `rotate_mcp_proxy_key` publishes a fresh key before bumping in-memory revision and closing active sessions; server CRUD operations validate server configurations, reject duplicate or missing IDs, close associated sessions on update/delete, and enforce `expectedRevision`; `import_mcp_proxy_server` discovers servers from project or global MCP configs, validating expected source paths before converting; `close_mcp_proxy_unit` validates caller `expectedGeneration` to prevent killing units from obsolete runtime generations.
- **Tests**: `app.rs` includes `app/tests.rs` via `#[cfg(test)] mod tests` for serialization and mock add-project IPC guards. Filesystem and agent snapshot/JavaScript numeric-roundtrip coverage in `../../tests/app.rs` uses temporary homes with `ConfigPaths::for_home`, never the native chooser.

## Sync

This file is maintained as the directory scope changes. When files in this directory are added, modified, or deleted (components, logic units, utilities, constants, etc.), update this file's directory structure, key files, and conventions to keep the documentation in sync with the code.
