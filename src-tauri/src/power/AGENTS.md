# Power

## Overview

Owns opt-in lid protection through a serialized PowerService controller and selected-service activity monitoring. macOS delegates privileged policy to the embedded helper; Windows journals owned lid-action changes and restores them conditionally.

## Directory Structure

```text
power/
├── mod.rs       # Service, controller engine and workers
├── macos.rs     # Helper authorization and authenticated IPC
├── windows.rs   # Journaled policy and platform backend
├── settings.rs  # Strict preferences and durable writes
├── tests.rs     # Mock controller lifecycle
├── settings/    # Preference regression tests
└── windows/     # Mock policy and recovery tests
```

## Key Files

| Responsibility | File | Description |
| :--- | :--- | :--- |
| Serialized lifecycle | `mod.rs` | Coordinate generations, observations, native effects, and cleanup. |
| macOS boundary | `macos.rs` | Authorize the helper and verify authenticated acknowledgments. |
| Windows ownership | `windows.rs` | Journal originals and restore only still-owned policy values. |
| Durable state | `settings.rs` | Persist strict preferences and recovery data. |

## Conventions

- **Controller boundary**: Keep native effects behind PowerController and serialize effects against target commits; never hold shared state during controller calls or locks during interactive authorization and HTTP queries.
- **Generations**: Reject stale observations before native effects; target changes preserve enablement and authorization, while explicit control changes invalidate prior work and stop late effects before publication.
- **Observed state**: Publish protection or idle only after acknowledged effects; query failures mean Unknown, never idle, and failed restoration blocks new protection until cleanup succeeds.
- **Opt-out**: Enabling requires persisted preference success, but failed writes must never prevent in-memory disablement and restoration; retain the existing strict parser and durable publication path.
- **Policy ownership**: Journal before mutation, verify apply and restore readback, preserve external policy changes, and retain unresolved recovery state rather than claiming cleanup succeeded.
- **Helper security**: Reuse the shared protocol and preserve socket, peer, token, sequence, and acknowledgment checks; authorization is explicit and restoration failures must remain visible to callers.
- **Verification**: Exercise controller, timing, recovery, and cleanup through mocks and channel barriers; native lid-close acceptance remains manual in README and must never run through automated policy tests.

## Details

| Topic | File |
| --- | --- |
| Effects, generations, leases and cleanup | [Controller lifecycle](docs/controller-lifecycle.md) |
| Authorization, protocol and recovery | [Platform policy](docs/platform-policy.md) |

## Sync

Keep this guide and linked docs in sync with changes to this directory.
