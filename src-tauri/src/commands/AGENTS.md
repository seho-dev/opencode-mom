# Tauri Commands

## Overview

Domain-scoped Tauri handlers validate requests, adapt wire DTOs, and delegate backend operations. This directory owns IPC exposure and notifications, not persistence, native policy, or OpenCode transport implementations.

## Directory Structure

```text
commands/
├── mod.rs          # Domain exports and tray re-exports
├── agent.rs        # Agent DTOs and mutations
├── app.rs          # App state, projects and HTTP settings
├── app/            # App command regression tests
├── autostart.rs    # Platform autostart
├── group.rs        # Group mutations
├── island.rs       # Mini Island IPC
├── mcp_proxy.rs    # Proxy configuration and runtime IPC
├── mcp_proxy/      # Proxy command tests
├── model.rs        # Model mutations and reference guards
├── opencode.rs     # Local service and CLI operations
├── power.rs        # Managed power boundary
├── provider.rs     # Provider mutations
├── resources.rs    # MCP and skill operations
└── updates.rs      # Manual checks and fixed project links
```

## Key Files

| Responsibility | File | Description |
| :--- | :--- | :--- |
| Command exposure | `mod.rs` | Re-export handlers registered by the crate root. |
| App boundary | `app.rs` | Return app state and manage preferences, projects, and shared HTTP settings. |
| Resource mutations | `agent.rs`, `resources.rs` | Enforce scope and source-snapshot guards before domain calls. |
| Local service | `opencode.rs`, `island.rs` | Adapt service-target and generation-bound action requests. |
| Proxy lifecycle | `mcp_proxy.rs` | Adapt revision-guarded saves, imports, key rotation, and unit teardown. |

## Conventions

- **Exposure**: Keep handlers in their domain module, re-export them through mod.rs, and register snake_case commands in the crate root without moving persistence into IPC code.
- **Results**: Return the shared AppError contract and stable wire DTOs; validate inputs and referenced resources before invoking mutations, preserving missing, stale, and busy distinctions.
- **Resource safety**: Reject project-scoped writes before filesystem access; existing agent updates must echo backend source/content snapshots unchanged rather than regenerating JSON through JavaScript.
- **Blocking work**: Offload blocking filesystem, HTTP, chooser, and opener work through existing task helpers; retain owned publication and teardown inside the service that commits the mutation.
- **Island actions**: Require the caller's connection generation, preserve cancellation through action completion, and never retry writes or claim that cancellation withdrew an already-accepted POST.
- **Service settings**: Keep backend credentials private, preserve committed target saves despite refresh failures, and separate application-wide HTTP settings from revision-guarded proxy configuration and runtime readback.
- **Notifications**: Notify only after successful persistence or authoritative native readback; log notification failures without failing committed mutations, and test boundaries without launching native UI or real upgrades.

## Details

| Topic | File |
| --- | --- |
| Commands, DTOs and resource guards | [Command contracts](docs/command-contracts.md) |
| Service targets, Island and proxy lifecycle | [Service IPC](docs/service-ipc.md) |

## Sync

Keep this guide and linked docs in sync with changes to this directory.
