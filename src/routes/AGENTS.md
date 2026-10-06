# Routes

## Overview

Client-side pages for runtime switching, usage monitoring, and configuration management. Route wrappers compose co-located forms; the shared layout initializes application contexts and separates the main console from the tray window.

## Directory Structure

```text
src/routes/                         # SvelteKit page module
├── +layout.ts                      # Exports ssr = false
├── +layout.svelte                  # Config/i18n initialization and window-specific listeners
├── +page.svelte                    # /: runtime dashboard, metrics, and token usage
├── __tests__/                      # Dashboard, layout preferences, and shared list refresh tests
├── providers/                      # /providers: provider registry
│   ├── +page.svelte                # Searchable provider list and delete confirmation
│   ├── new/+page.svelte            # /providers/new: creation wrapper
│   ├── [id]/edit/+page.svelte       # /providers/[id]/edit: provider lookup and edit wrapper
│   ├── ProviderForm.svelte         # Shared creation/editing form
│   ├── schema.ts                   # Provider validation and JSON header parsing
│   └── __tests__/                  # List/form Vitest tests and schema.test.mjs
├── models/                         # /models: custom and built-in model catalog
│   ├── +page.svelte                # Catalog tabs, search, and custom-model actions
│   ├── new/+page.svelte            # /models/new: creation and provider prerequisite
│   ├── [id]/edit/+page.svelte       # /models/[id]/edit: encoded model-reference lookup
│   ├── ModelForm.svelte            # Shared validated model editor
│   ├── modelForm.ts                # Form/model conversion, modalities, and JSON parsing
│   └── __tests__/                  # Model form behavior tests
├── agents/                         # /agents: custom agents and read-only built-in tabs
│   ├── +page.svelte                # Agent lists and storage-aware deletion
│   ├── new/+page.svelte            # /agents/new: creation wrapper
│   ├── [id]/edit/+page.svelte       # /agents/[id]/edit: ID-driven form wrapper
│   ├── AgentForm.svelte            # Source snapshots, mutations, and editor
│   ├── PermissionEditor.svelte     # Ordered permission rules and JSON editing
│   ├── agentForm.ts                # Schema, form factory, selectors, and permissions
│   └── __tests__/                  # List/form Vitest tests and agent-form.test.mjs
├── groups/                         # /groups: runtime groups and switching
│   ├── +page.svelte                # Runtime tabs, activation, and reload prompt
│   ├── new/+page.svelte            # /groups/new: creation wrapper
│   ├── [id]/edit/+page.svelte       # /groups/[id]/edit: group lookup and edit wrapper
│   ├── GroupForm.svelte            # Group identity, type, agent/category mappings
│   └── __tests__/                  # Validation, mappings, and type-switch tests
├── mcp/                            # /mcp: global MCP server management
│   ├── +page.svelte                # Server list, diagnostics, and actions
│   ├── new/+page.svelte            # /mcp/new: creation wrapper
│   ├── [id]/+page.svelte            # /mcp/[id]: masked detail and inline edit mode
│   ├── McpForm.svelte              # Full native JSON creation/editing
│   ├── DeleteMcpDialog.svelte       # Shared pending-aware delete confirmation
│   ├── config-view.ts              # Non-mutating credential masking for display
│   └── __tests__/                  # List/detail/edit/delete and masking tests
├── skills/                         # /skills: user-global local and remote skills
│   ├── +page.svelte                # Skill list, diagnostics, and local edit links
│   ├── new/+page.svelte            # /skills/new: local skill creation wrapper
│   ├── [id]/+page.svelte            # /skills/[id]: plain-text content and local edit mode
│   ├── SkillForm.svelte            # Full Markdown document creation/editing
│   └── __tests__/                  # List/detail/edit and safe content tests
├── settings/                       # /settings: launch-at-login and lid protection
│   ├── +page.svelte                # Autostart readback and independent lid control
│   ├── LidProtection.svelte        # Native phase display, polling, and restore actions
│   └── __tests__/                  # Autostart, lid lifecycle, and navigation tests
└── tray/                           # /tray: special tray-window mode
    ├── +page.svelte                # Group switching, runtime health, and native window actions
    └── __tests__/                  # Tray lifecycle, commands, preferences, and locale tests
```

