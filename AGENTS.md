# opencode-mom

## Overview

Cross-platform desktop app for switching Oh My OpenAgent model groups. It stores named groups (native / slim / OMO) plus provider, agent, MCP, and skill metadata in its own config, then merges the selected group into target application configs. Tauri 2 is the shell; the Svelte 5 SPA is the management console; the Rust backend owns config persistence, projection, backups, and macOS/Windows power behavior. README.md is the authoritative user-facing behavior reference.

## Stack

- Frontend: SvelteKit 2, Svelte 5 (runes), TypeScript strict, Tailwind CSS 4, Bits UI, Vitest, Biome.
- Backend: Tauri 2, Rust edition 2021 (main crate MSRV 1.90; standalone macOS helper MSRV 1.77.2; CI/release pin 1.96.0).
- Toolchain: Node 22.23.0 and npm. SvelteKit uses the static adapter with `ssr = false` (SPA fallback `index.html`).

## Commands

- `npm run dev` / `npm run build` - Vite dev server / production web build.
- `npm run check` / `npm run check:fix` - Biome format + lint (2-space, single quotes, semicolons, trailing commas, 120 cols).
- `npm run test:components` - Vitest component tests (`src/**/__tests__/*.test.ts`, jsdom).
- `npm run test:unit` - `node --test` unit tests (`src/**/__tests__/*.test.mjs`).
- `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` - Rust backend formatting.
- `cargo check|test --manifest-path src-tauri/Cargo.toml --locked` - Rust backend validation.
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings` - strict Rust lint gate on macOS and Windows.
- `npm run tauri dev` / `npm run tauri build` - run / package the native app.
- macOS helper is a separate locked workspace: run `cargo fmt --manifest-path src-tauri/native/macos-helper/Cargo.toml --all -- --check`, `cargo check|test --manifest-path src-tauri/native/macos-helper/Cargo.toml --offline --locked`, and `cargo clippy --manifest-path src-tauri/native/macos-helper/Cargo.toml --all-targets --offline --locked -- -D warnings`. Before the first parent or helper build on a fresh cache, run `cargo fetch --manifest-path src-tauri/native/macos-helper/Cargo.toml --locked`.

## Layout

```
src/routes/            # Pages: / (dashboard), providers, models, agents, groups, mcp, skills, settings, tray popup
src/config/            # CommandAdapter IPC boundary (adapter.ts) + runes config store (store.svelte.ts, context.ts)
src/shell/             # App chrome: shell, sidebar, header, mobile nav, splash, toasts
src/components/        # Bits UI wrappers + shared components (button, dialog, combobox, table, ...)
src/types/             # Shared wire and domain types
src/utils/             # Domain constants (single source for unions/options) + helpers
src/i18n/              # en/zh dictionaries, locale context
src-tauri/src/         # groups, projection, document/jsonc, replacement (backups), refs, paths, power/, opencode/, tray, agents, mcp, skills
src-tauri/src/commands/# Tauri IPC handlers, registered in lib.rs
shared/                # Builtin agent catalog JSON imported by the frontend
```

## Conventions

- **Documentation**: Write all repository documentation in English; `README.md` is the canonical README. This does not change the app's supported UI languages.
- **Svelte 5 runes only**: `$state`/`$derived`/`$props`/`$effect`; no Svelte stores.
- **IPC boundary**: UI never calls `invoke` directly. Extend `CommandAdapter` (`src/config/adapter.ts`), read state via the config store (`getConfig()`), and register new handlers in `src-tauri/src/lib.rs`.
- **i18n**: route every user-visible string through keys in `en.ts` and `zh.ts`; keep both dictionaries in sync. Code comments in English.
- **Backend errors**: return `AppError` codes (`not_found`, `references_blocked`, `validation_failed`, `configuration_failed`); the frontend `CommandError` mirrors them.
- **Tests**: `.test.ts` (Vitest + jsdom, `src/test-utils/`) for component behavior, `.test.mjs` (node `--test`) for pure logic; Rust tests live in module `tests.rs` files or `src-tauri/tests/`.
- **Accessibility**: use semantic controls (Bits UI) and preserve keyboard interaction.
- **README Sync**: The README should always be in sync with README.zh-CN.

## Verification

- Frontend: run `npm run check` and the narrowest affected test; run `npm run build` when routing, IPC, or build output changes.
- Rust: run the backend formatting, locked check and strict Clippy commands above; add locked tests when behavior changes. On macOS, validate helper formatting/check/test/Clippy separately. CI also checks the main crate with `cargo +1.90.0 check --manifest-path src-tauri/Cargo.toml --all-targets --locked` and the helper with Rust 1.77.2 check/test; never run the parent with the helper's older toolchain.

## Agent Team

- @doc: Any document drafting or revision work
- @test: Any test-related work
