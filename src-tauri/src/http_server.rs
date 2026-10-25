use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::{Extension, Router};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::error::AppError;
use crate::paths::ConfigPaths;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct MomHttpSettings {
    pub port: u16,
    pub allow_lan: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test(flavor = "current_thread")]
    async fn cancelled_http_transition_waiter_still_publishes_and_serves() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("mom-http.json");
        let http = Arc::new(HttpServer::new_with_path(path.clone(), Router::new()).unwrap());
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let settings = MomHttpSettings {
            port: listener.local_addr().unwrap().port(),
            allow_lan: false,
        };
        drop(listener);
        let guard = http.runtime.lock().await;
        let setter = http.clone();
        let waiter = tokio::spawn(async move { setter.set_settings(settings).await });
        tokio::task::yield_now().await;
        waiter.abort();
        assert!(waiter.await.unwrap_err().is_cancelled());
        drop(guard);
        tokio::time::timeout(Duration::from_secs(1), async {
            while http.bound_addr().await.is_none() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("owned HTTP transition must outlive its waiter");
        assert_eq!(http.settings().await, settings);
        assert_eq!(
            serde_json::from_slice::<MomHttpSettings>(&std::fs::read(&path).unwrap()).unwrap(),
            settings
        );
        http.shutdown(Duration::from_secs(2)).await.unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn single_worker_transition_offloads_io_and_survives_cancel() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("mom-http.json");
        let router = Router::new().route("/health", axum::routing::get(|| async { "ok" }));
        let mut http = HttpServer::new_with_path(path.clone(), router).unwrap();
        let (entered_tx, mut entered_rx) = tokio::sync::mpsc::unbounded_channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let release_rx = std::sync::Mutex::new(release_rx);
        let runtime_thread = std::thread::current().id();
        http.publish_hook = Some(Arc::new(move || {
            assert_ne!(
                runtime_thread,
                std::thread::current().id(),
                "filesystem work must not run on the async worker"
            );
            entered_tx.send(()).unwrap();
            release_rx.lock().unwrap().recv().unwrap();
        }));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let settings = MomHttpSettings {
            port: listener.local_addr().unwrap().port(),
            allow_lan: false,
        };
        drop(listener);
        let setter = http.clone();
        let waiter = tokio::spawn(async move { setter.set_settings(settings).await });
        tokio::time::timeout(Duration::from_secs(1), entered_rx.recv())
            .await
            .unwrap()
            .unwrap();
        waiter.abort();
        assert!(waiter.await.unwrap_err().is_cancelled());
        assert!(http.runtime.try_lock().is_err());
        release_tx.send(()).unwrap();
        assert_eq!(http.settings().await, settings);
        let addr = http.bound_addr().await.unwrap();
        assert_eq!(
            serde_json::from_slice::<MomHttpSettings>(&std::fs::read(&path).unwrap()).unwrap(),
            settings
        );
        assert_eq!(
            reqwest::Client::new()
                .get(format!("http://{addr}/health"))
                .send()
                .await
                .unwrap()
                .status(),
            reqwest::StatusCode::OK
        );
        http.shutdown(Duration::from_secs(2)).await.unwrap();
    }
}

impl Default for MomHttpSettings {
    fn default() -> Self {
        Self {
            port: 18765,
            allow_lan: false,
        }
    }
}

impl MomHttpSettings {
    fn validate(self) -> Result<(), AppError> {
        if self.port == 0 {
            return Err(AppError::validation("MOM HTTP port must be in 1..=65535"));
        }
        Ok(())
    }
}

#[derive(Clone)]
pub(crate) struct HttpContext {
    pub settings: MomHttpSettings,
    pub shutdown_token: CancellationToken,
}

struct Runtime {
    settings: MomHttpSettings,
    bound_addr: Option<SocketAddr>,
    listener_handle: Option<JoinHandle<()>>,
    shutdown_token: CancellationToken,
}

