# Shared Builtin Agent Catalog

## Overview

This directory holds the shipped builtin-agent catalog shared by frontend presentation and backend deletion protection. It is read-only runtime metadata, not user-editable agent definitions: the frontend imports it into builtin lists, and Rust embeds its IDs when compiling the backend.

## Directory Structure

```text
shared/                       # Metadata consumed by both application layers + README assets
├── agents.catalog.json       # Builtin agents grouped by native, slim, and omo systems
├── app.png                   # App screenshot embedded by the root README
└── mom-logo.svg              # 120px README logo (copy of src-tauri/icons/app-icon.svg)
```

## Key Files

| Responsibility | File | Description |
| :--- | :--- | :--- |
| Builtin metadata | `agents.catalog.json` | JSON object containing an `agents` array; every entry has `id`, `system`, `type`, and `description`. |
| README assets | `app.png`, `mom-logo.svg` | Display-only assets referenced by the root `README.md` / `README.zh-CN.md`; not imported by app code. `mom-logo.svg` mirrors `src-tauri/icons/app-icon.svg` at 120px intrinsic size — regenerate it if the app icon changes. |

## Conventions

- **Entry shape**: Keep all four fields as strings. `system` is `native`, `slim`, or `omo`; `type` is `primary`, `subagent`, or `internal`; `description` is display text.
- **Frontend consumption**: `src/utils/index.ts` imports `../../shared/agents.catalog.json`, maps entries to `BUILTIN_AGENTS`, and derives `BUILTIN_AGENT_IDS` for `isBuiltinAgentId`.
- **Backend consumption**: `src-tauri/src/agents.rs` embeds the same file with `include_str!`, parses its `agents` IDs, and blocks physical deletion of those IDs.
- **Identity**: Treat `(system, id)` as the catalog entry identity; IDs such as `oracle`, `librarian`, and `explore` intentionally occur in multiple systems. ID-only sets collapse these duplicates.
- **Read-only metadata**: Keep per-user definitions and overrides outside this file; builtin UI tables expose metadata without edit/delete actions, and backend changes require rebuilding the embedded catalog.

## Anti-patterns

- Do not deduplicate the array by `id` alone: each system needs its own entry even when deletion-protection sets share the ID.
- Do not substitute agent mode values for catalog `type`: `internal` is a builtin role, while the separate agent-mode union includes `all`.

## Sync

This file is maintained as the directory scope changes. When files in this directory are added, modified, or deleted (components, logic units, utilities, constants, etc.), update this file's directory structure, key files, and conventions to keep the documentation in sync with the code.
