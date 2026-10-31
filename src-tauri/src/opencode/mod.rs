mod active;
mod binary;
mod command;
pub(crate) mod generate;
pub(crate) mod installation;
pub(crate) mod island;
mod models;
mod reload;
pub(crate) mod target;
mod token_usage;
pub(crate) mod transport;

pub use active::active_sessions_via_service;
pub use binary::resolve_opencode_binary;
pub use installation::{OpenCodeInstallation, OpenCodeUpdateCheck};
pub use models::{list_models_via_service, ModelCatalogEntry};
pub use reload::reload_via_service;
pub use target::{OpenCodeServiceStatus, OpenCodeServiceTarget};
pub use token_usage::{token_usage_records_via_service, TokenUsageRecord};
