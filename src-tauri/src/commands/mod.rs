pub mod agent;
pub mod app;
pub mod autostart;
pub mod behavior;
pub mod custom_commands;
pub mod group;
pub(crate) mod island;
pub mod logging;
pub mod mcp_proxy;
pub mod model;
pub mod opencode;
pub mod plugins;
pub mod power;
pub mod provider;
pub mod resources;
pub mod updates;

use crate::error::AppError;
use crate::logging::{operation, LogType};
pub use crate::tray::*;
pub use agent::*;
pub use app::*;
pub use autostart::*;
pub use behavior::*;
pub use custom_commands::*;
pub use group::*;
pub(crate) use island::*;
pub(crate) use logging::*;
pub use mcp_proxy::*;
pub use model::*;
pub use opencode::*;
pub use plugins::*;
pub use power::*;
pub use provider::*;
pub use resources::*;
pub use updates::*;

fn critical_operation(label: &str) -> Option<(LogType, &'static str)> {
    Some(match label {
        "Save group" => (LogType::Group, "group_save"),
        "Copy group" => (LogType::Group, "group_copy"),
        "Delete group" => (LogType::Group, "group_delete"),
        "Create provider" => (LogType::Resource, "provider_create"),
        "Update provider" => (LogType::Resource, "provider_update"),
        "Delete provider" => (LogType::Resource, "provider_delete"),
        "Create model" => (LogType::Resource, "model_create"),
        "Update model" => (LogType::Resource, "model_update"),
        "Delete model" => (LogType::Resource, "model_delete"),
        "Create agent" => (LogType::Resource, "agent_create"),
        "Update agent" => (LogType::Resource, "agent_update"),
        "Delete agent" => (LogType::Resource, "agent_delete"),
        "Create MCP" => (LogType::Resource, "mcp_create"),
        "Update MCP" => (LogType::Resource, "mcp_update"),
        "Delete MCP" => (LogType::Resource, "mcp_delete"),
        "Create skill" => (LogType::Resource, "skill_create"),
        "Update skill" => (LogType::Resource, "skill_update"),
        "Create slash command" => (LogType::Resource, "slash_command_create"),
        "Update slash command" => (LogType::Resource, "slash_command_update"),
        "Delete slash command" => (LogType::Resource, "slash_command_delete"),
        "Save Global instructions" => (LogType::Config, "global_instructions_save"),
        "Add project" => (LogType::Config, "project_add"),
        "Remove project" => (LogType::Config, "project_remove"),
        "Select project" => (LogType::Config, "project_select"),
        "Save preferences" => (LogType::Config, "preferences_save"),
        "Update plugin package" => (LogType::Resource, "plugin_package_update"),
        "Plugin declaration removal" => (LogType::Resource, "plugin_declaration_remove"),
        "OpenCode service target" => (LogType::Opencode, "opencode_target_set"),
        "OpenCode update check" => (LogType::Opencode, "opencode_update_check"),
        "OpenCode upgrade" => (LogType::Opencode, "opencode_upgrade"),
        "OpenCode reload" => (LogType::Opencode, "opencode_reload"),
        "Check for updates" => (LogType::App, "app_update_check"),
        "Set autostart" => (LogType::App, "autostart_set"),
        "Set lid protection" => (LogType::Power, "lid_protection_set"),
        "Set Mini Island enabled" => (LogType::Island, "island_enabled_set"),
        "MCP proxy import" => (LogType::McpProxy, "mcp_proxy_import_prepare"),
        _ => return None,
    })
}

fn read_failure_operation(label: &str) -> Option<(LogType, &'static str)> {
    Some(match label {
        "Load app state" => (LogType::Config, "app_state_load"),
        "Provider discovery" => (LogType::Resource, "provider_list"),
        "Model discovery" => (LogType::Resource, "model_list"),
        "Agent discovery" => (LogType::Resource, "agent_list"),
        "Read agent" => (LogType::Resource, "agent_read"),
        "MCP discovery" => (LogType::Resource, "mcp_list"),
        "Read MCP" => (LogType::Resource, "mcp_read"),
        "Skill discovery" => (LogType::Resource, "skill_list"),
        "Read skill" => (LogType::Resource, "skill_read"),
        "Slash-command discovery" => (LogType::Resource, "slash_command_list"),
        "Read slash command" => (LogType::Resource, "slash_command_read"),
        "Read Global instructions" => (LogType::Config, "global_instructions_read"),
        "Plugin runtime" => (LogType::Resource, "plugin_runtime_list"),
        "Plugin removal preview" => (LogType::Resource, "plugin_removal_preview"),
        "OpenCode model catalog" => (LogType::Opencode, "opencode_model_catalog"),
        "OpenCode installation" => (LogType::Opencode, "opencode_installation_read"),
        "Open project page" => (LogType::App, "project_page_open"),
        _ => return None,
    })
}

