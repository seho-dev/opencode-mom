use std::time::Duration;

use axum::body::Body;
use axum::extract::{Extension, Request, State};
use axum::http::{Method, StatusCode};
use axum::response::{IntoResponse, Response};
use futures::StreamExt;
use rmcp::model::*;
use rmcp::transport::streamable_http_server::session::{ServerSseMessage, SessionManager};
use rmcp::transport::OneshotTransport;
use rmcp::RoleServer;
use tokio_util::sync::CancellationToken;

use crate::http_server::HttpContext;
use crate::logging::{operation, record, LogStatus, LogType};
use crate::mcp_proxy::config::{ProxyProtocol, ProxyServiceConfig};
use crate::mcp_proxy::process::OwnerBudget;
use crate::mcp_proxy::session::{LegacySessionManager, LowLevelBridge, ModernBridge, Observation};
use crate::mcp_proxy::ProxyService;

pub const MAX_BODY_BYTES: usize = 8 * 1024 * 1024; // 8 MiB

#[derive(Clone)]
pub struct HttpState {
    pub budget: OwnerBudget,
    pub legacy_sessions: LegacySessionManager,
    pub shutdown_token: CancellationToken,
    pub proxy_token: CancellationToken,
    pub events: Option<tokio::sync::mpsc::UnboundedSender<Observation>>,
    pub admission: std::sync::Arc<tokio::sync::RwLock<()>>,
}

pub fn create_router(proxy_state: ProxyService) -> axum::Router {
    axum::Router::new()
        .route("/mcp-proxy", axum::routing::any(handle_mcp))
        .layer(axum::extract::DefaultBodyLimit::max(MAX_BODY_BYTES))
        .with_state(proxy_state)
}

fn extract_server_id(query_str: &str) -> Result<String, StatusCode> {
    let mut server_id: Option<String> = None;
    for pair in query_str.split('&') {
        if pair.is_empty() {
            continue;
        }

        let mut parts = pair.splitn(2, '=');
        let key = parts.next().unwrap_or("");
        let value = parts.next().unwrap_or("");
        if key == "server" {
            if server_id.is_some() {
                return Err(StatusCode::BAD_REQUEST);
            }

            let decoded = percent_encoding::percent_decode_str(value).decode_utf8_lossy();
            if decoded.is_empty() {
                return Err(StatusCode::BAD_REQUEST);
            }
            server_id = Some(decoded.to_string());
        }
    }
    server_id.ok_or(StatusCode::BAD_REQUEST)
}

fn sse_response(
    stream: impl futures::Stream<Item = Result<axum::response::sse::Event, serde_json::Error>>
        + Send
        + 'static,
    shutdown_token: CancellationToken,
    proxy_token: CancellationToken,
) -> Response {
    axum::response::Sse::new(stream.take_until(async move {
        tokio::select! {
            _ = shutdown_token.cancelled() => {},
            _ = proxy_token.cancelled() => {},
        }
    }))
    .keep_alive(axum::response::sse::KeepAlive::new().interval(Duration::from_millis(25)))
    .into_response()
}

fn legacy_sse_event(
    event: ServerSseMessage,
) -> Result<axum::response::sse::Event, serde_json::Error> {
    let mut outgoing = axum::response::sse::Event::default();
    if let Some(id) = event.event_id {
        outgoing = outgoing.id(id);
    }

    if let Some(message) = event.message {
        outgoing = outgoing.data(serde_json::to_string(message.as_ref())?);
    } else {
        outgoing = outgoing.data("");
    }

    Ok(outgoing)
}

