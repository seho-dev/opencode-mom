pub(crate) mod auth;
pub(crate) mod config;
pub(crate) mod http;
pub(crate) mod process;
pub(crate) mod session;

#[cfg(test)]
mod tests;

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{Mutex, RwLock};
use tokio_util::sync::CancellationToken;

use crate::error::AppError;
use crate::mcp_proxy::config::{GatewaySettings, ProxyConfig};
use crate::mcp_proxy::process::OwnerBudget;
use crate::mcp_proxy::session::{LegacySessionManager, Observation};
use crate::paths::ConfigPaths;

#[allow(dead_code)]
#[derive(Clone)]
pub struct ProxyService {
    config_path: PathBuf,
    config: Arc<RwLock<ProxyConfig>>,
    budget: OwnerBudget,
    legacy_sessions: LegacySessionManager,
    shutdown_token: Arc<Mutex<CancellationToken>>,
    generation: Arc<AtomicU64>,
    events: Option<tokio::sync::mpsc::UnboundedSender<Observation>>,
    #[cfg(test)]
    publish_hook: Option<Arc<dyn Fn() + Send + Sync>>,
}

#[allow(dead_code)]
impl ProxyService {
    pub fn new(paths: &ConfigPaths) -> Result<Self, AppError> {
        let path = paths.app_config_dir().join("mcp-proxy.json");
        Self::new_with_path(path)
    }

    pub fn new_with_path(config_path: PathBuf) -> Result<Self, AppError> {
        let config = config::load_config(&config_path)?;
        Ok(Self::from_config(config_path, config))
    }

    pub fn new_disabled(paths: &ConfigPaths) -> Self {
        Self::from_config(
            paths.app_config_dir().join("mcp-proxy.json"),
            ProxyConfig::default(),
        )
    }

    fn from_config(config_path: PathBuf, config: ProxyConfig) -> Self {
        let max_units = config.gateway.max_units;
        Self {
            config_path,
            config: Arc::new(RwLock::new(config)),
            budget: OwnerBudget::new(max_units),
            legacy_sessions: LegacySessionManager::new(),
            shutdown_token: Arc::new(Mutex::new(CancellationToken::new())),
            generation: Arc::new(AtomicU64::new(1)),
            events: None,
            #[cfg(test)]
            publish_hook: None,
        }
    }

    pub async fn config(&self) -> ProxyConfig {
        self.config.read().await.clone()
    }

    pub fn router(&self) -> axum::Router {
        http::create_router(self.clone())
    }

    pub fn budget(&self) -> &OwnerBudget {
        &self.budget
    }

    pub fn cleanup_failed(&self) -> bool {
        self.budget.cleanup_failed()
    }

