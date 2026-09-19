//! HTTP request handler functions delegating to `GameService`.

#![forbid(unsafe_code)]

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
};
use roulette_domain::{LobbyView, RoomView, ServerMode, TabId, TerrainProperties, UserIdentity};
use roulette_host::{CreateRoomConfig, GameService, JoinRoomConfig, ServerSettings};

use crate::interfaces::http::dtos::{
    ApiError, CommandRequest, CreateRoomRequest, FounderAuthRequest, FounderAuthResponse,
    HealthResponse, JoinRoomRequest, MemberRequest, RoomImageQuery, ServerSettingsRequest,
    StartRoomRequest, StepRequest, TabQuery, TabRequest,
};

const INDEX_HTML: &str = include_str!("../../../../../clients/web/index.html");
const STYLES_CSS: &str = include_str!("../../../../../clients/web/styles.css");
const APP_JS: &str = include_str!("../../../../../clients/web/app.js");

/// Resolves a unified `UserIdentity` from an HTTP tab identifier and optional authorization header.
/// In the future public server mode, `auth_header` can be validated as a bearer token or session cookie.
/// For now, tabs are treated as temporary session identities (`IdentityKind::TemporaryTab`).
#[must_use]
pub fn resolve_identity(
    tab_id: &TabId,
    _auth_header: Option<&str>,
    _mode: ServerMode,
) -> UserIdentity {
    UserIdentity::temporary_tab(tab_id.clone())
}

/// Serves the single-page HTML client shell.
pub async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

/// Serves static CSS styling for the client web interface.
pub async fn styles() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        STYLES_CSS,
    )
}

/// Serves static JavaScript logic for the client web interface.
pub async fn script() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        APP_JS,
    )
}

/// Serves basic service health status.
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

/// Queries authoritative definitions and hardness properties of game terrains.
pub async fn terrain_rules(
    State(service): State<Arc<GameService>>,
) -> Json<Vec<TerrainProperties>> {
    Json(service.get_terrain_rules())
}

/// Fetches current lobby rooms visible to requesting tab.
///
/// # Errors
///
/// Returns `ApiError` if room query fails.
pub async fn lobby_view(
    State(service): State<Arc<GameService>>,
    Query(query): Query<TabQuery>,
) -> Result<Json<LobbyView>, ApiError> {
    service
        .get_lobby_view(&query.tab_id)
        .map(Json)
        .map_err(|e| ApiError::from_service(&e))
}

/// Refreshes tab heartbeat to prevent session expiry.
///
/// # Errors
///
/// Returns `ApiError` on internal error.
pub async fn heartbeat(
    State(service): State<Arc<GameService>>,
    Json(request): Json<TabRequest>,
) -> Result<StatusCode, ApiError> {
    service.heartbeat(&request.tab_id);
    Ok(StatusCode::NO_CONTENT)
}

/// Creates a new game room.
///
/// # Errors
///
/// Returns `ApiError` if room validation fails or room limit is reached.
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

/// Joins an existing game room.
///
/// # Errors
///
/// Returns `ApiError` if room does not exist, password is wrong, or room is full.
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

/// Fetches state and room view for a specific tab in a room.
///
/// # Errors
///
/// Returns `ApiError` if room does not exist or tab is not a member.
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

/// Leaves a room and returns updated lobby view.
///
/// # Errors
///
/// Returns `ApiError` if room is not found or leave fails.
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

/// Adds an AI bot player to the waiting room.
///
/// # Errors
///
/// Returns `ApiError` if room is already full or requester is not owner.
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

/// Removes an AI bot player from the waiting room.
///
/// # Errors
///
/// Returns `ApiError` if bot member is not found or requester is not owner.
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

/// Transfers room ownership to another member.
///
/// # Errors
///
/// Returns `ApiError` if target member does not exist or requester is not owner.
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

/// Starts match for the given room.
///
/// # Errors
///
/// Returns `ApiError` if room cannot be started (e.g. insufficient players or wrong phase).
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

/// Dissolves the room by its owner.
///
/// # Errors
///
/// Returns `ApiError` if requester is not room owner or room not found.
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

/// Submits an in-game player command.
///
/// # Errors
///
/// Returns `ApiError` if OCC revision conflicts or command is invalid.
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

/// Advances single bot turn in the current match.
///
/// # Errors
///
/// Returns `ApiError` on OCC revision conflict or invalid turn owner.
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

/// Authenticates a tab as the local founder.
///
/// # Errors
///
/// Returns `ApiError` on incorrect password or authentication failure.
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

