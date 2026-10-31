# Routes

## Overview

Client-side pages for runtime switching, usage monitoring, and configuration management. Route wrappers compose co-located forms; the shared layout initializes application contexts and separates the main console from the tray and Island windows.

## Directory Structure

```text
src/routes/                         # SvelteKit page module
├── +layout.ts                      # Exports ssr = false
├── +layout.svelte                  # Config/i18n initialization and window-specific listeners
├── +page.svelte                    # /: runtime dashboard, quick actions, system status, and token usage
├── __tests__/                      # Dashboard, layout preferences, and shared list refresh tests
├── providers/                      # /providers: provider registry
├── models/                         # /models: custom and built-in model catalog
├── agents/                         # /agents: custom agents and read-only built-in tabs
├── groups/                         # /groups: runtime groups and switching
├── mcp/                            # /mcp: Global management, project-root scopes, and MCP Proxy
├── skills/                         # /skills: Global skills and project .opencode skill(s)
├── settings/                       # /settings: system settings, OpenCode CLI, and app updates
├── island/                         # /island: native mini island window
└── tray/                           # /tray: special tray-window mode
```

## Key Files

| Responsibility | File | Description |
| --- | --- | --- |
| Window bootstrap | `+layout.svelte` | Context setup and window-specific initialization. |
| Dashboard | `+page.svelte` | Runtime actions, status, and shared activity reads. |
| Agent drafts | `agents/agentForm.ts` | Validation, model selection, and permission editing. |
| Document editors | `mcp/McpForm.svelte`, `skills/SkillForm.svelte` | Full-document editing with source snapshots. |
| Cross-route checks | `__tests__/list-refresh.test.ts`, `__tests__/layout-preferences.test.ts` | Draft retention and listener cleanup. |

## Conventions

- **Page composition**: Compose translated route wrappers around shared forms; preserve native submit
  ownership and independent field scrolling. Follow [editor
  composition](docs/editor-contracts.md#form-composition-and-model-selection) rather than duplicating form
  layouts.

- **Form models**: Keep schema-based editors distinct from full JSON or Markdown editors; model conversion
  stays with its form. Preserve the existing [form
  contracts](docs/editor-contracts.md#form-composition-and-model-selection).

- **Navigation**: Use links for entry and route APIs after successful saves; encode resource identifiers and
  preserve explicit read scope. See [navigation
  contracts](docs/editor-contracts.md#navigation-and-immutable-edit-drafts).

- **Resource scopes**: Scope pickers belong on resource lists; project and comparison views remain read-only.
  Preserve [scope boundaries](docs/resource-lifecycle.md#explicit-read-only-scopes) without changing shared
  selection from details.

- **Refresh ownership**: Lists refresh only their owned data and reject stale responses; dashboard periods
  share activity reads. Preserve [refresh
  contracts](docs/resource-lifecycle.md#dashboard-and-refresh-ownership) and independent error states.

- **Draft lifecycle**: Keep edit snapshots immutable across background refresh and failed saves; reset only
  through existing identity or explicit-reset paths. Follow [draft
  contracts](docs/editor-contracts.md#navigation-and-immutable-edit-drafts).

- **Window isolation**: Keep tray and Island initialization separate from the main console. Follow the
  [settings](settings/AGENTS.md), [MCP](mcp/AGENTS.md), and [Island](island/AGENTS.md) guides for their
  specialized lifecycles.

- **Prompt previews**: Generated previews replace only editable local drafts after confirmation; saving and
  source validation remain explicit. Preserve [preview
  application](docs/editor-contracts.md#preview-application-and-translated-copy) without introducing automatic
  writes.

## Sync

Keep this guide and linked docs in sync with changes to this directory.
