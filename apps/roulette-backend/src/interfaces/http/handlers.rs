//! HTTP request handler functions delegating to `GameService`.

#![forbid(unsafe_code)]

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse},
};
use roulette_domain::{LobbyView, RoomView, TerrainProperties};
use roulette_host::{CreateRoomConfig, GameService, JoinRoomConfig, ServerSettings};

use crate::interfaces::http::dtos::{
    ApiError, CommandRequest, CreateRoomRequest, FounderAuthRequest, FounderAuthResponse,
    HealthResponse, JoinRoomRequest, MemberRequest, ServerSettingsRequest, StartRoomRequest,
    StepRequest, TabQuery, TabRequest,
};

const INDEX_HTML: &str = include_str!("../../../../../clients/web/index.html");
const STYLES_CSS: &str = include_str!("../../../../../clients/web/styles.css");
const APP_JS: &str = include_str!("../../../../../clients/web/app.js");

pub async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

pub async fn styles() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        STYLES_CSS,
    )
}

pub async fn script() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        APP_JS,
    )
}

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

pub async fn terrain_rules(
    State(service): State<Arc<GameService>>,
) -> Json<Vec<TerrainProperties>> {
    Json(service.get_terrain_rules())
}

pub async fn lobby_view(
    State(service): State<Arc<GameService>>,
    Query(query): Query<TabQuery>,
) -> Result<Json<LobbyView>, ApiError> {
    service
        .get_lobby_view(&query.tab_id)
        .map(Json)
        .map_err(|e| ApiError::from_service(&e))
}

pub async fn heartbeat(
    State(service): State<Arc<GameService>>,
    Json(request): Json<TabRequest>,
) -> Result<StatusCode, ApiError> {
    service.heartbeat(&request.tab_id);
    Ok(StatusCode::NO_CONTENT)
}

pub async fn create_room(
    State(service): State<Arc<GameService>>,
    Json(request): Json<CreateRoomRequest>,
) -> Result<Json<RoomView>, ApiError> {
    service
        .create_room(&CreateRoomConfig {
            tab_id: request.tab_id,
            room_name: request.room_name,
            player_name: request.player_name,
            password: request.password,
        })
        .map(Json)
        .map_err(|e| ApiError::from_service(&e))
}

pub async fn join_room(
    State(service): State<Arc<GameService>>,
    Json(request): Json<JoinRoomRequest>,
) -> Result<Json<RoomView>, ApiError> {
    service
        .join_room(&JoinRoomConfig {
            tab_id: request.tab_id,
            room_id: request.room_id,
            player_name: request.player_name,
            password: request.password,
        })
        .map(Json)
        .map_err(|e| ApiError::from_service(&e))
}

pub async fn room_view(
    State(service): State<Arc<GameService>>,
    Path(room_id): Path<String>,
    Query(query): Query<TabQuery>,
) -> Result<Json<RoomView>, ApiError> {
    service
        .get_room_view(&room_id, &query.tab_id)
        .map(Json)
        .map_err(|e| ApiError::from_service(&e))
}

pub async fn leave_room(
    State(service): State<Arc<GameService>>,
    Path(room_id): Path<String>,
    Json(request): Json<TabRequest>,
) -> Result<Json<LobbyView>, ApiError> {
    service
        .leave_room(&room_id, &request.tab_id)
        .map(Json)
        .map_err(|e| ApiError::from_service(&e))
}

pub async fn add_bot(
    State(service): State<Arc<GameService>>,
    Path(room_id): Path<String>,
    Json(request): Json<TabRequest>,
) -> Result<Json<RoomView>, ApiError> {
    service
        .add_bot(&room_id, &request.tab_id)
        .map(Json)
        .map_err(|e| ApiError::from_service(&e))
}

pub async fn remove_bot(
    State(service): State<Arc<GameService>>,
    Path(room_id): Path<String>,
    Json(request): Json<MemberRequest>,
) -> Result<Json<RoomView>, ApiError> {
    service
        .remove_bot(&room_id, &request.tab_id, &request.member_id)
        .map(Json)
        .map_err(|e| ApiError::from_service(&e))
}