pub(crate) async fn spawn_blocking_task<T, F>(label: &'static str, task: F) -> Result<T, AppError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, AppError> + Send + 'static,
{
    let critical = critical_operation(label);
    let tracked = critical
        .or_else(|| read_failure_operation(label))
        .or_else(|| {
            (label == "Switch group").then_some((LogType::Group, "group_switch_worker_failed"))
        })
        .map(|(kind, event)| std::sync::Arc::new(operation(kind, event)));
    let worker_operation = tracked.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = task();
        if let Some(operation) = worker_operation
            .filter(|_| critical.is_some() || (label != "Switch group" && result.is_err()))
        {
            operation.finish(&result);
        }
        result
    })
    .await
    .map_err(|_| {
        let error = AppError::configuration(format!("{label} worker unavailable"));
        if let Some(operation) = tracked {
            operation.finish::<()>(&Err(error.clone()));
        }
        error
    })?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logging_allowlist_excludes_reads_polls_and_domain_owned_switch() {
        assert!(critical_operation("Save group").is_some());
        assert!(critical_operation("Switch group").is_none());
        assert!(critical_operation("Load app state").is_none());
        assert!(read_failure_operation("Load app state").is_some());

        for label in [
            "OpenCode service status",
            "OpenCode active sessions",
            "OpenCode token usage",
            "Read lid protection",
            "Read Mini Island state",
            "Read autostart",
            "Set Mini Island view",
            "unknown-user-supplied-label",
        ] {
            assert!(critical_operation(label).is_none());
            assert!(read_failure_operation(label).is_none());
        }

        for label in [
            "Logging preference",
            "Query logs",
            "Open logs",
            "Clear logs",
        ] {
            assert!(critical_operation(label).is_none());
            assert!(read_failure_operation(label).is_none());
        }
    }

    #[test]
    fn blocking_task_runs_off_the_caller_thread_and_preserves_results() {
        let caller = std::thread::current().id();
        tauri::async_runtime::block_on(async {
            let worker = spawn_blocking_task("Thread check", || Ok(std::thread::current().id()))
                .await
                .unwrap();
            assert_ne!(worker, caller);

            let expected = AppError::validation("invalid input");
            let error = expected.clone();
            assert_eq!(
                spawn_blocking_task::<(), _>("Save group", move || Err(error)).await,
                Err(expected)
            );
        });
    }

    #[test]
    fn blocking_task_maps_worker_panics_to_configuration_errors() {
        let result =
            tauri::async_runtime::block_on(spawn_blocking_task::<(), _>("Save group", || {
                panic!("worker failed")
            }));
        assert_eq!(
            result,
            Err(AppError::configuration("Save group worker unavailable"))
        );
    }

    #[test]
    fn cancelling_a_waiter_does_not_cancel_its_blocking_mutation() {
        let (entered, started) = std::sync::mpsc::channel();
        let (release, proceed) = std::sync::mpsc::channel();
        let (completed, result) = std::sync::mpsc::channel();
        tauri::async_runtime::block_on(async {
            let waiter = tokio::spawn(spawn_blocking_task("Save group", move || {
                entered.send(()).unwrap();
                proceed.recv().unwrap();
                completed.send(()).unwrap();

                Ok(())
            }));
            started
                .recv_timeout(std::time::Duration::from_secs(2))
                .unwrap();
            waiter.abort();
            assert!(waiter.await.unwrap_err().is_cancelled());
            release.send(()).unwrap();
            result
                .recv_timeout(std::time::Duration::from_secs(2))
                .unwrap();
        });
    }
}
