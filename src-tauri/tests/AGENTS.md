# Backend Integration Tests

## Overview

Public-crate integration tests cover configuration, projection, and resource workflows against real temporary filesystem fixtures without launching the Tauri shell. An optional smoke test reads existing machine configuration but never mutates it.

## Directory Structure

```text
tests/
├── app.rs        # App config, agents and group projection
└── resources.rs  # Resource discovery, mutations and catalog fixtures
```

## Key Files

| Responsibility | File | Description |
| :--- | :--- | :--- |
| App workflows | `app.rs` | Isolated app/group/agent fixtures and snapshot round-trips. |
| Resource workflows | `resources.rs` | Scope, provenance, guarded writes, and loopback catalog fixtures. |
| Real-home exception | `app.rs` | Optional read-only smoke coverage, never a source of mutation fixtures. |

## Conventions

- **API boundary**: Exercise public crate results and persisted files; mock IPC may verify guards without launching native UI, but tests must not replace observable behavior with implementation assertions.
- **Isolation**: Keep temporary-home owners alive through each mutation and use explicit ConfigPaths inputs; never change process environment or borrow real-home paths for writable fixtures.
- **Fixture reuse**: Extend existing app, resource, and loopback helpers; seed through current document writers, join HTTP workers, and assert requested paths rather than introducing parallel harnesses.
- **Mutation checks**: Reread successful writes, compare original bytes or directory snapshots on rejection, and verify provenance, preserved JSONC, stale-source rejection, and absence of unintended files.
- **Agent fidelity**: Echo backend source/content snapshots unchanged and use real Node JSON round-trips for numeric cases; retain static-symlink support, alias-retarget rejection, and recovery coverage.
- **Project scope**: Cover exact-root isolation, read-only project writes, missing-root errors, and Global independence; forgetting metadata must not delete files, and chooser or power acceptance stays manual.
- **Host boundaries**: Gate Unix-only behavior, keep all writable fixtures temporary, and restrict optional real-config smoke coverage to reads that safely skip when configuration is absent.

## Details

| Topic | File |
| --- | --- |
| Harnesses, snapshots, scopes and HTTP fixtures | [Fixture contracts](docs/fixture-contracts.md) |

## Sync

Keep this guide and linked docs in sync with changes to this directory.
