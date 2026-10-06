# OpenCode CLI

## Overview

Integrates with the local OpenCode CLI for binary discovery, model catalogs, reload, and running-session counts. Token usage bootstraps the managed V2 service when needed, then scans authenticated local HTTP pages with shared deadlines, bounded metadata, and fork-aware usage attribution. `mod.rs` exposes the functions and DTOs consumed by backend commands and the power monitor.

## Directory Structure

```text
opencode_cli/                 # Local CLI and managed-service integration
├── AGENTS.md                 # Directory-specific guidance
├── mod.rs                    # Public function and DTO re-exports
├── binary.rs                 # Ordered binary candidates and resolver
├── command.rs                # Deadline- and byte-bounded stdout collector
├── models.rs                 # CLI model refs merged with local provider metadata
├── reload.rs                 # CLI reload and exit-status errors
├── active.rs                 # Strict V2 running-session map parser
├── token_usage.rs            # Paging, budgets, worker scan, fork attribution
├── command/                  # Collector test submodule
│   └── tests.rs              # EOF, timeout, limits, child ownership, FD cleanup
├── reload/                   # Reload test submodule
│   └── tests.rs              # Nonzero-status reporting
├── active/                   # Active-session test submodule
│   └── tests.rs              # Shape validation, redaction, count and bounds
└── token_usage/              # Usage tests and private HTTP transport
    ├── tests.rs              # Paging, forks, limits, fake HTTP, ignored live scan
    ├── transport.rs          # Registration, identity, bootstrap, HTTP requests
    └── transport/            # Transport test submodule
        └── tests.rs          # Trust checks, bootstrap, response bounds, deadlines
```

## Key Files

| Responsibility | File | Description |
| --- | --- | --- |
| Public boundary | `mod.rs` | Re-exports CLI operations, `ModelCatalogEntry`, and `TokenUsageRecord`. |
| Binary discovery | `binary.rs` | `opencode_candidates` and `resolve_opencode_binary`; final bare command resolves through PATH. |
| Bounded collection | `command.rs` | `collect_output` drains stdout without blocking indefinitely and reaps only its direct child. |
| Model catalog | `models.rs` | Combines CLI refs with configured models; local config supplies metadata and provider filtering. |
| Reload | `reload.rs` | Tries candidates until spawn succeeds; unsuccessful exit prefers stderr, then stdout. |
| Active work | `active.rs` | `opencode api session.active`, strict map validation, and running-session count. |
| Usage attribution | `token_usage.rs` | `token_usage_records_via_cli` scans timelines and returns only `{time, input, output}`. |
| Managed transport | `token_usage/transport.rs` | Validated registration, Basic auth, health identity checks, optional CLI bootstrap, bounded HTTP pages. |

## Conventions

- **Candidate order**: Reuse `opencode_candidates`: existing trimmed `OPENCODE_BIN`, known install paths, then bare `opencode`. Skip missing explicit paths but still try the final PATH candidate.
- **Fallback behavior**: Active sessions, reload, and bootstrap stop candidate fallback after a successful spawn, even if the operation then fails; model listing also retries command failures. The resolver checks existence, not executable validity.
- **Collector ownership**: Bounded operations pipe stdout and null stdin/stderr. Success requires both successful exit and stdout EOF; nonzero exit discards stdout immediately. On failure, kill/reap only the direct CLI child, never a process group that might contain the service.
- **Timeout scope**: Active queries use 5 seconds/1 MiB; bootstrap uses at most 10 seconds/32 KiB within the usage deadline. `reload.rs` and `models.rs` currently use `Command::output` without collector timeout/byte bounds.
- **Active-session contract**: Accept only an object envelope with a `data` map of unique `ses_` IDs with nonempty ASCII alphanumeric/underscore/hyphen suffixes and object values of type `running`. Count children too; malformed responses and nonzero exits are errors, not idle.
- **Error privacy**: Active, collector, and usage paths retain error kind/category/location or status, not response bodies, session identifiers, credentials, or raw serde/HTTP error text. Reload/model failure diagnostics intentionally include CLI stderr/stdout.
- **Registration trust**: Read the regular, at-most-8-KiB `opencode/service.json` under `XDG_STATE_HOME` or `~/.local/state`. Require a UUID, positive PID, V2 version, bounded password, and explicit-port HTTP URL on literal `127.0.0.1` or `[::1]` only.
- **Managed identity**: Use Basic auth user `opencode`, disable proxies/redirects, and match `/api/info` PID/version to registration. Bootstrap once for missing, unreachable, or identity-mismatched registration, then reread/recheck; invalid registration or unauthorized health fails closed.
- **HTTP paging**: Fetch sessions at limit 1000 and messages at limit 100, initially ascending; thereafter follow cursors until an empty page or no next cursor. Encode session IDs as path segments and cursors as query values; reject duplicate IDs and cursor loops.
- **Scan limits**: Share one 60-second deadline across connection, pages, bodies, workers, and aggregation. Use at most four timeline workers, 32 MiB per page, 300,000 metadata items/64 MiB retained metadata, and 100,000 records; failures never return partial usage.
- **Recorded usage**: Include recorded root `tokens.input/output` only for non-pending assistant/compaction messages, including terminal cancellation/error cases. Missing usage is not zero billing; exclude reasoning/cache fields and enforce safe JavaScript integer/date bounds.
- **Fork ownership**: Validate origin chains, before/through boundaries, native copy eligibility, and copied metadata projections before excluding inherited prefixes; missing origins/boundaries, cycles, and mismatches fail the scan.
- **Catalog editing**: Keep config-provided models listed even when disabled; config metadata is authoritative. Apply the provider filter after collecting refs, not as a positional V2 CLI argument.
- **Test fixtures**: Tests are `tests.rs` submodules; active/command/reload shell fixtures are Unix-gated, while usage tests use `FakeApi` and synthetic registration/auth. Keep `process::read_only_live_scan` ignored unless explicitly reviewing and testing the managed target.

## Anti-patterns

- Do not fetch large usage pages through CLI stdout: CLI `process.exit` can truncate responses; stdout is reserved for the small bootstrap health result.
- Do not deduplicate fork usage by message IDs or timestamp cutoffs: copied messages receive new IDs, and owned child messages may have older timestamps.

## Sync

This file is maintained as the directory scope changes. When files in this directory are added, modified, or deleted (components, logic units, utilities, constants, etc.), update this file's directory structure, key files, and conventions to keep the documentation in sync with the code.
