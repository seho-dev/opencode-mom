# Rust Backend Modules

## Overview

Rust crate root for configuration persistence, resource management, group projection, and desktop lifecycle. Domain modules own operations, `commands/` exposes IPC, and `lib.rs` initializes shared services and coordinates shutdown.

## Directory Structure

```text
src/
├── lib.rs               # Tauri initialization and shutdown
├── commands/            # Domain IPC boundary
├── models.rs            # App config types and persistence
├── paths.rs             # Captured paths and resource scopes
├── agents.rs            # Agent discovery and mutations
├── agents_md.rs         # Preserving Markdown edits
├── document.rs          # Document and plain file I/O
├── jsonc.rs             # Comment-preserving path patches
├── groups.rs            # Group lifecycle
├── projection.rs        # Target document transformations
├── refs.rs              # Deletion protection
├── resource_file.rs     # Checked publication and recovery
├── http_server.rs       # Application-wide HTTP listener
├── mcp_proxy/           # MCP gateway service
├── opencode/            # Local OpenCode integration
├── island/              # Native Mini Island
└── power/               # Lid protection
```

## Key Files

| Responsibility | File | Description |
| :--- | :--- | :--- |
| Desktop ownership | `lib.rs` | Initialize captured state and coordinate service shutdown. |
| Persistence | `models.rs` | Own app config types and serialized writes. |
| Scope authority | `paths.rs` | Resolve Global sources and isolated project roots. |
| Preserving edits | `document.rs`, `jsonc.rs` | Choose plain writes or JSONC path patches. |
| Resource publication | `resource_file.rs` | Validate source snapshots and retain recovery backups on failure. |
| Group application | `groups.rs`, `projection.rs` | Persist selections and project group-owned mappings. |

## Conventions

- **Ownership**: Keep domain work outside IPC handlers and initialize captured paths once; use the [command](commands/AGENTS.md), [OpenCode](opencode/AGENTS.md), [Island](island/AGENTS.md), and [power](power/AGENTS.md) guides for their contracts.
- **Persistence**: Share the existing app-config write lock and atomic save path; preserve unrelated selection fields, symlink targets, and modes rather than replacing document ownership rules.
- **Resource scope**: Keep project resources exact-root and read-only; omitted scope remains Global, and forgetting a project must never delete filesystem content or change Global paths.
- **Mutation safety**: Echo backend source/content snapshots unchanged, reject stale sources and protected references, and use checked resource publication with recovery backups for existing-file edits.
- **Document edits**: Patch only owned JSONC paths, preserving unrelated comments and residual mappings; distinguish plain document writes from guarded resource replacement before choosing a write API.
- **Service boundaries**: Reuse the selected local transport and backend-only credential handling; never rediscover service targets or expose secrets through frontend state, errors, logs, or events.
- **Completion semantics**: Publish runtime changes only after persistence; notification or post-commit cleanup errors must not misrepresent a successful save as rollback or discard a rotated key.
- **Validation**: Follow the documented main/helper toolchain boundaries and narrow test seams; use temporary paths and mocks, never native policy or real service mutations in automated coverage.

## Details

| Topic | File |
| --- | --- |
| Configuration and resource safety | [Configuration contracts](docs/config-contracts.md) |
| Desktop ownership and validation | [Runtime lifecycle](docs/runtime-lifecycle.md) |

## Sync

Keep this guide and linked docs in sync with changes to this directory.
