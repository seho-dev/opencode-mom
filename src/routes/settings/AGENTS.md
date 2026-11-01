# Settings

## Overview

Settings compose app behavior, local OpenCode service and CLI controls, MOM HTTP, and MCP Proxy configuration. Each section owns its drafts and native lifecycle; app and CLI updates remain separate.

## Directory Structure

```text
settings/
├── +page.svelte                   # Retained hidden General, MCP Proxy and Behavior panels; active-gated sections
├── Behavior.svelte                # Global AGENTS.md draft, shared prompt previews and explicit snapshot-guarded saves
├── SettingsSearch.svelte          # Local bilingual static-key index; navigation results without reading drafts
├── LidProtection.svelte           # Native lid preference switch, without status polling
├── LoggingSettings.svelte         # Independent diagnostic switch; history lives on /logs
├── AppUpdates.svelte              # Explicit update checks and project pages
├── McpProxySettings.svelte        # General HTTP / MCP Proxy sections with explicit readback and revision guards
├── MiniIsland.svelte              # macOS-only Island state and enable readback
├── OpenCodeSettings.svelte        # Local CLI inspection, explicit checks, upgrade confirmation/readback
├── OpenCodeService.svelte         # Saved target, read-only connection probes, secret-local drafts
├── RotateKeyDialog.svelte         # Ephemeral key rotation dialog
└── __tests__/                     # Settings lifecycle coverage
    ├── behavior.test.ts           # Global document snapshots, prompt previews and inactive/pending guards
    ├── settings-search.test.ts    # Bilingual lookup, draft retention, cross-tab focus and no-polling lifecycle
    ├── rotate-key-dialog.test.ts  # External close and tab changes discard one-time token results
    ├── lid-protection.test.ts     # Preference readback, no polling, deferred target reads, safe errors and teardown
    ├── mini-island.test.ts        # Visibility, pending guards, readback, errors, teardown
    └── opencode-settings.test.ts  # Native reads, CLI confirmation, target drafts, safe errors and cross-panel guards
```

## Key Files

| Responsibility | File | Description |
| --- | --- | --- |
| Settings shell | `+page.svelte`, `SettingsSearch.svelte` | Retained panels and static bilingual navigation. |
| Global behavior | `Behavior.svelte` | Shared prompt generation, local drafts and explicit source-checked saves. |
| Native preferences | `LidProtection.svelte`, `MiniIsland.svelte` | Independent preference readback; lid enablement is not a protection claim. |
| Diagnostic recording | `LoggingSettings.svelte` | Confirmed logging enablement and writer health, without a log viewer. |
| Service target | `OpenCodeService.svelte` | Saved target, connection probes, and private drafts. |
| CLI updates | `OpenCodeSettings.svelte` | Explicit checks, confirmation, and guarded upgrade. |
| App updates | `AppUpdates.svelte` | Manual release checks and official project pages. |
| HTTP and Proxy | `McpProxySettings.svelte`, `RotateKeyDialog.svelte` | Separate settings writes and one-time token display. |

## Conventions

- **Panel lifecycle**: Keep tab panels mounted and drafts intact; inactive sections start no native work. Use
  the typed config facade and preserve [active-lifecycle
  guards](docs/settings-lifecycle.md#active-native-work-and-manual-app-updates).

- **Behavior drafts**: Reuse the shared prompt dialog; confirmed previews replace only the local draft.
  Keep explicit snapshot-guarded saves and invalidate previews on inactivity, reads/saves or teardown as
  described in [ownership](docs/settings-lifecycle.md#ownership).

- **Search privacy**: Search only explicit bilingual keys, never drafts or runtime values; results navigate
  without remounting settings. Preserve [retained-panel
  search](docs/settings-lifecycle.md#retained-panels-search-and-help) and non-action focus targets.

- **Logging separation**: General owns only the independent logging switch and readback. Keep history,
  filters and deletion on the separate `/logs` navigation page; neither surface polls for changes.

- **Service drafts**: Retain local target drafts on failure, never read back passwords, and show only safe
  errors. Follow [credential contracts](docs/cli-service.md#service-drafts-and-credential-safety) without
  changing monitor preferences.

- **CLI confirmation**: Check releases explicitly and confirm the exact checked installation before upgrading;
  keep service and CLI operations mutually guarded. Preserve [upgrade
  contracts](docs/cli-service.md#explicit-local-cli-checks-and-upgrades).

- **Island safety**: Keep Island enablement independent and confirmed by native readback; disabling never
  interrupts sessions or replies to requests. Preserve [monitor
  ownership](docs/settings-lifecycle.md#island-enablement-and-monitor-readback).

- **Target readback**: Reread Island state and the lid preference after successful target changes; reject stale
  results without remounting or toggling preferences. Lid reads only on active entry or target changes, never
  through polling. Follow [target
  readback](docs/settings-lifecycle.md#island-enablement-and-monitor-readback).

- **HTTP and Proxy**: Keep listener and Proxy payloads separate. Read settings/status on active entry, manual
  refresh and post-operation readback only; never poll. Preserve dirty drafts, edit revisions, pending guards
  and queued refreshes through tab changes. Follow [settings
  contracts](docs/settings-lifecycle.md#http-proxy-drafts-and-token-visibility), validation and LAN security warnings.

- **Token isolation**: General's `http=true` section alone owns shared MOM HTTP token controls and rotation,
  independent of Proxy availability/state. Read back only listener settings and preserve dirty port/LAN drafts;
  never write Proxy settings or refresh global state. Display tokens once and clear them on close or leaving
  General; late rotation/clipboard results cannot revive secrets or copied state. Follow [token
  visibility](docs/settings-lifecycle.md#http-proxy-drafts-and-token-visibility).

## Sync

Keep this guide and linked docs in sync with changes to this directory.
