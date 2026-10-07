# Rust Backend Modules

## Overview

Crate root for configuration persistence, group projection, resource discovery and mutation, deletion protection, and desktop lifecycle. Domain modules perform operations; `commands/` exposes IPC handlers, while `lib.rs` owns shell initialization and shutdown.

## Directory Structure

```text
src/                    # Rust crate root
├── AGENTS.md            # Directory-specific guidance
├── lib.rs               # Module declarations and Tauri lifecycle
├── main.rs              # Calls the library run entry point
├── agents.rs            # Inline and global Markdown agent operations
├── agents_md.rs         # Markdown frontmatter parsing and preserving edits
├── commands/            # Domain IPC handlers and app response tests
├── document.rs          # JsoncDoc and plain file I/O
├── error.rs             # Serialized AppError and ErrorCode
├── groups.rs            # Group persistence, selection, and projection orchestration
├── jsonc.rs             # CST-based JSONC parsing and path patches
├── mcp.rs               # Global MCP overlays and guarded mutations
├── mcp/                 # MCP symlink and snapshot safety tests
├── models.rs            # Wire types, app config loading, and persistence
├── models/              # App config serialization tests
├── opencode_cli/        # Binary discovery, models, reload, sessions, and usage
├── paths.rs             # ConfigPaths resolution and resource source precedence
├── paths/               # Path resolution and precedence tests
├── power/               # Lid protection service, platform controllers, and tests
├── projection.rs        # Native, Slim, and OMO document transformations
├── providers.rs         # Provider/model CRUD and ModelRef parsing
├── refs.rs              # Model and agent reference collection
├── replacement.rs       # Slim/OMO model reference scanning
├── resource_file.rs     # Snapshots, checked publication, and recovery backups
├── resource_file/       # File identity, permissions, and publication safety tests
├── skills.rs            # Local/HTTP skill discovery and local mutations
├── skills/              # Management safety and remote catalog tests
├── tray.rs              # Native tray/window handlers and inline tests
├── updates.rs           # App metadata, manual stable-release checks, and fixed project links
└── updates/             # Version, HTTP, wire contract, and destination safety tests
```

## Key Files

| Responsibility | File | Description |
| --- | --- | --- |
| Shell lifecycle | `lib.rs` | Builds Tauri, installs single-instance/autostart plugins, manages ConfigPaths/PowerService, registers handlers, and shuts power down on RunEvent::Exit. |
| Error contract | `error.rs` | AppError serializes into frontend CommandError; ErrorCode uses snake_case. |
| Document editing | `document.rs`, `jsonc.rs` | JsoncDoc retains source text and raw values; CST path patches preserve untouched comments and formatting. |
| Path authority | `paths.rs` | Resolves selected config, ordered global sources, resource directories, and relative skill paths. |
| Group lifecycle | `groups.rs`, `projection.rs` | Validates group drafts, updates selection, and applies target-specific mappings. |
| Deletion protection | `refs.rs`, `replacement.rs` | Indexes agent/model references, including persisted groups and plugin mappings. |
| File safety | `resource_file.rs` | Validates snapshots and aliases, stages replacements, and retains recovery backups on failure. |
| Agent documents | `agents.rs`, `agents_md.rs` | Merges inline/Markdown sources and edits selected storage while retaining untouched frontmatter sections. |
| Provider/model editing | `providers.rs`, `models.rs` | ModelRef identity, field-level config edits, and persisted app data types. |
| Resource management | `mcp.rs`, `skills.rs` | Ordered global discovery, provenance, diagnostics, and guarded MCP/local skill writes. |
| Manual updates | `updates.rs` | Reads native platform, checks the fixed public GitHub latest-release API, and opens only predefined project pages. |

## Conventions

- **Initialization**: Resolve production paths once with `ConfigPaths::from_environment` in `lib.rs`; handlers consume managed state rather than rediscovering paths.
- **Path precedence**: `global_config_files` and `global_skill_dirs` keep ordered sources with the last duplicate retained; relative overrides and skill paths use the captured working directory.
- **Error serialization**: Preserve `{ code, message, detail? }`; absent detail is omitted. `AppError::io` includes operation/path context without embedding file contents.
- **JSONC edits**: Use `JsoncDoc::patch` for target mutations; `None` removes a path. Object roots, comments, and trailing commas are supported; invalid documents fail rather than being overwritten.
- **Projection boundaries**: Native projection changes existing `agents.<name>.model` only and warns for missing agents; variants join as `provider/model#variant`. OMO mappings live under `opencode.agents` and `opencode.categories`.
- **Group deletion**: `delete_group` removes only app data and clears a deleted active selection; target files and their residual mappings remain untouched.
- **Reference guards**: Provider/model IPC deletes query `refs` before CRUD; agent deletes receive `collect_agent_references`. The model index includes inline/Markdown agents, app groups, Slim presets, and OMO mappings.
- **Backup-before-replace**: MCP replacements and local skill updates use `resource_file::replace`: validate snapshots, create recovery `.bak` files before publication, preserve modes, and remove owned backups only on success. `document::write_file` and `JsoncDoc::save` are plain writes without backups or locking.
- **Resource freshness**: MCP/skill updates compare expected source and content/config; resource snapshots revalidate aliases, identity, and content. MCP validates the effective patched server before publication to catch duplicate-key mismatches.
- **Update safety**: Manual checks use bounded blocking HTTP with timeouts, no credentials, and no redirects; only stable `vX.Y.Z`/`X.Y.Z` tags are compared numerically. HTTP 404 means no stable release; malformed/network/rate-limit failures remain errors. Never install updates or accept arbitrary API/browser URLs.
- **Project links**: Validate `repository`/`releases` before building native opener commands; pass fixed URLs as arguments without shell interpolation and check launch status. Tests inspect commands without running an opener.
- **Tests**: Module tests use `tests.rs` submodules, explicit `#[path]` safety/remote suites, or inline tests in `tray.rs`; public filesystem workflows live in `../tests/app.rs` and `../tests/resources.rs`.

## Sync

This file is maintained as the directory scope changes. When files in this directory are added, modified, or deleted (components, logic units, utilities, constants, etc.), update this file's directory structure, key files, and conventions to keep the documentation in sync with the code.
