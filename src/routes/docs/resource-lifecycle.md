# Resource reads, dashboard and tray lifecycle

Source-relative paths below are relative to `src/routes/`, not this docs directory.

## Dashboard and refresh ownership

- **Dashboard status**: Keep runtime browsing/switch dialogs separate from automatic config writes. Translate MCP Proxy status with `mcp.proxy.running`/`mcp.proxy.stopped` in compact wrap-safe text, not numeric-metric typography; failed reads stay unknown. Scope Heatmap assertions to its labelled region because Token Usage may share period captions.

- **List refresh**: Provider/model/agent/group lists call `config.refresh(true)` and guard loading/saving/local refresh; MCP/skill lists reload only their own data and diagnostics. Pagination uses five rows for the former and ten for the latter.

- **Async reads**: MCP/skill lists and details invalidate request sequences on teardown or route changes; preserve these guards so stale responses cannot replace current data.

## Explicit read-only scopes

- **Resource scopes**: Scope controls belong only to MCP/Agent/Skill lists, never chrome/drafts. Exact project reads use saved IDs; scope effects clear old data/search/page/delete state and invalidate stale requests. Agent project/comparison reads never replace config.agents; built-in tabs stay static. Optional Global comparison lazily reads Global, appends scope-keyed rows without merging equal names, and owns independent guards/errors. Project Refresh rereads both visible scopes.

- **Read-only navigation**: MCP/skill details use `?project=<saved ID>` and ignore project edit intent/Global saved banners; retry/refresh preserve scope. Global comparison uses `?readonly=1`; Agent `[id]` is always read-only. Details never change shared scope; new/Agent edit routes stay Global with a short destination notice when a project is selected.

## Tray window

- **Tray mode**: `/tray` bypasses `AppShell`/`SplashScreen`, applies `tray-window`, and disables catalog prefetch. Main listens for `tray:navigate`/`tray:config-changed`; tray layout listens for `app:preferences-changed`.

## Implementation references

| Responsibility | File | Description |
| --- | --- | --- |
| Bootstrap | `+layout.svelte` | Creates config/i18n contexts; disables global initialization for `/island`; registers main/tray listeners and disposes subscriptions. |
| Dashboard | `+page.svelte` | Composes runtime selectors, Quick Actions, System Status, Heatmap, and Token Usage while sharing one activity read between independent periods. |
| Resource scope | `agents/+page.svelte`, `mcp/+page.svelte`, `skills/+page.svelte` | Local ResourceScope uses saved project roots; Global retains CRUD while project and Global comparison entries are read-only. |
| Tray lifecycle | `tray/+page.svelte` | Coalesces focus/show events; refreshes config and health when shown. |
| Cross-route tests | `__tests__/list-refresh.test.ts`, `__tests__/layout-preferences.test.ts` | Cover draft-preserving refresh and window-specific listener cleanup. |

## Independent native windows

Island lifecycle contracts belong to [the Island guide](../island/AGENTS.md), not the main route refresh lifecycle. Settings lifecycle contracts belong to [the settings guide](../settings/AGENTS.md); MCP configuration and Proxy behavior belong to [the MCP guide](../mcp/AGENTS.md).
