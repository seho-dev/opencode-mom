# Rust Backend Modules

`http_server.rs` owns the application-wide MOM HTTP listener, `mom-http.json` (`port`, `allowLan`), serialized rebinds and bounded graceful shutdown. `lib.rs` merges `ProxyService::router()` into this listener independently of proxy enablement; exit shuts HTTP and proxy down concurrently. `ProxyService` owns MCP configuration, cancellation, sessions and process budgets only, never a listener or network settings.

HTTP settings transitions and MCP config/key publications run in owned async tasks, offloading filesystem publication to `spawn_blocking` while retaining their runtime/config lock. Cancelling an IPC waiter never cancels disk-to-memory completion or listener restoration. HTTP admission reads the gateway and selected service under the config lock, then captures its cancellation epoch in config -> token lock order; rotation and disable cancel the old epoch. Session close uses a cached owned coordinator; bulk cleanup owns drained sessions, finishes every SDK/bridge/process branch, and reports failures after cleanup. Post-commit session cleanup errors are logged, not returned as a failed config save or a lost rotated key. MCP serialized/decrypted temporary plaintext reuses `protected_data::SecretBytes`; it does not promise wiping long-lived Rust strings or serializer-internal allocations.

## Overview

Crate root for configuration persistence, group projection, resource discovery and mutation, deletion protection, and desktop lifecycle. Domain modules perform operations; `commands/` exposes IPC handlers, while `lib.rs` owns shell initialization and shutdown.

## Directory Structure

