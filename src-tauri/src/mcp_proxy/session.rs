use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};

use rmcp::{
    model::*,
    service::{NotificationContext, RequestContext, Service, SubscriptionContext},
    transport::{
        async_rw::AsyncRwTransport, streamable_http_server::session::local::LocalSessionManager,
        streamable_http_server::session::SessionManager, Transport,
    },
    RoleServer, ServerHandler,
};
use tokio::sync::{watch, Mutex, RwLock};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::error::AppError;
use crate::logging::{operation, record, LogStatus, LogType};
use crate::mcp_proxy::config::ProxyServiceConfig;
use crate::mcp_proxy::process::{
    CloseReason, OwnerBudget, OwnerPermit, ProcessConfig, ProcessOwner,
};
use crate::mcp_proxy::CLEANUP_TIMEOUT;

#[derive(Debug, Clone)]
#[cfg_attr(not(test), allow(dead_code))]
pub enum Observation {
    Started(u32),
    Cancelled(RequestId),
    Closed(u32),
}

type CloseReceiver = watch::Receiver<Option<Result<(), AppError>>>;

pub struct LegacySession {
    pub id: Arc<str>,
    pub service_id: String,
    pub manager: Arc<LocalSessionManager>,
    pub owner: ProcessOwner,
    pub last_activity: Arc<Mutex<Instant>>,
    pub idle_timeout: Duration,
    pub task: Mutex<Option<JoinHandle<Result<(), AppError>>>>,
    bridge_cancel: CancellationToken,
    proxy_token: CancellationToken,
    close_coordinator: StdMutex<Option<CloseReceiver>>,
}

impl LegacySession {
    pub async fn touch(&self) {
        *self.last_activity.lock().await = Instant::now();
    }

    pub async fn is_expired(&self) -> bool {
        let last = *self.last_activity.lock().await;
        last.elapsed() > self.idle_timeout
    }

    pub async fn close(self: &Arc<Self>) -> Result<(), AppError> {
        let mut rx = {
            let mut coordinator = self
                .close_coordinator
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if let Some(rx) = &*coordinator {
                rx.clone()
            } else {
                let (tx, rx) = watch::channel(None);
                *coordinator = Some(rx.clone());
                let session = self.clone();
                tokio::spawn(async move {
                    let result = session.close_owned().await;
                    if result.is_err() {
                        session.owner.budget().record_cleanup_failure();
                        record(
                            LogType::McpProxy,
                            LogStatus::Error,
                            "mcp_proxy_session_cleanup_failed",
                        );
                    }

                    let _ = tx.send(Some(result));
                });
                rx
            }
        };
        let result = tokio::time::timeout(CLEANUP_TIMEOUT, async {
            loop {
                if let Some(result) = &*rx.borrow() {
                    return result.clone();
                }

                if rx.changed().await.is_err() {
                    return Err(AppError::configuration(
                        "MCP session close coordinator terminated unexpectedly",
                    ));
                }
            }
        })
        .await
        .map_err(|_| {
            record(
                LogType::McpProxy,
                LogStatus::Error,
                "mcp_proxy_session_cleanup_timeout",
            );
            AppError::configuration("timed out closing MCP session; cleanup continues")
        })
        .and_then(|result| result);
        if result.is_err() {
            self.owner.budget().record_cleanup_failure();
        }
        result
    }

    async fn close_owned(&self) -> Result<(), AppError> {
        let cleanup = operation(LogType::McpProxy, "mcp_proxy_session_cleanup");
        self.bridge_cancel.cancel();
        let bridge = async {
            let task = self.task.lock().await.take();
            if let Some(handle) = task {
                match handle.await {
                    Ok(result) => return result,
                    Err(error) if error.is_cancelled() => {}
                    Err(error) => {
                        return Err(AppError::configuration(format!(
                            "MCP bridge task failed: {error}"
                        )))
                    }
                }
            }

            Ok(())
        };
        let (bridge, sdk, owner) = tokio::join!(
            bridge,
            self.manager.close_session(&self.id),
            self.owner.close(CloseReason::Normal),
        );
        let result = finish_cleanup([
            bridge,
            sdk.map_err(|error| AppError::configuration(format!("close MCP SDK session: {error}"))),
            owner.map(|_| ()).map_err(|error| {
                AppError::configuration(format!("close MCP process owner: {error}"))
            }),
        ]);
        cleanup.finish(&result);
        result
    }
}

