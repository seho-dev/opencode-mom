# macOS Lid Helper

## Overview

Standalone macOS helper for authenticated, journaled SleepDisabled policy changes and owned restoration on lease expiry, disconnect, or parent exit. The parent embeds its executable; this isolated Cargo workspace has no Tauri dependency.

## Directory Structure

```text
macos-helper/
├── Cargo.toml      # Isolated workspace and pinned dependency
├── Cargo.lock      # Independent lockfile
└── src/
    ├── main.rs     # macOS-only entry point
    ├── protocol.rs # Shared authentication and message frames
    ├── policy.rs   # Journal and restoration ownership
    ├── watchdog.rs # Lease and process lifetime checks
    └── platform.rs # Trusted files, IPC and system backend
```

## Key Files

| Responsibility | File | Description |
| :--- | :--- | :--- |
| Workspace boundary | `Cargo.toml`, `Cargo.lock` | Keep toolchain and dependency validation independent of the parent crate. |
| Shared wire protocol | `src/protocol.rs` | Maintain compatible client/server framing and authentication. |
| Restoration safety | `src/policy.rs`, `src/watchdog.rs` | Combine journaled ownership with bounded lease and process lifetimes. |
| Privileged runtime | `src/platform.rs` | Enforce trusted files, peers, descriptors, and socket handling. |

## Conventions

- **Build boundary**: Preserve the separate workspace, locked offline embedding path, signing verification, and helper-specific toolchain gates; parent-crate tests do not replace validation of this binary.
- **Unsafe ownership**: Validate inherited descriptors before constructing sole owners, transfer native descriptors only once, and keep local SAFETY proofs at the FFI calls they justify.
- **Shared protocol**: Modify frames compatibly for the imported parent client and helper server; preserve token authentication, consecutive sequence validation, bounded leases, and verified acknowledgments.
- **Authorization**: Verify peer identity and token before serving operations; retain root-file ownership, permissions, link-count, and no-follow checks around trusted socket and recovery state.
- **Policy ownership**: Durably journal before changing policy, verify application and restoration by readback, preserve external changes, and retain the journal whenever recovery remains unresolved.
- **Lifetime safety**: Start watchdog deadlines before slow policy work; expired leases restore policy, while disconnect and parent exit restore before ending the authenticated session.
- **Test isolation**: Extend mock backends and temporary-socket harnesses; never invoke live system policy, privileged entry points, pmset, or sudo from automated tests.

## Details

| Topic | File |
| --- | --- |
| Framing, authorization, journal and watchdog | [Policy and protocol](docs/policy-protocol.md) |

## Sync

Keep this guide and linked docs in sync with changes to this directory.
