# Domain and Command Types

## Overview

This directory defines frontend domain records, command DTOs, and small UI result unions, with one file per domain area. It describes data exchanged by configuration actions and consumed by pages without implementing persistence, validation, or state management.

## Directory Structure

```text
src/types/            # Domain contracts without runtime implementations
├── agents.ts         # Agent definitions, source previews, permissions, and field mutations
├── app.ts            # App snapshot, projects, preferences, app/CLI/target metadata, updates, command errors
├── groups.ts         # Group records, agent bindings, and category mappings
├── mcp.ts            # MCP entries, lists, drafts, and guarded updates
├── models.ts         # Model references, definitions, variants, and catalog entries
├── power.ts          # Lid-protection state and phases
├── providers.ts      # Provider settings and model dictionaries
├── skills.ts         # Skill entries, lists, drafts, and guarded updates
├── stats.ts          # Token records and daily aggregates
├── island.ts         # Mini Island state, sessions, requests, messages, and answers
└── ui.ts             # Permission parsing and presentation unions
```

## Key Files

| Responsibility | File | Description |
| :--- | :--- | :--- |
| App contract | `app.ts` | `AppState`, `ProjectFolder`, `AppPreferences`, `AppInfo`, `UpdateCheck`, `ProjectPage`, and `CommandError`; project folders, selections, and diagnostics are optional snapshot fields. |
| OpenCode installation | `app.ts` | `OpenCodeInstallation` and `OpenCodeUpdateCheck`; exact nullable binary/version, backend-owned method/reason, manual command, and stable-update availability. |
| Service target | `app.ts` | `OpenCodeServiceTarget` and `OpenCodeServiceStatus`; Shared/fixed mode, public password presence and nullable connected identity/error fields. |
| Group contract | `groups.ts` | `Group`, `AgentModelBinding`, and `CategoryMapping`; preset-specific collections can be `null`. |
| Model contract | `models.ts` | `ModelRef`, `ModelDef`, and `ModelCatalogEntry`; catalog limits and variants have distinct optional/nullable shapes. |
| Agent contract | `agents.ts` | `AgentDefinition`, typed optional `AgentSourcePreview[]` and `AgentFieldOverride[]`, permissive source records, permission rules, and `AgentWrite` mutation metadata. |
| Resource writes | `mcp.ts`, `skills.ts` | Separate saved entries, creation drafts, and updates carrying expected source/content snapshots. |

## Conventions

- **Domain ownership**: Extend the existing domain file rather than adding runtime helpers or a cross-domain type barrel; sibling contracts use `import type` from their owning files.
- **Constant-derived unions**: `agents.ts` and `groups.ts` import and re-export their domain unions from `src/utils/constants.ts`; extend those source constants rather than duplicating unions here.
- **References and dictionaries**: Use `ModelRef` (`${string}/${string}`) for model bindings; `ProviderDef.models` is a `Record<string, ModelDef>`, while model variants are arrays.
- **Optional versus null**: Preserve required nullable preset collections in `Group` and optional nullable catalog fields; `AppState.selectedGroupId` can be absent or `null`.
- **Project folders**: `ProjectFolder` stores only `id` and `path`; display names are derived by the UI. Optional `AppState.projects`/`selectedProjectId` preserve compatibility with older payloads and mocks.
- **Agent previews**: Optional `sources` retain storage, path, required raw content, fields, and nullable prompt; optional `overrides` contain `AgentFieldOverride` records with required `field`, `inline`, `markdown`, and `effective` values matching the backend wire contract.
- **Manual updates**: `AppInfo.platform` is `macos`/`windows`/`linux`/`other`; `UpdateCheck.latestVersion` is required but nullable when no stable release exists. `ProjectPage` limits browser destinations to `repository`/`releases`; metadata/check results are not part of `AppState`.
- **CLI updates**: Keep installation method and upgrade-reason unions wire-compatible. `OpenCodeInstallation.binaryPath`, `version`, `upgradeReason`, and `manualCommand` are required nullable fields; `OpenCodeUpdateCheck.latestVersion` is non-null. Backend values are authoritative; do not infer version ordering or upgrade capability in the frontend.
- **Service target**: Target mode is `shared`/`explicit`, url is required nullable, and `hasPassword` is the only credential metadata. Status connected/url/pid/version/error remain separate from target and local CLI metadata; nullable identity/error fields are required. Never add passwords to returned DTOs or AppState. Preserve `busy`/`conflict` command error codes for guarded operations.
- **Guarded updates**: Keep `expectedConfig`/`expectedSourcePath` on `McpUpdate` and `expectedContent`/`expectedPath` on `SkillUpdate`; creation drafts do not carry these fields.
- **Open-ended data**: Retain `Record<string, unknown>` for raw/config/settings objects, `unknown` for variant bodies and error detail, and open strings for agent modes and permission effects.
- **Permission results**: Preserve the `ok` discriminator in `PermissionParse` and `mode` discriminator in `PermissionView`; only successful/rules branches carry rules.
- **Usage shapes**: Keep timestamped `TokenUsageRecord.time` separate from aggregated `DailyTokenUsage.date`; both retain numeric input/output counts.
- **Island contract**: Keep the Island wire records in `island.ts`; required numeric `connectionGeneration` is the native cache generation, not a URL or credential, and invalidates target-local UI state monotonically. `IslandView` is `compact`/`peek`/`expanded`. Permission `save?: string[]` carries only server-provided rules, and `IslandPermissionDecision` is `once`/`always`/`reject`. Always requires explicit user consent and a fresh backend check of the exact pending request with nonempty save rules; never infer rules from resources. Question answers use field keys and option values, while messages expose only public user/assistant text.

## Sync

This file is maintained as the directory scope changes. When files in this directory are added, modified, or deleted (components, logic units, utilities, constants, etc.), update this file's directory structure, key files, and conventions to keep the documentation in sync with the code.
