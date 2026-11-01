# Settings panels, native lifecycles and listener settings

Source-relative paths below are relative to `src/routes/settings/` unless a reference table names another scope.

## Ownership

`MiniIsland.svelte` owns the independent macOS-only Island enable switch, reads `getIslandState`, writes `setIslandEnabled`, uses native readback, preserves confirmed state on errors, and hides the switch in browser/non-macOS modes. `OpenCodeSettings.svelte` owns background local CLI inspection, explicit stable-version checks and guarded upgrades; settings show CLI updates, not standalone installation details, app updates stay in `AppUpdates.svelte`, and CLI metadata stays outside the global config snapshot. `OpenCodeService.svelte` owns the saved Shared/fixed local V2 target, local connection probes and component-local password drafts; credentials never join the config snapshot or draft recovery.

General's `http=true` branch of `McpProxySettings.svelte` owns MOM HTTP port (default 18765), allowLan and shared token rotation. It reads only `getMomHttpSettings`, writes listener settings through `setMomHttpSettings` and rotates the token through no-argument `rotateMomHttpKey()`. The MCP Proxy tab owns only revision-guarded enabled, timeouts and maxUnits settings; HTTP/proxy write fields remain separate. General's token controls do not depend on Proxy availability or state. Its `RotateKeyDialog.svelte` shows the generated token once for copy, keeps it out of global state and clears it on dismissal or deactivation.

`Behavior.svelte` edits the Global `AGENTS.md` independently of Project selection. It reuses `PromptGenerator.svelte` with kind `behavior`: generation receives the current draft, confirmed previews replace only that draft, and saving remains explicit with the original source snapshot. Inactive panels, pending reads/saves and teardown prevent preview application; model options load only when the dialog opens.

## Diagnostic recording and history

`LoggingSettings.svelte` owns General's independent logging switch and confirmed backend state. Reads run only on active entry, manual refresh and explicit changes, never on timers or in hidden panels. Preserve the last confirmed switch on a failed save, guard duplicate writes and ignore responses after teardown. An enabled preference is not proof that the writer is healthy; show recording failures separately.

`/logs` is an independent navigation page. Keep the viewer, type/status/time filters, operation details, directory opening and confirmed deletion there rather than embedding them in Settings. Disabled recording does not hide or erase history. History and detail requests are bounded, manually refreshed, and reject late results after filter changes, selection changes, clear or unmount.

## Retained panels, search and help

- **General sections**: Use four independently labelled sections with headings outside `max-w-3xl` cards and 24px spacing: app behavior (autostart/Mini Island/lid), OpenCode V2 (connection before CLI updates), MOM HTTP, app updates (secondary support). Preserve platform conditions/search anchors/guarded lifecycles/per-section saves; MCP Proxy layout stays independent.

- **Search**: Index only explicit English/Chinese translation keys/technical names, never live DOM/drafts/runtime values. Query edits/clear only update overlaid results, never filter/key/hide/mount/refresh/tear down settings. Selection uses existing tab navigation and scroll/focus on non-action anchors. Keep tab panels mounted, inactive hidden, and preserve URL/password/clear-password/HTTP/proxy/behavior drafts across tabs without discard prompts. Keep query local, outside URLs/config/recovery/storage; browser-only targets fall back to OpenCode heading, unavailable Island to tab panel. Search stays sticky in settings scroll area.

- **Help copy**: Ordinary help uses `.field-hint`; warning/error banners, credential notices, lid safety and rotation risks remain distinct. Keep copy short/action-oriented and stable `name` attributes on every input.

- **Config boundary**: Use only the typed config facade; never call Tauri `invoke` from the component.

- **Settings tests**: Query visible controls by role/name or `within` the intended tab panel; hidden controls remain mounted. Explicit hidden-panel queries only verify retention/inactive guards; never change mounting to satisfy tests.

## Island enablement and monitor readback

- **Island default**: Keep the switch off by default and disabled until native app info and Island state are confirmed.

- **Shared Service copy**: Keep the V2 Shared Service description explicit, including that enabling may start the service and closing the Island does not interrupt work.

- **Disable safety**: Disabling calls only `setIslandEnabled(false)`; session interruption and request replies are never settings side effects.

- **Target readback**: Successful saves increment facade read-only `serviceTargetVersion`; Island rereads native state and lid rereads its enabled preference without remount/toggle. Reject old-version reads/saves and defer one latest-target read through pending work. Failed target saves preserve version and confirmed preferences.

- **Lid preference**: Render one native checkbox styled as a switch, a short description, visible administrator-authorization and heat/closed-bag notices, and concise localized read/save errors. The checkbox displays only returned `enabled`, not whether protection is active; the static authorization notice explains that macOS requires administrator authorization each app run and users must toggle off then on after restarting MOM to authorize. Never display phase/session state or raw backend errors, and provide no status, disclosure, refresh/retry/off actions or frontend timers/polling. Read results ignore phase/error for display; explicit writes adopt returned `enabled` even when phase/error or an enabled mismatch requires the generic save error.

- **Lid lifecycle**: Read on active mount/re-entry and successful target changes only. Disable the switch until the current preference is known and during reads/writes; preserve native keyboard interaction and focus after writes. Clear queued target reload only when a read starts; pending read/save completion releases its guard and drains one latest read only while active. Inactive panels retain pending guards and queued reads without starting work; teardown invalidates requests and prevents queued reads.

