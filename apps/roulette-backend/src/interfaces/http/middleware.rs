//! HTTP middleware components.

#![forbid(unsafe_code)]

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    extract::{ConnectInfo, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use roulette_host::GameService;

/// Guards against non-loopback requests when LAN exposure is not permitted.
pub async fn lan_guard(
    State(service): State<Arc<GameService>>,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Response {
    if remote.ip().is_loopback() || service.is_lan_exposed() {
        next.run(request).await
    } else {
        (StatusCode::FORBIDDEN, "局域网访问尚未由本机创始人开启").into_response()
    }
}
