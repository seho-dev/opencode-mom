# Shell Interaction and Feedback Contracts

## Composition and navigation

`AppShell.svelte` renders the `children` snippet inside `main.content` and mounts feedback and toaster
once. `AppHeader.svelte` and `AppSidebar.svelte` compose header controls, the default-browser GitHub
opener, and desktop navigation. `AppearanceControls.svelte`'s leading icon restarts OpenCode (not a
config refresh); Reload OpenCode as a Dashboard action and manual dashboard refresh are gone —
the Dashboard refreshes on entry. `navigation.ts` defines `href`, `labelKey`, and icon entries.

- **Shared navigation**: Change links in `navigationItems`; both navigation components use `isNavigationItemActive` for `aria-current`. `/` matches exactly; other entries match the route or a slash-delimited descendant.
- **Drawer focus**: Opening focuses the close button after `tick`; explicit closing restores menu-button focus. Escape closes and Tab/Shift+Tab cycle the close button and links.
- **Navigation completion**: Drawer links await `goto(href)` before closing.
- **Control guards**: The header restart button confirms first (it can cancel pending requests) and is disabled during `reloading`, `switching`, or while its confirmation is open; the sidebar update hint is disabled while its open request is in flight. Theme and locale changes go through config methods.
- **GitHub opener**: Use `config.openProjectPage('repository')` in the header with translated accessible name/title and an appearance-sized icon control; guard pending opens, toast failures through the shared error toast, and invalidate late responses on teardown.

`AppearanceControls.svelte` reads preferences and calls `setTheme`, `setLocale`, and `refreshAll`.

## Command feedback and overlays

`ConfigFeedback.svelte` maps errors to recovery actions and clears consumed success notices.
`toast.svelte.ts` and `Toaster.svelte` expose `toast`, `dismissToast`, and `getToasts`, rendering entries
by ID. `SplashScreen.svelte` uses the `visible` prop and `.splash-exit` without unmounting.

- **Recovery actions**: Preserve three conflict actions (`reloadKeepingDraft`, `continueEditing`, `discardDraftAndRefresh`); busy, blocked-reference, and fallback errors offer `continueEditing` with branch-specific descriptions.
- **Success notices**: `switchGroup` maps to `toast.groupSwitched`, operations starting with `delete` to `toast.deleted`, and other notices to `toast.saved`; clear the notice after enqueueing.
- **Toast lifecycle**: Use `toast(options)`/`dismissToast(id)`, not queue mutation; `actions` overrides `action`. Defaults: 5000 ms, 8000 ms for errors, then 250 ms leaving before removal.
- **Splash lifecycle**: Keep the overlay mounted so reload can show it again; preserve the 3000 ms minimum, timer cleanup, and global `.splash-exit` rule that disables pointer interception.

## Interaction regression tests

- **Component tests**: Render with `Harness`, `wrapperProps: { config }`, and translated accessible-role appearance assertions. Feedback tests hoist/mock `toast`, change a `SvelteMap`-backed error, await `tick`, and invoke captured callbacks.
