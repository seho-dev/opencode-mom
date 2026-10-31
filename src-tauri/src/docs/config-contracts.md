# Configuration and Resource Contracts

Paths in this table are relative to `src-tauri/src/`.

## Key Files

| Responsibility | File | Description |
| :--- | :--- | :--- |
| Error contract | `error.rs` | AppError serializes into frontend CommandError; ErrorCode uses snake_case. |
| Document editing | `document.rs`, `jsonc.rs` | JsoncDoc retains source text and raw values; CST path patches preserve untouched comments and formatting. |
| Path authority | `paths.rs` | Resolves selected config, ordered resource aliases, absolute/home skill paths, and temporary exact-root project scopes. |
| Group lifecycle | `groups.rs`, `projection.rs` | Validates group drafts, updates selection, and applies target-specific mappings. |
| Deletion protection | `refs.rs` | Indexes agent/model references, including persisted groups and plugin mappings. |
| File safety | `resource_file.rs` | Validates snapshots and aliases, stages replacements, and retains recovery backups on failure. |
| Agent documents | `agents.rs`, `agents_md.rs` | Merges inline/Markdown sources and edits selected storage while retaining untouched frontmatter sections. |
| Provider/model editing | `providers.rs`, `models.rs` | ModelRef identity, field-level config edits, and persisted app data types. |
| Resource management | `mcp.rs`, `skills.rs` | Ordered global discovery, provenance, diagnostics, and guarded MCP/local skill writes. |

## Conventions

- **App config writes**: Share `models::config_write_lock` across Island/preferences/group/project read-modify-write. `save_config` serializes before sibling staging/sync/atomic rename without `.bak`; sync the Unix directory, warning rather than claiming rollback after commit. Preserve unrelated selection, symlink targets/modes, and existing general target-write behavior.
- **Path precedence**: Ordered sources retain the last duplicate; resolve overrides against captured cwd. Scan singular before plural agent/skill aliases, including config-dir overrides, never mode/modes; ignore configured relative skills without filesystem access, retaining Global absolute/home/HTTP and .claude/.agents roots.
- **Projects**: Persist canonical UTF-8 paths/UUIDs, not names; reject filesystem roots and canonical overlaps with captured Global fixed JSON/agent/skill sources, including overrides, without a home/depth blacklist or symlink-race guarantee. Exact-root ConfigPaths clones ignore project skills arrays and preserve Global paths/cwd; missing roots return `not_found`, unreadable/retargeted roots fail without fallback. Reject all project resource mutations; omitted/null scope stays Global regardless of selection, and removal only forgets metadata.
- **Agent provenance**: Keep every actual scoped inline JSON path and Global selected-config behavior; previews return logical `path`, `resolvedPath`, and backend-generated inline JSON/full Markdown `content`. Updates require unchanged `expectedPath`/`expectedContent` echoed through JavaScript, never numeric normalization; reject missing (`validation_failed`) or stale snapshots. Leave create unchanged; reuse Markdown aliases, reject distinct canonical targets, and support static Global symlinks.
- **Error serialization**: Preserve `{ code, message, detail? }`; absent detail is omitted. `AppError::io` includes operation/path context without embedding file contents.
- **JSONC edits**: Use `JsoncDoc::patch` for target mutations; `None` removes a path. Object roots, comments, and trailing commas are supported; invalid documents fail rather than being overwritten.
- **Projection boundaries**: Native projection changes existing `agents.<name>.model` only and warns for missing agents; variants join as `provider/model#variant`. OMO mappings live under `opencode.agents` and `opencode.categories`.
- **Group deletion**: `delete_group` removes only app data and clears a deleted active selection; target files and their residual mappings remain untouched.
- **Reference guards**: Provider/model IPC deletes query `refs` before CRUD; agent deletes receive `collect_agent_references`. The model index includes inline/Markdown agents, app groups, Slim presets, and OMO mappings.
- **Backup-before-replace**: Existing-file inline agent/Markdown/MCP/local-skill updates use `resource_file::replace`: validate snapshots, back up to recovery `.bak` before publication, preserve modes, and remove owned backups only on success. New agents use checked creation; `document::write_file`/`JsoncDoc::save` remain plain writes without backups/locking.
- **Resource freshness**: Compare expected agent/MCP/skill source and content/config; revalidate aliases/identity/content and reject detected retargets. Validate effective patched MCP servers against duplicate-key mismatches; retain the last-check/rename race and per-file atomicity, never claim arbitrary-file security against same-account writers.
