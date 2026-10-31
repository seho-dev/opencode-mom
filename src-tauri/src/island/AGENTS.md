# Mini Island Native Service

## Overview

macOS-only capsule window with cached IPC state and a cancellable OpenCode SSE monitor. It is opt-in; disabling destroys the window and clears private local state without interrupting sessions or rejecting backend requests.

## Directory Structure

```text
island/
├── mod.rs         # Service, snapshots and request lifetimes
├── window.rs      # Native window and animation ownership
├── transition.rs  # Transition identity and readback guards
├── hover.rs       # Non-key pointer tracking
├── geometry.rs    # Bounded logical frames
└── tests.rs       # Lifecycle and cancellation regressions
```

## Key Files

| Responsibility | File | Description |
| :--- | :--- | :--- |
| Service lifetime | `mod.rs` | Own cache generations, monitor replacement, and cancellable actions. |
| Native transitions | `window.rs`, `transition.rs` | Serialize view changes and verify current native completion. |
| Placement and hover | `geometry.rs`, `hover.rs` | Bound real window frames and manage non-key tracking ownership. |
| Regression seams | `tests.rs` | Exercise stale responses, cancellation, private-cache clearing, and wire contracts. |

## Conventions

- **Opt-in state**: Persist enablement through the app-config lock; setup failure returns safe disabled readback, while disable clears private state and cancels local work without session writes.
- **Target changes**: Reuse the selected transport and guarded refresh transaction; invalidate old consumers only after commit, preserve preferences and view, and restart outside the target transition lock.
- **Lock boundaries**: Never hold cache locks or native-state borrows across awaits, AppKit calls, callbacks, or event emission; tasks retain weak notification ownership rather than service cycles.
- **Action safety**: Require originating connection generations and cancellation-owned API contexts before reads or writes; serialize mutations, reject stale identities, never retry, and never promise withdrawal of accepted POSTs.
- **Snapshot flow**: Subscribe before snapshots, coalesce only SSE invalidations, and limit message notifications to the expanded selected session; reconnect and collapse must clear the appropriate local lifetimes.
- **Native authority**: Change views only explicitly, focus only current explicit expansion, and commit geometry after verified native readback; keep AppKit on its main thread without global monitors or timers.
- **Acceptance**: Keep diagnostics free of session data and credentials; use synthetic lifecycle tests, then rebuild and restart for manual native hover, notch, focus, and display acceptance.

## Details

| Topic | File |
| --- | --- |
| Cache, targets and action ownership | [Service lifecycle](docs/service-lifecycle.md) |
| Views, animation, placement and hover | [Native window](docs/native-window.md) |

## Sync

Keep this guide and linked docs in sync with changes to this directory.
