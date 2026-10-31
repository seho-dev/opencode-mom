# Service IPC and Publication Contracts

## Overview

Domain-scoped Tauri IPC handlers translate frontend requests into backend operations for agents, app state, autostart, OpenCode, groups, models, power, providers, MCP, skills, and manual updates. Backend modules own the underlying operations; handlers adapt DTOs, validate command inputs, and send window notifications. `app.rs` exposes `get_mom_http_settings` / `set_mom_http_settings` for application-wide `{ port, allowLan }`. MCP settings contain only enabled, timeout, capacity, and revision; proxy state reads network settings from managed `HttpServer` and requires HTTP, enablement, and authentication to report running, without restarting the shared listener.

Paths in this table are relative to `src-tauri/src/commands/`.

## Key Files

| Responsibility | File | Description |
| :--- | :--- | :--- |
| OpenCode bridge | `opencode.rs` | Delegates catalog, binary, installation, explicit update/upgrade, reload, active-session, and token-usage operations to `opencode`; existing `opencode_*` IPC names stay unchanged. |
| Mini Island | `island.rs` | Cached state, enable/disable, explicit compact/peek/expanded view, refresh, messages and guarded session actions through `IslandService`. |
| MCP Proxy boundary | `mcp_proxy.rs` | Built-in MCP HTTP proxy state, settings, key rotation, server CRUD, import, and unit teardown IPC handlers. |

## Conventions

- **Island boundary**: Never load the aggregate catalog; cache-only state/preferences update before notifications. Window setup failures return errors with disabled readback; explicit enabled Refresh repairs missing/error connections without writes. Messages/send/permission/question/cancel/interrupt require caller `expectedConnectionGeneration`, rejected under the cache lock before selection/API/token cloning, never defaulting; guard through mutation-lock waits/completion. Never retry writes or claim cancellation withdrew accepted POSTs; message SSE targets only the expanded selected session, and session arguments use default Tauri camelCase.
- **Island view**: Preserve async `set_island_view { view: 'compact' | 'peek' | 'expanded' }` enum-to-void IPC; capture enabled generation before `on_main`, return its oneshot receiver, and await outside main/cache/TLS. Resolve only after current final native readback; pending replacement returns safe superseded errors without stale success/focus/rollback. Notifications are content-only; peek never activates, only explicit expansion focuses, and compact/blur/close immediately cache compact, clear selection, and emit empty `island:collapsed` before shrink without actions. Expose cache-only numeric `connectionGeneration` for target-local lifetimes.
- **MCP proxy async ownership**: Import resolution/discovery/conversion uses `spawn_blocking_task`; ProxyService owns save/rotation publication and changed/deleted cleanup, never cancellable post-save IPC teardown. Explicit unit close reports cleanup errors after every branch finishes.
- **Service target IPC**: `opencode_get_service_status` never bootstraps. `opencode_set_service_target(url?, password?)` prepares power/Island guards under transition, persists, then invalidates before release; failures preserve monitors, and outside-lock wake/reconnect precedes probing without turning committed saves into failures. Preserve preferences/authorization/disabled state; bootstrap/writes/initial authorization may return `busy`. Null URL selects Shared/clears password; omitted/null password retains only for unchanged normalized URL, empty clears. Offline saves return connected=false; DTOs always include nullable url/pid/version/error and only hasPassword, never password.
- **OpenCode installation**: `opencode_get_installation` is local-only; explicit `opencode_check_update` checks stable V2. `opencode_upgrade` accepts default-camelCase `expectedBinaryPath`/`expectedVersion`/`targetVersion`, delegating all guards; no service lifecycle actions or automatic checks/upgrades.
- **Manual updates**: `get_app_info` performs no network work; `check_for_updates` has no caller-controlled endpoint or credentials; `open_project_page` accepts only the domain-validated `repository`/`releases` destination names. No automatic checks or installation.
- **MCP proxy**: State exposes actual/boolean desiredState/nullable boundAddr/owner-budget activeUnits/capacity/sanitized summaries, never commands/env. Settings reads/saves never expose tokens/verifiers and saves enforce expectedRevision; rotate publishes before revision bump/session closure. CRUD validates configurations, duplicate/missing IDs, and expectedRevision, closing updated/deleted sessions; import validates expected project/Global source paths before conversion. `close_mcp_proxy_unit` checks expectedGeneration against obsolete runtimes.

## Backend Integration

Paths in this table are relative to `src-tauri/src/`.

### Backend Ownership

| Responsibility | File | Description |
| :--- | :--- | :--- |
| MCP proxy | `mcp_proxy/` | Owns MCP configuration, cancellation, sessions, and process budgets, never a listener or network settings. |

### Publication and Lifecycle Rules

- **MCP proxy lifecycle**: Invalid config yields an in-memory disabled service without overwriting source or failing startup. Save/rotation owns filesystem publication in `spawn_blocking` while retaining runtime/config locks; publish cloned config before memory/session changes and update the live owner budget including outstanding permits on capacity reduction. HTTP admission reads gateway/selected service under config lock, then captures cancellation epoch in config -> token order; rotation/disable cancels it. Session close uses a cached owned coordinator; bulk cleanup owns drained sessions, finishes every SDK/bridge/process branch, and reports failures after cleanup. Log post-commit failures, never return failed saves/lost keys. Wrap serialized/decrypted temporary plaintext in `protected_data::SecretBytes`, without claiming to wipe long-lived strings/serializer allocations. Shutdown cancels admission/SSE, closes sessions alongside listener drain, and waits boundedly for permits; block every ExitRequested until the single 8-second task completes, retaining bounded Exit cleanup.
