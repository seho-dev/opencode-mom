# omo-switch

## Project

- SvelteKit 2 application using Svelte 5, Vite, Tailwind CSS 4, and Bits UI.
- Tauri 2 provides the desktop application shell.

## Commands

- `npm run dev` - start the Vite development server.
- `npm run build` - create a production web build.
- `npm test` - run unit tests with Vitest.
- `npm run test:e2e` - build and run Playwright end-to-end tests.
- `npm run tauri` - run Tauri CLI commands.

## Working Rules

- Follow existing Svelte, TypeScript, and Tailwind patterns in the files you edit.
- Keep changes scoped to the requested behavior; avoid unrelated refactors.
- Use accessible, semantic controls and preserve keyboard interaction where applicable.
- Do not commit secrets, API keys, generated artifacts, or local environment files.

## Verification

- Run the narrowest relevant test or check for each change.
- Run `npm run build` for changes affecting application integration or production output.
- Run `npm test` when changing tested application logic.
- Run `npm run test:e2e` for user-facing workflow changes when appropriate.
