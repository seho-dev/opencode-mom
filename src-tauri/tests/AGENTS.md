# Backend Integration Tests

## Overview

Integration tests exercise the public `opencode_mom_tauri` crate API against real filesystem fixtures without starting the Tauri shell. `app.rs` covers agents, groups, providers, models, and persisted selection; `resources.rs` covers resource paths, MCP, skills, HTTP catalogs, and document preservation. Mutating tests use temporary homes; one optional app smoke test reads the machine's existing config.

## Directory Structure

```text
tests/           # Public-API integration test targets
├── AGENTS.md     # Directory-specific guidance
├── app.rs        # App configuration lifecycles and group projection
└── resources.rs  # Resource discovery, management, and catalog fixtures
```

## Key Files

| Responsibility | File | Description |
| --- | --- | --- |
| App harness | `app.rs` | `temp_home` returns `TestHome` with explicit `ConfigPaths`; `Drop` removes its root. |
| App fixtures | `app.rs` | `provider`, `model_def`, `binding`, `group`, and `write_opencode` seed lifecycle tests; `load_config` rereads persisted state. |
| Resource harness | `resources.rs` | `Home::new`, `paths`, and `write` isolate filesystem fixtures with `Drop` cleanup. |
| HTTP fixtures | `resources.rs` | `serve`, `configure_catalog`, and `assert_catalog_requests` provide loopback catalog responses and request assertions. |
| Mutation guards | `resources.rs` | `mcp_update` and `skill_update` carry expected source/content; `mcp_directory_snapshot` captures files for no-write assertions. |
| Live smoke | `app.rs` | `readonly_smoke_against_real_v2_config` reads providers, agents, and references only when the real config exists. |

## Conventions

- **API boundary**: Import public modules from `opencode_mom_tauri`; assert observable results and persisted files rather than invoking Tauri handlers or launching the app.
- **Temporary homes**: Keep `TestHome` or `Home` alive for each mutating test so `Drop` cleans up its unique temporary root, including on panic.
- **Explicit paths**: App fixtures use `ConfigPaths::for_home`; resource fixtures use `ConfigPaths::with_resource_parts` to supply home, overrides, config directory, and working directory without changing process environment variables.
- **Fixture reuse**: Reuse the helpers in the relevant test target; `write_opencode` and `Home::write` seed files through `document::write_file`.
- **Round-trips**: Reread persisted config or Markdown after successful mutations. For rejected mutations, compare original bytes or directory snapshots and check that no unwanted files appear.
- **Resource assertions**: Cover provenance, shadow precedence, diagnostics, unrelated JSONC content, and stale source/content rejection alongside CRUD results.
- **HTTP isolation**: Use `serve` on `127.0.0.1:0` with its unique catalog prefix; join the server worker and assert requested paths. `assert_catalog_requests` requires the index request first and sorts the remaining paths.
- **Unix coverage**: Gate symlink and permission-mode tests with `#[cfg(unix)]` and keep all fixtures inside the temporary home.
- **Real-home exception**: Keep `readonly_smoke_against_real_v2_config` read-only; it returns early when `HOME` or the resolved config is absent. Do not reuse its real paths for mutation fixtures.

## Sync

This file is maintained as the directory scope changes. When files in this directory are added, modified, or deleted (components, logic units, utilities, constants, etc.), update this file's directory structure, key files, and conventions to keep the documentation in sync with the code.
