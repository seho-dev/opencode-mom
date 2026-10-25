#![allow(deprecated)]

use std::borrow::Cow;

use rmcp::{
    model::*,
    service::{NotificationContext, RequestContext, SubscriptionContext},
    RoleServer, ServerHandler, ServiceExt,
};
use serde_json::json;

pub const OPAQUE_STATE: &str = "opaque.state/+== \"quotes\"\n\ttab-\u{1f980}";

#[derive(Clone)]
struct FakePeer {
    mode: String,
    tag: String,
}

impl ServerHandler for FakePeer {
    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        Cow::Owned(match self.mode.as_str() {
            "legacy" => vec![ProtocolVersion::V_2025_11_25],
            "modern" | "modern-narrow" => vec![ProtocolVersion::V_2026_07_28],
            "old" => vec![ProtocolVersion::V_2024_11_05],
            "dual" => vec![ProtocolVersion::V_2025_11_25, ProtocolVersion::V_2026_07_28],
            _ => panic!("unknown fake peer mode"),
        })
    }

    fn get_info(&self) -> ServerConfig {
        let mut capabilities = ServerCapabilities::builder().enable_tools().build();
        if self.mode != "modern-narrow" {
            capabilities.tools.as_mut().unwrap().list_changed = Some(true);
        }
        let mut config = ServerConfig::new(capabilities)
            .with_server_info(Implementation::new(self.tag.clone(), "0.0.0"));
        config.protocol_version = self.supported_protocol_versions()[0].clone();
        config
    }

    async fn call_tool(
        &self,
        params: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let token = context
            .meta
            .get_progress_token()
            .unwrap_or(ProgressToken(NumberOrString::String("ready".into())));
        context
            .peer
            .notify_progress(
                ProgressNotificationParam::new(token, 1.0)
                    .with_total(2.0)
                    .with_message(self.tag.clone()),
            )
            .await
            .map_err(|_| ErrorData::internal_error("progress failed", None))?;
        match params.name.as_ref() {
            "wait" => {
                context.ct.cancelled().await;
                Ok(CallToolResult::success(vec![]).into())
            }
            "mrtr" if params.request_state.is_none() => {
                let mut requests = InputRequests::new();
                requests.insert(
                    "roots".into(),
                    InputRequest::ListRoots(ListRootsRequest::default()),
                );
                Ok(InputRequiredResult::new(Some(requests), Some(OPAQUE_STATE.into())).into())
            }
            "mrtr" => {
                if params.request_state.as_deref() != Some(OPAQUE_STATE)
                    || params.input_responses.as_ref().and_then(|r| r.get("roots"))
                        != Some(&json!({"roots": []}))
                {
                    return Err(ErrorData::invalid_params("continuation changed", None));
                }
                Ok(CallToolResult::structured(
                    json!({"continued": true, "pid": std::process::id()}),
                )
                .into())
            }
            "interaction" => {
                context.peer.notify_tool_list_changed().await.unwrap();
                let roots = context.peer.list_roots().await.unwrap();
                Ok(CallToolResult::structured(json!({"tag": self.tag, "roots": roots})).into())
            }
            _ => Ok(CallToolResult::structured(json!({
                "tag": self.tag,
                "pid": std::process::id(),
                "id": context.id,
                "meta": context.meta,
                "capabilities": context.client_capabilities(),
                "clientInfo": context.client_info(),
                "arguments": params.arguments,
            }))
            .into()),
        }
    }

    fn accepted_subscription_filter(
        &self,
        requested: &SubscriptionFilter,
    ) -> Option<SubscriptionFilter> {
        Some(requested.supported_by(&self.get_info().capabilities))
    }

    async fn listen(&self, context: SubscriptionContext) -> Result<(), ErrorData> {
        if context.accepted().tools_list_changed == Some(true) {
            context.sink().notify_tool_list_changed().await.unwrap();
        }
        context.cancelled().await;
        Ok(())
    }

    async fn on_cancelled(
        &self,
        _: CancelledNotificationParam,
        context: NotificationContext<RoleServer>,
    ) {
        let _ = context
            .peer
            .notify_progress(
                ProgressNotificationParam::new(
                    ProgressToken(NumberOrString::String("cancel-seen".into())),
                    0.0,
                )
                .with_message(self.tag.clone()),
            )
            .await;
    }
}

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let mode = args.next().expect("mode");
    let tag = args.next().expect("tag");
    let peer = FakePeer { mode, tag };
    let server = peer
        .serve((tokio::io::stdin(), tokio::io::stdout()))
        .await
        .expect("SDK lifecycle");
    server.waiting().await.expect("SDK worker");
}
