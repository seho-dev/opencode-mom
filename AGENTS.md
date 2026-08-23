# opencode-mom

## Project

- SvelteKit 2 application using Svelte 5, Vite, Tailwind CSS 4, and Bits UI.
- Tauri 2 provides the desktop application shell.

## Commands

- `npm run dev` - start the Vite development server.
- `npm run build` - create a production web build.
- `npm run format` - format frontend code with Prettier.
- `npm run format:check` - verify frontend formatting.
- `cargo check --manifest-path src-tauri/Cargo.toml` - type-check the Rust backend.
- `npm run tauri` - run Tauri CLI commands.

## Working Rules

- Follow existing Svelte, TypeScript, and Tailwind patterns in the files you edit.
- Keep changes scoped to the requested behavior; avoid unrelated refactors.
- Run Prettier (`npm run format`) on changed frontend files; keep all UI text and code comments in English.
- Use accessible, semantic controls and preserve keyboard interaction where applicable.
- Do not commit secrets, API keys, generated artifacts, or local environment files.

## Verification

- Run the narrowest relevant check for each change.
- Run `npm run build` for changes affecting application integration or production output.
- Run `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check` and `cargo check --manifest-path src-tauri/Cargo.toml` for Rust changes.
