# Settings panels, native lifecycles and listener settings

Source-relative paths below are relative to `src/routes/settings/` unless a reference table names another scope.

## Ownership

`MiniIsland.svelte` owns the independent macOS-only Island enable switch, reads `getIslandState`, writes `setIslandEnabled`, uses native readback, preserves confirmed state on errors, and hides the switch in browser/non-macOS modes. `OpenCodeSettings.svelte` owns background local CLI inspection, explicit stable-version checks and guarded upgrades; settings show CLI updates, not standalone installation details, app updates stay in `AppUpdates.svelte`, and CLI metadata stays outside the global config snapshot. `OpenCodeService.svelte` owns the saved Shared/fixed local V2 target, local connection probes and component-local password drafts; credentials never join the config snapshot or draft recovery. `McpProxySettings.svelte` shares a settings/readback lifecycle between General `http` mode (MOM HTTP port default 18765, allowLan and runtime status via `getMomHttpSettings`/`setMomHttpSettings`) and the revision-guarded MCP Proxy tab (enabled, timeouts, maxUnits and token rotation), never mixing HTTP/proxy write fields; `RotateKeyDialog.svelte` confirms rotation through `rotateMcpProxyKey`, shows the generated token once for copy, keeps it out of global state and clears it on dismissal.

## Retained panels, search and help

- **General sections**: Use four independently labelled sections with headings outside `max-w-3xl` cards and 24px spacing: app behavior (autostart/Mini Island/lid), OpenCode V2 (connection before CLI updates), MOM HTTP, app updates (secondary support). Preserve platform conditions/search anchors/guarded lifecycles/per-section saves; MCP Proxy layout stays independent.

- **Search**: Index only explicit English/Chinese translation keys/technical names, never live DOM/drafts/runtime values. Query edits/clear only update overlaid results, never filter/key/hide/mount/refresh/tear down settings. Selection uses existing tab navigation and scroll/focus on non-action anchors. Keep both panels mounted, inactive hidden, and preserve URL/password/clear-password/HTTP/proxy drafts across tabs without discard prompts. Keep query local, outside URLs/config/recovery/storage; browser-only targets fall back to OpenCode heading, unavailable Island to tab panel. Search stays sticky in settings scroll area.

- **Help copy**: Ordinary help uses `.field-hint`; warning/error banners, credential notices, lid safety and rotation risks remain distinct. Keep copy short/action-oriented and stable `name` attributes on every input.

- **Config boundary**: Use only the typed config facade; never call Tauri `invoke` from the component.

- **Settings tests**: Query visible controls by role/name or `within` the intended tab panel; hidden controls remain mounted. Explicit hidden-panel queries only verify retention/inactive guards; never change mounting to satisfy tests.

## Island enablement and monitor readback

- **Island default**: Keep the switch off by default and disabled until native app info and Island state are confirmed.

- **Shared Service copy**: Keep the V2 Shared Service description explicit, including that enabling may start the service and closing the Island does not interrupt work.

- **Disable safety**: Disabling calls only `setIslandEnabled(false)`; session interruption and request replies are never settings side effects.

- **Target readback**: Successful saves increment facade read-only `serviceTargetVersion`; Island/lid reread native state without remount/toggle. Reject old-version reads/saves, defer latest read through pending work, and never show old-backend protected counts as current while refreshing. Failed saves preserve version/confirmed state.

- **Lid polling**: Clear queued target reload only when a read starts; pending read/save completion drains one latest read, then resumes ordinary 2-second non-overlapping polling. Teardown clears timers/prevents queued reads.

## Active native work and manual app updates

- **Active lifecycle**: Effects track only mounted/active readiness and start async methods through Svelte `untrack`, never querying/loading/pending guards. Inactive sections start no reads/writes and clear polling timers, not pending guards; in-flight work may finish but never start inactive readback/polling. Mini Island retains confirmed platform info/rereads on re-entry; CLI inspection/checked updates persist, while service refresh preserves drafts.

