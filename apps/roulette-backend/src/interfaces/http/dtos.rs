//! HTTP request and response DTO schemas and error mappings.

#![forbid(unsafe_code)]

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use roulette_domain::{PlayerCommand, RoomMemberId, TabId};
use roulette_host::ServiceError;
use serde::{Deserialize, Serialize};

/// Query parameter holding tab identity.
#[derive(Deserialize)]
pub struct TabQuery {
    /// Local browser tab identifier.
    pub tab_id: TabId,
}

/// Request body holding tab identity.
#[derive(Deserialize)]
pub struct TabRequest {
    /// Local browser tab identifier.
    pub tab_id: TabId,
}

/// Request to create a new room.
#[derive(Deserialize)]
pub struct CreateRoomRequest {
    /// Creating tab identifier.
    pub tab_id: TabId,
    /// Room display name.
    pub room_name: String,
    /// Player display name.
    pub player_name: String,
    /// Optional room password.
    pub password: Option<String>,
}

/// Request to join an existing room.
#[derive(Deserialize)]
pub struct JoinRoomRequest {
    /// Joining tab identifier.
    pub tab_id: TabId,
    /// Target room identifier code.
    pub room_id: String,
    /// Player display name.
    pub player_name: String,
    /// Optional room password.
    pub password: Option<String>,
}

/// Request targeting a specific room member (e.g. kicking or transfer).
#[derive(Deserialize)]
pub struct MemberRequest {
    /// Requesting tab identifier.
    pub tab_id: TabId,
    /// Target member identifier.
    pub member_id: RoomMemberId,
}

/// Request to start a room match.
#[derive(Deserialize)]
pub struct StartRoomRequest {
    /// Requesting tab identifier (must be owner).
    pub tab_id: TabId,
    /// Optional deterministic random seed.
    pub seed: Option<u64>,
}

/// Request to submit an in-game command.
#[derive(Deserialize)]
pub struct CommandRequest {
    /// Requesting tab identifier.
    pub tab_id: TabId,
    /// Expected match revision for concurrency check.
    pub expected_revision: u64,
    /// Player action command to execute.
    pub command: PlayerCommand,
}

/// Request to step a bot in a room match.
#[derive(Deserialize)]
pub struct StepRequest {
    /// Requesting tab identifier.
    pub tab_id: TabId,
    /// Expected match revision.
    pub expected_revision: u64,
}

/// Request to authenticate as server founder.
#[derive(Deserialize)]
pub struct FounderAuthRequest {
    /// Requesting tab identifier.
    pub tab_id: TabId,
    /// Founder temporary password.
    pub password: String,
}

/// Request to update server settings.
#[derive(Deserialize)]
pub struct ServerSettingsRequest {
    /// Requesting tab identifier (must be authenticated founder).
    pub tab_id: TabId,
    /// Whether LAN exposure should be enabled.
    pub expose_lan: bool,
}

/// Direct JSON-RPC/MCP call request body.
#[derive(Deserialize)]
pub struct McpCallRequest {
    /// Tool name to invoke.
    pub tool: String,
    /// Tool invocation arguments.
    pub arguments: serde_json::Value,
}

/// Health check response.
#[derive(Serialize)]
pub struct HealthResponse {
    /// Health status string.
    pub status: &'static str,
}

/// Founder authentication response.
#[derive(Serialize)]
pub struct FounderAuthResponse {
    /// Whether authentication succeeded.
    pub authenticated: bool,
}

/// Generic error response payload.
#[derive(Serialize)]
pub struct ErrorResponse {
    /// Human-readable error message.
    pub error: String,
}

/// Query parameters for room image rendering.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct RoomImageQuery {
    /// Desired format: "png" (default) or "svg".
    pub format: Option<String>,
    /// Desired view mode: "full" (default complete HUD card) or "board" (standalone map).
    pub view_mode: Option<String>,
    /// Optional cell size in pixels for standalone board mode (default 96).
    pub cell_size: Option<u32>,
}

/// API error wrapper converting service errors to HTTP responses.
pub struct ApiError {
    /// HTTP status code.
    pub status: StatusCode,
    /// Error message.
    pub message: String,
}

impl ApiError {
    /// Constructs an internal server error with message.
    #[must_use]
    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: msg.into(),
        }
    }

    /// Maps a `ServiceError` into an `ApiError` with appropriate HTTP status code.
    #[must_use]
    pub fn from_service(error: &ServiceError) -> Self {
        let status = match error {
            ServiceError::NotFound(_) => StatusCode::NOT_FOUND,
            ServiceError::Forbidden(_) => StatusCode::FORBIDDEN,
            ServiceError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ServiceError::Conflict(_) => StatusCode::CONFLICT,
            ServiceError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        Self {
            status,
            message: error.to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorResponse {
                error: self.message,
            }),
        )
            .into_response()
    }
}
