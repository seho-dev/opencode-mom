# Tauri Commands

## Overview

Domain-scoped Tauri IPC handlers translate frontend requests into backend operations for agents, app state, autostart, CLI, groups, models, power, providers, MCP, and skills. Backend modules own the underlying operations; handlers adapt DTOs, validate command inputs, and send window notifications.

## Directory Structure

```text
commands/              # IPC handler boundary
├── AGENTS.md           # Directory-specific instructions
├── mod.rs              # Domain exports and tray handler re-exports
├── agent.rs            # Agent DTO conversion, CRUD, and model validation
├── app.rs              # Aggregate app state and preference persistence
├── app/                # App command tests
│   └── tests.rs        # AppStateResponse serialization regression test
├── autostart.rs        # Read and toggle platform autostart
├── cli.rs              # OpenCode CLI discovery, reload, and usage queries
├── group.rs            # Save, copy, delete, and switch groups
├── model.rs            # Provider-scoped model CRUD and reference checks
├── power.rs            # Lid protection through managed PowerService
├── provider.rs         # Provider CRUD and custom-provider filtering
└── resources.rs        # MCP and skill list, get, and mutation handlers
```

## Key Files

| Responsibility | File | Description |
| :--- | :--- | :--- |
| Command exports | `mod.rs` | Declares nine domain modules and re-exports their handlers plus `crate::tray::*`. |
| Agent boundary | `agent.rs` | Converts source/effective data to DTOs; selects storage and validates model references. |
| App state | `app.rs` | Returns `AppStateResponse`; saves preferences and notifies the tray. |
| Serialization test | `app/tests.rs` | Checks `selectedGroupId` naming and nullable serialization. |
| Autostart | `autostart.rs` | Uses the autostart plugin on macOS, Windows, and Linux; rejects unsupported platforms. |
| CLI bridge | `cli.rs` | Delegates catalog, binary, reload, active-session, and token-usage operations to `opencode_cli`. |
| Group mutations | `group.rs` | Delegates persistence/projection to `groups`; tray switches notify the main window. |
| Model mutations | `model.rs` | Parses `ModelRef` and blocks deletion when references remain. |
| Power boundary | `power.rs` | Delegates reads and toggles to managed `PowerService`. |
| Provider mutations | `provider.rs` | Lists all/custom providers and checks references before deletion. |
| Resource boundary | `resources.rs` | Delegates MCP operations directly and skill operations through blocking tasks. |

## Conventions

- **Domains**: Keep handlers in their resource module; delegate persistence and system operations to the existing backend modules.
- **Exposure**: Use snake_case `#[tauri::command]` functions, re-export through `mod.rs`, and add `commands::name` to `../lib.rs`'s `invoke_handler` list.
- **Managed state**: Config-backed handlers accept `State<'_, ConfigPaths>`; power handlers accept `State<'_, PowerService>` instead.
- **Reads and mutations**: `load_app_state`, `list_*`, and `get_*` expose reads; `create_*`, `update_*`, `delete_*`, `save_*`, `copy_group`, `switch_group`, and `set_*` perform mutations. `list_models` returns provider-grouped `Vec<ProviderDef>`.
- **Command results**: Return `Result<T, AppError>` directly or via local `CommandResult<T>`. Serialization yields frontend `CommandError` fields `{ code, message, detail? }`; codes are `not_found`, `references_blocked`, `validation_failed`, and `configuration_failed`.
- **Agent inputs**: Creation/update require explicit storage and consume `mutation`; validate nonempty model references against existing provider models. Deletion may infer storage only for an unambiguous source.
- **Deletion guards**: Collect agent references before agent deletion; reject referenced models/providers with `AppError::references` and location details before deleting them.
- **Wire DTOs**: `AgentDefinitionDto`, `AgentSourcePreviewDto`, and `AppStateResponse` serialize fields as camelCase; `AgentStorageDto` serializes variants as snake_case.
- **Notifications**: Emit `app:preferences-changed` to `tray` after saving preferences; tray-origin group switches emit `tray:config-changed` to `main`. Log notification failures without failing a completed mutation.
- **Blocking work**: Async CLI reload/session/usage and skill handlers use `tauri::async_runtime::spawn_blocking`; map join failures to configuration errors.
- **Tests**: `app.rs` includes `app/tests.rs` via `#[cfg(test)] mod tests` for response serialization. Filesystem workflow coverage in `../../tests/app.rs` uses temporary homes with `ConfigPaths::for_home`.

## Sync

This file is maintained as the directory scope changes. When files in this directory are added, modified, or deleted (components, logic units, utilities, constants, etc.), update this file's directory structure, key files, and conventions to keep the documentation in sync with the code.
