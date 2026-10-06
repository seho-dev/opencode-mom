# macOS Lid Helper

## Overview

Standalone macOS-only helper for journaled changes to the global `SleepDisabled` policy. It authenticates the authorized app over a Unix socket, verifies policy readback, and restores owned changes on lease expiry, disconnect, or parent exit. The parent app builds and embeds the executable; this Cargo workspace has no Tauri dependency.

## Directory Structure

```text
macos-helper/       # Standalone binary workspace
├── AGENTS.md       # Directory-specific guidance
├── Cargo.toml      # Isolated workspace, pinned libc, release profile
├── Cargo.lock      # Helper's own dependency lock
└── src/            # Runtime modules and inline unit tests
    ├── main.rs     # Module declarations and macOS-only entry point
    ├── protocol.rs # Authentication, request, and acknowledgment frames
    ├── policy.rs   # Backend-driven journal and restoration logic
    ├── watchdog.rs # Lease deadline and lifetime checks
    └── platform.rs # pmset, trusted files, peer checks, and socket server
```

## Key Files

| Responsibility | File | Description |
| --- | --- | --- |
| Workspace | `Cargo.toml`, `Cargo.lock` | Separate lockfile; libc is pinned to `=0.2.186`. |
| Entry point | `src/main.rs` | Calls `platform::run`; compilation rejects non-macOS targets. |
| Shared wire format | `src/protocol.rs` | Fixed-size frames, token validation, sequence checks, and bounded TTLs. |
| Policy ownership | `src/policy.rs` | `Backend` and `Policy` separate system access from recovery logic; tests use `Mock`. |
| Lease safety | `src/watchdog.rs` | `must_restore` combines expiry, connection, and parent lifetime. |
| macOS runtime | `src/platform.rs` | Implements the system backend and `serve`; socket tests use a mock-policy `Harness`. |

## Conventions

- **Build boundary**: `src-tauri/build.rs` builds this manifest with `--release --offline --locked`, ad-hoc signs and verifies the artifact, then hashes the signed bytes before embedding in `src-tauri/src/power/macos.rs`.
- **Validation scope**: Run helper checks and tests against this manifest separately; its policy, watchdog, and platform test modules belong to this binary, not the parent app's test targets.
- **Shared protocol**: The parent macOS controller imports `src/protocol.rs` directly; keep encoders, parsers, and frame constants compatible on both sides.
- **Framing**: Preserve the 40-byte hello, 24-byte request, and 16-byte acknowledgment. Busy leases are 1–10,000 ms; requests start at sequence 1, advance consecutively, and reject zero or `u64::MAX`.
- **Policy ownership**: Save the durable journal before applying changes; verify application and restoration by readback. Preserve external changes and retain the journal when restoration fails.
- **Authorization**: Verify peer UID, PID, and token before serving operations; retain root-file ownership, mode, link-count, and no-follow checks around recovery state.
- **Lease timing**: Start the watchdog deadline before slow policy work. Expiry restores policy but leaves the authenticated session available; disconnect and parent exit restore before ending the session.
- **Test isolation**: Extend inline `#[cfg(test)]` tests using `Backend` mocks and temporary Unix sockets. Never invoke `System`, `run`, `run_inner`, `pmset`, or `sudo` in automated tests.

## Sync

This file is maintained as the directory scope changes. When files in this directory are added, modified, or deleted (components, logic units, utilities, constants, etc.), update this file's directory structure, key files, and conventions to keep the documentation in sync with the code.