pub async fn transfer_owner(
    State(service): State<Arc<GameService>>,
    Path(room_id): Path<String>,
    Json(request): Json<MemberRequest>,
) -> Result<Json<RoomView>, ApiError> {
    service
        .transfer_owner(&room_id, &request.tab_id, &request.member_id)
        .map(Json)
        .map_err(|e| ApiError::from_service(&e))
}

pub async fn start_room(
    State(service): State<Arc<GameService>>,
    Path(room_id): Path<String>,
    Json(request): Json<StartRoomRequest>,
) -> Result<Json<RoomView>, ApiError> {
    service
        .start_room(&room_id, &request.tab_id, request.seed)
        .map(Json)
        .map_err(|e| ApiError::from_service(&e))
}

pub async fn dissolve_room(
    State(service): State<Arc<GameService>>,
    Path(room_id): Path<String>,
    Json(request): Json<TabRequest>,
) -> Result<Json<LobbyView>, ApiError> {
    service
        .dissolve_room(&room_id, &request.tab_id)
        .map(Json)
        .map_err(|e| ApiError::from_service(&e))
}

pub async fn command(
    State(service): State<Arc<GameService>>,
    Path(room_id): Path<String>,
    Json(request): Json<CommandRequest>,
) -> Result<Json<RoomView>, ApiError> {
    service
        .submit_command(
            &room_id,
            &request.tab_id,
            request.expected_revision,
            request.command,
        )
        .map(Json)
        .map_err(|e| ApiError::from_service(&e))
}

pub async fn step(
    State(service): State<Arc<GameService>>,
    Path(room_id): Path<String>,
    Json(request): Json<StepRequest>,
) -> Result<Json<RoomView>, ApiError> {
    service
        .step_bot(&room_id, &request.tab_id, request.expected_revision)
        .map(Json)
        .map_err(|e| ApiError::from_service(&e))
}

pub async fn founder_auth(
    State(service): State<Arc<GameService>>,
    Json(request): Json<FounderAuthRequest>,
) -> Result<Json<FounderAuthResponse>, ApiError> {
    service
        .authenticate_founder(&request.tab_id, &request.password)
        .map(|auth| {
            Json(FounderAuthResponse {
                authenticated: auth,
            })
        })
        .map_err(|e| ApiError::from_service(&e))
}

pub async fn server_settings(
    State(service): State<Arc<GameService>>,
    Query(query): Query<TabQuery>,
) -> Result<Json<ServerSettings>, ApiError> {
    service
        .get_server_settings(&query.tab_id)
        .map(Json)
        .map_err(|e| ApiError::from_service(&e))
}

pub async fn update_server_settings(
    State(service): State<Arc<GameService>>,
    Json(request): Json<ServerSettingsRequest>,
) -> Result<Json<ServerSettings>, ApiError> {
    service
        .update_server_settings(&request.tab_id, request.expose_lan)
        .map(Json)
        .map_err(|e| ApiError::from_service(&e))
}

pub async fn mcp_tools(
    State(service): State<Arc<GameService>>,
) -> Json<Vec<crate::interfaces::mcp::McpToolDefinition>> {
    let dispatcher = crate::interfaces::mcp::McpDispatcher::new(service);
    Json(dispatcher.tools().to_vec())
}

pub async fn mcp_call(
    State(service): State<Arc<GameService>>,
    Json(request): Json<crate::interfaces::http::dtos::McpCallRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let dispatcher = crate::interfaces::mcp::McpDispatcher::new(service);
    dispatcher
        .dispatch(&request.tool, &request.arguments)
        .map(Json)
        .map_err(|e: crate::interfaces::mcp::McpError| ApiError {
            status: StatusCode::BAD_REQUEST,
            message: e.to_string(),
        })
}
