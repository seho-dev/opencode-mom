# Power Controller Lifecycle

## Overview

Owns opt-in lid protection through `PowerService`, a serialized controller state machine, and platform backends. It monitors running sessions in the selected local OpenCode service, applies bounded protection, and restores owned policy changes. macOS delegates privileged policy and recovery to the embedded helper; Windows manages journaled AC/DC lid actions directly.

Paths in this table are relative to `src-tauri/src/power/`.

## Key Files

| Responsibility | File | Description |
| :--- | :--- | :--- |
| Service lifecycle | `mod.rs` | `PowerService::new/get/set/shutdown`, guarded target refresh, and generation-scoped `Engine<C: PowerController>`. |
| Session monitoring | `mod.rs` | Separate query worker calls `active_sessions_via_service`; controller worker owns timers and mutations. |
| Persistence | `settings.rs` | `power.json` preference; `durable_write` also persists the Windows journal. |
| State-machine regression | `tests.rs` | Leases, stale results, cleanup failures, shutdown, serialization, and failed preference writes. |

## Conventions

- **Controller boundary**: Keep native effects behind PowerController::prepare/busy/idle/stop; effects mutex serializes effects/generation checks/publication against target commits, never holding shared-state during calls. Interactive prepare holds no transition/effects/shared lock; mark Shared.preparing through result/late stop. HTTP queries hold no mutex and allow only one actual in-flight request.
- **Controller cleanup**: Engine owns teardown; successful stop prevents duplicates until another effect. Log disconnection stop failures; every normal run_worker return retries under effects lock before completion. Unwind cleanup is best-effort without reacquiring/recovering poisoned locks; contain panics only there, retaining normal Result paths. Test controller/cleanup failures with mocks, never native policy.
- **Generations**: Settings/shutdown increment observation/control generations; target refresh changes observation only, clears counts/grace, restores idle, and preserves enabled/authorization/preparation. Never stop/reauthorize for target changes; failed/missing authorization stays Error until explicit enablement. Publish only current generations and stop late control-change effects first.
- **Target commit**: Under transition, prepare_target_refresh holds effects then shared locks; active authorization returns busy without waiting and dropping preserves state. Invalidate infallibly only after persistence/revision publication; finish_target_refresh wakes outside transition, logs disconnected channels safely without failing committed saves, and never writes preferences. Disabled monitors never prepare/query.
- **Monitoring scope**: Query bounded selected-service active HTTP every 2 seconds including running children, never machine-wide processes; errors mean Unknown, not zero/idle. PowerService::set shares transition; refresh retains old query slot until response and rejects it before native effects.
- **Lease timing**: Renew leases of at most 10 seconds within the 30-second grace since the last positive observation. Query failures publish `Unknown` without extending grace; a confirmed zero count restores immediately.
- **Acknowledged state**: Publish Protected/Idle only after successful busy/restoration; restoration failure blocks preparation/queries/busy and retries cleanup every 2 seconds. Target-only idle failure preserves preparation/resumes queries after recovery; prior preparation failure retains error without automatic authorization retry.
- **Preference semantics**: Missing `power.json` defaults to disabled; malformed or unknown fields fail. Enabling requires a successful save, but failed preference writes must not block an in-memory opt-out and restoration.
- **Durability**: Use `settings::durable_write` for app-owned power data: sync the pending file, replace the destination, and use the platform-specific durable replacement path.
- **Test seams**: Extend Engine<Mock>/Policy<Mock> in tests.rs/windows/tests.rs; share effects/state/channels, never PowerService::new. Barriers cover busy/renew versus commit in both orders and unlocked authorization/late-stop cleanup; compile Windows policy under cfg(test) elsewhere. macOS inline tests use socket pairs; helper policy/watchdog tests stay in their workspace.
- **Hardware acceptance**: For native policy changes, follow `README.md` → “Manual lid protection acceptance (macOS / Windows)”; keep the trial procedure there.
