# Island Service and Action Lifetimes

## Overview

macOS-only capsule window, cached IPC state, and one cancellable OpenCode SSE task. Disabled by default; disabling destroys the window and clears local private data, never interrupts sessions or rejects requests.

Paths in this table are relative to `src-tauri/src/island/`.

## Key Files

| Responsibility | File | Description |
| :--- | :--- | :--- |
| Service lifecycle | `mod.rs` | Managed `IslandService`, immutable `Arc` snapshots copied outside the cache lock, shared explicit `IslandView`, generation-guarded requests, event coalescing, reconnect and shutdown. |
| Regression tests | `tests.rs` | Explicit view wire contract, native transition lifecycle/readback guards, stale-response exclusion, private-cache clearing, task abortion, async action-lifetime cancellation, dual synthetic-backend IPC epoch rejection and actual queued-request cancellation. |

## Conventions

- **Persistence**: Persist `state.islandEnabled` outside `AppPreferences`; acquire `models::config_write_lock` around config read/modify/write.
- **Target refresh**: Use shared local transport (Shared or fixed loopback V2, never explicit fallback/bootstrap); `set_enabled` takes transition only inside native dispatch. Prepare `TargetRefresh` under transition before persistence; drop leaves state unchanged, post-commit invalidate clears private snapshots/API/selection and cancels lifetimes but preserves enabled/preferences/view/notch, and disabled refresh is a no-op. Release transition before `finish_target_refresh(app, generation)`; native dispatch reacquires/rechecks generation before restart. Missing windows return safe Refresh-repair errors without opening/disabling/persisting; finalization cannot fail committed saves, and reads/pre-write dispatch check captured target revision.
- **Window setup**: Fail setup with an error, cleared runtime enablement, and persisted disabled default. Reject any registered `island` label before native setup; queued destruction never permits reuse/tracking reinstallation, and remain disabled until explicit retry after label removal, without auto-retry/waiting. Active enable is a no-op; readback never invents a window, and enabled Refresh without API immediately replaces the monitor, including initial/error states, without session writes.
- **Cache locks**: Never hold cache locks across awaits, AppKit/window calls, or event emission. The task captures cache data and a weak notification slot owned by the run loop, never its own service or a permanent `AppHandle`.
- **Monitor ownership**: Serialize replacement/shutdown task ownership before cache-generation checks/updates; abort/spawn outside cache lock and forbid stale finalizers from replacing/aborting current monitors. Expose required camelCase `connectionGeneration` directly from cache generation, resetting frontend lifetimes even for equal session IDs.
- **Snapshot scheduling**: Subscribe before snapshot. Only SSE invalidations trigger coalesced snapshots; reconnect delays are 1/2/5/10 seconds. No full snapshot polling or disabled watchers.
- **Message invalidation**: `island_messages` selects only while expanded; ignore other-session/collapsed SSE and clear selection on collapse/disable. First eligible invalidation fixes the 250 ms deadline, never extended by tokens; Snapshot flushes publish before notifying.
- **SSE lifetime**: SSE is live-only with no replay; every reconnect subscribes anew and rebuilds the snapshot. Publish snapshots and connection errors only while the generation matches under the cache lock.
- **Action lifetime**: Require current enabled API and separate per-API action watch token; fail/replacement/stop/reconnect invalidate API/cancel token under cache lock, but connection failure preserves monitor SSE retries. Race mutation-lock waits and entire action including fresh GET/POST against cancellation with pre/post generation guards; serialize mutations, never retry, and never claim cancellation withdrew issued POSTs.
- **Expected generation**: Messages/all five action IPCs require originating unsigned `expectedConnectionGeneration`, never omitted/current fallback. `request(expected, action, mutation)`/`messages(session_id, expected)` reject stale generations under cache lock before API/token cloning, reads/actions/selection, even for reused backend IDs. Only internal explicit Refresh may capture current context; views stay unchanged.
- **Events and preferences**: Emit only empty `island:changed`/`island:collapsed`/`island:pointer-enter`/`island:pointer-leave` to `island`; reads are cache-only. Initialize preferences once from app config and update cache inside successful persistence before notifying.
- **Test isolation**: Tests never launch the app, power helper, or live OpenCode API.

## Backend Integration

Paths in this table are relative to `src-tauri/src/`.

### Backend Lifecycle Boundary

| Responsibility | File | Description |
| :--- | :--- | :--- |
| Mini Island | `island/` | macOS-only bounded transparent window, opt-in persisted flag, cancellable validated OpenCode SSE cache, and generation-guarded actions. |

### Publication and Lifecycle Rules

- **Island lifecycle**: Default `state.islandEnabled` false outside preferences; setup errors return safe disabled readback without failing startup, and explicit enabled Refresh rebuilds unavailable connections. Disable/exit aborts tasks and clears private state, never sessions; state/preferences are cache-only and message invalidations target only the expanded selected session. Other platforms report unsupported reads and reject enabling.