## Key Files

| Responsibility | File | Description |
| :--- | :--- | :--- |
| Bootstrap | `+layout.svelte` | Creates config/i18n contexts; registers main/tray listeners and disposes subscriptions. |
| Dashboard | `+page.svelte` | Shares one token-usage read between independently selected usage and heatmap periods. |
| Provider validation | `providers/schema.ts` | Requires package on creation; parses optional string-valued headers. |
| Model mapping | `models/modelForm.ts` | Converts editor strings/variant rows to model definitions and back. |
| Agent editing | `agents/agentForm.ts`, `agents/PermissionEditor.svelte` | Validates fields, joins model variants, and serializes ordered permission rules. |
| Group editing | `groups/GroupForm.svelte` | Edits runtime-specific bindings and confirms type changes when mappings exist. |
| MCP display | `mcp/config-view.ts` | Masks credentials in targets and nested configuration without modifying source values. |
| Document editors | `mcp/McpForm.svelte`, `skills/SkillForm.svelte` | Preserve full documents and send expected source snapshots on updates. |
| Power status | `settings/LidProtection.svelte` | Polls every two seconds without overlapping reads; pauses during writes. |
| Tray lifecycle | `tray/+page.svelte` | Coalesces focus/show events; refreshes config and health when shown. |
| Cross-route tests | `__tests__/list-refresh.test.ts`, `__tests__/layout-preferences.test.ts` | Cover draft-preserving refresh and window-specific listener cleanup. |

## Conventions

- **Page composition**: New/edit wrappers provide translated `PageHead` and `<svelte:head>` titles, then reuse their route's form component; MCP/skill details host inline editors.
- **Forms**: Provider/model/agent/group editors use TanStack Form with Zod; MCP/skill editors retain full JSON/Markdown drafts. `ModelForm` delegates persistence through `onSave`.
- **Navigation**: Use links for route entry and `goto` from `$app/navigation` after successful saves; read IDs through `$app/state`. Model/agent/MCP/skill links encode identifiers with `encodeURIComponent`.
- **List refresh**: Provider/model/agent/group lists call `config.refresh(true)` and guard loading/saving/local refresh; MCP/skill lists reload only their own data and diagnostics. Pagination uses five rows for the former and ten for the latter.
- **Async reads**: MCP/skill lists and details invalidate request sequences on teardown or route changes; preserve these guards so stale responses cannot replace current data.
- **Detail editing**: MCP/skill `?edit=1` opens editing after loading; keyed forms retain immutable expected snapshots, keep drafts on failure, and do not automatically reload OpenCode. Remote skills remain read-only.
- **MCP display**: Read-only targets/search use `maskTarget`; detail fields use `maskConfig`. Only the explicit JSON editor receives unmasked configuration.
- **Draft resets**: Agent/group forms initialize on identity changes and `config.formResetVersion`, rather than resetting on every reactive store update; agent edits use the selected storage snapshot.
- **Tray mode**: `/tray` bypasses `AppShell`/`SplashScreen`, applies `tray-window`, and disables catalog prefetch. Main listens for `tray:navigate`/`tray:config-changed`; tray layout listens for `app:preferences-changed`.
- **Route copy**: Page titles, headings, controls, hints, and feedback use route-specific translation namespaces plus `common.*`, `configuration.*`, `validation.*`, and `toast.*`; tray status labels/details react to locale changes.
- **Route tests**: Co-located `*.test.ts` use Vitest, Testing Library, and the config/i18n `Harness`, mocking navigation/state or native APIs as needed. `providers/__tests__/schema.test.mjs` and `agents/__tests__/agent-form.test.mjs` coexist as `node:test` checks.

## Sync

This file is maintained as the directory scope changes. When files in this directory are added, modified, or deleted (components, logic units, utilities, constants, etc.), update this file's directory structure, key files, and conventions to keep the documentation in sync with the code.
