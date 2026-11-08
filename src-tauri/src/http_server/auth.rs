use std::sync::{Arc, RwLock};

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use futures::StreamExt;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use tokio_util::sync::CancellationToken;

use super::{HttpContext, MomHttpSettings};

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct AuthConfig {
    pub token_verifier: Option<String>,
}

pub(super) struct AuthState {
    pub config: AuthConfig,
    pub epoch: CancellationToken,
}

#[derive(Clone)]
pub(super) struct HttpAuth {
    pub settings: MomHttpSettings,
    pub listener_token: CancellationToken,
    pub state: Arc<RwLock<AuthState>>,
}

pub(crate) struct GeneratedKey {
    pub raw_token: String,
    pub verifier: String,
}

fn to_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(hex, "{byte:02x}").expect("write to String");
    }
    hex
}

pub(crate) fn valid_verifier(verifier: &str) -> bool {
    verifier.len() == 64 && verifier.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(crate) fn generate_key() -> GeneratedKey {
    let mut bytes = [0; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    let raw_token = to_hex(&bytes);
    let verifier = to_hex(&Sha256::digest(raw_token.as_bytes()));
    GeneratedKey {
        raw_token,
        verifier,
    }
}

pub(crate) fn verify_token(token: &str, verifier: &str) -> bool {
    if !valid_verifier(verifier) {
        return false;
    }

    let mut expected = [0; 32];
    for (index, byte) in expected.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&verifier[index * 2..index * 2 + 2], 16)
            .expect("validated hexadecimal verifier");
    }
    Sha256::digest(token.as_bytes()).ct_eq(&expected).into()
}

fn valid_host(headers: &HeaderMap, settings: MomHttpSettings) -> bool {
    if headers.get_all("host").iter().count() != 1 {
        return false;
    }

    let Some(authority) = headers
        .get("host")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<axum::http::uri::Authority>().ok())
    else {
        return false;
    };
    if authority.as_str().contains('@') || authority.port_u16().unwrap_or(80) != settings.port {
        return false;
    }

    match authority.host() {
        "localhost" | "127.0.0.1" | "[::1]" => true,
        host => settings.allow_lan && host.parse::<std::net::Ipv4Addr>().is_ok(),
    }
}

fn valid_origin(headers: &HeaderMap, port: u16) -> bool {
    let mut origins = headers.get_all("origin").iter();
    let Some(origin) = origins.next() else {
        return true;
    };
    origins.next().is_none()
        && origin.to_str().is_ok_and(|origin| {
            ["127.0.0.1", "localhost", "[::1]"]
                .iter()
                .any(|host| origin == format!("http://{host}:{port}"))
        })
}

pub(super) async fn enforce(
    State(auth): State<HttpAuth>,
    mut request: Request,
    next: Next,
) -> Response {
    if !valid_host(request.headers(), auth.settings) {
        return StatusCode::BAD_REQUEST.into_response();
    }

    if !valid_origin(request.headers(), auth.settings.port) {
        return StatusCode::FORBIDDEN.into_response();
    }

    let epoch = {
        let state = auth.state.read().unwrap_or_else(|error| error.into_inner());
        if auth.listener_token.is_cancelled() || state.epoch.is_cancelled() {
            return StatusCode::SERVICE_UNAVAILABLE.into_response();
        }

        let Some(verifier) = &state.config.token_verifier else {
            return StatusCode::SERVICE_UNAVAILABLE.into_response();
        };
        if request.headers().get_all("authorization").iter().count() != 1
            || !request
                .headers()
                .get("authorization")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.strip_prefix("Bearer "))
                .is_some_and(|token| verify_token(token, verifier))
        {
            return StatusCode::UNAUTHORIZED.into_response();
        }
        state.epoch.clone()
    };
    request.extensions_mut().insert(HttpContext {
        shutdown_token: epoch.clone(),
    });
    let response = tokio::select! {
        biased;
        _ = epoch.cancelled() => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
        response = next.run(request) => response,
    };
    let (parts, body) = response.into_parts();
    Response::from_parts(
        parts,
        Body::from_stream(body.into_data_stream().take_until(epoch.cancelled_owned())),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_and_request_boundaries_fail_closed() {
        let key = generate_key();
        assert_eq!(key.raw_token.len(), 64);
        assert!(verify_token(&key.raw_token, &key.verifier));
        assert!(!verify_token("wrong", &key.verifier));
        assert!(!verify_token(&key.raw_token, "invalid"));
        let settings = MomHttpSettings::default();
        let mut headers = HeaderMap::new();
        for host in ["localhost:18765", "127.0.0.1:18765", "[::1]:18765"] {
            headers.insert("host", host.parse().unwrap());
            assert!(valid_host(&headers, settings));
        }

        for host in [
            "localhost",
            "localhost:no",
            "[::1]:1",
            "evil.test:18765",
            "x@localhost:18765",
        ] {
            headers.insert("host", host.parse().unwrap());
            assert!(!valid_host(&headers, settings));
        }
        headers.insert("origin", "null".parse().unwrap());
        assert!(!valid_origin(&headers, settings.port));
        headers.insert("origin", "http://localhost:18765".parse().unwrap());
        assert!(valid_origin(&headers, settings.port));
        headers.append("origin", "http://evil.test".parse().unwrap());
        assert!(!valid_origin(&headers, settings.port));
    }
}
