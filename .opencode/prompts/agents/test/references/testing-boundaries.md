# Testing Boundaries

## Boundary Table

| Test Type | Best For | Cover | Avoid |
| :--- | :--- | :--- | :--- |
| Component | UI components and Bits UI controls | Render output, user interaction, accessibility roles, localized feedback | Internal reactive flags, deep DOM selectors, cosmetic styles |
| Logic Unit | State machines, store methods, runes | State transitions, refresh loops, draft recovery, error handling | Retesting Svelte internals, mocking local pure helpers |
| Util | Pure helpers and schema mapping | Formatters, parsers, validators, model options, constants | DOM setup, mocking standard library, heavy wrappers |
| Integration | Multi-layer workflows and IPC contracts | CommandAdapter translation, store mutation lifecycle, projection | Full manual OS behavior, live daemon socket calls |
| Rust Backend | Config persistence, projection, JSONC | Safe replacement, atomic writes, recovery backup retention/cleanup, AppError mapping | Live desktop environment, unisolated filesystem writes |

## Component Tests

Use component tests (`src/**/__tests__/*.test.ts`) when verifying UI behavior, user input, or accessibility semantics:

- **Runner**: Vitest with jsdom environment.
- **Context Harness**: Use `Harness` (`src/test-utils/harness.svelte`) when the component requires `ConfigStore` or translation context:
  ```ts
  render(AppearanceControls, {}, { wrapper: Harness, wrapperProps: { config } });
  ```
- **Isolated Components**: Pure components (e.g. `Button`, `Checkbox`, `DataTable`) do not require `Harness` if they do not consume context.
- **Coverage**: User actions, keyboard navigation, role-based queries, and dynamic error notices.
- **Exclusions**: Do not test private component state or trigger unneeded layout renders.

## Logic Unit Tests

Use logic unit tests (`src/config/__tests__/*.test.mjs`, `src/routes/*/__tests__/*.test.mjs`, `src/utils/__tests__/*.test.mjs`) for reactive stores, state transitions, and async sequencing:

- **Runner**: Node `node:test` (`--test-concurrency=1`).
- **Pure JavaScript**: Executes directly without bundler or Vite overhead.
- **Vite SSR Loading**: Modules requiring TypeScript, Svelte 5 runes, or `$src/*` path aliases are loaded via Vite's SSR runtime (`createConfigStore` from `src/config/store.svelte.ts`).
- **SSR Limitations**: Note that Vite SSR evaluation does not run browser DOM lifecycles or Svelte `$effect` cleanup routines. Always clean up via `finally { await vite.close(); }`.
- **Coverage**: Store mutation lifecycles, optimistic updates, request serialization, and error recovery.

## Util Tests

Use util tests for standalone, deterministic logic:

- **Scope**: Model option computation, URL parsers, schema validators, date/time formatters, and constants.
- **Execution**: Run with Node `node:test` or Vitest with minimal overhead.
- **Approach**: Prefer parameterized or table-driven tests for edge-case coverage (null, undefined, malformed strings, boundary values).

## Integration Tests

Use integration tests when verifying cross-boundary behavior:

- **Scope**: Interactions between UI controllers, `ConfigStore`, and `CommandAdapter`.
- **Contracts**: Verify that frontend method calls serialize into the correct Tauri IPC payload and that backend responses or `CommandError` objects update store state predictably.
- **Boundaries**: Mock the IPC boundary at `CommandAdapter` rather than inventing an ad-hoc HTTP network layer.

## Rust Backend Tests

Backend tests live in `src-tauri` and are executed via Cargo:

- **Main Crate**: Validates config file operations, atomic replacements, JSONC formatting preservation, model projection, and error code conversions (`AppError`).
  - Distinguishes general document edits, callers serializing read-modify-write via `models::config_write_lock`, and guarded `resource_file::replace` with recovery backup retention/cleanup.
- **macOS Helper**: Separate workspace validating the standalone helper daemon, local UNIX domain sockets, `ReapOnDrop`, `nofollow`/`cloexec` descriptor checks, watchdog heartbeats, and policy journals (`src-tauri/native/macos-helper/src/policy.rs`).
- **Safety**: Main-crate filesystem tests must use RAII `tempfile::tempdir()`; inject sandboxed `ConfigPaths` only for tests exercising path-resolution APIs or filesystem operations. Helper tests use existing uniquely owned temporary-socket cleanup without system dependencies. Pure backend logic, parsers, and in-memory state tests must NOT construct temporary directories or mock ConfigPaths.

## Escalation Rules

- **Move upward to integration**: When unit tests require excessive mocking of tightly coupled collaborators or when confidence depends on IPC boundary translation.
- **Move downward to logic unit or util**: When a component test becomes slow, flaky, or overly complex due to unrelated UI rendering concerns.

## Source References

- [Vitest setup](../../../../../vitest.config.ts) - Vitest jsdom environment and setup file configuration.
- [Test utilities](../../../../../src/test-utils/AGENTS.md) - Component test harness conventions.
- [Backend contracts](../../../../../src-tauri/src/docs/config-contracts.md) - Storage, atomic save, and replacement contracts.