pub(crate) fn finish_cleanup(
    results: impl IntoIterator<Item = Result<(), AppError>>,
) -> Result<(), AppError> {
    let errors: Vec<_> = results
        .into_iter()
        .filter_map(Result::err)
        .map(|error| error.to_string())
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(AppError::configuration(errors.join("; ")))
    }
}

pub(crate) struct PendingSession {
    manager: LegacySessionManager,
    session: Arc<LegacySession>,
    armed: bool,
}

impl PendingSession {
    pub(crate) fn into_session(mut self) -> Arc<LegacySession> {
        self.armed = false;
        self.session.clone()
    }
}

impl Drop for PendingSession {
    fn drop(&mut self) {
        if self.armed {
            let manager = self.manager.clone();
            let session = self.session.clone();
            tokio::spawn(async move {
                if manager.close_session(&session.id).await.is_err() {
                    session.owner.budget().record_cleanup_failure();
                    record(
                        LogType::McpProxy,
                        LogStatus::Error,
                        "mcp_proxy_session_cleanup_failed",
                    );
                }
            });
        }
    }
}

#[derive(Clone)]
pub struct LegacySessionManager {
    sessions: Arc<RwLock<HashMap<String, Arc<LegacySession>>>>,
    manager: Arc<LocalSessionManager>,
    close_coordinator: Arc<StdMutex<Option<CloseReceiver>>>,
}

