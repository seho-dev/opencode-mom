# Platform Policy Ownership and Recovery

Paths in this table are relative to `src-tauri/src/power/`.

## Key Files

| Responsibility | File | Description |
| :--- | :--- | :--- |
| macOS controller | `macos.rs` | Stages embedded bytes, requests authorization, verifies socket/peer, and checks sequenced acknowledgments. |
| Windows policy | `windows.rs` | `Policy<B: Backend>` journals originals, applies zero AC/DC lid actions, and conditionally restores. |
| Policy regression | `windows/tests.rs`, `settings/tests.rs` | Mock policy ownership/recovery and strict preference parsing. |

## Conventions

- **macOS authorization**: Persisted enablement does not authorize a new app run. Explicit enablement permits one authorization attempt per controller; ordinary opt-out sends `Idle` and preserves the authorized connection, whereas `Stop` ends it.
- **macOS protocol**: Import `../../native/macos-helper/src/protocol.rs` rather than copying frames. Preserve token authentication, socket ownership/mode and root-peer checks, bounded IPC, and verified acknowledgments; keep restoration-unconfirmed errors visible.
- **Journaled recovery**: Windows saves `windows-lid-journal.json` before policy writes; macOS recovery belongs to the helper's root journal. Verify apply/restore readback, preserve external changes, and retain unresolved recovery state.
- **Windows ownership**: Record both AC/DC originals for the active scheme; restore only values still equal to the applied zero, never activating old schemes during recovery. Without native policy TTL, rely on serialized app timer/exit or later journal recovery.
