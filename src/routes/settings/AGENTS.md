# Settings

## Overview

Settings compose app behavior, local OpenCode service and CLI controls, MOM HTTP, and MCP Proxy configuration. Each section owns its drafts and native lifecycle; app and CLI updates remain separate.

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

## Key Files

| Responsibility | File | Description |
| --- | --- | --- |
| Settings shell | `+page.svelte`, `SettingsSearch.svelte` | Retained panels and static bilingual navigation. |
| Native monitors | `LidProtection.svelte`, `MiniIsland.svelte` | Independent enablement and confirmed native readback. |
| Service target | `OpenCodeService.svelte` | Saved target, connection probes, and private drafts. |
| CLI updates | `OpenCodeSettings.svelte` | Explicit checks, confirmation, and guarded upgrade. |
| App updates | `AppUpdates.svelte` | Manual release checks and official project pages. |
| HTTP and Proxy | `McpProxySettings.svelte`, `RotateKeyDialog.svelte` | Separate settings writes and one-time token display. |

## Conventions

- **Panel lifecycle**: Keep both panels mounted and drafts intact; inactive sections start no native work. Use
  the typed config facade and preserve [active-lifecycle
  guards](docs/settings-lifecycle.md#active-native-work-and-manual-app-updates).

- **Search privacy**: Search only explicit bilingual keys, never drafts or runtime values; results navigate
  without remounting settings. Preserve [retained-panel
  search](docs/settings-lifecycle.md#retained-panels-search-and-help) and non-action focus targets.

- **Service drafts**: Retain local target drafts on failure, never read back passwords, and show only safe
  errors. Follow [credential contracts](docs/cli-service.md#service-drafts-and-credential-safety) without
  changing monitor preferences.

- **CLI confirmation**: Check releases explicitly and confirm the exact checked installation before upgrading;
  keep service and CLI operations mutually guarded. Preserve [upgrade
  contracts](docs/cli-service.md#explicit-local-cli-checks-and-upgrades).

- **Island safety**: Keep Island enablement independent and confirmed by native readback; disabling never
  interrupts sessions or replies to requests. Preserve [monitor
  ownership](docs/settings-lifecycle.md#island-enablement-and-monitor-readback).

- **Target readback**: Reread native monitor state after successful target changes; reject stale protection
  claims without remounting or toggling preferences. Follow [target
  readback](docs/settings-lifecycle.md#island-enablement-and-monitor-readback).

- **HTTP and Proxy**: Keep listener and Proxy payloads separate; polling preserves dirty drafts and revision
  snapshots. Preserve [settings contracts](docs/settings-lifecycle.md#http-proxy-drafts-and-token-visibility),
  validation, and LAN security warnings.

- **Token isolation**: Display rotated tokens only once and clear them on close or tab changes; late results
  cannot revive secrets. Follow [token
  visibility](docs/settings-lifecycle.md#http-proxy-drafts-and-token-visibility).

## Sync

Keep this guide and linked docs in sync with changes to this directory.
