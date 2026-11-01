# Configuration Boundary and State

## Overview

Typed adapters connect backend commands to reactive application state and Svelte context. The store owns shared initialization, mutations, preferences, and recovery; resource and service facades stay lazy.

## Directory Structure

```text
src/config/                         # Command translation and shared state
├── adapter.ts                      # CommandAdapter implementations and group-type conversion
├── context.ts                      # Symbol-keyed ConfigStore context access
├── store.svelte.ts                 # Reactive state, refresh sequencing, recovery, and island facade
└── __tests__/                      # Node regression tests with Vite module loading
```

## Key Files

| Responsibility | File | Description |
| --- | --- | --- |
| IPC boundary | `adapter.ts` | Typed commands, wire compatibility, and argument mapping. |
| Shared state | `store.svelte.ts` | Reactive state and operation-specific lifecycle ownership. |
| Context | `context.ts` | Parent-provided access to the config facade. |
| Regression checks | `__tests__/` | Injected-adapter and Vite-backed lifecycle coverage. |

## Conventions

- **Command boundary**: Keep IPC translation and wire compatibility in the adapter; UI consumes the typed
  facade. Preserve [command contracts](docs/command-contracts.md), including exact optional-argument
  semantics.

- **Mutation ownership**: Use tracked mutations only where global state requires them; resource writes retain
  their own refresh behavior. Follow [mutation
  lifecycle](docs/state-lifecycle.md#mutations-and-lossless-source-snapshots) and immutable agent snapshots.

- **Read ownership**: Only current requests may commit state or errors; reloads keep their existing
  serialization and splash behavior. Preserve [request
  ownership](docs/state-lifecycle.md#request-and-reload-ownership) during refactors.

- **Private drafts**: Retain recoverable non-secret drafts without storing sensitive resource content or
  private failures. Keep preference rollback and committed selection consistent with [state
  recovery](docs/state-lifecycle.md#drafts-and-committed-preferences).

- **Explicit scopes**: Pass project scope explicitly rather than inferring shared selection; project reads
  never replace Global agents. Preserve [project
  persistence](docs/state-lifecycle.md#explicit-project-resource-scope) and stale-response protection.

- **Lazy services**: App updates, CLI tools, HTTP, Proxy, and prompt generation stay outside global startup
  and recovery. Preserve their distinct [command payloads](docs/command-contracts.md).

- **Target generations**: Target changes invalidate old service reads; Island mutations carry the displayed
  generation without retries. Follow [target
  readback](docs/command-contracts.md#service-targets-and-monitor-readback) and [Island wire
  contracts](docs/command-contracts.md#island-wire-contracts).

- **Regression harness**: Use injected adapters and the existing Vite-backed harness; always close servers.
  Preserve [lifecycle coverage](docs/state-lifecycle.md#regression-harness) and [prompt isolation
  checks](docs/command-contracts.md#prompt-regression-coverage) when changing command ownership.

## Sync

Keep this guide and linked docs in sync with changes to this directory.