impl LegacySessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            manager: Arc::new(LocalSessionManager::default()),
            close_coordinator: Arc::new(StdMutex::new(None)),
        }
    }

    pub async fn create_session(
        &self,
        service: &ProxyServiceConfig,
        permit: OwnerPermit,
        idle_timeout_secs: u64,
        admission: Arc<RwLock<()>>,
        http_token: CancellationToken,
        proxy_token: CancellationToken,
    ) -> Result<Arc<LegacySession>, AppError> {
        let manager = self.clone();
        let service = service.clone();
        tokio::spawn(async move {
            let _admission = admission.read().await;
            if http_token.is_cancelled() || proxy_token.is_cancelled() {
                return Err(AppError::configuration("MOM HTTP request was revoked"));
            }

            let session = manager
                .create_session_owned(&service, permit, idle_timeout_secs, proxy_token.clone())
                .await?;
            if http_token.is_cancelled() || proxy_token.is_cancelled() {
                manager.close_session(&session.id).await?;

                return Err(AppError::configuration("MOM HTTP request was revoked"));
            }
            Ok::<_, AppError>(manager.pending_session(session))
        })
        .await
        .map_err(|error| {
            AppError::configuration(format!("MCP session creation task failed: {error}"))
        })?
        .map(PendingSession::into_session)
    }

    pub(crate) fn pending_session(&self, session: Arc<LegacySession>) -> PendingSession {
        PendingSession {
            manager: self.clone(),
            session,
            armed: true,
        }
    }

    async fn create_session_owned(
        &self,
        service: &ProxyServiceConfig,
        permit: OwnerPermit,
        idle_timeout_secs: u64,
        proxy_token: CancellationToken,
    ) -> Result<Arc<LegacySession>, AppError> {
        let (executable, args) = service
            .command
            .split_first()
            .filter(|(executable, _)| !executable.is_empty())
            .ok_or_else(|| AppError::validation("MCP service command argv cannot be empty"))?;
        let (id, mut http) = self
            .manager
            .create_session()
            .await
            .map_err(|e| AppError::configuration(format!("failed to create session: {e}")))?;

        let mut cmd = tokio::process::Command::new(executable);
        cmd.args(args)
            .current_dir(&service.cwd)
            .envs(&service.environment);

        let process_config = ProcessConfig {
            graceful_timeout: Duration::from_millis(300),
            kill_timeout: Duration::from_secs(service.timeouts.startup_secs.max(2)),
        };

        // Publish the session before a registered owner can be selected for teardown.
        let mut sessions = self.sessions.write().await;
        let owner = match ProcessOwner::spawn(cmd, permit, process_config) {
            Ok(owner) => owner,
            Err(e) => {
                drop(sessions);
                let error =
                    AppError::configuration(format!("failed to spawn service process: {e}"));

                if let Err(cleanup) = self.manager.close_session(&id).await {
                    record(
                        LogType::McpProxy,
                        LogStatus::Error,
                        "mcp_proxy_session_cleanup_failed",
                    );

                    return Err(AppError::configuration(format!(
                        "{error}; close MCP SDK session: {cleanup}"
                    )));
                }

                return Err(error);
            }
        };
        let pipes = (|| {
            owner.identify(&service.id);
            owner.activate().map_err(|error| {
                AppError::configuration(format!("failed to activate process: {error}"))
            })?;
            let stdin = owner
                .take_stdin()
                .ok_or_else(|| AppError::configuration("failed to take child stdin"))?;
            let stdout = owner
                .take_stdout()
                .ok_or_else(|| AppError::configuration("failed to take child stdout"))?;
            Ok::<_, AppError>((stdin, stdout))
        })();
        let (stdin, stdout) = match pipes {
            Ok(pipes) => pipes,
            Err(error) => {
                drop(sessions);
                let (sdk, owner) = tokio::join!(
                    self.manager.close_session(&id),
                    owner.close(CloseReason::Normal)
                );

                return Err(finish_cleanup([
                    Err(error.clone()),
                    sdk.map_err(|error| {
                        AppError::configuration(format!("close MCP SDK session: {error}"))
                    }),
                    owner.map(|_| ()).map_err(|error| {
                        AppError::configuration(format!("close MCP process owner: {error}"))
                    }),
                ])
                .err()
                .unwrap_or(error));
            }
        };

        let mut process_transport =
            AsyncRwTransport::new_client(stdout, stdin).with_max_line_length(8 * 1024 * 1024);

        let owner_clone = owner.clone();
        let bridge_cancel = CancellationToken::new();
        let worker_cancel = bridge_cancel.clone();
        let worker_proxy = proxy_token.clone();
        let task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    biased;
                    _ = worker_cancel.cancelled() => break,
                    _ = worker_proxy.cancelled() => break,
                    incoming = http.receive() => match incoming {
                        Some(message) => {
                            let sent = tokio::select! {
                                _ = worker_cancel.cancelled() => break,
                                _ = worker_proxy.cancelled() => break,
                                sent = process_transport.send(message) => sent,
                            };
                            if sent.is_err() {
                                break;
                            }
                        }
                        None => break,
                    },
                    outgoing = process_transport.receive() => match outgoing {
                        Some(message) => {
                            let sent = tokio::select! {
                                _ = worker_cancel.cancelled() => break,
                                _ = worker_proxy.cancelled() => break,
                                sent = http.send(message) => sent,
                            };
                            if sent.is_err() {
                                break;
                            }
                        }
                        None => break,
                    },
                }
            }

            let (transport, owner, http) = tokio::join!(
                process_transport.close(),
                owner_clone.close(CloseReason::Normal),
                http.close(),
            );
            let result = finish_cleanup([
                transport.map_err(|error| {
                    AppError::configuration(format!("close MCP process transport: {error}"))
                }),
                owner.map(|_| ()).map_err(|error| {
                    AppError::configuration(format!("close MCP process owner: {error}"))
                }),
                http.map_err(|error| {
                    AppError::configuration(format!("close MCP HTTP transport: {error}"))
                }),
            ]);

            if result.is_err() {
                owner_clone.budget().record_cleanup_failure();
                record(
                    LogType::McpProxy,
                    LogStatus::Error,
                    "mcp_proxy_bridge_cleanup_failed",
                );
            }
            result
        });

        let session = Arc::new(LegacySession {
            id: id.clone(),
            service_id: service.id.clone(),
            manager: self.manager.clone(),
            owner,
            last_activity: Arc::new(Mutex::new(Instant::now())),
            idle_timeout: Duration::from_secs(idle_timeout_secs),
            task: Mutex::new(Some(task)),
            bridge_cancel,
            proxy_token,
            close_coordinator: StdMutex::new(None),
        });

        sessions.insert(id.to_string(), session.clone());
        drop(sessions);

        record(
            LogType::McpProxy,
            LogStatus::Success,
            "mcp_proxy_session_created",
        );
        Ok(session)
    }

    pub async fn get_session(&self, session_id: &str) -> Option<Arc<LegacySession>> {
        self.sweep_idle().await;
        let guard = self.sessions.read().await;
        guard.get(session_id).cloned()
    }

    pub async fn close_process_session(&self, process_id: &str) -> Result<bool, AppError> {
        let session = self
            .sessions
            .read()
            .await
            .values()
            .find(|session| session.owner.summary().0 == process_id)
            .cloned();
        if let Some(session) = session {
            self.close_registered_session(session).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn close_session(
        &self,
        session_id: &str,
    ) -> Result<Option<Arc<LegacySession>>, AppError> {
        let session = self.sessions.read().await.get(session_id).cloned();
        if let Some(session) = &session {
            self.close_registered_session(session.clone()).await?;
        }
        Ok(session)
    }

    async fn close_registered_session(&self, session: Arc<LegacySession>) -> Result<(), AppError> {
        let manager = self.clone();
        tokio::spawn(async move {
            let result = session.close().await;
            let mut sessions = manager.sessions.write().await;
            if sessions
                .get(session.id.as_ref())
                .is_some_and(|current| Arc::ptr_eq(current, &session))
            {
                sessions.remove(session.id.as_ref());
            }
            result
        })
        .await
        .map_err(|error| {
            AppError::configuration(format!("MCP session cleanup task failed: {error}"))
        })?
    }

    pub async fn sweep_idle(&self) {
        let mut expired = Vec::new();
        {
            let guard = self.sessions.read().await;
            for (id, session) in guard.iter() {
                if session.is_expired().await {
                    expired.push(id.clone());
                }
            }
        }

        for id in expired {
            if self.close_session(&id).await.is_err() {
                record(
                    LogType::McpProxy,
                    LogStatus::Error,
                    "mcp_proxy_session_cleanup_failed",
                );
            }
        }
    }

    pub async fn close_all(&self) -> Result<(), AppError> {
        let mut rx = {
            let mut coordinator = self
                .close_coordinator
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if let Some(rx) = coordinator.as_ref().filter(|rx| rx.borrow().is_none()) {
                rx.clone()
            } else {
                let (tx, rx) = watch::channel(None);
                *coordinator = Some(rx.clone());
                let manager = self.clone();
                tokio::spawn(async move {
                    let sessions: Vec<_> = manager
                        .sessions
                        .write()
                        .await
                        .drain()
                        .map(|(_, session)| session)
                        .collect();
                    let result = finish_cleanup(
                        futures::future::join_all(sessions.iter().map(|session| session.close()))
                            .await,
                    );

                    if result.is_err() {
                        record(
                            LogType::McpProxy,
                            LogStatus::Error,
                            "mcp_proxy_session_cleanup_failed",
                        );
                    }

                    let _ = tx.send(Some(result));
                });
                rx
            }
        };
        loop {
            if let Some(result) = &*rx.borrow() {
                return result.clone();
            }

            if rx.changed().await.is_err() {
                return Err(AppError::configuration(
                    "MCP session bulk cleanup coordinator terminated unexpectedly",
                ));
            }
        }
    }

    pub async fn active_count(&self) -> usize {
        self.sessions.read().await.len()
    }

    pub async fn session_counts_by_service(&self) -> HashMap<String, usize> {
        let guard = self.sessions.read().await;
        let mut counts = HashMap::new();
        for session in guard.values() {
            *counts.entry(session.service_id.clone()).or_insert(0) += 1;
        }
        counts
    }

    pub async fn close_revoked_service_sessions(&self, service_id: &str) -> Result<(), AppError> {
        let manager = self.clone();
        let service_id = service_id.to_string();
        tokio::spawn(async move {
            let sessions: Vec<_> = {
                let mut guard = manager.sessions.write().await;
                let ids: Vec<_> = guard
                    .iter()
                    .filter(|(_, session)| {
                        session.service_id == service_id && session.proxy_token.is_cancelled()
                    })
                    .map(|(id, _)| id.clone())
                    .collect();
                ids.into_iter().filter_map(|id| guard.remove(&id)).collect()
            };
            finish_cleanup(
                futures::future::join_all(sessions.iter().map(|session| session.close())).await,
            )
        })
        .await
        .map_err(|error| {
            AppError::configuration(format!("MCP service session cleanup task failed: {error}"))
        })?
    }
}

impl Default for LegacySessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct ModernBridge {
    pub service: ProxyServiceConfig,
    pub budget: OwnerBudget,
    pub events: Option<tokio::sync::mpsc::UnboundedSender<Observation>>,
    admission: Arc<RwLock<()>>,
    http_token: CancellationToken,
    proxy_token: CancellationToken,
}

impl ModernBridge {
    pub fn new(
        service: ProxyServiceConfig,
        budget: OwnerBudget,
        events: Option<tokio::sync::mpsc::UnboundedSender<Observation>>,
        admission: Arc<RwLock<()>>,
        http_token: CancellationToken,
        proxy_token: CancellationToken,
    ) -> Self {
        Self {
            service,
            budget,
            events,
            admission,
            http_token,
            proxy_token,
        }
    }

    pub async fn forward(
        &self,
        request: ClientJsonRpcMessage,
        context: RequestContext<RoleServer>,
    ) -> Result<ServerResult, ErrorData> {
        let owner = self.clone();
        let request_token = context.ct.child_token();
        // The owned worker must also stop when the SDK drops its forwarding future.
        let guard = request_token.clone().drop_guard();
        let handle = tokio::spawn(async move {
            let forward = operation(LogType::McpProxy, "mcp_proxy_forward");
            let result = owner.forward_owned(request, context, request_token).await;
            forward.finish(
                &result
                    .as_ref()
                    .map(|_| ())
                    .map_err(|_| AppError::configuration("MCP forwarding failed")),
            );
            result
        });
        let result = handle
            .await
            .map_err(|_| ErrorData::internal_error("MCP forwarding task failed", None))?;
        guard.disarm();
        result
    }

    async fn forward_owned(
        &self,
        request: ClientJsonRpcMessage,
        context: RequestContext<RoleServer>,
        request_token: CancellationToken,
    ) -> Result<ServerResult, ErrorData> {
        let _admission = tokio::select! {
            biased;
            _ = request_token.cancelled() => return Err(ErrorData::internal_error("request cancelled", None)),
            _ = self.http_token.cancelled() => return Err(ErrorData::internal_error("request cancelled", None)),
            _ = self.proxy_token.cancelled() => return Err(ErrorData::internal_error("request cancelled", None)),
            admission = self.admission.read() => admission,
        };
        if request_token.is_cancelled()
            || self.http_token.is_cancelled()
            || self.proxy_token.is_cancelled()
        {
            return Err(ErrorData::internal_error("request cancelled", None));
        }

        let permit = self
            .budget
            .try_acquire()
            .map_err(|_| ErrorData::internal_error("process owner budget exhausted", None))?;

        let (executable, args) = self
            .service
            .command
            .split_first()
            .filter(|(executable, _)| !executable.is_empty())
            .ok_or_else(|| {
                ErrorData::internal_error("MCP service command argv cannot be empty", None)
            })?;
        let mut cmd = tokio::process::Command::new(executable);
        cmd.args(args)
            .current_dir(&self.service.cwd)
            .envs(&self.service.environment);

        let process_config = ProcessConfig {
            graceful_timeout: Duration::from_millis(300),
            kill_timeout: Duration::from_secs(self.service.timeouts.startup_secs.max(2)),
        };

        let owner = ProcessOwner::spawn(cmd, permit, process_config).map_err(|e| {
            ErrorData::internal_error(format!("failed to spawn process: {e}"), None)
        })?;
        let pipes = (|| {
            owner.identify(&self.service.id);
            owner.activate().map_err(|error| {
                ErrorData::internal_error(format!("failed to activate process: {error}"), None)
            })?;
            let stdin = owner
                .take_stdin()
                .ok_or_else(|| ErrorData::internal_error("stdin unavailable", None))?;
            let stdout = owner
                .take_stdout()
                .ok_or_else(|| ErrorData::internal_error("stdout unavailable", None))?;
            Ok::<_, ErrorData>((stdin, stdout))
        })();
        let (stdin, stdout) = match pipes {
            Ok(pipes) => pipes,
            Err(error) => {
                if let Err(cleanup) = owner.close(CloseReason::Normal).await {
                    record(
                        LogType::McpProxy,
                        LogStatus::Error,
                        "mcp_proxy_bridge_cleanup_failed",
                    );

                    return Err(ErrorData::internal_error(
                        format!("{error}; {cleanup}"),
                        None,
                    ));
                }

                return Err(error);
            }
        };

        let pid = owner.pid().unwrap_or(0);
        if let Some(ref tx) = self.events {
            let _ = tx.send(Observation::Started(pid));
        }

        let mut process_transport =
            AsyncRwTransport::new_client(stdout, stdin).with_max_line_length(8 * 1024 * 1024);

        let cancelled = async {
            tokio::select! {
                _ = request_token.cancelled() => {},
                _ = self.http_token.cancelled() => {},
                _ = self.proxy_token.cancelled() => {},
            }
        };
        tokio::pin!(cancelled);
        let sent = tokio::select! {
            biased;
            _ = &mut cancelled => {
                record(LogType::McpProxy, LogStatus::Success, "mcp_proxy_forward_cancelled");
                Err(ErrorData::internal_error("request cancelled; outcome uncertain", None))
            },
            sent = tokio::time::timeout(
                Duration::from_secs(self.service.timeouts.execution_secs),
                process_transport.send(request),
            ) => match sent {
                Ok(result) => result.map_err(|error| {
                    record(LogType::McpProxy, LogStatus::Error, "mcp_proxy_backend_send_failed");
                    ErrorData::internal_error(format!("failed to send request to process: {error}"), None)
                }),
                Err(_) => {
                    record(LogType::McpProxy, LogStatus::Error, "mcp_proxy_forward_timeout");
                    Err(ErrorData::internal_error("timed out sending request to process; outcome uncertain", None))
                },
            },
        };
        let result = match sent {
            Err(error) => Err(error),
            Ok(()) => {
                record(
                    LogType::McpProxy,
                    LogStatus::Success,
                    "mcp_proxy_backend_sent",
                );
                loop {
                    tokio::select! {
                        biased;
                        _ = &mut cancelled => {
                            record(LogType::McpProxy, LogStatus::Success, "mcp_proxy_forward_cancelled");
                            let cancel_msg = ClientJsonRpcMessage::notification(
                                ClientNotification::CancelledNotification(CancelledNotification::new(
                                    CancelledNotificationParam::new(Some(context.id.clone()), None),
                                )),
                            );
                            let seen = tokio::time::timeout(Duration::from_secs(1), async {
                                if process_transport.send(cancel_msg).await.is_err() {
                                    return false;
                                }
                                loop {
                                    let reply = process_transport.receive().await;
                                    let Some(msg) = reply else { break false; };
                                    if matches!(&msg,
                                        JsonRpcMessage::Notification(reply)
                                            if matches!(&reply.notification,
                                                ServerNotification::ProgressNotification(progress)
                                                    if matches!(&progress.params.progress_token.0,
                                                        NumberOrString::String(token) if token.as_ref() == "cancel-seen"
                                                    )
                                            )
                                    ) {
                                        break true;
                                    }
                                }
                            }).await.unwrap_or(false);
                            if seen {
                                if let Some(ref tx) = self.events {
                                    let _ = tx.send(Observation::Cancelled(context.id.clone()));
                                }
                            }
                            break Err(ErrorData::internal_error("HTTP stream cancelled; outcome uncertain", None));
                        }
                        reply = process_transport.receive() => match reply {
                            Some(JsonRpcMessage::Response(reply)) => {
                                if reply.id != context.id {
                                    record(LogType::McpProxy, LogStatus::Error, "mcp_proxy_backend_response_mismatch");
                                    break Err(ErrorData::internal_error("backend response ID mismatch", None));
                                }
                                break Ok(reply.result);
                            }
                            Some(JsonRpcMessage::Error(reply)) => {
                                if reply.id.as_ref() != Some(&context.id) {
                                    record(LogType::McpProxy, LogStatus::Error, "mcp_proxy_backend_response_mismatch");
                                    break Err(ErrorData::internal_error("backend response ID mismatch", None));
                                }
                                break Err(reply.error);
                            }
                            Some(JsonRpcMessage::Notification(reply)) => {
                                tokio::select! {
                                    biased;
                                    _ = &mut cancelled => {
                                        record(LogType::McpProxy, LogStatus::Success, "mcp_proxy_forward_cancelled");
                                        break Err(ErrorData::internal_error("HTTP stream cancelled; outcome uncertain", None));
                                    },
                                    _ = context.peer.send_notification(reply.notification) => {},
                                }
                            }
                            Some(JsonRpcMessage::Request(_)) => {
                                record(LogType::McpProxy, LogStatus::Error, "mcp_proxy_backend_request_unsupported");
                                break Err(ErrorData::internal_error("modern backend emitted active request", None));
                            }
                            None => {
                                record(LogType::McpProxy, LogStatus::Error, "mcp_proxy_backend_closed");
                                break Err(ErrorData::internal_error("backend closed", None));
                            }
                        }
                    }
                }
            }
        };

        let (transport, cleanup) =
            tokio::join!(process_transport.close(), owner.close(CloseReason::Normal));
        let cleanup = finish_cleanup([
            transport.map_err(|error| {
                AppError::configuration(format!("close MCP process transport: {error}"))
            }),
            cleanup.map(|_| ()).map_err(|error| {
                AppError::configuration(format!("close MCP process owner: {error}"))
            }),
        ]);

        if let Err(error) = cleanup {
            owner.budget().record_cleanup_failure();
            record(
                LogType::McpProxy,
                LogStatus::Error,
                "mcp_proxy_bridge_cleanup_failed",
            );

            return Err(ErrorData::internal_error(
                format!("MCP process cleanup failed; outcome uncertain: {error}"),
                None,
            ));
        }

        if let Some(ref tx) = self.events {
            let _ = tx.send(Observation::Closed(pid));
        }
        result
    }
}

impl ServerHandler for ModernBridge {
    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        Cow::Borrowed(&[ProtocolVersion::V_2026_07_28])
    }

    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()
                .build(),
        )
        .with_server_info(Implementation::new(self.service.name.clone(), "1.0.0"))
    }

    async fn call_tool(
        &self,
        params: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let mut request = ClientRequest::CallToolRequest(CallToolRequest::new(params));
        *request.get_meta_mut() = context.meta.clone();
        let wire = ClientJsonRpcMessage::request(request, context.id.clone());
        match self.forward(wire, context).await? {
            ServerResult::CallToolResult(result) => Ok(result.into()),
            ServerResult::InputRequiredResult(result) => Ok(result.into()),
            _ => Err(ErrorData::internal_error("unexpected tool result", None)),
        }
    }

    fn accepted_subscription_filter(
        &self,
        requested: &SubscriptionFilter,
    ) -> Option<SubscriptionFilter> {
        Some(requested.supported_by(&ServerHandler::get_info(self).capabilities))
    }

    async fn listen(&self, subscription: SubscriptionContext) -> Result<(), ErrorData> {
        let context = subscription.request_context().clone();
        let mut request =
            ClientRequest::SubscriptionsListenRequest(SubscriptionsListenRequest::new(
                SubscriptionsListenRequestParams::new(subscription.requested().clone()),
            ));
        *request.get_meta_mut() = context.meta.clone();
        self.forward(
            ClientJsonRpcMessage::request(request, context.id.clone()),
            context,
        )
        .await?;

        Ok(())
    }
}

#[derive(Clone)]
pub struct LowLevelBridge(pub ModernBridge);

impl Service<RoleServer> for LowLevelBridge {
    async fn handle_request(
        &self,
        mut request: ClientRequest,
        context: RequestContext<RoleServer>,
    ) -> Result<ServerResult, ErrorData> {
        *request.get_meta_mut() = context.meta.clone();
        let mut result = self
            .0
            .forward(
                ClientJsonRpcMessage::request(request, context.id.clone()),
                context,
            )
            .await?;
        if let ServerResult::DiscoverResult(ref mut discovery) = result {
            discovery
                .supported_versions
                .retain(|version| version == &ProtocolVersion::V_2026_07_28);
        }

        Ok(result)
    }

    async fn handle_notification(
        &self,
        _: ClientNotification,
        _: NotificationContext<RoleServer>,
    ) -> Result<(), ErrorData> {
        Ok(())
    }

    fn get_info(&self) -> ServerConfig {
        ServerHandler::get_info(&self.0)
    }

    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        ServerHandler::supported_protocol_versions(&self.0)
    }
}