async fn handle_mcp(
    State(proxy): State<ProxyService>,
    Extension(http): Extension<HttpContext>,
    req: Request,
) -> Response {
    let request = operation(LogType::McpProxy, "mcp_proxy_http_request");
    record(
        LogType::McpProxy,
        LogStatus::Success,
        "mcp_proxy_request_received",
    );
    let response = dispatch_mcp(proxy, http, req).await;
    let result = if response.status().is_success() {
        Ok(())
    } else {
        record(
            LogType::McpProxy,
            LogStatus::Error,
            match response.status() {
                StatusCode::BAD_REQUEST => "mcp_proxy_request_invalid",
                StatusCode::NOT_FOUND => "mcp_proxy_route_unavailable",
                StatusCode::METHOD_NOT_ALLOWED => "mcp_proxy_method_rejected",
                StatusCode::TOO_MANY_REQUESTS => "mcp_proxy_capacity_exhausted",
                StatusCode::SERVICE_UNAVAILABLE => "mcp_proxy_admission_closed",
                _ => "mcp_proxy_request_failed",
            },
        );
        Err(crate::error::AppError::configuration(
            "MCP HTTP request failed",
        ))
    };
    request.finish(&result);
    response
}

async fn dispatch_mcp(proxy: ProxyService, http: HttpContext, req: Request) -> Response {
    let config_guard = proxy.config.read().await;
    let gateway = &config_guard.gateway;
    let proxy_token = proxy.request_token().await;
    let mut state = HttpState {
        budget: proxy.budget().clone(),
        legacy_sessions: proxy.legacy_sessions().clone(),
        shutdown_token: http.shutdown_token,
        proxy_token,
        events: proxy.events.clone(),
        admission: proxy.admission.clone(),
    };
    if state.shutdown_token.is_cancelled() || state.proxy_token.is_cancelled() {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }

    let method = req.method().clone();
    if method != Method::POST && method != Method::GET && method != Method::DELETE {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    }

    if !gateway.enabled {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }

    let query_str = req.uri().query().unwrap_or("");
    let server_id = match extract_server_id(query_str) {
        Ok(id) => id,
        Err(status) => return status.into_response(),
    };

    let service = match config_guard.services.iter().find(|s| s.id == server_id) {
        Some(s) if s.enabled => s.clone(),
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    state.proxy_token = proxy.service_token(&service.id).await;
    let idle_timeout_secs = gateway.idle_timeout_secs;
    drop(config_guard);

    let protocol = match service.protocol {
        ProxyProtocol::Legacy => ProxyProtocol::Legacy,
        ProxyProtocol::Modern => ProxyProtocol::Modern,
        ProxyProtocol::Auto => {
            if req
                .headers()
                .get("MCP-Protocol-Version")
                .and_then(|h| h.to_str().ok())
                == Some("2026-07-28")
            {
                ProxyProtocol::Modern
            } else {
                ProxyProtocol::Legacy
            }
        }
    };

    match protocol {
        ProxyProtocol::Legacy => {
            record(
                LogType::McpProxy,
                LogStatus::Success,
                "mcp_proxy_route_legacy",
            );
            handle_legacy(state, service, idle_timeout_secs, req).await
        }
        ProxyProtocol::Modern | ProxyProtocol::Auto => {
            record(
                LogType::McpProxy,
                LogStatus::Success,
                "mcp_proxy_route_modern",
            );
            handle_modern(state, service, req).await
        }
    }
}

async fn handle_legacy(
    state: HttpState,
    service: ProxyServiceConfig,
    idle_timeout_secs: u64,
    req: Request,
) -> Response {
    let method = req.method().clone();
    let headers = req.headers().clone();

    if method == Method::POST {
        let body = match axum::body::to_bytes(req.into_body(), MAX_BODY_BYTES).await {
            Ok(bytes) => bytes,
            Err(_) => return StatusCode::PAYLOAD_TOO_LARGE.into_response(),
        };

        let message = match serde_json::from_slice::<ClientJsonRpcMessage>(&body) {
            Ok(msg) => msg,
            Err(_) => return StatusCode::BAD_REQUEST.into_response(),
        };

        let is_initialize = matches!(
            message,
            JsonRpcMessage::Request(ref r) if matches!(r.request, ClientRequest::InitializeRequest(_))
        );

        if state.shutdown_token.is_cancelled() || state.proxy_token.is_cancelled() {
            return StatusCode::SERVICE_UNAVAILABLE.into_response();
        }

        if is_initialize {
            let permit = match state.budget.try_acquire() {
                Ok(p) => p,
                Err(_) => return StatusCode::TOO_MANY_REQUESTS.into_response(),
            };

            let session = match state
                .legacy_sessions
                .create_session(
                    &service,
                    permit,
                    idle_timeout_secs,
                    state.admission.clone(),
                    state.shutdown_token.clone(),
                    state.proxy_token.clone(),
                )
                .await
            {
                Ok(s) => s,
                Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            };
            let pending = state.legacy_sessions.pending_session(session.clone());

            let initialized = tokio::select! {
                biased;
                _ = state.shutdown_token.cancelled() => {
                    let _ = state.legacy_sessions.close_session(&session.id).await;

                    return StatusCode::SERVICE_UNAVAILABLE.into_response();
                }
                _ = state.proxy_token.cancelled() => {
                    let _ = state.legacy_sessions.close_session(&session.id).await;

                    return StatusCode::SERVICE_UNAVAILABLE.into_response();
                }
                result = session.manager.initialize_session(&session.id, message) => result,
            };
            let result = match initialized {
                Ok(res) => res,
                Err(_) => {
                    record(
                        LogType::McpProxy,
                        LogStatus::Error,
                        "mcp_proxy_initialize_failed",
                    );
                    let _ = state.legacy_sessions.close_session(&session.id).await;

                    return StatusCode::INTERNAL_SERVER_ERROR.into_response();
                }
            };

            let body = match serde_json::to_vec(&result) {
                Ok(body) => body,
                Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            };
            let response = Response::builder()
                .status(StatusCode::OK)
                .header("Mcp-Session-Id", session.id.as_ref())
                .header("Content-Type", "application/json")
                .body(Body::from(body));
            return match response {
                Ok(response) => {
                    pending.into_session();
                    record(
                        LogType::McpProxy,
                        LogStatus::Success,
                        "mcp_proxy_initialized",
                    );
                    response
                }

                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            };
        }

        let Some(session_id) = headers.get("Mcp-Session-Id").and_then(|h| h.to_str().ok()) else {
            return StatusCode::BAD_REQUEST.into_response();
        };

        let Some(session) = state.legacy_sessions.get_session(session_id).await else {
            return StatusCode::NOT_FOUND.into_response();
        };

        if session.service_id != service.id {
            return StatusCode::NOT_FOUND.into_response();
        }

        session.touch().await;

        match message {
            JsonRpcMessage::Request(_) => {
                let sse_stream = match session.manager.create_stream(&session.id, message).await {
                    Ok(stream) => stream,
                    Err(_) => return StatusCode::NOT_FOUND.into_response(),
                };

                let event_stream = sse_stream.map(legacy_sse_event);
                record(
                    LogType::McpProxy,
                    LogStatus::Success,
                    "mcp_proxy_legacy_forwarded",
                );
                sse_response(event_stream, state.shutdown_token, state.proxy_token)
            }
            JsonRpcMessage::Notification(_) | JsonRpcMessage::Response(_) => {
                if session
                    .manager
                    .accept_message(&session.id, message)
                    .await
                    .is_err()
                {
                    return StatusCode::NOT_FOUND.into_response();
                }
                StatusCode::ACCEPTED.into_response()
            }
            JsonRpcMessage::Error(_) => StatusCode::BAD_REQUEST.into_response(),
        }
    } else if method == Method::GET {
        let Some(session_id) = headers.get("Mcp-Session-Id").and_then(|h| h.to_str().ok()) else {
            return StatusCode::BAD_REQUEST.into_response();
        };

        let Some(session) = state.legacy_sessions.get_session(session_id).await else {
            return StatusCode::NOT_FOUND.into_response();
        };

        if session.service_id != service.id {
            return StatusCode::NOT_FOUND.into_response();
        }

        session.touch().await;

        let sse_stream = match session.manager.create_standalone_stream(&session.id).await {
            Ok(stream) => stream,
            Err(_) => return StatusCode::NOT_FOUND.into_response(),
        };

        let event_stream = sse_stream.map(legacy_sse_event);
        sse_response(event_stream, state.shutdown_token, state.proxy_token)
    } else if method == Method::DELETE {
        let Some(session_id) = headers.get("Mcp-Session-Id").and_then(|h| h.to_str().ok()) else {
            return StatusCode::BAD_REQUEST.into_response();
        };

        let Some(session) = state.legacy_sessions.get_session(session_id).await else {
            return StatusCode::NOT_FOUND.into_response();
        };

        if session.service_id != service.id {
            return StatusCode::NOT_FOUND.into_response();
        }

        match state.legacy_sessions.close_session(session_id).await {
            Ok(_) => StatusCode::NO_CONTENT.into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    } else {
        StatusCode::METHOD_NOT_ALLOWED.into_response()
    }
}

async fn handle_modern(state: HttpState, service: ProxyServiceConfig, req: Request) -> Response {
    if req.method() != Method::POST {
        return StatusCode::METHOD_NOT_ALLOWED.into_response();
    }

    let headers = req.headers().clone();
    let body = match axum::body::to_bytes(req.into_body(), MAX_BODY_BYTES).await {
        Ok(bytes) => bytes,
        Err(_) => return StatusCode::PAYLOAD_TOO_LARGE.into_response(),
    };

    let message = match serde_json::from_slice::<ClientJsonRpcMessage>(&body) {
        Ok(msg) => msg,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    let request = match message {
        JsonRpcMessage::Request(ref r) => r,
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };

    let meta = request.request.get_meta();
    if headers
        .get("MCP-Protocol-Version")
        .and_then(|h| h.to_str().ok())
        != Some("2026-07-28")
        && meta.protocol_version().as_ref() != Some(&ProtocolVersion::V_2026_07_28)
    {
        return StatusCode::BAD_REQUEST.into_response();
    }

    if !meta
        .missing_required_keys(&ProtocolVersion::V_2026_07_28)
        .is_empty()
    {
        return StatusCode::BAD_REQUEST.into_response();
    }

    if let Some(method_header) = headers.get("Mcp-Method").and_then(|h| h.to_str().ok()) {
        if method_header != request.request.method() {
            return StatusCode::BAD_REQUEST.into_response();
        }
    }

    if state.shutdown_token.is_cancelled() || state.proxy_token.is_cancelled() {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    }

    if state.budget.available_permits() == 0 {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }

    let request_ct = state.shutdown_token.child_token();
    let bridge = ModernBridge::new(
        service,
        state.budget,
        state.events,
        state.admission,
        request_ct.clone(),
        state.proxy_token.clone(),
    );
    let low_bridge = LowLevelBridge(bridge);

    let (transport, receiver) = OneshotTransport::<RoleServer>::new(message);
    let worker_ct = request_ct.clone();

    tokio::spawn(async move {
        match rmcp::service::serve_server_with_ct(low_bridge, transport, worker_ct).await {
            Ok(service) => {
                if service.waiting().await.is_err() {
                    record(
                        LogType::McpProxy,
                        LogStatus::Error,
                        "mcp_proxy_sdk_task_failed",
                    );
                }
            }
            Err(_) => {
                record(
                    LogType::McpProxy,
                    LogStatus::Error,
                    "mcp_proxy_sdk_setup_failed",
                );
            }
        }
    });

    let guard = request_ct.drop_guard();
    let stream = futures::stream::unfold(
        (receiver, Some(guard)),
        move |(mut receiver, mut guard)| async move {
            let message = receiver.recv().await?;
            let data = match serde_json::to_string(&message) {
                Ok(data) => data,
                Err(error) => return Some((Err(error), (receiver, guard))),
            };
            if matches!(
                message,
                JsonRpcMessage::Response(_) | JsonRpcMessage::Error(_)
            ) {
                if let Some(guard) = guard.take() {
                    guard.disarm();
                }
            }

            let event = axum::response::sse::Event::default().data(data);

            Some((Ok::<_, serde_json::Error>(event), (receiver, guard)))
        },
    );

    sse_response(stream, state.shutdown_token, state.proxy_token)
}
