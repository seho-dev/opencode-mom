# App Shell

## Overview

Provides persistent application chrome, shared desktop/mobile navigation, appearance controls, configuration feedback, and the loading overlay. Route content is rendered through `AppShell` rather than owned here; `src/routes/+layout.svelte` supplies config/i18n contexts and mounts the shell and splash only outside `/tray`.

## Directory Structure

```text
shell/                              # Application chrome and global feedback
├── AppShell.svelte                 # Sidebar, header, route content, feedback, and toaster
├── AppHeader.svelte                # Mobile navigation and appearance controls
├── AppSidebar.svelte               # Desktop links, version, and OpenCode reload
├── MobileNavigation.svelte         # Modal drawer, focus handling, and navigation
├── navigation.ts                   # Shared route entries and active-route matching
├── AppearanceControls.svelte       # Refresh, theme toggle, and locale buttons
├── ConfigFeedback.svelte           # Config errors/notices translated into toasts
├── toast.svelte.ts                 # Reactive toast queue and timed dismissal
├── Toaster.svelte                  # Keyed toast rendering and exit state
├── SplashScreen.svelte             # Reusable loading overlay with minimum display time
└── __tests__/                      # Context-backed shell component tests
    ├── appearance-controls.test.ts # Translated appearance control labels
    └── config-feedback.test.ts     # Error-specific recovery callbacks
```

## Key Files

| Responsibility | File | Description |
| :--- | :--- | :--- |
| Composition | `AppShell.svelte` | Renders the `children` snippet inside `main.content`; mounts feedback and toaster once. |
| Navigation model | `navigation.ts` | Defines `href`, `labelKey`, and icon entries plus `isNavigationItemActive`. |
| Desktop chrome | `AppHeader.svelte`, `AppSidebar.svelte` | Compose header controls and desktop navigation/reload. |
| Mobile navigation | `MobileNavigation.svelte` | Handles drawer focus, Escape/Tab, `goto`, and reload. |
| Preferences | `AppearanceControls.svelte` | Reads preferences and calls `setTheme`, `setLocale`, and `refreshAll`. |
| Command feedback | `ConfigFeedback.svelte` | Maps errors to recovery actions and clears consumed success notices. |
| Toast lifecycle | `toast.svelte.ts`, `Toaster.svelte` | Expose `toast`, `dismissToast`, and `getToasts`; render entries by ID. |
| Loading overlay | `SplashScreen.svelte` | Uses the `visible` prop and `.splash-exit` without unmounting. |

## Conventions

- **Shared navigation**: Change links in `navigationItems`; both navigation components use `isNavigationItemActive` for `aria-current`. `/` matches exactly; other entries match the route or a slash-delimited descendant.
- **Shell composition**: Keep `ConfigFeedback` and `Toaster` at shell scope, not inside individual pages; the route layout owns the separate `SplashScreen` instance.
- **Drawer focus**: Opening focuses the close button after `tick`; explicit closing restores menu-button focus. Escape closes and Tab/Shift+Tab cycle the close button, links, and enabled reload button.
- **Navigation completion**: Drawer links await `goto(href)` before closing; drawer reload closes only after `reloadOpencode` succeeds.
- **Control guards**: Disable OpenCode reload during `reloading` or `switching`; disable refresh-all during `splashLoading` or `loading`. Theme and locale changes go through config methods.
- **Recovery actions**: Preserve three conflict actions (`reloadKeepingDraft`, `continueEditing`, `discardDraftAndRefresh`); busy, blocked-reference, and fallback errors offer `continueEditing` with branch-specific descriptions.
- **Success notices**: `switchGroup` maps to `toast.groupSwitched`, operations starting with `delete` to `toast.deleted`, and other notices to `toast.saved`; clear the notice after enqueueing.
- **Toast lifecycle**: Use `toast(options)` and `dismissToast(id)` instead of mutating the queue. `actions` takes precedence over `action`; defaults are 5000 ms, 8000 ms for errors, then a 250 ms leaving state before removal.
- **Splash lifecycle**: Keep the overlay mounted so reload can show it again; preserve the 3000 ms minimum, timer cleanup, and global `.splash-exit` rule that disables pointer interception.
- **Component tests**: Render with `Harness` and `wrapperProps: { config }`; appearance assertions use translated accessible roles. Feedback tests hoist/mock `toast`, change a `SvelteMap`-backed error, await `tick`, and invoke captured callbacks.

## Sync

This file is maintained as the directory scope changes. When files in this directory are added, modified, or deleted (components, logic units, utilities, constants, etc.), update this file's directory structure, key files, and conventions to keep the documentation in sync with the code.
