# Settings

## Overview

`MiniIsland.svelte` owns the independent macOS-only Island enable switch. It reads `getIslandState`, writes `setIslandEnabled`, uses native readback, preserves confirmed state on errors, and does not expose the switch in browser or non-macOS modes.

`OpenCodeSettings.svelte` owns background local CLI inspection, explicit stable-version checks, and guarded upgrades. Settings show only CLI updates, not a standalone installation-details panel. App updates remain in `AppUpdates.svelte`; CLI metadata is not part of the global config snapshot.

`OpenCodeService.svelte` owns the saved Shared/fixed local V2 target, local connection probes, and component-local password drafts. Service credentials never join the config snapshot or draft recovery.

`McpProxySettings.svelte` reuses one settings/readback lifecycle for two sections: `http` mode in General owns MOM HTTP port (default 18765), allowLan, and runtime status through `getMomHttpSettings`/`setMomHttpSettings`; the MCP Proxy tab owns enabled, timeouts, maxUnits, and token rotation through the revision-guarded proxy facade. HTTP writes never submit proxy fields, and proxy writes never submit listener settings.

`RotateKeyDialog.svelte` confirms token rotation, calls `rotateMcpProxyKey`, and shows the generated token once for copy. The token is never stored in global state and is cleared on dialog dismissal.

## Directory Structure

```text
settings/
├── +page.svelte                   # Retained hidden tab panels ('general' | 'mcp-proxy'), search and active-gated sections
├── SettingsSearch.svelte          # Local bilingual static-key index; navigation results without reading drafts
├── LidProtection.svelte           # Native lid-protection lifecycle
├── AppUpdates.svelte              # Explicit update checks and project pages
├── McpProxySettings.svelte        # General HTTP / MCP Proxy sections with shared polling and revision guards
├── MiniIsland.svelte              # macOS-only Island state and enable readback
├── OpenCodeSettings.svelte        # Local CLI inspection, explicit checks, upgrade confirmation/readback
├── OpenCodeService.svelte         # Saved target, read-only connection probes, secret-local drafts
├── RotateKeyDialog.svelte         # Ephemeral key rotation dialog
└── __tests__/                     # Settings lifecycle coverage
    ├── settings-search.test.ts    # Bilingual lookup, draft retention, cross-tab focus, lifecycle and polling safety
    ├── rotate-key-dialog.test.ts  # External close and tab changes discard one-time token results
    ├── lid-protection.test.ts     # Deferred target reloads, 2s fake-timer polling, stale read/save and teardown
    ├── mini-island.test.ts        # Visibility, pending guards, readback, errors, teardown
    └── opencode-settings.test.ts  # Native reads, CLI confirmation, target drafts, safe errors and cross-panel guards
```

## Conventions

