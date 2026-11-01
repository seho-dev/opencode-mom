# Command Boundary and Resource Contracts

Paths in this table are relative to `src-tauri/src/commands/`.

## Key Files

| Responsibility | File | Description |
| :--- | :--- | :--- |
| Command exports | `mod.rs` | Declares domain modules and re-exports their handlers plus `crate::tray::*`. |
| Agent boundary | `agent.rs` | Converts source/effective data to DTOs; selects storage and validates model references. |
| App state | `app.rs` | Returns `AppStateResponse`; saves preferences and project metadata; runs the native folder chooser in spawn_blocking. |
| App command tests | `app/tests.rs` | Checks snapshot serialization and rejects Global-overlapping project adds before persistence through mock IPC, without opening the chooser. |
| Autostart | `autostart.rs` | Uses the autostart plugin on macOS, Windows, and Linux; rejects unsupported platforms. |
| Group mutations | `group.rs` | Delegates persistence/projection to `groups`; tray switches notify the main window. |
| Model mutations | `model.rs` | Parses `ModelRef` and blocks deletion when references remain. |
| Power boundary | `power.rs` | Delegates reads and toggles to managed `PowerService`. |
| Provider mutations | `provider.rs` | Lists providers and checks references before deletion. |
| Resource boundary | `resources.rs` | Delegates MCP operations directly and skill operations through blocking tasks. |
| Update boundary | `updates.rs` | Reads `AppHandle.package_info()` for the actual version; delegates manual release checks and fixed-destination browser opens. |

## Conventions

- **Domains**: Keep handlers in their resource module; delegate persistence and system operations to the existing backend modules.
- **Exposure**: Use snake_case `#[tauri::command]` functions, re-export through `mod.rs`, and add `commands::name` to `../lib.rs`'s `invoke_handler` list.
- **Managed state**: Config-backed handlers accept `State<'_, ConfigPaths>`; power handlers accept `State<'_, PowerService>` instead.
- **Resource scope**: Reads accept optional projectId; omitted/null stays Global regardless of selection. Reuse ConfigPaths::for_project/domain functions; reject any provided projectId for mutations before reads/writes, with no project edit API.
- **Project folders**: pick_project_folder uses only the Rust dialog plugin and returns null on cancellation. add_project validates canonical roots against captured Global boundaries before models.rs persistence; scoped reads recheck fixed sources, rejecting roots/overlaps with `validation_failed` and missing roots with `not_found`, never Global fallback or home/depth blacklists. Add/remove/select share AppConfig locking; include projects/selectedProjectId in AppStateResponse, normalize stale selection to null, and require no frontend dialog/fs capability.
- **Reads and mutations**: `load_app_state`, `list_*`, and `get_*` expose reads; `create_*`, `update_*`, `delete_*`, `save_*`, `copy_group`, `switch_group`, and `set_*` perform mutations. `list_models` returns provider-grouped `Vec<ProviderDef>`.
- **Command results**: Return `Result<T, AppError>` or local `CommandResult<T>`; serialize `{ code, message, detail? }` as frontend CommandError. Keep `not_found`, `references_blocked`, `validation_failed`, `configuration_failed`, and guarded upgrade/service-target `busy` codes.
- **Agent inputs**: Create/update require explicit storage, `mutation`, and nonempty model references validated against existing provider models. Updates echo backend `sources[].resolvedPath`/`content` as `expectedPath`/`expectedContent`, never JavaScript-stringified `raw`; reject missing (`validation_failed`) or stale snapshots. Leave create unchanged; infer deletion storage only for an unambiguous source.
- **Deletion guards**: Collect agent references before agent deletion; reject referenced models/providers with `AppError::references` and location details before deleting them.
- **Wire DTOs**: `AgentDefinitionDto`, `AgentSourcePreviewDto`, and `AppStateResponse` serialize fields as camelCase; `AgentStorageDto` serializes variants as snake_case.
- **Notifications**: Emit `app:preferences-changed` to `tray` and `island` after saving preferences; tray-origin group switches emit `tray:config-changed` to `main`. Log notification failures without failing a completed mutation.
- **Blocking work**: Async OpenCode catalog/service-status/target-save/installation/update/upgrade/reload/session/usage, skill, update-check, and project-opener handlers use `tauri::async_runtime::spawn_blocking`; map join failures to configuration errors.
- **Tests**: `app.rs` includes `app/tests.rs` via `#[cfg(test)] mod tests` for serialization/mock project-add guards; `../../tests/app.rs` uses temporary ConfigPaths::for_home for filesystem/agent-snapshot/JavaScript-roundtrip cases, never the native chooser.
