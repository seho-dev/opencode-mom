pub mod agent;
pub mod app;
pub mod autostart;
pub mod cli;
pub mod group;
pub(crate) mod island;
pub mod model;
pub mod power;
pub mod provider;
pub mod resources;
pub mod updates;

use crate::error::AppError;
pub use crate::tray::*;
pub use agent::*;
pub use app::*;
pub use autostart::*;
pub use cli::*;
pub use group::*;
pub(crate) use island::*;
pub use model::*;
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
