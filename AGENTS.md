# opencode-mom

## Overview

Cross-platform desktop app for switching Oh My OpenAgent model groups. It stores named groups (native / slim / OMO) plus provider, agent, MCP, and skill metadata in its own config, then merges the selected group into target application configs. Tauri 2 is the shell; the Svelte 5 SPA is the management console; the Rust backend owns config persistence, projection, backups, and macOS/Windows power behavior. README.md is the authoritative user-facing behavior reference.

## Stack

- Frontend: SvelteKit 2, Svelte 5 (runes), TypeScript strict, Tailwind CSS 4, Bits UI, Vitest, Biome.
- Backend: Tauri 2, Rust edition 2021 (`rust-version = 1.77.2`; CI/release pin 1.96.0).
- Toolchain: Node 22.23.0 and npm. SvelteKit uses the static adapter with `ssr = false` (SPA fallback `index.html`).

## Commands

- `npm run dev` / `npm run build` - Vite dev server / production web build.
- `npm run check` / `npm run check:fix` - Biome format + lint (2-space, single quotes, semicolons, trailing commas, 120 cols).
- `npm run test:components` - Vitest component tests (`src/**/__tests__/*.test.ts`, jsdom).
- `npm run test:unit` - `node --test` unit tests (`src/**/__tests__/*.test.mjs`).
- `cargo fmt|check|test --manifest-path src-tauri/Cargo.toml` - Rust backend (fmt uses `--all -- --check`).
- `npm run tauri dev` / `npm run tauri build` - run / package the native app.
- macOS helper is a separate locked workspace: `cargo fmt|check|test --manifest-path src-tauri/native/macos-helper/Cargo.toml --offline --locked`; on a fresh cache run `cargo fetch --manifest-path src-tauri/native/macos-helper/Cargo.toml --locked` first.

## Layout

```
src/routes/            # Pages: / (dashboard), providers, models, agents, groups, mcp, skills, settings, tray popup
src/config/            # CommandAdapter IPC boundary (adapter.ts) + runes config store (store.svelte.ts, context.ts)
src/shell/             # App chrome: shell, sidebar, header, mobile nav, splash, toasts
src/components/        # Bits UI wrappers + shared components (button, dialog, combobox, table, ...)
src/types/             # Shared wire and domain types
src/utils/             # Domain constants (single source for unions/options) + helpers
src/i18n/              # en/zh dictionaries, locale context
src-tauri/src/         # groups, projection, document/jsonc, replacement (backups), refs, paths, power/, opencode_cli/, tray, agents, mcp, skills
src-tauri/src/commands/# Tauri IPC handlers, registered in lib.rs
shared/                # Builtin agent catalog JSON imported by the frontend
```

## Managed Config Paths

- App data: `~/.config/opencode-mom/config.json` (groups, selection, write metadata, backups).
- Targets: `~/.omo/omo.jsonc`, `~/.config/opencode/oh-my-opencode-slim.json`, `~/.config/opencode/opencode.jsonc` (fallback `opencode.json`); `OPENCODE_CONFIG_DIR` and `OPENCODE_CONFIG` override discovery.
- Writes are one-to-one key patches through `jsonc`/`document` helpers: preserve residual keys and remove only keys owned by the changed group. Plain config writes have no backups or locking; recovery `.bak` files exist only for MCP/skill replacement via `resource_file::replace`.

## Conventions

- **Svelte 5 runes only**: `$state`/`$derived`/`$props`/`$effect`; no Svelte stores.
- **IPC boundary**: UI never calls `invoke` directly. Extend `CommandAdapter` (`src/config/adapter.ts`), read state via the config store (`getConfig()`), and register new handlers in `src-tauri/src/lib.rs`.
- **Wire compatibility**: persisted `group.type` values are frozen (`opencode` / `slim` / `oh-my-openagent`); convert to canonical types at the adapter boundary only.
- **Imports**: use the `$src/*` alias with explicit `.js` extensions; there is no `src/lib`.
- **i18n**: route every user-visible string through keys in `en.ts` and `zh.ts`; keep both dictionaries in sync. Code comments in English.
- **Backend errors**: return `AppError` codes (`not_found`, `references_blocked`, `validation_failed`, `configuration_failed`); the frontend `CommandError` mirrors them.
- **Tests**: `.test.ts` (Vitest + jsdom, `src/test-utils/`) for component behavior, `.test.mjs` (node `--test`) for pure logic; Rust tests live in module `tests.rs` files or `src-tauri/tests/`.
- **Accessibility**: use semantic controls (Bits UI) and preserve keyboard interaction.

## Verification

- Frontend: run `npm run check` and the narrowest affected test; run `npm run build` when routing, IPC, or build output changes.
- Rust: run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` and `cargo check`; add `cargo test` when behavior changes.
- Never edit generated output: `build/`, `.svelte-kit/`, `src-tauri/target/`, `src-tauri/gen/`.

## Safety

- Lid protection changes real system power policy; automated tests and readback do not replace manual lid-close acceptance (README). Do not run the macOS helper with `sudo`; its tests use mock backends.
- Do not commit secrets, API keys, or local environment files.
