# Local Logging

## Overview

Opt-in, local-only JSONL diagnostics. Logging defaults off; the persisted switch is `state.loggingEnabled`. Instrumentation never changes business results and accepts only static event identifiers and typed, safe context.

## Directory Structure

```text
logging/
├── mod.rs       # Nonblocking instrumentation, writer queue and acknowledged controls
├── types.rs     # Wire records, safe fields and validated query filters
├── storage.rs   # Private bounded files, rotation and snapshot cursors
└── tests.rs     # Persistence, lifecycle, privacy and filesystem regressions
```

## Conventions

- Use `operation(kind, event)`, `checkpoint(event)` and `finish(&result)`; use `record(kind, status, event)` for standalone lifecycle events. Event names must match the kind-specific catalogue in `types.rs`; add new fixed identifiers there with instrumentation. Only declared group-switch checkpoints accept context, and their step must equal the event. Messages are fixed internally; only `ErrorCode` crosses the error boundary.
- Tests can create a `LoggingService` with temporary `ConfigPaths` and use its instance `operation`/`record` methods, `set_enabled`, and `query` to inspect records without installing or replacing the global logger.
- Persisted `enabled` is user intent, not recording success. After a committed setting change, writer failures return safe state (`enabled: true`, `recording: false`, fixed `error`); config save failures return `AppError` without changing recording. Re-enabling retries a failed writer even if the preference was already true.
- Enable persists configuration before starting or rotating files; a failed config save must never evict retained logs. Logging lifecycle identifiers are allowed, but setting/clear failures are exposed through safe state rather than recursively recorded.
- `app.started` marks completion of synchronous desktop setup, after target, proxy, power, Island and tray initialization. It does not claim that asynchronously started services are ready.
- Power phase and Island connection/window lifecycle events describe state changes at their existing publication boundaries only; never record polling iterations or attach raw state payloads. Deduplication belongs to those existing state-change seams, not the logger.
- Never capture console output, panic payloads, third-party logs, raw errors, credentials, paths, prompts, configuration contents or messages. No runtime logger arguments or arbitrary JSON context.
- One bounded queue owns writes and serialized acknowledged controls. A successful disable barrier drains previously accepted records before returning; a failed drain returns non-recording state with a safe error. Operation generations prevent pre-enable and previous-enable work from leaking into a new recording period. Shutdown has a deadline and never joins an unbounded I/O thread.
- Open the directory from a blocking worker and reap the opener with `status()`. Release the controls lock before waiting so a slow desktop opener cannot block logging settings, queries or shutdown.
- Keep at most five managed files of at most 5 MiB, bounded lines and pages of 1–200 records (default 100). Queries walk newest accepted records first and use bounded in-memory snapshot cursors; rotation, clear and file replacement invalidate cursors.
- Reject unsafe managed entries, symlinks and Windows reparse points. Unix directories/files require owner-only `0700`/`0600`; Windows uses inherited permissions, without claiming private ACL enforcement. This is not protection against a hostile same-account filesystem race.

## Sync

Keep this guide and linked docs in sync with changes to this directory.
