//! HTTP interface router assembly.

#![forbid(unsafe_code)]

pub mod dtos;
pub mod handlers;
pub mod middleware;

use std::sync::Arc;

use axum::{
    Router, middleware as axum_middleware,
    routing::{get, post},
};
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};
use roulette_host::GameService;

use crate::interfaces::mcp::RussianRouletteMcpServer;

/// Builds the Axum router with HTTP routes and middleware.
pub fn build_router(service: Arc<GameService>) -> Router {
    let mcp_service = StreamableHttpService::new(
        {
            let service = Arc::clone(&service);
            move || Ok(RussianRouletteMcpServer::new(Arc::clone(&service)))
        },
        Arc::new(LocalSessionManager::default()),
        StreamableHttpServerConfig::default(),
    );

    Router::new()
        .nest_service("/mcp", mcp_service)
        .route("/", get(handlers::index))
        .route("/styles.css", get(handlers::styles))
        .route("/app.js", get(handlers::script))
        .route("/api/health", get(handlers::health))
        .route("/api/rules/terrains", get(handlers::terrain_rules))
        .route("/api/lobby", get(handlers::lobby_view))
        .route("/api/session/heartbeat", post(handlers::heartbeat))
        .route("/api/rooms", post(handlers::create_room))
        .route("/api/rooms/join", post(handlers::join_room))
        .route("/api/rooms/{room_id}", get(handlers::room_view))
        .route(
            "/api/rooms/{room_id}/image",
            get(handlers::render_room_image),
        )
        .route(
            "/api/rooms/{room_id}/board",
            get(handlers::render_room_board),
        )
        .route("/api/assets/tilesheet", get(handlers::render_tilesheet))
        .route("/api/rooms/{room_id}/leave", post(handlers::leave_room))
        .route("/api/rooms/{room_id}/bots/add", post(handlers::add_bot))
        .route(
            "/api/rooms/{room_id}/bots/remove",
            post(handlers::remove_bot),
        )
        .route("/api/rooms/{room_id}/owner", post(handlers::transfer_owner))
        .route("/api/rooms/{room_id}/start", post(handlers::start_room))
        .route(
            "/api/rooms/{room_id}/dissolve",
            post(handlers::dissolve_room),
        )
        .route("/api/rooms/{room_id}/command", post(handlers::command))
        .route("/api/rooms/{room_id}/step", post(handlers::step))
        .route("/api/founder/auth", post(handlers::founder_auth))
        .route(
            "/api/server/settings",
            get(handlers::server_settings).post(handlers::update_server_settings),
        )
        .route("/api/mcp/tools", get(handlers::mcp_tools))
        .route("/api/mcp/call", post(handlers::mcp_call))
        .layer(axum_middleware::from_fn_with_state(
            Arc::clone(&service),
            middleware::access_guard,
        ))
        .with_state(service)
}