## Route app update integration

- **Settings updates**: Keep both tab panels mounted and child drafts retained; hidden sections start no native work and clear polling timers, not pending operations. Search indexes static bilingual keys and targets non-action focus anchors. `AppUpdates` reads local `getAppInfo` only while active; `checkForUpdates` needs an explicit icon-only click with translated check/pending/retry names and title tooltips. Downloads/optional Stars use `openProjectPage`, never frontend URLs/automatic installation. Show the expandable security guide only for native `platform === 'macos'` with an update; quarantine commands stay selectable, not executable. Guard pending operations and invalidate on teardown.

## HTTP proxy drafts and token visibility

- **MCP Proxy settings**: `?tab=general`/`?tab=mcp-proxy` retain both panels; `hidden`/`active` govern visibility/native work. Autostart/lid/update lifecycle runs only in General; header refresh targets active tab, including General MOM HTTP. Only active HTTP/proxy section polls settings/status every 2 seconds without overlap; deactivation clears timer, teardown also invalidates requests. Failed reads clear runtime status/stale bound address. Validate integer HTTP ports in 1..65535. Display keys once only in proxy-tab `RotateKeyDialog`, never AppState/localStorage, and clear on close; LAN access needs prominent security warnings.

- **MCP Proxy drafts**: Poll live state but replace settings only when draft is clean, preserving edit revision. Saves never overlap reads; success releases saving guard before active-only readback; success/failure resume polling only while active.

- **Token visibility**: Closing the token dialog externally or leaving the proxy tab clears its secret and invalidates pending display results. Reopening never reveals a previous token or retries rotation.

## Cross-route HTTP and Proxy ownership

- **MCP Proxy**: General owns MOM HTTP port/LAN/status; MCP Proxy owns enabled/timeouts/capacity/token rotation. Both reuse `McpProxySettings`; HTTP uses `setMomHttpSettings`, proxy sends only proxy fields plus expected revision. Polling preserves dirty drafts/revision; save/read guards never overlap and polling resumes after writes. Guard panel async completions by mount; preserve literal form command arguments/environment values.

## Implementation references

Paths relative to `src/routes/settings/`:

| Responsibility | File | Description |
| --- | --- | --- |
| Settings page | `+page.svelte` | Retained hidden tab panels ('general' \| 'mcp-proxy'), search and active-gated sections. |
| Settings search | `SettingsSearch.svelte` | Local bilingual static-key index; navigation results without reading drafts. |
| Lid protection | `LidProtection.svelte` | Native lid-protection lifecycle. |
| App updates | `AppUpdates.svelte` | Explicit update checks and project pages. |
| HTTP and MCP Proxy | `McpProxySettings.svelte` | General HTTP / MCP Proxy sections with shared polling and revision guards. |
| Mini Island | `MiniIsland.svelte` | macOS-only Island state and enable readback. |
| Token rotation | `RotateKeyDialog.svelte` | Ephemeral key rotation dialog. |
| Lifecycle coverage | `__tests__/settings-search.test.ts`, `__tests__/rotate-key-dialog.test.ts`, `__tests__/lid-protection.test.ts`, `__tests__/mini-island.test.ts`, `__tests__/opencode-settings.test.ts` | Search, retained drafts, cross-tab focus, one-time tokens, polling, stale reads/saves, visibility, CLI confirmation, safe errors and cross-panel guards. |

Paths relative to `src/routes/`:

| Responsibility | File | Description |
| --- | --- | --- |
| Power status | `settings/LidProtection.svelte` | Polls every two seconds without overlapping reads; pauses during writes. |
| Manual updates | `settings/AppUpdates.svelte` | Reads local app info; checks releases only on click and opens official project pages through config. |
| Mini Island | `island/+page.svelte`, `settings/MiniIsland.svelte` | Local V2 service interaction window and independent macOS-only enable switch. |