#[derive(Clone)]
pub struct HttpServer {
    config_path: PathBuf,
    router: Router,
    runtime: Arc<Mutex<Runtime>>,
    shutdown_token: CancellationToken,
    #[cfg(test)]
    publish_hook: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl HttpServer {
    pub fn new(paths: &ConfigPaths, router: Router) -> Result<Self, AppError> {
        Self::new_with_path(paths.app_config_dir().join("mom-http.json"), router)
    }

    pub(crate) fn new_with_path(config_path: PathBuf, router: Router) -> Result<Self, AppError> {
        let settings: MomHttpSettings = match crate::private_file::read(&config_path, 8192)
            .map_err(|error| {
                AppError::configuration(format!("read MOM HTTP settings: {error:?}"))
            })? {
            Some(bytes) => serde_json::from_slice(&bytes).map_err(|error| {
                AppError::configuration(format!("invalid MOM HTTP settings: {error}"))
            })?,
            None => MomHttpSettings::default(),
        };
        settings.validate()?;
        Ok(Self {
            config_path,
            router,
            runtime: Arc::new(Mutex::new(Runtime {
                settings,
                bound_addr: None,
                listener_handle: None,
                shutdown_token: CancellationToken::new(),
            })),
            shutdown_token: CancellationToken::new(),
            #[cfg(test)]
            publish_hook: None,
        })
    }

    pub async fn settings(&self) -> MomHttpSettings {
        self.runtime.lock().await.settings
    }

    pub async fn bound_addr(&self) -> Option<SocketAddr> {
        let runtime = self.runtime.lock().await;
        runtime
            .listener_handle
            .as_ref()
            .filter(|handle| !handle.is_finished())
            .and(runtime.bound_addr)
    }

    pub async fn start(&self) -> Result<SocketAddr, AppError> {
        let mut runtime = self.runtime.lock().await;
        if self.shutdown_token.is_cancelled() {
            return Err(AppError::configuration("MOM HTTP service is shutting down"));
        }
        if runtime
            .listener_handle
            .as_ref()
            .is_some_and(|handle| !handle.is_finished())
        {
            return Ok(runtime.bound_addr.expect("running HTTP listener address"));
        }
        let listener = Self::bind(runtime.settings).await?;
        self.serve(&mut runtime, listener)
    }

    async fn bind(settings: MomHttpSettings) -> Result<TcpListener, AppError> {
        let host = if settings.allow_lan {
            "0.0.0.0"
        } else {
            "127.0.0.1"
        };
        TcpListener::bind((host, settings.port))
            .await
            .map_err(|error| AppError::configuration(format!("bind MOM HTTP listener: {error}")))
    }

    fn serve(&self, runtime: &mut Runtime, listener: TcpListener) -> Result<SocketAddr, AppError> {
        let addr = listener.local_addr().map_err(|error| {
            AppError::configuration(format!("read MOM HTTP listener address: {error}"))
        })?;
        if self.shutdown_token.is_cancelled() {
            return Err(AppError::configuration("MOM HTTP service is shutting down"));
        }
        let shutdown_token = self.shutdown_token.child_token();
        let router = self.router.clone().layer(Extension(HttpContext {
            settings: runtime.settings,
            shutdown_token: shutdown_token.clone(),
        }));
        let stop = shutdown_token.clone();
        runtime.listener_handle = Some(tokio::spawn(async move {
            if let Err(error) = axum::serve(listener, router)
                .with_graceful_shutdown(stop.cancelled_owned())
                .await
            {
                eprintln!("warning: MOM HTTP listener failed: {error}");
            }
        }));
        runtime.shutdown_token = shutdown_token;
        runtime.bound_addr = Some(addr);
        Ok(addr)
    }

    pub async fn set_settings(&self, settings: MomHttpSettings) -> Result<(), AppError> {
        settings.validate()?;
        let server = self.clone();
        tokio::spawn(async move {
            let result = server.set_settings_owned(settings).await;
            if let Err(error) = &result {
                eprintln!("warning: MOM HTTP settings update failed: {error}");
            }
            result
        })
        .await
        .map_err(|error| {
            AppError::configuration(format!("MOM HTTP settings task failed: {error}"))
        })?
    }

    async fn set_settings_owned(&self, settings: MomHttpSettings) -> Result<(), AppError> {
        let mut runtime = self.runtime.lock().await;
        if self.shutdown_token.is_cancelled() {
            return Err(AppError::configuration("MOM HTTP service is shutting down"));
        }
        if settings == runtime.settings
            && runtime
                .listener_handle
                .as_ref()
                .is_some_and(|handle| !handle.is_finished())
        {
            return Ok(());
        }

        let was_running = runtime.bound_addr.is_some();
        Self::stop(&mut runtime, Duration::from_secs(8)).await?;
        let prepared = async {
            let listener = Self::bind(settings).await?;
            let path = self.config_path.clone();
            #[cfg(test)]
            let publish_hook = self.publish_hook.clone();
            tokio::task::spawn_blocking(move || {
                #[cfg(test)]
                if let Some(hook) = publish_hook {
                    hook();
                }
                let bytes = serde_json::to_vec_pretty(&settings).map_err(|error| {
                    AppError::configuration(format!("serialize MOM HTTP settings: {error}"))
                })?;
                if let Some(directory) = path.parent() {
                    std::fs::create_dir_all(directory)?;
                }
                crate::private_file::publish(&path, &bytes).map_err(|error| {
                    AppError::configuration(format!("save MOM HTTP settings: {error:?}"))
                })?;
                Ok::<_, AppError>(())
            })
            .await
            .map_err(|error| {
                AppError::configuration(format!("save MOM HTTP settings task failed: {error}"))
            })??;
            Ok::<_, AppError>(listener)
        }
        .await;
        match prepared {
            Ok(listener) => {
                runtime.settings = settings;
                self.serve(&mut runtime, listener)?;
                Ok(())
            }
            Err(error) => {
                if was_running {
                    let restored = Self::bind(runtime.settings).await;
                    match restored.and_then(|listener| self.serve(&mut runtime, listener)) {
                        Ok(_) => {}
                        Err(restore_error) => {
                            return Err(AppError::configuration(format!(
                                "{error}; restore MOM HTTP listener: {restore_error}"
                            )));
                        }
                    }
                }
                Err(error)
            }
        }
    }

    async fn stop(runtime: &mut Runtime, deadline: Duration) -> Result<(), AppError> {
        runtime.shutdown_token.cancel();
        runtime.bound_addr = None;
        if let Some(mut handle) = runtime.listener_handle.take() {
            match tokio::time::timeout(deadline, &mut handle).await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    return Err(AppError::configuration(format!(
                        "MOM HTTP listener task failed: {error}"
                    )))
                }
                Err(_) => {
                    handle.abort();
                    if let Err(error) = handle.await {
                        if !error.is_cancelled() {
                            eprintln!("warning: MOM HTTP listener task failed: {error}");
                        }
                    }
                    return Err(AppError::configuration(
                        "timed out waiting for MOM HTTP shutdown",
                    ));
                }
            }
        }
        Ok(())
    }

    pub async fn shutdown(&self, deadline: Duration) -> Result<(), AppError> {
        self.shutdown_token.cancel();
        let until = tokio::time::Instant::now() + deadline;
        let mut runtime = tokio::time::timeout_at(until, self.runtime.lock())
            .await
            .map_err(|_| AppError::configuration("timed out waiting for MOM HTTP shutdown"))?;
        Self::stop(
            &mut runtime,
            until.saturating_duration_since(tokio::time::Instant::now()),
        )
        .await
    }
}