/// Fetches server settings snapshot.
///
/// # Errors
///
/// Returns `ApiError` on service failure.
pub async fn server_settings(
    State(service): State<Arc<GameService>>,
    Query(query): Query<TabQuery>,
) -> Result<Json<ServerSettings>, ApiError> {
    service
        .get_server_settings(&query.tab_id)
        .map(Json)
        .map_err(|e| ApiError::from_service(&e))
}

/// Updates server settings (e.g. toggles LAN exposure).
///
/// # Errors
///
/// Returns `ApiError` if tab is not authenticated founder.
pub async fn update_server_settings(
    State(service): State<Arc<GameService>>,
    Json(request): Json<ServerSettingsRequest>,
) -> Result<Json<ServerSettings>, ApiError> {
    service
        .update_server_settings(&request.tab_id, request.expose_lan)
        .map(Json)
        .map_err(|e| ApiError::from_service(&e))
}

/// Lists Model Context Protocol (MCP) tool schemas.
pub async fn mcp_tools(
    State(service): State<Arc<GameService>>,
) -> Json<Vec<crate::interfaces::mcp::McpToolDefinition>> {
    let dispatcher = crate::interfaces::mcp::McpDispatcher::new(service);
    Json(dispatcher.tools().to_vec())
}

/// Dispatches an MCP tool call through HTTP POST.
///
/// # Errors
///
/// Returns `ApiError` if tool execution fails or parameter validation fails.
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

/// Renders an authoritative tactical HUD card image for a room in PNG or SVG format.
/// Renders an authoritative tactical HUD card image or standalone board for a room.
///
/// # Errors
///
/// Returns `ApiError` if room is not found or rasterization fails.
pub async fn render_room_image(
    State(service): State<Arc<GameService>>,
    Path(room_id): Path<String>,
    Query(query): Query<RoomImageQuery>,
) -> Result<Response, ApiError> {
    let room = service
        .referee_inspect_room(&room_id)
        .map_err(|e| ApiError::from_service(&e))?;

    let format_str = query.format.as_deref().unwrap_or("png").to_lowercase();
    let is_board_only = query.view_mode.as_deref() == Some("board");

    if let Some(game) = room.game.as_ref().filter(|_| is_board_only) {
        if format_str == "svg" {
            let svg = roulette_renderer::render_board_svg(
                &game.cells,
                &game.players,
                game.current_player_id,
                &game.records,
                query.cell_size,
            );
            return Ok((
                [(header::CONTENT_TYPE, "image/svg+xml; charset=utf-8")],
                svg,
            )
                .into_response());
        }
        let png = roulette_renderer::render_board_png(
            &game.cells,
            &game.players,
            game.current_player_id,
            &game.records,
            query.cell_size,
        )
        .map_err(|e| ApiError::internal(format!("render board png failed: {e}")))?;
        return Ok(([(header::CONTENT_TYPE, "image/png")], png).into_response());
    }

    if format_str == "svg" {
        let svg = roulette_renderer::render_match_svg(&room);
        Ok((
            [(header::CONTENT_TYPE, "image/svg+xml; charset=utf-8")],
            svg,
        )
            .into_response())
    } else {
        let png = roulette_renderer::render_match_png(&room)
            .map_err(|e| ApiError::internal(format!("render png failed: {e}")))?;
        Ok(([(header::CONTENT_TYPE, "image/png")], png).into_response())
    }
}

/// Renders a standalone dynamic tactical board without HUD card frames or roster panels.
///
/// # Errors
///
/// Returns `ApiError` if room is not found or rasterization fails.
pub async fn render_room_board(
    State(service): State<Arc<GameService>>,
    Path(room_id): Path<String>,
    Query(query): Query<RoomImageQuery>,
) -> Result<Response, ApiError> {
    let mut modified_query = query;
    modified_query.view_mode = Some("board".to_string());
    render_room_image(State(service), Path(room_id), Query(modified_query)).await
}

/// Renders the complete procedural vector asset catalog / tilesheet.
///
/// # Errors
///
/// Returns `ApiError` if rasterization fails.
pub async fn render_tilesheet(Query(query): Query<RoomImageQuery>) -> Result<Response, ApiError> {
    let format_str = query.format.as_deref().unwrap_or("png").to_lowercase();
    if format_str == "svg" {
        let svg = roulette_renderer::render_tilesheet_svg();
        Ok((
            [(header::CONTENT_TYPE, "image/svg+xml; charset=utf-8")],
            svg,
        )
            .into_response())
    } else {
        let png = roulette_renderer::render_tilesheet_png()
            .map_err(|e| ApiError::internal(format!("render tilesheet png failed: {e}")))?;
        Ok(([(header::CONTENT_TYPE, "image/png")], png).into_response())
    }
}
