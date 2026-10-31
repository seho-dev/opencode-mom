# App Shell

## Overview

Provides persistent application chrome, desktop/mobile navigation, appearance controls, feedback,
and the loading overlay. `AppShell` renders route content; `src/routes/+layout.svelte` supplies
config/i18n contexts and mounts shell and splash only outside `/tray`.

## Directory Structure

```text
shell/
├── AppShell.svelte            # Chrome and route composition
├── AppHeader.svelte           # Mobile menu, appearance, GitHub
├── AppSidebar.svelte          # Desktop links, version, reload
├── MobileNavigation.svelte    # Focus-managed navigation drawer
├── navigation.ts              # Shared routes and active matching
├── AppearanceControls.svelte  # Refresh, theme, locale
├── ConfigFeedback.svelte      # Errors and notices to toasts
├── toast.svelte.ts            # Reactive queue and dismissal
├── Toaster.svelte             # Keyed rendering and exit state
├── SplashScreen.svelte        # Reusable loading overlay
└── __tests__/                 # Context-backed shell coverage
```

## Key Files

| Responsibility | File | Description |
| :--- | :--- | :--- |
| Composition | `AppShell.svelte` | Owns route-content framing and shared feedback mounting. |
| Routing | `navigation.ts`, `MobileNavigation.svelte` | Change route definitions and drawer behavior together. |
| Preferences | `AppearanceControls.svelte` | Changes theme/locale and requests refresh through config. |
| Feedback | `ConfigFeedback.svelte`, `toast.svelte.ts`, `Toaster.svelte` | Connects command recovery and notices to the shared toast queue. |

## Conventions

- **Shared navigation**: Change routes through `navigationItems`; both navigation components use
  `isNavigationItemActive` for `aria-current`. Preserve exact-root and slash-delimited descendant matching.
- **Build version**: Sidebar and mobile drawer import `version` from root `package.json` and interpolate
  `header.version`; no native request is needed.
- **Shell composition**: Keep `ConfigFeedback` and `Toaster` at shell scope, not inside individual pages;
  the route layout owns the separate `SplashScreen` instance.
- **Control guards**: Disable reload and refresh during their corresponding busy states; change theme
  and locale through config methods. See the linked interaction contracts for exact guards.
- **Accessible interactions**: Preserve drawer focus restoration, keyboard trapping, and navigation
  completion; guard duplicate GitHub opens, toast failures, and invalidate late responses on teardown.
- **Feedback ownership**: Preserve conflict-specific recovery actions and consume success notices once;
  use the toast API rather than mutating the reactive queue.
- **Component tests**: Render with `Harness` and `wrapperProps: { config }`; assert translated accessible
  roles and invoke captured recovery callbacks in feedback tests.

## Details

| Section | File |
| --- | --- |
| Interaction and feedback contracts | [docs/interaction-lifecycle.md](docs/interaction-lifecycle.md) |

## Sync

Keep this guide and linked docs in sync with changes to this directory.
