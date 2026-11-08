# Route editor and navigation contracts

Source-relative paths below are relative to `src/routes/`, not this docs directory.

## Form composition and model selection

- **Page composition**: New/edit wrappers supply translated `PageHead`/`<svelte:head>` titles inside `FormPage` and reuse the route form. Keep `.form-body`/`FormActions` inside the native form for independent field scrolling; MCP/skill detail frames activate only while editing.

- **Forms**: Provider/model/agent/group editors use TanStack Form with Zod; MCP/skill editors retain full JSON/Markdown drafts. `ModelForm` delegates persistence through `onSave`.

## Navigation and immutable edit drafts

- **Navigation**: Use links for route entry and `goto` from `$app/navigation` after successful saves; read IDs through `$app/state`. Model/agent/MCP/skill links encode identifiers with `encodeURIComponent`. Group copy opens `/groups/new?copyFrom=<encoded-source-id>` and custom model copy opens `/models/new?copyFrom=<encoded-provider-model-ref>` without writing until explicit save.

- **Detail editing**: MCP/skill `?edit=1` opens editing after loading; keyed forms retain immutable expected snapshots, keep drafts on failure, and do not automatically reload OpenCode. Remote skills remain read-only. Group copying loads a detached in-memory draft of native, slim, and OMO settings with a localized copy name; a fresh ID and update timestamp are minted only on explicit save. Unsaved group drafts do not affect running configurations or group switching until explicitly saved; invalid or missing source groups show an error banner with a link back to the group list.

- **Model copying and formatting**: Custom model copying opens a detached in-memory draft with the source provider preselected (still editable) and an initial collision-free ID (`-copy`, `-copy-2`) within that source provider. Known model fields (capabilities, limits, pricing, settings, headers, variants) are deep-copied, source models remain untouched, and built-in catalog models remain read-only since the service catalog does not expose full editable definitions. Each variant row editor requires a single JSON object with override keys (excluding `id`, e.g. `{"settings":{"reasoningEffort":"high"}}`), not a variants array.

- **Optional fields**: Omit unset optional form values instead of writing `null`, including provider options, model numeric fields, and group variants. Group saves also normalize blank variants to absence. Preserve explicit zero/false values, required nullable group/selection fields, and user-authored JSON nulls; full-document MCP/skill editors retain their document content rather than recursively stripping values.

- **Model directory authority**: Custom model rows come from persisted provider configuration through `config.models()`, not the potentially stale service catalog; only the built-in tab depends on catalog loading. Successful model mutations refresh local configuration before the directory renders confirmed changes, without automatically reloading OpenCode.

- **Draft resets**: Agent/group forms initialize on identity changes and `config.formResetVersion`, not every reactive update. Agent edits freeze selected source fields/clear-fields baseline and backend `resolvedPath`/`content`; refresh/failed retry cannot replace them, but explicit source change/reset reloads them. Echo `expectedPath`/`expectedContent`, never stringify `raw` into a numeric-lossy token; create stays unchanged, missing update snapshots are backend-rejected.

- **Scrollable previews**: Height-bounded Agent/MCP/Skill previews remain keyboard-focusable and use translated accessible names.

## Preview application and translated copy

- **Prompt assistance**: Agent/Markdown command editors and Settings Global behavior use shared `PromptGenerator.svelte` below text fields; preview application replaces only the local prompt/document draft, preserving explicit-save/source-snapshot checks. Read-only, inactive or loading/saving editors cannot apply generated text.

- **Route copy**: Page titles, headings, controls, hints, and feedback use route-specific translation namespaces plus `common.*`, `configuration.*`, `validation.*`, and `toast.*`; tray status labels/details react to locale changes.

## Route test harness

- **Route tests**: Co-located `*.test.ts` use Vitest, Testing Library, and the config/i18n `Harness`, mocking navigation/state or native APIs as needed. `providers/__tests__/schema.test.mjs` and `agents/__tests__/agent-form.test.mjs` coexist as `node:test` checks.

## Implementation references

| Responsibility | File | Description |
| --- | --- | --- |
| Provider validation | `providers/schema.ts` | Requires package on creation; parses optional string-valued headers. |
| Model mapping | `models/modelForm.ts` | Converts editor strings/variant rows to model definitions and back; maps nullish numeric limits/costs to empty strings while preserving zero values, and omits blank optional numeric fields on save. |
| Agent editing | `agents/agentForm.ts`, `agents/PermissionEditor.svelte` | Validates fields, joins model variants, and serializes ordered permission rules. |
| Group editing | `groups/GroupForm.svelte` | Edits runtime-specific bindings, supports detached group copying via `/groups/new?copyFrom=`, and confirms type changes when mappings exist. |
| Document editors | `mcp/McpForm.svelte`, `skills/SkillForm.svelte` | Preserve full documents and send expected source snapshots on updates. |