    pub fn legacy_sessions(&self) -> &LegacySessionManager {
        &self.legacy_sessions
    }

    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }

    pub(crate) async fn request_token(&self) -> CancellationToken {
        self.shutdown_token.lock().await.clone()
    }

    pub async fn rotate_key(&self) -> Result<String, AppError> {
        let service = self.clone();
        tokio::spawn(async move {
            let result = service.rotate_key_owned().await;
            if let Err(error) = &result {
                eprintln!("warning: MCP proxy key rotation failed: {error}");
            }
            result
        })
        .await
        .map_err(|error| {
            AppError::configuration(format!("MCP proxy key rotation task failed: {error}"))
        })?
    }

    async fn rotate_key_owned(&self) -> Result<String, AppError> {
        let key = auth::generate_key();
        let mut config_guard = self.config.write().await;
        let mut updated = config_guard.clone();
        updated.gateway.token_verifier = Some(key.verifier);
        updated.revision = updated.revision.saturating_add(1);

        let path = self.config_path.clone();
        #[cfg(test)]
        let publish_hook = self.publish_hook.clone();
        let updated = tokio::task::spawn_blocking(move || {
            #[cfg(test)]
            if let Some(hook) = publish_hook {
                hook();
            }
            config::save_config(&path, &updated)?;
            Ok::<_, AppError>(updated)
        })
        .await
        .map_err(|error| {
            AppError::configuration(format!("save MCP proxy key task failed: {error}"))
        })??;
        let mut token = self.shutdown_token.lock().await;
        token.cancel();
        *token = CancellationToken::new();
        *config_guard = updated;
        self.generation.fetch_add(1, Ordering::SeqCst);
        drop(token);

        if let Err(error) = self.legacy_sessions.close_all().await {
            eprintln!("warning: MCP proxy key saved; session cleanup failed: {error}");
        }

        Ok(key.raw_token)
    }

    pub async fn save_gateway_settings(
        &self,
        settings: GatewaySettings,
        expected_revision: u64,
    ) -> Result<(), AppError> {
        let mut updated = self.config().await;
        updated.gateway = settings;
        self.save_config(updated, expected_revision).await
    }

    pub async fn save_config(
        &self,
        new_config: ProxyConfig,
        expected_revision: u64,
    ) -> Result<(), AppError> {
        let service = self.clone();
        tokio::spawn(async move {
            let result = service
                .save_config_owned(new_config, expected_revision)
                .await;
            if let Err(error) = &result {
                eprintln!("warning: MCP proxy configuration update failed: {error}");
            }
            result
        })
        .await
        .map_err(|error| {
            AppError::configuration(format!("MCP proxy configuration task failed: {error}"))
        })?
    }

    async fn save_config_owned(
        &self,
        mut new_config: ProxyConfig,
        expected_revision: u64,
    ) -> Result<(), AppError> {
        let mut config_guard = self.config.write().await;
        if config_guard.revision != expected_revision {
            return Err(AppError::validation(
                "proxy configuration revision conflict",
            ));
        }

        new_config.revision = config_guard.revision.saturating_add(1);
        let path = self.config_path.clone();
        #[cfg(test)]
        let publish_hook = self.publish_hook.clone();
        let new_config = tokio::task::spawn_blocking(move || {
            #[cfg(test)]
            if let Some(hook) = publish_hook {
                hook();
            }
            config::save_config(&path, &new_config)?;
            Ok::<_, AppError>(new_config)
        })
        .await
        .map_err(|error| {
            AppError::configuration(format!("save MCP proxy configuration task failed: {error}"))
        })??;
        self.budget.set_limit(new_config.gateway.max_units);
        let disabling = config_guard.gateway.enabled && !new_config.gateway.enabled;
        let changed_services: Vec<_> = config_guard
            .services
            .iter()
            .filter(|previous| {
                new_config
                    .services
                    .iter()
                    .find(|service| service.id == previous.id)
                    != Some(*previous)
            })
            .map(|service| service.id.clone())
            .collect();
        if disabling {
            let mut token = self.shutdown_token.lock().await;
            token.cancel();
            *token = CancellationToken::new();
        }
        *config_guard = new_config;
        self.generation.fetch_add(1, Ordering::SeqCst);
        if disabling {
            if let Err(error) = self.legacy_sessions.close_all().await {
                eprintln!(
                    "warning: MCP proxy configuration saved; session cleanup failed: {error}"
                );
            }
        } else {
            for id in changed_services {
                if let Err(error) = self.legacy_sessions.close_service_sessions(&id).await {
                    eprintln!(
                        "warning: MCP proxy configuration saved; session cleanup failed: {error}"
                    );
                }
            }
        }

        Ok(())
    }

    pub async fn shutdown(&self, deadline: Duration) -> Result<(), AppError> {
        let service = self.clone();
        tokio::spawn(async move { service.shutdown_owned(deadline).await })
            .await
            .map_err(|error| {
                AppError::configuration(format!("MCP proxy shutdown task failed: {error}"))
            })?
    }

    async fn shutdown_owned(&self, deadline: Duration) -> Result<(), AppError> {
        self.shutdown_token.lock().await.cancel();
        let mut cleanup = None;
        let wait = tokio::time::timeout(deadline, async {
            cleanup = Some(self.legacy_sessions.close_all().await);
            self.budget.wait_idle().await;
        })
        .await
        .map_err(|_| AppError::configuration("timed out waiting for MCP proxy shutdown"));
        let result = session::finish_cleanup(cleanup.into_iter().chain([wait]));
        if result.is_err() {
            self.budget.record_cleanup_failure();
        }
        result?;
        self.generation.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}
