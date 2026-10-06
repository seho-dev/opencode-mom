# Test Utils

## Overview

Provides the shared component-test bootstrap and a wrapper that supplies config and translation contexts without mounting the application layout. It accepts a caller-provided ConfigStore; mocks, fixtures, and tested components remain in their own test directories.

## Directory Structure

```text
test-utils/                   # Shared component-test setup and context wrapper
├── AGENTS.md                 # Directory-specific guidance
├── harness.svelte            # ConfigStore and i18n context provider
└── setup.ts                  # Testing Library's Svelte Vitest integration
```

## Key Files

| Responsibility | File | Description |
| :--- | :--- | :--- |
| Test bootstrap | `setup.ts` | Imports `@testing-library/svelte/vitest`; loaded by root `vitest.config.ts`. |
| Context wrapper | `harness.svelte` | Accepts `config` and a children Snippet, installs both contexts, then renders children. |

## Conventions

- **Vitest wiring**: Root `vitest.config.ts` loads `setup.ts` via `setupFiles`, uses jsdom and browser resolution, and includes `src/**/__tests__/*.test.ts`; the harness is imported explicitly by tests, not registered in config.
- **Render wrapper**: Import `Harness` from `harness.svelte` and use `render(Component, props, { wrapper: Harness, wrapperProps: { config } })` for context-dependent components.
- **Caller-owned config**: Supply the ConfigStore or test double through `wrapperProps.config`, including `preferences.locale` and the fields/methods read by the tested component; the harness creates no store or adapter.
- **Context initialization**: Keep `setConfig(config)` and `setI18n(createI18n(() => config.preferences.locale))` before rendering the children Snippet so descendants receive both contexts.
- **Locale behavior**: Drive translations through the supplied config preference rather than a fixed harness locale; the shell's `appearance-controls.test.ts` exercises the wrapper with Chinese labels.

## Sync

This file is maintained as the directory scope changes. When files in this directory are added, modified, or deleted (components, logic units, utilities, constants, etc.), update this file's directory structure, key files, and conventions to keep the documentation in sync with the code.
