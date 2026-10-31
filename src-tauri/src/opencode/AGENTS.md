# OpenCode Integration

## Overview

Integrates CLI discovery and guarded upgrades with one selected local OpenCode service. Shared transport supplies catalogs, reload, active work, usage, prompt assistance, and Mini Island without exposing credentials to the frontend.

## Directory Structure

```text
opencode/
├── mod.rs             # Public boundary and DTO exports
├── binary.rs          # Ordered CLI discovery
├── command.rs         # Bounded child-output collection
├── installation.rs    # Ownership inspection and upgrades
├── target.rs          # Private target persistence and revisions
├── transport.rs       # Pinned authenticated local HTTP
├── models.rs          # Service catalog and configured metadata
├── reload.rs          # Non-retried reload
├── active.rs          # Validated running-session map
├── generate.rs        # One-shot prompt assistance
├── island.rs          # Async Island API boundary
├── island/            # Projection, SSE and synthetic tests
├── token_usage.rs     # Bounded paging and fork attribution
└── token_usage/       # Usage fixtures and regression tests
```

## Key Files

| Responsibility | File | Description |
| :--- | :--- | :--- |
| Target authority | `target.rs`, `transport.rs` | Persist private selections and pin validated identity, credentials, and revision. |
| CLI ownership | `binary.rs`, `installation.rs`, `command.rs` | Resolve binaries and bound inspection, collection, and explicit upgrades. |
| Service reads | `models.rs`, `active.rs`, `token_usage.rs` | Validate catalog, active-work, and usage responses. |
| Mutating requests | `reload.rs`, `generate.rs` | Perform explicit bounded operations without automatic write retries. |
| Island lane | `island.rs`, `island/` | Project public data and preserve cancellation through asynchronous actions and SSE. |

## Conventions

- **Target authority**: Use captured local selections and shared transport; fixed endpoints never fall back or bootstrap, and malformed private configuration fails closed without disabling management.
- **Credentials**: Keep passwords backend-only, use validated private-file or platform protection, and wrap temporary secret bytes immediately; never include secrets or raw response bodies in public failures.
- **Publication**: Prepare monitor guards before durable target publication, invalidate only after commit, and release transition locks before network or native work; active reservations must block conflicting target changes.
- **Identity**: Pin endpoint, authentication, backend identity, and revision across operations; validate registration and health without redirects, proxies, implicit restarts, or credential fallback.
- **Write safety**: Guard fresh identities and cancellation through dispatch, never retry possibly accepted writes or billable generation, and require explicit consent plus fresh server state for privileged answers.
- **Bounded data**: Apply shared deadlines, response and concurrency limits, validated identifiers, and fork-aware attribution; fail rather than publish partial usage, malformed state, or private provider/tool data.
- **Installation**: Upgrade only explicitly requested, proven supported installations with expected path/version guards; never use sudo, upgrade bundled binaries, alter service lifecycle, or run real upgrades in tests.

## Details

| Topic | File |
| --- | --- |
| Selection, credentials and transport ownership | [Target lifecycle](docs/target-lifecycle.md) |
| CLI, HTTP, Island and usage contracts | [Service operations](docs/service-operations.md) |
| Unsafe shortcuts | [Anti-patterns](docs/anti-patterns.md) |

## Sync

Keep this guide and linked docs in sync with changes to this directory.
