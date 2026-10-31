# Domain and Command Types

## Overview

Defines domain records, command DTOs, and small UI result unions in domain-owned files for configuration
actions and pages. This directory implements no persistence, validation, or state management.

## Directory Structure

```text
types/
├── agents.ts             # Sources, permissions, mutations
├── app.ts                # Snapshots, preferences, CLI/service metadata
├── groups.ts             # Groups, bindings, categories
├── mcp.ts                # MCP records and guarded writes
├── mcp-proxy.ts          # Proxy and MOM HTTP contracts
├── models.ts             # References, variants, catalog
├── power.ts              # Lid-protection state and phases
├── providers.ts          # Provider settings and model dictionaries
├── skills.ts             # Skill records and guarded writes
├── stats.ts              # Token records and daily aggregates
├── island.ts             # Native Island sessions and requests
├── prompt-generation.ts  # Generation request and derived preset/kind types
└── ui.ts                 # Permission result unions
```

## Key Files

| Responsibility | File | Description |
| :--- | :--- | :--- |
| App and service boundary | `app.ts` | Snapshots, projects, preferences, command errors, and backend-owned installation/service metadata. |
| Model bindings | `groups.ts`, `models.ts` | Group collections, typed references, model definitions, and catalog shapes. |
| Guarded resource writes | `agents.ts`, `mcp.ts`, `skills.ts` | Source previews, creation drafts, and update snapshot contracts. |
| Native/service records | `mcp-proxy.ts`, `island.ts`, `prompt-generation.ts` | Proxy/listener separation, native request state, and generation payloads. |

## Conventions

- **Domain ownership**: Extend the existing domain file rather than adding runtime helpers or a
  cross-domain type barrel; sibling contracts use `import type` from their owning files.
- **Derived unions**: Extend source constants in `src/utils/constants.ts` for agent, group, and generation
  vocabulary rather than duplicating unions; retain typed `ModelRef` bindings.
- **Wire compatibility**: Preserve optional versus nullable fields, open-ended data, and discriminated
  result branches; consult the linked contracts before changing a domain payload's shape.
- **Guarded resource writes**: Echo frozen source paths and backend-generated content unchanged, never
  regenerate content from JavaScript raw data; preserve update snapshot requirements without changing create payloads.
- **Public metadata**: Never return passwords in DTOs or `AppState`; installation versions and upgrade
  capability are backend-authoritative, and app/check metadata remains separate from snapshots.
- **Listener isolation**: Keep MOM HTTP listener settings separate from revision-guarded MCP Proxy
  writes; do not merge listener fields into proxy settings when extending either contract.
- **Island consent**: Always requires explicit user consent and a fresh backend check of the exact
  pending request with nonempty server-provided save rules; never infer permission rules from resources.
- **Generation privacy**: Keep preset instructions and provider credentials out of generation payloads;
  derive kind/preset types from shared constants and use service defaults when model/variant are omitted.

## Details

| Section | File |
| --- | --- |
| Domain wire shapes and mutation contracts | [docs/wire-contracts.md](docs/wire-contracts.md) |

## Sync

Keep this guide and linked docs in sync with changes to this directory.
