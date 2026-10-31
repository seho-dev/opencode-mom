# Route editor and navigation contracts

Source-relative paths below are relative to `src/routes/`, not this docs directory.

## Form composition and model selection

- **Page composition**: New/edit wrappers supply translated `PageHead`/`<svelte:head>` titles inside `FormPage` and reuse the route form. Keep `.form-body`/`FormActions` inside the native form for independent field scrolling; MCP/skill detail frames activate only while editing.

- **Forms**: Provider/model/agent/group editors use TanStack Form with Zod; MCP/skill editors retain full JSON/Markdown drafts. `ModelForm` delegates persistence through `onSave`.

## Navigation and immutable edit drafts

- **Navigation**: Use links for route entry and `goto` from `$app/navigation` after successful saves; read IDs through `$app/state`. Model/agent/MCP/skill links encode identifiers with `encodeURIComponent`.

- **Detail editing**: MCP/skill `?edit=1` opens editing after loading; keyed forms retain immutable expected snapshots, keep drafts on failure, and do not automatically reload OpenCode. Remote skills remain read-only.

- **Draft resets**: Agent/group forms initialize on identity changes and `config.formResetVersion`, not every reactive update. Agent edits freeze selected source fields/clear-fields baseline and backend `resolvedPath`/`content`; refresh/failed retry cannot replace them, but explicit source change/reset reloads them. Echo `expectedPath`/`expectedContent`, never stringify `raw` into a numeric-lossy token; create stays unchanged, missing update snapshots are backend-rejected.

- **Scrollable previews**: Height-bounded Agent/MCP/Skill previews remain keyboard-focusable and use translated accessible names.

## Preview application and translated copy

- **Prompt assistance**: Agent/Markdown command editors use shared `PromptGenerator.svelte` below text fields; preview application replaces only the local prompt/document draft, preserving explicit-save/source-snapshot checks. Read-only/saving editors cannot apply generated text.

- **Route copy**: Page titles, headings, controls, hints, and feedback use route-specific translation namespaces plus `common.*`, `configuration.*`, `validation.*`, and `toast.*`; tray status labels/details react to locale changes.

## Route test harness

- **Route tests**: Co-located `*.test.ts` use Vitest, Testing Library, and the config/i18n `Harness`, mocking navigation/state or native APIs as needed. `providers/__tests__/schema.test.mjs` and `agents/__tests__/agent-form.test.mjs` coexist as `node:test` checks.

## Implementation references

| Responsibility | File | Description |
| --- | --- | --- |
| Provider validation | `providers/schema.ts` | Requires package on creation; parses optional string-valued headers. |
| Model mapping | `models/modelForm.ts` | Converts editor strings/variant rows to model definitions and back. |
| Agent editing | `agents/agentForm.ts`, `agents/PermissionEditor.svelte` | Validates fields, joins model variants, and serializes ordered permission rules. |
| Group editing | `groups/GroupForm.svelte` | Edits runtime-specific bindings and confirms type changes when mappings exist. |
| Document editors | `mcp/McpForm.svelte`, `skills/SkillForm.svelte` | Preserve full documents and send expected source snapshots on updates. |
