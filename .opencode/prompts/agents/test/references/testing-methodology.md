# Testing Methodology

## Goal

Ensure testing across opencode-mom protects product behavior, catches real regressions, and maintains refactoring confidence without coupling to implementation details or endangering live systems.

## Architecture and Frameworks

opencode-mom organizes tests across distinct runtimes:

| Layer | Runner / Framework | Target Path | Verification Scope |
| :--- | :--- | :--- | :--- |
| Svelte 5 Components | Vitest + jsdom | `src/**/__tests__/*.test.ts` | Reactive UI, user interaction, accessibility, Bits UI controls |
| State & Adapters | Node `node:test` | `src/config/__tests__/*.test.mjs`, `src/routes/*/__tests__/*.test.mjs`, `src/utils/__tests__/*.test.mjs` | Reactive store transitions, IPC adapter translations, utilities |
| Main Rust Backend | Cargo test (`--locked`) | `src-tauri/src/**/tests.rs` | Config persistence, projection, JSONC preservation, backups, AppError |
| macOS Helper | Cargo test (`--offline --locked`) | `src-tauri/native/macos-helper` | Standalone helper daemon, socket protocol, watchdog, policy journal |

## What to Cover

Every meaningful test suite must explicitly cover relevant scenarios:

- **Happy path**: Successful operation under expected inputs and permissions.
- **Meaningful edge cases**: Empty resource lists, missing fields, unicode or long strings, boundary values.
- **Standardized error states**:
  - Backend errors emitted as `AppError` (`src-tauri/src/error.rs`): `not_found`, `references_blocked`, `validation_failed`, `configuration_failed`, `busy`. Note: `AppError::io` carries operation and path context without raw file content.
  - Frontend command errors (`src/types/app.ts`): includes the backend error codes plus `conflict` and `ipc_error`.
- **Async transitions**: Loading indicators, optimistic updates, background refreshes, and retry recovery.
- **State rollback and recovery**: Verify that failed operations (such as theme/locale saves) roll back store state without unhandled rejections. See `src/config/__tests__/config-refresh.test.mjs`.

## Backend Regression Priorities

Backend tests verify strict file and config contracts:

- **JSONC preservation**: `JsoncDoc::patch` applies lossless edits to owned CST paths while retaining untouched comments, whitespace, and formatting.
- **Write taxonomy**:
  - General document edits (`document::write_file`, `JsoncDoc::save`): stage in parent directory, fsync, and atomically rename without locking or backups.
  - Atomic config saves (`models::save_config`): callers serialize read-modify-write via `models::config_write_lock`, then sibling staging, sync, and atomic rename without `.bak`.
  - Guarded replacements (`resource_file::replace`): validates frozen snapshots, creates recovery `.bak` files before publication, preserves permissions, and deletes `.bak` only on success.
- **Snapshot integrity**: Frozen `expectedPath` and `expectedContent` are echoed unchanged through JavaScript; missing fields fail with `validation_failed`, and modified/stale targets fail with `configuration_failed` (`agent_changed()`).
- **Scope guards**: Read-only project scopes reject resource mutations; null or omitted scopes remain Global.
- **Protected-reference guards**: Provider, model, and agent deletion must check `refs` and reject active references with `references_blocked`. Built-in agent deletion is also rejected with `references_blocked`.
- **Privacy and redaction**: In-memory credential protection and diagnostic allowlist redaction prevent secret leakage.

## Assertion Priorities

Evaluate assertions in this order:

1. **Semantic behavior and contract outcomes**: Verify rendered roles, visible names, localized text, active states, and return payloads.
2. **Stable accessibility selectors**: Use `getByRole`, `getByLabelText`, and `getByPlaceholderText` over CSS queries.
3. **Explicit state changes**: Assert changes in reactive state or emitted adapter payloads.
4. **Error classification**: Assert specific error codes and failure details rather than generic exception catching.

Avoid asserting purely cosmetic details, such as Tailwind utility classes, DOM tag hierarchies, or SVG icon attributes, unless they directly represent a dynamic business contract.

## Determinism and Timing

- **Async coordination**: Use `waitFor`, deferred promises, or event channels rather than arbitrary sleep calls.
- **Controlled timers**: Controlled and fake timers are permitted when testing debounce intervals, periodic refresh, or polling loops. Advance them deterministically; avoid mixing manual promise resolution with conflicting timers.
- **State and resource isolation**: Ensure mocks, stores, timers, event listeners, and fetch mocks are reset and cleared between test runs.

## Safety and Sensitive Data

- **Live target isolation**: Never allow tests or test scripts to read or write live user directories:
  - `~/.config/opencode-mom` (contains credentials, diagnostic logs, HTTP tokens)
  - `~/.config/opencode`
  - `~/.omo`
- **Temporary directories**: Main-crate filesystem tests must use RAII `tempfile::tempdir()`; inject sandboxed `ConfigPaths` only for tests exercising path-resolution APIs or filesystem operations. Helper tests use existing uniquely owned temporary-socket cleanup without system dependencies. Pure backend logic, parsers, and in-memory state tests must NOT construct temporary directories or mock ConfigPaths.
- **Privileged operations**: Never execute system-level or hardware power management commands in automated tests (`sudo`, `pmset`, `powercfg`).
- **Domain privacy contracts**: Never use real credentials, private user files, or real tokens. Synthetic sentinel values (e.g. `test-token-123`) are explicitly permitted to test authentication wire contracts, token rotation, and redaction logic without leaking real secrets.

## Anti-Patterns

- Retesting third-party libraries (e.g., verifying Bits UI or Svelte internals rather than product wiring).
- Mocking the system under test or replacing internal reactive state with static objects.
- Masking errors with broad catch blocks or assertions that pass on any error.
- Adding test-only methods or fields to production modules to simplify assertions.

## Source References

- [Config contracts](../../../../../src/config/docs/command-contracts.md) - Frontend command specifications and generation contracts.
- [Backend contracts](../../../../../src-tauri/src/docs/config-contracts.md) - Storage, atomic save, and replacement contracts.
- [Store regression](../../../../../src/config/__tests__/config-refresh.test.mjs) - Store lifecycle and preference rollback tests.
