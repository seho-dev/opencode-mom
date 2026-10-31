# Shared Builtin Agent Catalog

## Overview

Holds the shipped builtin-agent catalog shared by frontend presentation and backend deletion
protection. This read-only runtime metadata is not user-editable agent definitions: the frontend
imports the catalog for builtin lists, and Rust embeds IDs at compile time.

## Directory Structure

```text
shared/
├── agents.catalog.json # Builtins grouped by native, slim, omo
├── app.png             # Root README screenshot
└── mom-logo.svg        # README logo
```

## Key Files

| Responsibility | File | Description |
| :--- | :--- | :--- |
| Builtin metadata | `agents.catalog.json` | JSON object containing an `agents` array; every entry has `id`, `system`, `type`, and `description`. |
| README assets | `app.png`, `mom-logo.svg` | Display-only assets referenced by the canonical root `README.md`, not imported by app code. |

## Conventions

- **Entry shape**: Keep all four fields as strings. `system` is `native`, `slim`, or `omo`; `type` is
  `primary`, `subagent`, or `internal`; `description` is display text.
- **Frontend consumption**: `src/utils/index.ts` imports `../../shared/agents.catalog.json`, maps entries
  to `BUILTIN_AGENTS`, and derives `BUILTIN_AGENT_IDS` for `isBuiltinAgentId`.
- **Backend consumption**: `src-tauri/src/agents.rs` embeds the same file with `include_str!`, parses its
  `agents` IDs, and blocks physical deletion of those IDs.
- **Identity**: Treat `(system, id)` as the catalog entry identity; IDs such as `oracle`, `librarian`, and
  `explore` intentionally occur in multiple systems. ID-only sets collapse these duplicates.
- **Read-only metadata**: Keep per-user definitions and overrides outside this file; builtin UI tables
  expose metadata without edit/delete actions, and backend changes require rebuilding the embedded catalog.
- **README logo**: `mom-logo.svg` mirrors `src-tauri/icons/app-icon.svg` at 120px intrinsic size;
  regenerate the README logo whenever the app icon changes.

## Anti-patterns

- Do not deduplicate the array by `id` alone: each system needs its own entry even when
  deletion-protection sets share the ID.
- Do not substitute agent mode values for catalog `type`: `internal` is a builtin role, while the
  separate agent-mode union includes `all`.

## Sync

Keep this guide and linked docs in sync with changes to this directory.