```text
src/                    # Rust crate root
├── AGENTS.md            # Directory-specific guidance
├── lib.rs               # Module declarations and Tauri lifecycle
├── main.rs              # Calls the library run entry point
├── agents.rs            # Scoped inline/Markdown reads and guarded global agent operations
├── agents/              # Snapshot, alias and checked-publication safety tests
├── agents_md.rs         # Markdown frontmatter parsing and preserving edits
├── commands/            # Domain IPC handlers and app response tests
├── document.rs          # JsoncDoc and plain file I/O
├── error.rs             # Serialized AppError and ErrorCode
├── groups.rs            # Group persistence, selection, and projection orchestration
├── island/              # macOS Mini Island window, cached IPC, SSE lifecycle, and geometry tests
├── jsonc.rs             # CST-based JSONC parsing and path patches
├── mcp.rs               # Global MCP overlays and guarded mutations
├── mcp/                 # MCP symlink and snapshot safety tests
├── mcp_proxy/           # Built-in MCP HTTP proxy service, process owner, and protocol bridging
├── models.rs            # Wire types, app config and project folder persistence
├── models/              # App config serialization tests
├── opencode/            # Shared service transport, binary discovery, installation/upgrades, and HTTP consumers
├── paths.rs             # ConfigPaths resolution and resource source precedence
├── paths/               # Path resolution and precedence tests
├── power/               # Lid protection service, platform controllers, and tests
├── private_file.rs      # Bounded private file I/O, mode validation, and atomic publication primitives
├── projection.rs        # Native, Slim, and OMO document transformations
├── protected_data.rs    # Platform DPAPI credential protection, memory wiping, and native cleanup
├── providers.rs         # Provider/model CRUD and ModelRef parsing
├── refs.rs              # Model and agent reference collection, including plugin mappings
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
| Shell lifecycle | `lib.rs` | Builds Tauri, installs shell plugins, manages ConfigPaths/PowerService/IslandService/ProxyService, routes tray then island window events, guards shutdown with an atomic barrier on RunEvent::ExitRequested, and shuts services down on RunEvent::Exit. |
| Mini Island | `island/` | macOS-only bounded transparent window, opt-in persisted flag, cancellable validated OpenCode SSE cache, and generation-guarded actions. |
| Error contract | `error.rs` | AppError serializes into frontend CommandError; ErrorCode uses snake_case. |
| Document editing | `document.rs`, `jsonc.rs` | JsoncDoc retains source text and raw values; CST path patches preserve untouched comments and formatting. |
| Path authority | `paths.rs` | Resolves selected config, ordered resource aliases, absolute/home skill paths, and temporary exact-root project scopes. |
| Group lifecycle | `groups.rs`, `projection.rs` | Validates group drafts, updates selection, and applies target-specific mappings. |
| Deletion protection | `refs.rs` | Indexes agent/model references, including persisted groups and plugin mappings. |
| File safety | `resource_file.rs` | Validates snapshots and aliases, stages replacements, and retains recovery backups on failure. |
| Agent documents | `agents.rs`, `agents_md.rs` | Merges inline/Markdown sources and edits selected storage while retaining untouched frontmatter sections. |
| Provider/model editing | `providers.rs`, `models.rs` | ModelRef identity, field-level config edits, and persisted app data types. |
| Resource management | `mcp.rs`, `skills.rs` | Ordered global discovery, provenance, diagnostics, and guarded MCP/local skill writes. |
| OpenCode installation | `opencode/installation.rs` | Local exact-binary ownership inspection, explicit stable V2 checks and guarded Unix curl/npm/bun/pnpm/yarn native upgrades without OpenCode service lifecycle actions. |
| Manual updates | `updates.rs` | Reads native platform, checks the fixed public GitHub latest-release API, and opens only predefined project pages. |

## Conventions

- **Toolchain**: Main crate MSRV is Rust 1.90 (edition 2021); CI/release uses 1.96.0. The standalone `../native/macos-helper` workspace retains MSRV 1.77.2 and needs its own formatting/check/test/Clippy gates. Fetch its locked dependencies before the first parent build because `../build.rs` builds it offline.
- **Lint policy**: `../Cargo.toml` denies Clippy correctness and implicit unsafe operations inside unsafe functions, and warns on undocumented unsafe blocks and redundant clones. CI runs `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings` on macOS and Windows. Explain unsafe invariants with local `// SAFETY:` comments; fix warnings rather than adding blanket allowances. Do not enable all pedantic/nursery lints or blanket unwrap/missing-docs restrictions.
- **Initialization**: Resolve production paths once with `ConfigPaths::from_environment` in `lib.rs`; initialize the captured local service target before PowerService/Island. Invalid private target config leaves management available but service operations fail closed. Handlers consume captured state rather than rediscovering paths.
- **Local service target**: `opencode/target.rs` privately persists the endpoint and backend-only credential outside AppConfig; Shared defaults to validated registration with optional bounded bootstrap, fixed literal loopback V2 never falls back/bootstrap. Status has no bootstrap. Active/catalog/reload/usage/power/Island share `opencode/transport.rs`. Target saves and actual power/Island enable transitions share a mutex. Prepare local power/Island guards before persistence, invalidate old results only after successful publication, then wake/reconnect outside the transition before probing status. Enabled preferences and current power authorization survive; disabled features remain disabled. Active writes/bootstrap and initial power authorization can return `busy`. Power effects are separately serialized against commit; never hold the transition across network, authorization or native dispatch waits. Native Island dispatch acquires the mutex only inside the dispatched transition, never while waiting for dispatch.
- **Service credentials**: Unix uses a regular account-owned 0600 plaintext `opencode-service.json` capped at 8 KiB. Windows persists numeric-byte `encryptedPassword` with native current-account DPAPI (32 KiB file/8 KiB ciphertext), never plaintext fallback; decryption failures fail closed. Shared registration credentials are separate. Never return passwords to frontend DTOs/events/errors or log them. Neither permissions nor DPAPI protect against account compromise, same-account malicious processes or administrators. Windows-native DPAPI tests have not run on this macOS host; native acceptance remains outside automated host tests, with no verified Windows ACL claim.
- **Island lifecycle**: `state.islandEnabled` defaults false outside preferences. Island setup errors return safe disabled readback without failing app startup; explicit enabled Refresh rebuilds unavailable connections. Disable/exit aborts the task and clears private state without stopping sessions. State/preferences reads are cache-only; message invalidations are limited to the expanded selected session. Cross-platform reads report unsupported and enabling fails validation.
- **App config writes**: Island, preferences, group and project read/modify/write operations share `models::config_write_lock`. `save_config` serializes first, stages/syncs a sibling file and atomically renames it, without `.bak`; sync the Unix directory after publication, warning rather than reporting rollback on post-commit sync failure. Preserve unrelated selection fields and existing symlink targets/modes. General target config writes retain their existing behavior.
- **Path precedence**: Ordered sources keep the last duplicate retained; config overrides resolve against the captured working directory. Agent/skill directory aliases scan singular before plural, including config-dir overrides; no mode/modes discovery. Configured relative skill strings are ignored without filesystem access; Global retains absolute/home/HTTP sources and .claude/.agents skill roots.
- **Projects**: App config stores canonical UTF-8 directory paths and UUIDs, not names. Add and scoped reads reject filesystem roots and canonical overlaps of the four fixed JSON sources/native agent/skill directories with captured Global config files/resource roots, including overrides. No home/depth blacklist or symlink-race guarantee. A temporary ConfigPaths clone reads only that exact root, ignores all project skills arrays and leaves Global paths/cwd unchanged. Missing roots return `not_found`, unreadable/retargeted roots fail without Global fallback. All project resource mutations are rejected; omitted/null scope remains Global independently of saved selection; removal only forgets metadata.
- **Agent provenance**: Inline previews retain every actual JSON path within the scope; Global inline reads retain the selected config behavior. Previews carry logical `path`, `resolvedPath` and backend-generated `content` (inline agent JSON string or full Markdown). Updates require `expectedPath`/`expectedContent`; missing snapshots return `validation_failed`, stale path/content is rejected. Echo strings unchanged across JavaScript to avoid numeric normalization/rounding. Create payloads are unchanged. Markdown mutations reuse existing aliases and reject distinct canonical targets; static Global symlinks remain supported.
- **Error serialization**: Preserve `{ code, message, detail? }`; absent detail is omitted. `AppError::io` includes operation/path context without embedding file contents.
- **OpenCode upgrades**: CLI commands offload installation inspection/check/upgrade to blocking tasks. Automatic upgrade requires strictly proven Unix curl/npm/bun/pnpm/yarn ownership and current stable V2 >=2.0.25. Use `selected_binary upgrade <stable-V2-target> --method <method>` with expected path/version and latest stable V2 target guards, revalidate ownership and read back the same path afterward; concurrent upgrades return `busy`. Windows/Homebrew/Vite Plus/older V2/unproven/manual/bundled sources remain manual. Never use sudo, upgrade bundled binaries, or start/stop/restart services; CLI version is not running backend version. Use fake installation tests only.
- **JSONC edits**: Use `JsoncDoc::patch` for target mutations; `None` removes a path. Object roots, comments, and trailing commas are supported; invalid documents fail rather than being overwritten.
- **Projection boundaries**: Native projection changes existing `agents.<name>.model` only and warns for missing agents; variants join as `provider/model#variant`. OMO mappings live under `opencode.agents` and `opencode.categories`.
- **Group deletion**: `delete_group` removes only app data and clears a deleted active selection; target files and their residual mappings remain untouched.
- **Reference guards**: Provider/model IPC deletes query `refs` before CRUD; agent deletes receive `collect_agent_references`. The model index includes inline/Markdown agents, app groups, Slim presets, and OMO mappings.
- **Backup-before-replace**: Existing-file inline agent CRUD, Markdown agent updates, MCP replacements and local skill updates use `resource_file::replace`: validate snapshots, create recovery `.bak` files before publication, preserve modes, and remove owned backups only on success. New agent files use checked creation. `document::write_file` and `JsoncDoc::save` remain plain writes without backups or locking.
- **Resource freshness**: Agent/MCP/skill updates compare expected source and content/config; resource snapshots revalidate aliases, identity, and content, rejecting detected retargets. MCP validates the effective patched server before publication to catch duplicate-key mismatches. Checks retain a last-check/rename race and per-file atomicity, not arbitrary-file security against concurrent same-account writers.
- **Update safety**: Manual checks use bounded blocking HTTP with timeouts, no credentials, and no redirects; only stable `vX.Y.Z`/`X.Y.Z` tags are compared numerically. HTTP 404 means no stable release; malformed/network/rate-limit failures remain errors. Never install updates or accept arbitrary API/browser URLs.
- **Project links**: Validate `repository`/`releases` before building native opener commands; pass fixed URLs as arguments without shell interpolation and check launch status. Tests inspect commands without running an opener.
- **MCP proxy lifecycle**: `ProxyService` manages the built-in HTTP MCP proxy gateway, dynamic session bridges, and process owners. Invalid configuration falls back to an in-memory disabled service without overwriting the source file or failing app startup. Key rotation publishes a cloned config before changing memory or closing sessions; successful config saves update the shared live owner budget, including outstanding permits when reducing capacity. Shutdown cancels HTTP admission and SSE streams, closes sessions concurrently with listener draining, and waits for process permits within its deadline. `RunEvent::ExitRequested` blocks every exit request until the single 8-second shutdown task completes; only then is exit allowed. A bounded defense-in-depth shutdown is retained on `RunEvent::Exit`.
- **Tests**: Module tests use `tests.rs` submodules, explicit `#[path]` safety/remote suites, or inline tests in `tray.rs`; public filesystem workflows live in `../tests/app.rs` and `../tests/resources.rs`.

## Sync

This file is maintained as the directory scope changes. When files in this directory are added, modified, or deleted (components, logic units, utilities, constants, etc.), update this file's directory structure, key files, and conventions to keep the documentation in sync with the code.
