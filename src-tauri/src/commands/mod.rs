pub mod agent;
pub mod app;
pub mod autostart;
pub mod behavior;
pub mod custom_commands;
pub mod group;
pub(crate) mod island;
pub mod mcp_proxy;
pub mod model;
pub mod opencode;
pub mod power;
pub mod provider;
pub mod resources;
pub mod updates;

use crate::error::AppError;
pub use crate::tray::*;
pub use agent::*;
pub use app::*;
pub use autostart::*;
pub use behavior::*;
pub use custom_commands::*;
pub use group::*;
pub(crate) use island::*;
pub use mcp_proxy::*;
pub use model::*;
pub use opencode::*;
pub use power::*;
pub use provider::*;
pub use resources::*;
pub use updates::*;

pub(crate) async fn spawn_blocking_task<T, F>(
    operation: &'static str,
    task: F,
) -> Result<T, AppError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, AppError> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|_| AppError::configuration(format!("{operation} worker unavailable")))?
}

#[cfg(test)]
mod tests {
    use super::*;

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
                spawn_blocking_task::<(), _>("Failure check", move || Err(error)).await,
                Err(expected)
            );
        });
    }

    #[test]
    fn blocking_task_maps_worker_panics_to_configuration_errors() {
        let result =
            tauri::async_runtime::block_on(spawn_blocking_task::<(), _>("Panic check", || {
                panic!("worker failed")
            }));
        assert_eq!(
            result,
            Err(AppError::configuration("Panic check worker unavailable"))
        );
    }
}