## Active native work and manual app updates

- **Active lifecycle**: Effects track only mounted/active readiness and start async methods through Svelte `untrack`, never querying/loading/pending guards. Inactive sections start no reads/writes and retain pending guards and queued refreshes; in-flight work may finish but never start inactive readback. HTTP and Proxy use no polling timers. Mini Island retains confirmed platform info/rereads on re-entry; CLI inspection/checked updates persist, while service refresh preserves drafts.

## Route app update integration

- **Settings updates**: Keep tab panels mounted and child drafts retained; hidden sections start no native work and preserve pending operations. Search indexes static bilingual keys and targets non-action focus anchors. `AppUpdates` reads local `getAppInfo` only while active; `checkForUpdates` needs an explicit icon-only click with translated check/pending/retry names and title tooltips. Downloads/optional Stars use `openProjectPage`, never frontend URLs/automatic installation. Show the expandable security guide only for native `platform === 'macos'` with an update; quarantine commands stay selectable, not executable. Guard pending operations and invalidate on teardown.

## HTTP proxy drafts and token visibility

- **HTTP and MCP Proxy settings**: `?tab=general`/`?tab=mcp-proxy` retain both panels; `hidden`/`active` govern visibility/native work. Autostart/lid/update lifecycle runs only in General; header refresh targets the active tab, including General MOM HTTP. Read HTTP settings and Proxy settings/status only on active entry, manual refresh and post-operation readback; never poll or schedule refresh timers. Deactivation preserves pending guards and queued refreshes without starting native work; teardown invalidates requests. Failed reads clear runtime status/stale bound address. Validate integer HTTP ports in 1..65535. Display the shared MOM HTTP token once only in General's `http=true` `RotateKeyDialog`, never AppState/localStorage, and clear on close or deactivation; LAN access needs prominent security warnings.

- **MCP Proxy drafts**: Explicit readback replaces settings only when the draft is clean, preserving dirty inputs and their edit revision. Reads and saves never overlap. Successful saves release the saving guard before active-only readback. Retain pending guards and queued post-operation refreshes across tab changes; drain queued work only while active and idle. Inactive completion never starts readback.

- **Token visibility**: Closing the token dialog externally or leaving General clears its secret, copied state and timer, and invalidates pending rotation/clipboard display results. Reopening never reveals a previous token or retries rotation. Closing a successfully rotated token queues only `getMomHttpSettings` readback, preserves dirty port/LAN drafts and never writes Proxy settings or refreshes global state. Rotation invalidates the old shared MOM HTTP token and terminates active client connections, including MCP sessions.

## Cross-route HTTP and Proxy ownership

- **MOM HTTP and MCP Proxy**: General owns MOM HTTP port/LAN settings and shared token rotation; MCP Proxy owns enabled/timeouts/capacity. Both reuse `McpProxySettings`; HTTP uses `setMomHttpSettings` and no-argument `rotateMomHttpKey`, while Proxy sends only Proxy fields plus expected revision. General reads only `getMomHttpSettings`; Proxy settings/status reads, including the MCP route's `ProxyPanel.svelte`, remain independent. Use entry/manual/post-operation readback without polling. Preserve dirty drafts, revision snapshots, non-overlapping read/write guards and queued refreshes. Guard panel async completions by mount; preserve literal form command arguments/environment values.

## Implementation references

Paths relative to `src/routes/settings/`:

| Responsibility | File | Description |
| --- | --- | --- |
| Settings page | `+page.svelte` | Retained hidden General, MCP Proxy and Behavior panels; search and active-gated sections. |
| Global behavior | `Behavior.svelte`, `__tests__/behavior.test.ts` | Global document snapshots, shared prompt generation, explicit saves and inactive/pending guards. |
| Settings search | `SettingsSearch.svelte` | Local bilingual static-key index; navigation results without reading drafts. |
| Lid preference | `LidProtection.svelte` | Guarded native preference switch and concise authorization/safety copy; no polling or runtime status. |
| App updates | `AppUpdates.svelte` | Explicit update checks and project pages. |
| HTTP and MCP Proxy | `McpProxySettings.svelte` | General HTTP / MCP Proxy sections with entry/manual/post-operation readback and revision guards; no polling. |
| Mini Island | `MiniIsland.svelte` | macOS-only Island state and enable readback. |
| Token rotation | `RotateKeyDialog.svelte` | Ephemeral key rotation dialog. |
| Lifecycle coverage | `__tests__/settings-search.test.ts`, `__tests__/rotate-key-dialog.test.ts`, `__tests__/lid-protection.test.ts`, `__tests__/mini-island.test.ts`, `__tests__/opencode-settings.test.ts` | Search, retained drafts, cross-tab focus, one-time tokens, HTTP/Proxy explicit readback without polling, lid preference readback without polling, stale reads/saves, visibility, CLI confirmation, safe errors and cross-panel guards. |

Paths relative to `src/routes/`:

| Responsibility | File | Description |
| --- | --- | --- |
| Lid preference | `settings/LidProtection.svelte` | Reads on active entry and target changes; serializes pending reads/writes and rejects stale results. |
| Manual updates | `settings/AppUpdates.svelte` | Reads local app info; checks releases only on click and opens official project pages through config. |
| Mini Island | `island/+page.svelte`, `settings/MiniIsland.svelte` | Local V2 service interaction window and independent macOS-only enable switch. |
