# Adapter Notes

## What the Adapter Means

In opencode-mom, `CommandAdapter` (`src/config/adapter.ts`) is the typed boundary bridging the SvelteKit frontend to the Tauri Rust backend.

### Key Adapter Responsibilities

- **Wire type translation**: Maps frontend domain representations to backend wire types (e.g., group types `native` to `opencode`, `omo` to `oh-my-openagent`).
- **Command translation**: `createTauriAdapter(invoke)` maps frontend method calls to named Tauri commands and argument payloads.
- **Dynamic wrapper**: Structured backend `AppError` payloads pass through typed commands; `createCommandAdapter` normalizes only unstructured bridge exceptions to `ipc_error`.
- **Error types**: `CommandError` is a TypeScript interface defining `{ code: CommandErrorCode, message: string, detail?: unknown }`.
- **Optional argument normalization**: Preserve omitted, undefined, null, and empty-string semantics exactly as expected by existing tests.

## Component Testing with Harness

When testing Svelte components that consume application state or translations, mount them with the test harness:

```ts
import { render } from '@testing-library/svelte';
import Harness from '$src/test-utils/harness.svelte';

render(AppearanceControls, {}, {
  wrapper: Harness,
  wrapperProps: { config },
});
```

### Harness Rules

- **Isolated components**: Components that only take props and emit callbacks (e.g. `Button`, `Checkbox`, `DataTable`) do not require `Harness`.
- **Context injection**: `harness.svelte` calls `setConfig(config)` and `setI18n(createI18n(() => config.preferences.locale))` to initialize Svelte context before mounting children.
- **Valid signatures**: Pass the test double via `wrapperProps: { config }`. Do not invent non-existent wrapper options or fake store properties.

## Unit and Adapter Testing Patterns

Pure JavaScript needs no Vite. Modules importing TypeScript, Svelte 5 runes, or `$src/*` path aliases use existing Vite SSR loading. SSR does not provide DOM or `$effect` lifecycle coverage; mounting and DOM cleanup belong in Vitest. Always close any created Vite server in a `finally` block.

### Testing the IPC Wire Boundary

When asserting that frontend methods invoke the exact Tauri command names and serialized payloads, instantiate `createTauriAdapter` with a stub `invoke` function:

```js
const { createTauriAdapter } = await vite.ssrLoadModule('/src/config/adapter.ts');
const calls = [];
const adapter = createTauriAdapter(async (command, args) => {
  calls.push({ command, args });
  return stubResponse;
});
```

### Testing Store Logic and Lifecycle

When verifying store state transitions, draft retention, rollback, or refresh sequencing, instantiate `createConfigStore`:

```js
const { createConfigStore } = await vite.ssrLoadModule('/src/config/store.svelte.ts');
const store = createConfigStore(stubAdapter, false, false);
```

- Pass `false, false` for `catalogOnRefresh` and `initialize` to disable automatic background catalog loading and startup app state initialization in tests.
- Always close the Vite server in a `finally` block:
  ```js
  const vite = await createServer({ server: { middlewareMode: true }, appType: 'custom' });
  try {
    // run store assertions
  } finally {
    await vite.close();
  }
  ```

## Rust Backend and Helper Boundaries

Backend tests operate across two distinct Cargo workspaces:

| Workspace | Manifest | Toolchain & Flags | Testing Focus |
| :--- | :--- | :--- | :--- |
| Main Crate | `src-tauri/Cargo.toml` | MSRV 1.90, CI pin 1.96.0; `--locked` | IPC handlers, config projection, atomic file replacement, JSONC formatting |
| macOS Helper | `src-tauri/native/macos-helper/Cargo.toml` | MSRV 1.77.2; `--offline --locked` | Helper daemon, UNIX domain socket, `ReapOnDrop`, `nofollow`/`cloexec`, watchdog, policy journals |

### Backend Testing Constraints

- **Toolchain separation**: NEVER compile or check the parent crate with the macOS helper's older 1.77.2 toolchain.
- **Prefetch rule**: Run `cargo fetch --manifest-path src-tauri/native/macos-helper/Cargo.toml --locked` before running offline helper tests on a fresh machine.
- **Policy and journal testing**: Validate journal-before-write, external drift detection, and state restoration via mock backends (`src-tauri/native/macos-helper/src/policy.rs`) without requiring live system policy changes.
- **RAII filesystem isolation**: Main-crate filesystem tests must use RAII `tempfile::tempdir()`; inject sandboxed `ConfigPaths` only for tests exercising path-resolution APIs or filesystem operations. Helper tests use existing uniquely owned temporary-socket cleanup without system dependencies. Pure backend logic, parsers, and in-memory state tests must NOT construct temporary directories or mock ConfigPaths.

## Keep-Real vs Mock Mapping

| Dependency | Treatment in Component Tests | Treatment in Node Unit Tests | Treatment in Rust Backend Tests |
| :--- | :--- | :--- | :--- |
| `CommandAdapter` | Mock with stubbed responses | `createTauriAdapter(stubInvoke)` or stub adapter | Real handler invoked via internal functions |
| `ConfigStore` | Supply configured store | Test real store transitions | N/A (Frontend only) |
| Filesystem paths | N/A | Mock paths in adapter | Isolated temporary directories (`tempfile::tempdir()`) |
| Live config files | Never access live files | Never access live files | Never touch `~/.config/opencode-mom` |
| Local HTTP server | Mock adapter endpoints | Mock network layer | Isolated loopback port with ephemeral tokens |

## Common Risk Areas

- **Mismatched error codes**: Passing generic Error strings instead of `CommandError` with expected codes.
- **Stale generation tokens**: Ignoring target generation IDs in Island or service state updates.
- **Secret leakage**: Accidental inclusion of real sensitive data in mock data or error assertions; synthetic auth/redaction sentinels remain valid.

## Source References

- [Adapter regression](../../../../../src/config/__tests__/adapter.test.mjs) - Adapter command mocking and payload verification.
- [Store regression](../../../../../src/config/__tests__/config-refresh.test.mjs) - Store lifecycle and preference rollback tests.
- [Helper contracts](../../../../../src-tauri/native/macos-helper/AGENTS.md) - Helper socket and policy testing rules.
