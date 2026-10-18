# Routes

## Overview

Client-side pages for runtime switching, usage monitoring, and configuration management. Route wrappers compose co-located forms; the shared layout initializes application contexts and separates the main console from the tray and Island windows.

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
│   ├── [id]/+page.svelte            # Read-only source detail with optional ?project= saved ID
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
├── mcp/                            # /mcp: Global management and exact project-root browsing
│   ├── +page.svelte                # Scoped server list, optional Global comparison, diagnostics
│   ├── new/+page.svelte            # /mcp/new: creation wrapper
│   ├── [id]/+page.svelte            # /mcp/[id]: masked detail and inline edit mode
│   ├── McpForm.svelte              # Full native JSON creation/editing
│   ├── DeleteMcpDialog.svelte       # Shared pending-aware delete confirmation
│   ├── config-view.ts              # Non-mutating credential masking for display
│   └── __tests__/                  # List/detail/edit/delete and masking tests
├── skills/                         # /skills: Global skills and project .opencode skill(s)
│   ├── +page.svelte                # Scoped skill list, readonly comparison and Global edit links
│   ├── new/+page.svelte            # /skills/new: local skill creation wrapper
│   ├── [id]/+page.svelte            # /skills/[id]: plain-text content and local edit mode
│   ├── SkillForm.svelte            # Full Markdown document creation/editing
│   └── __tests__/                  # List/detail/edit and safe content tests
├── settings/                       # /settings: system settings, OpenCode CLI, and app updates
│   ├── +page.svelte                # Autostart readback and independent lid/update sections
│   ├── LidProtection.svelte        # Native phase display, polling, and restore actions
│   ├── AppUpdates.svelte           # Installed version, explicit release check, and native macOS guide
│   ├── MiniIsland.svelte           # Independent macOS-only enable switch and readback
│   ├── OpenCodeSettings.svelte     # Native local CLI inspection, explicit checks, guarded upgrades
│   ├── OpenCodeService.svelte      # Saved Shared/fixed target, local probes, component-local secret draft
│   └── __tests__/                  # Autostart, lid lifecycle, updates, openers, and navigation tests
├── island/                         # /island: native mini island window
│   ├── +page.svelte                # Native-sized backdrop, CSS content motion, lazy sessions/messages and guarded actions
│   ├── QuestionForm.svelte         # Field-key/value question response form
│   └── __tests__/                  # Lifecycle, stale reads, send safety, and question interaction tests
└── tray/                           # /tray: special tray-window mode
    ├── +page.svelte                # Group switching, runtime health, and native window actions
    └── __tests__/                  # Tray lifecycle, commands, preferences, and locale tests
