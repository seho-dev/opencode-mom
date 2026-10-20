# Power

## Overview

Owns opt-in lid protection through `PowerService`, a serialized controller state machine, and platform backends. It monitors running sessions in the CLI-connected OpenCode service, applies bounded protection, and restores owned policy changes. macOS delegates privileged policy and recovery to the embedded helper; Windows manages journaled AC/DC lid actions directly.

## Directory Structure

```text
power/                 # Lid-protection service and platform controllers
├── AGENTS.md           # Directory-specific guidance
├── mod.rs              # PowerService, Engine, DTOs, workers, platform selection
├── macos.rs            # Helper authorization, authenticated IPC, inline tests
├── windows.rs          # Backend-driven policy, recovery journal, Windows APIs
├── settings.rs         # Strict power preference and durable writes
├── tests.rs            # Mock-controller state-machine tests
├── settings/           # Preference test submodule
│   └── tests.rs        # Defaults, persistence, malformed preferences
└── windows/            # Policy test submodule
    └── tests.rs        # Mock-backend rollback, recovery, external changes
```

## Key Files

| Responsibility | File | Description |
| --- | --- | --- |
| Service lifecycle | `mod.rs` | `PowerService::new/get/set/shutdown`, guarded target refresh, and generation-scoped `Engine<C: PowerController>`. |
| Session monitoring | `mod.rs` | Separate query worker calls `active_sessions_via_cli`; controller worker owns timers and mutations. |
| macOS controller | `macos.rs` | Stages embedded bytes, requests authorization, verifies socket/peer, and checks sequenced acknowledgments. |
| Windows policy | `windows.rs` | `Policy<B: Backend>` journals originals, applies zero AC/DC lid actions, and conditionally restores. |
| Persistence | `settings.rs` | `power.json` preference; `durable_write` also persists the Windows journal. |
| State-machine regression | `tests.rs` | Leases, stale results, cleanup failures, shutdown, serialization, and failed preference writes. |
| Policy regression | `windows/tests.rs`, `settings/tests.rs` | Mock policy ownership/recovery and strict preference parsing. |

## Conventions

- **Controller boundary**: Keep native effects behind `PowerController::prepare/busy/idle/stop`. A shared effects mutex serializes busy/renew/idle/stop and their generation checks/publication against target commits; never hold the shared-state mutex during controller calls. Interactive prepare holds no transition/effects/shared lock; mark `Shared.preparing` until its result and any late stop are handled. Session queries hold no mutex during HTTP and have at most one actual request in flight.
- **Generations**: Setting changes and shutdown increment both observation and control generations. Target refresh increments only observation generation, clears counts/grace, restores idle, and preserves enabled/authorization/preparation outcome. Never stop or reauthorize solely for a target change; failed/missing authorization remains an Error until explicit enablement. Publish only for the current generation and stop late control-change effects before publication.
- **Target commit**: Under the target transition mutex, call `prepare_target_refresh` to hold effects then shared locks; active authorization returns `busy` without waiting. Dropping the guard leaves state unchanged. Only after persistence and target revision publication call its infallible `invalidate`; call `finish_target_refresh` outside the transition to queue a wake. A disconnected wake channel logs a fixed safe warning and never fails the committed save. It never writes preferences. Disabled monitors remain disabled without prepare/query.
- **Monitoring scope**: Query every 2 seconds through the selected local service's bounded HTTP active endpoint, including running child sessions; this is not a machine-wide process inventory. Errors publish Unknown, never zero/idle. `PowerService::set` shares the target transition mutex. Refresh retains the old query's slot until its result arrives and rejects that result before any native effect.
- **Lease timing**: Renew leases of at most 10 seconds within the 30-second grace since the last positive observation. Query failures publish `Unknown` without extending grace; a confirmed zero count restores immediately.
- **Acknowledged state**: Publish `Protected` only after `busy` succeeds and `Idle` only after restoration succeeds. Failed restoration blocks preparation/queries/busy and retries cleanup every 2 seconds. Target-only idle failures retain prior preparation, resuming queries after recovery; prior preparation failures retain their error without automatic authorization retries.
- **Preference semantics**: Missing `power.json` defaults to disabled; malformed or unknown fields fail. Enabling requires a successful save, but failed preference writes must not block an in-memory opt-out and restoration.
- **Durability**: Use `settings::durable_write` for app-owned power data: sync the pending file, replace the destination, and use the platform-specific durable replacement path.
- **macOS authorization**: Persisted enablement does not authorize a new app run. Explicit enablement permits one authorization attempt per controller; ordinary opt-out sends `Idle` and preserves the authorized connection, whereas `Stop` ends it.
- **macOS protocol**: Import `../../native/macos-helper/src/protocol.rs` rather than copying frames. Preserve token authentication, socket ownership/mode and root-peer checks, bounded IPC, and verified acknowledgments; keep restoration-unconfirmed errors visible.
- **Journaled recovery**: Windows saves `windows-lid-journal.json` before policy writes; macOS recovery belongs to the helper's root journal. Verify apply/restore readback, preserve external changes, and retain unresolved recovery state.
- **Windows ownership**: Record both AC/DC originals for the active scheme; restore only values still equal to the applied zero. Never activate an old scheme during recovery. Windows has no native policy TTL, so restoration depends on the serialized app timer/exit or later journal recovery.
- **Test seams**: Extend `Engine<Mock>` in `tests.rs` and `Policy<Mock>` in `windows/tests.rs`; service fixtures share mock effects/state and channels, never start `PowerService::new`. Channel barriers cover both native busy/renew versus commit orders and unlocked authorization/late-stop cleanup. Windows policy tests compile under `cfg(test)` on other hosts. macOS inline tests use local socket pairs; helper policy/watchdog tests live in the separate helper workspace.
- **Hardware acceptance**: For native policy changes, follow `README.md` → “Manual lid protection acceptance (macOS / Windows)”; keep the trial procedure there.

## Sync

This file is maintained as the directory scope changes. When files in this directory are added, modified, or deleted (components, logic units, utilities, constants, etc.), update this file's directory structure, key files, and conventions to keep the documentation in sync with the code.
