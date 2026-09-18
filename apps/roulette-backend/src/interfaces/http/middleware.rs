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
use roulette_domain::ServerMode;
use roulette_host::GameService;

/// Guards against unauthorized remote requests based on server mode.
/// In public server mode, access is open.
/// In local mode, requests from non-loopback IP require LAN exposure to be enabled.
pub async fn access_guard(
    State(service): State<Arc<GameService>>,
    ConnectInfo(remote): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Response {
    if service.server_mode() == ServerMode::Public
        || remote.ip().is_loopback()
        || service.is_lan_exposed()
    {
        next.run(request).await
    } else {
        (StatusCode::FORBIDDEN, "局域网访问尚未由本机创始人开启").into_response()
    }
}