```

## Key Files

| Responsibility | File | Description |
| :--- | :--- | :--- |
| Bootstrap | `+layout.svelte` | Creates config/i18n contexts; disables global initialization for `/island`; registers main/tray listeners and disposes subscriptions. |
| Dashboard | `+page.svelte` | Shares one token-usage read between independently selected usage and heatmap periods. |
| Provider validation | `providers/schema.ts` | Requires package on creation; parses optional string-valued headers. |
| Model mapping | `models/modelForm.ts` | Converts editor strings/variant rows to model definitions and back. |
| Agent editing | `agents/agentForm.ts`, `agents/PermissionEditor.svelte` | Validates fields, joins model variants, and serializes ordered permission rules. |
| Group editing | `groups/GroupForm.svelte` | Edits runtime-specific bindings and confirms type changes when mappings exist. |
| MCP display | `mcp/config-view.ts` | Masks credentials in targets and nested configuration without modifying source values. |
| Document editors | `mcp/McpForm.svelte`, `skills/SkillForm.svelte` | Preserve full documents and send expected source snapshots on updates. |
| Resource scope | `agents/+page.svelte`, `mcp/+page.svelte`, `skills/+page.svelte` | Local ResourceScope uses saved project roots; Global retains CRUD while project and Global comparison entries are read-only. |
| Power status | `settings/LidProtection.svelte` | Polls every two seconds without overlapping reads; pauses during writes. |
| Manual updates | `settings/AppUpdates.svelte` | Reads local app info; checks releases only on click and opens official project pages through config. |
| OpenCode CLI | `settings/OpenCodeSettings.svelte` | Reads native CLI details locally; checks stable updates explicitly and confirms exact upgrade path/versions. |
| Service target | `settings/OpenCodeService.svelte` | Lazy local target probes, draft-preserving saves, safe errors, and independent backend version. |
| Mini Island | `island/+page.svelte`, `settings/MiniIsland.svelte` | Local V2 service interaction window and independent macOS-only enable switch. |
| Tray lifecycle | `tray/+page.svelte` | Coalesces focus/show events; refreshes config and health when shown. |
| Cross-route tests | `__tests__/list-refresh.test.ts`, `__tests__/layout-preferences.test.ts` | Cover draft-preserving refresh and window-specific listener cleanup. |

## Conventions

- **Page composition**: New/edit wrappers provide translated `PageHead` and `<svelte:head>` titles, then reuse their route's form component; MCP/skill details host inline editors.
- **Forms**: Provider/model/agent/group editors use TanStack Form with Zod; MCP/skill editors retain full JSON/Markdown drafts. `ModelForm` delegates persistence through `onSave`.
- **Navigation**: Use links for route entry and `goto` from `$app/navigation` after successful saves; read IDs through `$app/state`. Model/agent/MCP/skill links encode identifiers with `encodeURIComponent`.
- **List refresh**: Provider/model/agent/group lists call `config.refresh(true)` and guard loading/saving/local refresh; MCP/skill lists reload only their own data and diagnostics. Pagination uses five rows for the former and ten for the latter.
- **Async reads**: MCP/skill lists and details invalidate request sequences on teardown or route changes; preserve these guards so stale responses cannot replace current data.
- **Resource scopes**: Scope controls exist only on MCP/Agent/Skill lists, not app chrome or drafts. Exact project reads pass the saved project ID; list scope effects clear old data/search/page/delete state and invalidate stale requests. Agent project/comparison reads never replace config.agents; built-in tabs remain static. Optional Global comparison lazily reads Global, appends rows with scope-aware keys without merging equal names, and has independent request guards/errors. Project Refresh rereads both visible scopes.
- **Read-only navigation**: MCP/skill details use `?project=<saved ID>`; project edit intent and Global saved banners are ignored, and retry/refresh preserve scope. Global comparison links use `?readonly=1`; Agent `[id]` detail is always read-only. Details never change shared scope. New and Agent edit routes remain Global and show a short destination notice when a project is selected.
- **Settings updates**: `AppUpdates` only calls local `getAppInfo` on mount; `checkForUpdates` requires an explicit click from an icon-only action with translated check/pending/retry accessible names and title tooltips. Release downloads and optional Stars use `openProjectPage`, never frontend URLs or automatic installation. Show the expandable macOS security guide only for native `platform === 'macos'` with an available update; quarantine commands remain selectable text, not executable controls. Pending operations are guarded and invalidated on teardown.
- **CLI updates**: `OpenCodeSettings` remains separate from app updates. Native mount confirms app info, reads local installation and independently probes the saved service without bootstrap; browser mode performs no calls. Installation metadata stays in background state; the update header groups accessible icon-only local refresh and explicit check actions, and inspection retry/service refresh controls are icon-only too. Explicit checks own installation/version results. Upgrade requires `canUpgrade`, an available checked update, and an exact path/version confirmation shown only in the dialog; CLI and service pending operations guard both panels. Failure preserves known details and requires explicit refresh before another check/upgrade. Success reads back installation and service status once, never reloads or restarts a service.
- **Service target**: `OpenCodeService` keeps URL/password/clear drafts local across refresh/failure and commits only successful saves, including offline targets. Passwords never read back and clear on success/teardown. Saved target and connected URL/PID/backend version remain separate from the draft and CLI version; failed reads or status errors cannot advertise stale connection data. Render only safe generic or code-based busy/validation hints, never backend error bodies or credentials.
- **Detail editing**: MCP/skill `?edit=1` opens editing after loading; keyed forms retain immutable expected snapshots, keep drafts on failure, and do not automatically reload OpenCode. Remote skills remain read-only.
- **MCP display**: Read-only targets/search use `maskTarget`; detail fields use `maskConfig`. Only the explicit JSON editor receives unmasked configuration.
- **Draft resets**: Agent/group forms initialize on identity changes and `config.formResetVersion`, rather than resetting on every reactive store update; agent edits use the selected storage snapshot.
- **Tray mode**: `/tray` bypasses `AppShell`/`SplashScreen`, applies `tray-window`, and disables catalog prefetch. Main listens for `tray:navigate`/`tray:config-changed`; tray layout listens for `app:preferences-changed`.
- **Island mode**: `/island` bypasses `AppShell`/`SplashScreen` and all global config preload. Every screen starts and collapses compact; native notch geometry changes information density, not the view. Non-notched compact adds short status/count text beside the same single activity indicator. Before its initial snapshot read, the native-targeted Island registers only `island:changed`, `island:collapsed`, `island:pointer-enter`, `island:pointer-leave`, and `app:preferences-changed`. Void pointer events update native hover alongside separate DOM hover using the shared 100 ms entry/180 ms protected leave timers, never focus/expand/read/mutate; expanded ignores hover, and disable/collapse/teardown clear both sources. It coalesces reads, fetches messages only while expanded, and cleans listeners/timers including late registrations on teardown.
- **Island safety**: Ambiguous send/409 failures retain the Session draft and caller message ID; retries require explicit Refresh and never generate a replacement ID. Disconnected or failed snapshot reads show unknown session status, not Idle. Backend `answerable` owns metadata/constraint checks; UI accepts only string/multiselect fields and field-key/option-value answers. Collapse/disable/teardown never reply to requests or stop sessions.
- **Island motion**: Native `NSAnimationContext`/`NSWindow.animator()` grows changed frames in 220 ms and shrinks in 190 ms; completion correction/readback precedes grow view/focus/IPC success. Frontend sends compact immediately during pending grow, deduplicates pending targets and guards results by latest request identity. Native compact intent/blur/close emits authoritative collapse before shrink, removing old frontend controls without an outro; the same compact request retains final shrink error feedback without restoring controls or retrying. One active/latest pending native request bounds reversal waiting to the remaining current animation (approximately 220 ms maximum), not total reversal duration. CSS fills real intermediate bounds with the unmoving black screen-edge backdrop and mounts target content only after grow IPC success; 180–220 ms content entry preserves peek header focus and immediate keyed replacement. CSS reduced-motion responds at runtime independently of native OS reduced-motion sampled per transition; startup/placement/correction are instant. Native bounds, seams, focus timing, physical camera gap and actual smoothness require rebuilt-app manual acceptance.
- **Route copy**: Page titles, headings, controls, hints, and feedback use route-specific translation namespaces plus `common.*`, `configuration.*`, `validation.*`, and `toast.*`; tray status labels/details react to locale changes.
- **Route tests**: Co-located `*.test.ts` use Vitest, Testing Library, and the config/i18n `Harness`, mocking navigation/state or native APIs as needed. `providers/__tests__/schema.test.mjs` and `agents/__tests__/agent-form.test.mjs` coexist as `node:test` checks.

## Sync

This file is maintained as the directory scope changes. When files in this directory are added, modified, or deleted (components, logic units, utilities, constants, etc.), update this file's directory structure, key files, and conventions to keep the documentation in sync with the code.
