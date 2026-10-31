# Backend Runtime and Desktop Lifecycle

## Overview

Crate root for configuration persistence, group projection, resource discovery and mutation, deletion protection, and desktop lifecycle. Domain modules perform operations; `commands/` exposes IPC handlers, while `lib.rs` owns shell initialization and shutdown. `http_server.rs` owns the application-wide MOM HTTP listener and `mom-http.json`; `lib.rs` merges the listener-independent `ProxyService::router()` and shuts HTTP and proxy down concurrently. HTTP settings and MCP publications run in owned async tasks so cancelling an IPC waiter cannot interrupt disk-to-memory completion or listener restoration.

Paths in this table are relative to `src-tauri/src/`.

## Key Files

| Responsibility | File | Description |
| :--- | :--- | :--- |
| Shell lifecycle | `lib.rs` | Builds Tauri, installs shell plugins, manages ConfigPaths/PowerService/IslandService/ProxyService, routes tray then island window events, guards shutdown with an atomic barrier on RunEvent::ExitRequested, and shuts services down on RunEvent::Exit. |
| HTTP listener | `http_server.rs` | Owns `mom-http.json` (`port`, `allowLan`), serialized rebinds, and bounded graceful shutdown. |
| Manual updates | `updates.rs` | Reads native platform, checks the fixed public GitHub latest-release API, and opens only predefined project pages. |

## Conventions

- **Toolchain**: Keep main crate MSRV 1.90/edition 2021 and CI/release 1.96.0; `../native/macos-helper` retains MSRV 1.77.2 and separate formatting/check/test/Clippy gates. Fetch its locked dependencies before the first parent build; `../build.rs` builds it offline.
- **Lint policy**: Keep `../Cargo.toml` correctness/implicit-unsafe denials and undocumented-unsafe/redundant-clone warnings; run `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings` on macOS/Windows. Explain unsafe invariants with local `// SAFETY:` comments; fix warnings, not blanket allowances or pedantic/nursery/unwrap/missing-docs restrictions.
- **Initialization**: Resolve production paths once with `ConfigPaths::from_environment` in `lib.rs`; initialize the captured local service target before PowerService/Island. Invalid private target config keeps management available but service operations fail closed; handlers reuse captured paths.
- **Update safety**: Use bounded blocking HTTP/timeouts without credentials/redirects; compare only stable `vX.Y.Z`/`X.Y.Z` numerically. HTTP 404 means no stable release; malformed/network/rate-limit failures remain errors. Never install updates or accept arbitrary API/browser URLs.
- **Project links**: Validate `repository`/`releases` before building native opener commands; pass fixed URLs as arguments without shell interpolation and check launch status. Tests inspect commands without running an opener.
- **Tests**: Module tests use `tests.rs` submodules, explicit `#[path]` safety/remote suites, or inline tests in `tray.rs`; public filesystem workflows live in `../tests/app.rs` and `../tests/resources.rs`.
