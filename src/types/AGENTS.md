# Domain and Command Types

## Overview

This directory defines frontend domain records, command DTOs, and small UI result unions, with one file per domain area. It describes data exchanged by configuration actions and consumed by pages without implementing persistence, validation, or state management.

## Directory Structure

```text
src/types/            # Domain contracts without runtime implementations
├── agents.ts         # Agent definitions, permissions, and field mutations
├── app.ts            # App snapshot, preferences, and command errors
├── groups.ts         # Group records, agent bindings, and category mappings
├── mcp.ts            # MCP entries, lists, drafts, and guarded updates
├── models.ts         # Model references, definitions, variants, and catalog entries
├── power.ts          # Lid-protection state and phases
├── providers.ts      # Provider settings and model dictionaries
├── skills.ts         # Skill entries, lists, drafts, and guarded updates
├── stats.ts          # Token records and daily aggregates
└── ui.ts             # Permission parsing and presentation unions
```

## Key Files

| Responsibility | File | Description |
| :--- | :--- | :--- |
| App contract | `app.ts` | `AppState`, `AppPreferences`, and `CommandError`; selection and diagnostics are optional. |
| Group contract | `groups.ts` | `Group`, `AgentModelBinding`, and `CategoryMapping`; preset-specific collections can be `null`. |
| Model contract | `models.ts` | `ModelRef`, `ModelDef`, and `ModelCatalogEntry`; catalog limits and variants have distinct optional/nullable shapes. |
| Agent contract | `agents.ts` | `AgentDefinition`, permissive source records, permission rules, and `AgentWrite` mutation metadata. |
| Resource writes | `mcp.ts`, `skills.ts` | Separate saved entries, creation drafts, and updates carrying expected source/content snapshots. |

## Conventions

- **Domain ownership**: Extend the existing domain file rather than adding runtime helpers or a cross-domain type barrel; sibling contracts use `import type` from their owning files.
- **Constant-derived unions**: `agents.ts` and `groups.ts` import and re-export their domain unions from `src/utils/constants.ts`; extend those source constants rather than duplicating unions here.
- **References and dictionaries**: Use `ModelRef` (`${string}/${string}`) for model bindings; `ProviderDef.models` is a `Record<string, ModelDef>`, while model variants are arrays.
- **Optional versus null**: Preserve required nullable preset collections in `Group` and optional nullable catalog fields; `AppState.selectedGroupId` can be absent or `null`.
- **Guarded updates**: Keep `expectedConfig`/`expectedSourcePath` on `McpUpdate` and `expectedContent`/`expectedPath` on `SkillUpdate`; creation drafts do not carry these fields.
- **Open-ended data**: Retain `Record<string, unknown>` for raw/config/settings objects, `unknown` for variant bodies and error detail, and open strings for agent modes and permission effects.
- **Permission results**: Preserve the `ok` discriminator in `PermissionParse` and `mode` discriminator in `PermissionView`; only successful/rules branches carry rules.
- **Usage shapes**: Keep timestamped `TokenUsageRecord.time` separate from aggregated `DailyTokenUsage.date`; both retain numeric input/output counts.

## Sync

This file is maintained as the directory scope changes. When files in this directory are added, modified, or deleted (components, logic units, utilities, constants, etc.), update this file's directory structure, key files, and conventions to keep the documentation in sync with the code.