- General uses four independently labelled sections with headings outside `max-w-3xl` cards and 24px section spacing: app behavior (autostart, Mini Island, lid protection), OpenCode V2 (service connection before CLI updates), MOM HTTP, and app updates (support remains secondary). Keep existing platform conditions, search anchors, guarded lifecycles, and per-section save semantics; the MCP Proxy tab layout is independent.
- Search indexes only explicitly listed English/Chinese translation keys and technical key names, never live DOM text, URL/password/proxy drafts, or runtime values. Query edits and clearing update only the overlaid result list: do not filter, key, hide, mount, refresh, or tear down native settings. Selecting a result uses existing tab navigation and scrolls/focuses a non-action anchor. Both tab panels stay mounted and inactive panels are hidden; switching tabs preserves service URL/password/clear-password and HTTP/proxy drafts without discard confirmation. Keep the query local and out of URLs, config, recovery, and storage. Browser-only sections fall back to the OpenCode heading; unavailable Island results fall back to the tab panel. Keep the search bar sticky within the settings scroll area.
- Ordinary setting help uses shared `.field-hint`; warning/error banners, credential notices, lid safety guidance, and token-rotation risks remain distinct. Keep help copy short and action-oriented; all inputs have stable `name` attributes.
- Keep the switch off by default and disabled until native app info and Island state are confirmed.
- Use only the typed config facade; never call Tauri `invoke` from the component.
- Keep the V2 Shared Service description explicit, including that enabling may start the service and closing the Island does not interrupt work.
- Disabling calls only `setIslandEnabled(false)`; session interruption and request replies are never settings side effects.
- Successful target saves increment the config facade's read-only `serviceTargetVersion`; Island and lid sections reread native state without remounting or toggling. Reject old-version read/save results, defer a latest read through pending operations, and never treat an old backend's protected count as current while refreshing. Failed target saves leave the version and confirmed states unchanged.
- Lid status clears its queued target reload only when a read actually starts; pending read/save completion drains one latest read, then ordinary polling resumes after two seconds without overlapping reads. Teardown clears timers and prevents queued reads.
- Active lifecycle effects track only mounted/active readiness and start async methods through Svelte `untrack`, never their querying/loading/pending guards. Inactive sections start no reads or writes and clear polling timers without clearing pending guards; in-flight work may finish but cannot start inactive readback or polling. Mini Island remembers confirmed platform information and rereads status on re-entry; CLI inspection and checked updates persist, while the service refreshes without replacing drafts.
- Settings page tests query visible controls by role/name or use `within` on the intended tab panel. Hidden controls remain in the DOM; use explicit hidden-panel queries only to verify retention or inactive guards, never change product mounting to satisfy a query.
- OpenCode CLI inspection first confirms native app info; browser mode makes no CLI or update calls. Mount and refresh are local reads only; release checks require a click.
- App update check/retry, CLI refresh/check, inspection retry, and connection refresh are icon-only buttons with translated accessible names and title tooltips. CLI refresh/check share the updates header; installation path/current version appear only in upgrade confirmation.
- Only checked, upgradeable installations expose Upgrade. The Bits UI confirmation snapshots the exact path, current version, and target version for the guarded command.
- Pending CLI operations disable connection controls; pending service reads/saves disable CLI actions and prevent upgrade confirmation. Invalidate reads on teardown; no late readback, retries, or toasts. CLI errors retain confirmed details and require explicit refresh before retrying an upgrade.
- A successful upgrade rereads CLI installation and service status once while active, without global refresh or service restart. Completion while inactive retains the refresh-required state without starting readback. Keep CLI update feedback separate from backend version. Unsupported installations show no metadata or manual command; only an explicit available-update result adds a short manual-update hint. CLI errors show translated hints without raw backend messages.
- Service mount and explicit Refresh probe only the confirmed target without bootstrap; refresh never overwrites URL/password/clear-password drafts. Failed reads cannot show stale Connected/PID/version. Failed saves preserve confirmed target and local drafts; successful offline saves commit the returned normalized target.
- Password inputs never read back a secret and are cleared on successful save and teardown. Blank omits the password argument, explicit clearing sends an empty string, Shared sends null URL. Describe direct V2 loopback APIs and Basic backend passwords, not OpenChamber UI URLs/passwords or Bearer fallback.
- Service errors render only translated generic or `busy`/`validation_failed` action hints; never render raw message/detail/status.error or log credentials. Busy asks users to wait for the ongoing operation and retry; validation points only to the direct HTTP loopback URL, positive port, and backend password. Do not ask users to disable Mini Island or lid protection before changing targets; the backend owns monitor refresh and input validation, with enabled preferences preserved. Settings never toggles monitors or power policy to save a target. Credential copy states plaintext/non-encrypted storage and does not claim Windows ACL validation before native verification.
- **MCP Proxy settings**: Tab switching via `?tab=general` and `?tab=mcp-proxy` keeps both panels mounted, with `hidden` and `active` controlling visibility and native work. Autostart/lid/update lifecycle only runs on the general tab. Header refresh acts only on the active tab, including MOM HTTP in General. Only the active HTTP or proxy section polls settings/status every 2 seconds without overlapping queries; deactivation clears its timer only, while teardown also invalidates requests. Failed reads clear runtime status rather than retaining a stale bound address. HTTP ports must be integers in 1..65535 before saving. Secret keys are displayed only once in `RotateKeyDialog` on the proxy tab, never stored in AppState or localStorage, and cleared on close. LAN access displays prominent security warnings.
- **MCP Proxy drafts**: Polling updates live state but replaces settings only while the local draft is clean, preserving its revision while editing. Saves do not overlap reads; successful saves release the saving guard before active-only readback, and both success and failure resume polling only while active.
- **Token visibility**: Closing the token dialog externally or leaving the proxy tab clears its secret and invalidates pending display results. Reopening never reveals a previous token or retries rotation.

## Sync

Update this file when settings ownership, CLI lifecycle, or the macOS Island switch lifecycle changes.
